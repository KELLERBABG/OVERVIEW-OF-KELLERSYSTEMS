"""Tests for the P1 authentication hardening: password policy, lockout, rate limits, revocation."""
import pytest
from sqlmodel import Session, select

from src.api.database import engine
from src.core.config import settings
from src.core.models import UserModel
from src.core.rate_limit import api_limiter, auth_limiter, webhook_limiter


@pytest.fixture(autouse=True)
def _reset_limiters():
    """The limiters are process-wide singletons - reset them between tests.

    (conftest.py already does this globally; kept explicit here because this module
    asserts on the limiter state directly.)
    """
    auth_limiter.clear()
    api_limiter.clear()
    webhook_limiter.clear()
    yield
    auth_limiter.clear()
    api_limiter.clear()
    webhook_limiter.clear()


def _register(client, email, password='growerpass123'):
    resp = client.post('/api/v1/auth/register', json={'email': email, 'password': password})
    assert resp.status_code == 200, resp.text
    return resp.json()


# --------------------------------------------------------------------------------------
# Password policy
# --------------------------------------------------------------------------------------

@pytest.mark.parametrize('weak', ['short1', 'nodigitshere', '12345678'])
def test_weak_passwords_are_rejected(client, weak):
    resp = client.post('/api/v1/auth/register', json={'email': f'weak_{weak}@terraflux.ag', 'password': weak})
    assert resp.status_code == 400
    assert 'password' in resp.json()['detail'].lower()


def test_strong_password_is_accepted(client):
    data = _register(client, 'strong_policy@terraflux.ag', 'orchard2026')
    assert data['role'] == 'grower'


# --------------------------------------------------------------------------------------
# Account lockout
# --------------------------------------------------------------------------------------

def test_account_is_locked_after_repeated_failures(client):
    email = 'lockout@terraflux.ag'
    _register(client, email, 'orchard2026')

    for _ in range(settings.LOGIN_MAX_FAILURES - 1):
        resp = client.post('/api/v1/auth/login', json={'email': email, 'password': 'wrongpass1'})
        assert resp.status_code == 401

    # The next failure trips the lockout
    resp = client.post('/api/v1/auth/login', json={'email': email, 'password': 'wrongpass1'})
    assert resp.status_code == 429
    assert 'locked' in resp.json()['detail'].lower()

    # Even the correct password is rejected while the lock is active
    resp = client.post('/api/v1/auth/login', json={'email': email, 'password': 'orchard2026'})
    assert resp.status_code == 429


def test_unknown_account_is_locked_as_well(client):
    """Unknown emails must be rate limited too, otherwise the endpoint enumerates users."""
    email = 'ghost@terraflux.ag'
    for _ in range(settings.LOGIN_MAX_FAILURES):
        client.post('/api/v1/auth/login', json={'email': email, 'password': 'whatever1'})
    resp = client.post('/api/v1/auth/login', json={'email': email, 'password': 'whatever1'})
    assert resp.status_code == 429


def test_successful_login_resets_the_failure_counter(client):
    email = 'reset@terraflux.ag'
    _register(client, email, 'orchard2026')
    client.post('/api/v1/auth/login', json={'email': email, 'password': 'wrongpass1'})
    resp = client.post('/api/v1/auth/login', json={'email': email, 'password': 'orchard2026'})
    assert resp.status_code == 200


# --------------------------------------------------------------------------------------
# Rate limiting
# --------------------------------------------------------------------------------------

def test_login_rate_limit_per_ip(client, monkeypatch):
    monkeypatch.setattr(settings, 'LOGIN_RATE_LIMIT_PER_MINUTE', 3)
    email = 'ratelimit@terraflux.ag'
    _register(client, email, 'orchard2026')
    for _ in range(3):
        client.post('/api/v1/auth/login', json={'email': email, 'password': 'orchard2026'})
    resp = client.post('/api/v1/auth/login', json={'email': email, 'password': 'orchard2026'})
    assert resp.status_code == 429
    assert 'retry' in resp.json()['detail'].lower()


def test_api_rate_limit_per_user(client, admin_headers, monkeypatch):
    monkeypatch.setattr(settings, 'API_RATE_LIMIT_PER_MINUTE', 3)
    for _ in range(3):
        assert client.get('/api/v1/fields', headers=admin_headers).status_code == 200
    resp = client.get('/api/v1/fields', headers=admin_headers)
    assert resp.status_code == 429


def test_webhook_rate_limit(client, monkeypatch):
    monkeypatch.setattr(settings, 'WEBHOOK_RATE_LIMIT_PER_MINUTE', 2)
    payload = {'From': '+4915123456789', 'Body': 'STATUS'}
    for _ in range(2):
        assert client.post('/api/v1/whatsapp/webhook', data=payload).status_code == 200
    resp = client.post('/api/v1/whatsapp/webhook', data=payload)
    assert resp.status_code == 429


# --------------------------------------------------------------------------------------
# Token revocation
# --------------------------------------------------------------------------------------

def test_logout_revokes_the_token(client):
    data = _register(client, 'revoke@terraflux.ag', 'orchard2026')
    headers = {'Authorization': f"Bearer {data['access_token']}"}

    assert client.get('/api/v1/auth/me', headers=headers).status_code == 200

    resp = client.post('/api/v1/auth/logout', headers=headers)
    assert resp.status_code == 200
    assert resp.json()['revoked_tokens'] is True

    # The very same token must no longer be accepted
    resp = client.get('/api/v1/auth/me', headers=headers)
    assert resp.status_code == 401
    assert 'revoked' in resp.json()['detail'].lower()

    # A fresh login issues a working token again
    relogin = client.post('/api/v1/auth/login', json={'email': 'revoke@terraflux.ag', 'password': 'orchard2026'})
    assert relogin.status_code == 200
    new_headers = {'Authorization': f"Bearer {relogin.json()['access_token']}"}
    assert client.get('/api/v1/auth/me', headers=new_headers).status_code == 200


def test_token_version_is_stored_and_incremented(client):
    data = _register(client, 'version@terraflux.ag', 'orchard2026')
    headers = {'Authorization': f"Bearer {data['access_token']}"}
    with Session(engine) as session:
        user = session.exec(select(UserModel).where(UserModel.email == 'version@terraflux.ag')).first()
        assert user.token_version == 1
    client.post('/api/v1/auth/logout', headers=headers)
    with Session(engine) as session:
        user = session.exec(select(UserModel).where(UserModel.email == 'version@terraflux.ag')).first()
        assert user.token_version == 2
