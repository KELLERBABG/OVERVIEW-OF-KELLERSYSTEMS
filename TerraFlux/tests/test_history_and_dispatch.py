"""Tests for the operational history endpoints and the dispatch audit trail."""
import pytest
from fastapi.testclient import TestClient
from sqlmodel import Session, select

from src.api.main import app
from src.api.database import engine
from src.bot.dispatch import AdvisoryDispatcher
from src.core.models import DispatchLogModel, FieldModel


@pytest.fixture()
def admin_client(admin_headers):
    client = TestClient(app)
    client.headers.update(admin_headers)
    return client


def _first_field_id():
    with Session(engine) as session:
        field = session.exec(select(FieldModel).where(FieldModel.active == True).order_by(FieldModel.id)).first()  # noqa: E712
        return field.id


def _dispatch_rows(field_id):
    with Session(engine) as session:
        return session.exec(
            select(DispatchLogModel).where(DispatchLogModel.field_id == field_id)
            .order_by(DispatchLogModel.created_at.desc())
        ).all()


def test_advisory_evaluation_is_recorded_in_the_history(admin_client):
    field_id = _first_field_id()
    resp = admin_client.post(f'/api/v1/advisory/evaluate/{field_id}')
    assert resp.status_code == 200, resp.text
    decision = resp.json()['decision']

    resp = admin_client.get(f'/api/v1/fields/{field_id}/advisories')
    assert resp.status_code == 200
    advisories = resp.json()['advisories']
    assert len(advisories) >= 1
    latest = advisories[0]
    assert latest['net_depth_mm'] == decision['target_depth_mm']
    assert latest['gross_depth_mm'] == decision['gross_depth_mm']
    assert latest['status'] in ('PENDING', 'AUTO_LOGGED')


def test_every_dispatch_attempt_is_persisted(admin_client):
    field_id = _first_field_id()
    admin_client.post(f'/api/v1/advisory/evaluate/{field_id}')

    rows = _dispatch_rows(field_id)
    channels = {r.channel for r in rows}
    assert 'telegram' in channels
    assert 'whatsapp' in channels
    for row in rows:
        assert row.status in ('sent', 'simulated', 'failed', 'skipped', 'error', 'exception')
        assert row.attempts >= 1

    resp = admin_client.get(f'/api/v1/fields/{field_id}/dispatches')
    assert resp.status_code == 200
    assert len(resp.json()['dispatches']) >= 2


def test_missing_channel_is_logged_as_skipped(admin_client):
    """A block without a Telegram chat id must log a skip, never a silent success."""
    created = admin_client.post('/api/v1/fields', json={'name': 'No Chat Id Block'}).json()
    field_id = created['id']

    admin_client.post(f'/api/v1/advisory/evaluate/{field_id}')

    rows = _dispatch_rows(field_id)
    telegram_rows = [r for r in rows if r.channel == 'telegram']
    assert len(telegram_rows) == 1
    assert telegram_rows[0].status == 'skipped'
    assert 'chat_id' in (telegram_rows[0].detail or '')


def test_failed_delivery_is_retried_and_recorded(admin_client, monkeypatch):
    created = admin_client.post('/api/v1/fields', json={
        'name': 'Failing Delivery Block',
        'farmer_chat_id': '999888777'
    }).json()
    field_id = created['id']

    attempts = {'count': 0}

    def failing_send(self, chat_id, text, reply_markup=None, parse_mode='HTML'):
        attempts['count'] += 1
        return {'status': 'error', 'code': 400, 'detail': 'chat not found'}

    monkeypatch.setattr('src.bot.dispatch.TelegramBotClient.send_message', failing_send)
    monkeypatch.setattr(AdvisoryDispatcher, 'RETRY_BACKOFF_SECONDS', 0)

    resp = admin_client.post(f'/api/v1/advisory/evaluate/{field_id}')
    assert resp.status_code == 200
    dispatch = resp.json()['dispatch']
    assert dispatch['failures'] >= 1
    assert dispatch['delivered'] is False
    assert attempts['count'] == AdvisoryDispatcher.MAX_ATTEMPTS

    telegram_rows = [r for r in _dispatch_rows(field_id) if r.channel == 'telegram']
    assert telegram_rows[0].status in ('error', 'exception', 'failed')
    assert telegram_rows[0].attempts == AdvisoryDispatcher.MAX_ATTEMPTS
    assert 'chat not found' in (telegram_rows[0].detail or '')


def test_transient_failure_is_retried_successfully(monkeypatch):
    calls = {'count': 0}

    def flaky_send():
        calls['count'] += 1
        if calls['count'] == 1:
            return {'status': 'error', 'detail': 'timeout'}
        return {'status': 'sent', 'message_id': 42}

    monkeypatch.setattr(AdvisoryDispatcher, 'RETRY_BACKOFF_SECONDS', 0)

    outcome, attempts = AdvisoryDispatcher._send_with_retry(flaky_send)
    assert outcome['status'] == 'sent'
    assert attempts == 2
    assert calls['count'] == 2


def test_irrigation_history_records_confirmed_watering(admin_client):
    field_id = _first_field_id()
    client = TestClient(app)
    resp = client.post('/api/v1/whatsapp/webhook', data={'From': '+4915123456789', 'Body': 'DONE'})
    assert resp.status_code == 200

    resp = admin_client.get(f'/api/v1/fields/{field_id}/irrigations')
    assert resp.status_code == 200
    irrigations = resp.json()['irrigations']
    assert len(irrigations) >= 1
    assert irrigations[0]['net_depth_mm'] > 0
    assert irrigations[0]['source'] == 'whatsapp_reply'
    assert 'net' in (irrigations[0]['notes'] or '')


def test_combined_history_endpoint(admin_client):
    field_id = _first_field_id()
    admin_client.post(f'/api/v1/advisory/evaluate/{field_id}')

    resp = admin_client.get(f'/api/v1/fields/{field_id}/history')
    assert resp.status_code == 200
    data = resp.json()
    assert data['field_id'] == field_id
    assert data['summary']['advisories'] >= 1
    assert data['summary']['dispatches'] >= 1
    assert 'failed_dispatches' in data['summary']
    assert 'applied_net_depth_mm' in data['summary']


def test_history_endpoints_require_authentication(client):
    field_id = _first_field_id()
    for path in (f'/api/v1/fields/{field_id}/advisories',
                 f'/api/v1/fields/{field_id}/irrigations',
                 f'/api/v1/fields/{field_id}/dispatches',
                 f'/api/v1/fields/{field_id}/history'):
        assert client.get(path).status_code == 401


def test_history_is_tenant_scoped(admin_client):
    created = admin_client.post('/api/v1/fields', json={'name': 'Private History Block'}).json()
    field_id = created['id']

    other = admin_client.post('/api/v1/auth/register', json={
        'email': f'history_{id(created)}@terraflux.ag', 'password': 'history2026'
    }).json()
    other_client = TestClient(app)
    other_client.headers.update({'Authorization': f"Bearer {other['access_token']}"})

    assert other_client.get(f'/api/v1/fields/{field_id}/history').status_code == 403
    assert other_client.get(f'/api/v1/fields/{field_id}/advisories').status_code == 403
    assert other_client.get(f'/api/v1/fields/{field_id}/dispatches').status_code == 403
