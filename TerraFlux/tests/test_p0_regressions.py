"""Regression tests for the P0 remediation items.

Covered:
  #1  DELAY / PAUSE must pause the field instead of logging a bogus irrigation event
  #2  Field-scoped endpoints require authentication and enforce tenant isolation
  #3  Public registration cannot grant the administrator role
  #4  The simulation playground no longer overwrites production probe telemetry
  #5  Seeded fields no longer share a single Telegram chat id
"""
import pytest
from sqlmodel import Session, select

from src.api.database import engine
from src.core.models import (
    FieldModel,
    IrrigationEventModel,
    TelemetryReadingModel,
    UserModel,
)

GROWER_PHONE = '+4915123456789'


def _event_count(field_id: int) -> int:
    with Session(engine) as session:
        return len(session.exec(
            select(IrrigationEventModel).where(IrrigationEventModel.field_id == field_id)
        ).all())


def _field_by_phone(phone: str):
    with Session(engine) as session:
        return session.exec(select(FieldModel).where(FieldModel.farmer_phone == phone)).first()


# --------------------------------------------------------------------------------------
# #1 Command routing: DELAY / PAUSE vs WATER
# --------------------------------------------------------------------------------------

def test_delay_command_pauses_field_without_logging_irrigation(client):
    field = _field_by_phone(GROWER_PHONE)
    before_events = _event_count(field.id)

    resp = client.post('/api/v1/whatsapp/webhook', data={'From': GROWER_PHONE, 'Body': 'DELAY 1D'})
    assert resp.status_code == 200
    assert 'postponed' in resp.json()['reply'].lower()

    # No phantom irrigation event may be created
    assert _event_count(field.id) == before_events

    refreshed = _field_by_phone(GROWER_PHONE)
    assert refreshed.paused_until is not None

    # Cleanup so later tests are not affected by the pause
    with Session(engine) as session:
        f = session.get(FieldModel, field.id)
        f.paused_until = None
        session.add(f)
        session.commit()


def test_pause_command_pauses_field_without_logging_irrigation(client):
    field = _field_by_phone(GROWER_PHONE)
    before_events = _event_count(field.id)

    resp = client.post('/api/v1/whatsapp/webhook', data={'From': GROWER_PHONE, 'Body': 'PAUSE 3D'})
    assert resp.status_code == 200
    assert 'postponed' in resp.json()['reply'].lower()
    assert _event_count(field.id) == before_events

    refreshed = _field_by_phone(GROWER_PHONE)
    assert refreshed.paused_until is not None

    with Session(engine) as session:
        f = session.get(FieldModel, field.id)
        f.paused_until = None
        session.add(f)
        session.commit()


def test_explicit_water_command_still_logs_irrigation(client):
    """A grower's custom depth is GROSS; only the efficient share reaches the root zone."""
    field = _field_by_phone(GROWER_PHONE)
    before_events = _event_count(field.id)

    resp = client.post('/api/v1/whatsapp/webhook', data={'From': GROWER_PHONE, 'Body': 'WATER 15MM'})
    assert resp.status_code == 200
    # 15 mm gross at 90% application efficiency = 13.5 mm net
    assert '13.5mm' in resp.json()['reply']

    assert _event_count(field.id) == before_events + 1

    with Session(engine) as session:
        event = session.exec(
            select(IrrigationEventModel)
            .where(IrrigationEventModel.field_id == field.id)
            .order_by(IrrigationEventModel.timestamp.desc())
        ).first()
        assert event.depth_applied_mm == 13.5
        assert 'gross 15.0mm' in event.notes


def test_bare_number_is_not_treated_as_a_watering_depth(client):
    field = _field_by_phone(GROWER_PHONE)
    before_events = _event_count(field.id)

    resp = client.post('/api/v1/whatsapp/webhook', data={'From': GROWER_PHONE, 'Body': '15'})
    assert resp.status_code == 200
    assert _event_count(field.id) == before_events


# --------------------------------------------------------------------------------------
# #2 Authentication & tenant isolation
# --------------------------------------------------------------------------------------

@pytest.mark.parametrize('method,path', [
    ('get', '/api/v1/fields'),
    ('get', '/api/v1/fields/1/telemetry'),
    ('post', '/api/v1/advisory/evaluate/1'),
    ('post', '/api/v1/simulation/run?field_id=1&days=2&scenario=drought'),
])
def test_field_scoped_endpoints_require_authentication(client, method, path):
    resp = getattr(client, method)(path)
    assert resp.status_code == 401


def test_grower_cannot_access_another_tenants_field(client, grower_headers):
    with Session(engine) as session:
        other = FieldModel(name='Foreign Tenant Vineyard', user_id=None, area_ha=4.0)
        session.add(other)
        session.commit()
        session.refresh(other)
        other_id = other.id

    resp = client.get(f'/api/v1/fields/{other_id}/telemetry', headers=grower_headers)
    assert resp.status_code == 403

    resp = client.post(f'/api/v1/advisory/evaluate/{other_id}', headers=grower_headers)
    assert resp.status_code == 403

    resp = client.post(f'/api/v1/simulation/run?field_id={other_id}&days=2', headers=grower_headers)
    assert resp.status_code == 403


def test_admin_can_read_every_field(client, admin_headers):
    resp = client.get('/api/v1/fields', headers=admin_headers)
    assert resp.status_code == 200
    assert len(resp.json()) >= 2


# --------------------------------------------------------------------------------------
# #3 Privilege escalation
# --------------------------------------------------------------------------------------

def test_registration_cannot_grant_admin_role(client):
    resp = client.post(
        '/api/v1/auth/register',
        json={'email': 'escalation_probe@terraflux.ag', 'password': 'secret1234', 'role': 'admin'}
    )
    assert resp.status_code == 200
    assert resp.json()['role'] == 'grower'

    with Session(engine) as session:
        user = session.exec(
            select(UserModel).where(UserModel.email == 'escalation_probe@terraflux.ag')
        ).first()
        assert user.role == 'grower'


def test_role_management_requires_an_administrator(client, grower_headers, admin_headers):
    with Session(engine) as session:
        target = session.exec(
            select(UserModel).where(UserModel.email == 'escalation_probe@terraflux.ag')
        ).first()
        target_id = target.id

    # A grower may not promote anybody
    resp = client.post(f'/api/v1/admin/users/{target_id}/role', json={'role': 'admin'}, headers=grower_headers)
    assert resp.status_code == 403

    # An administrator may
    resp = client.post(f'/api/v1/admin/users/{target_id}/role', json={'role': 'admin'}, headers=admin_headers)
    assert resp.status_code == 200
    assert resp.json()['role'] == 'admin'

    resp = client.post(f'/api/v1/admin/users/{target_id}/role', json={'role': 'grower'}, headers=admin_headers)
    assert resp.status_code == 200
    assert resp.json()['role'] == 'grower'


# --------------------------------------------------------------------------------------
# #4 Simulation must not corrupt production telemetry
# --------------------------------------------------------------------------------------

def test_simulation_does_not_persist_by_default(client, admin_headers, telemetry_row_count, sensor_moisture):
    before_rows = telemetry_row_count()
    before_moisture = sensor_moisture(1)

    resp = client.post('/api/v1/simulation/run?field_id=1&days=3&scenario=drought', headers=admin_headers)
    assert resp.status_code == 200
    assert len(resp.json()['timeline']) > 0

    assert telemetry_row_count() == before_rows
    assert sensor_moisture(1) == before_moisture


def test_simulation_can_persist_when_explicitly_requested(client, admin_headers, telemetry_row_count):
    before_rows = telemetry_row_count()

    resp = client.post(
        '/api/v1/simulation/run?field_id=1&days=3&scenario=drought&persist=true',
        headers=admin_headers
    )
    assert resp.status_code == 200
    assert telemetry_row_count() > before_rows

    with Session(engine) as session:
        simulated = session.exec(
            select(TelemetryReadingModel).where(TelemetryReadingModel.source == 'simulation')
        ).all()
        assert len(simulated) > 0


# --------------------------------------------------------------------------------------
# #5 Telegram chat ids must be unique per field
# --------------------------------------------------------------------------------------

def test_seeded_fields_have_distinct_telegram_chat_ids():
    with Session(engine) as session:
        fields = session.exec(select(FieldModel)).all()
        chat_ids = [f.farmer_chat_id for f in fields if f.farmer_chat_id]
        assert len(set(chat_ids)) == len(chat_ids)

        demo_fields = [f for f in fields if 'Kaiserstuhl' in f.name or 'Wachau' in f.name]
        assert len(demo_fields) == 2
        assert all(f.farmer_chat_id for f in demo_fields)
