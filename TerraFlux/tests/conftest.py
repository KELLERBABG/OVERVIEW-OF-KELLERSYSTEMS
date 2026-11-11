"""Shared pytest fixtures.

Two guarantees provided here:

1. **Database isolation** - the application creates and seeds its database at import
   time (``src/api/main.py``), so the test suite runs against a throwaway SQLite file
   in the system temp directory instead of the developer's real ``fieldsense.db``.
2. **Authenticated clients** - the API is tenant-scoped, so most tests need a bearer
   token. ``admin_headers`` and ``grower_headers`` provide ready-made ones.

The environment must be configured *before* ``src.api.main`` is imported, hence the
module-level statements above the imports.
"""
import os
import pathlib
import tempfile
import uuid

_TMP_DB = pathlib.Path(tempfile.gettempdir()) / 'terraflux_pytest.db'
for _suffix in ('', '-wal', '-shm'):
    _stale = pathlib.Path(str(_TMP_DB) + _suffix)
    if _stale.exists():
        _stale.unlink()

os.environ['DATABASE_URL'] = f'sqlite:///{_TMP_DB}'
os.environ['AUTO_SCHEDULER_ENABLED'] = 'false'
os.environ['ENVIRONMENT'] = 'development'
os.environ['LOG_LEVEL'] = 'WARNING'

import pytest  # noqa: E402
import httpx  # noqa: E402
from fastapi.testclient import TestClient  # noqa: E402
from sqlmodel import Session, select  # noqa: E402

from src.api.main import app  # noqa: E402
from src.api.database import create_db_and_tables, engine  # noqa: E402
from src.core.models import FieldModel, SensorModel, TelemetryReadingModel  # noqa: E402

ADMIN_EMAIL = 'admin@terraflux.ag'
ADMIN_PASSWORD = 'kaiserstuhl2026'

# Deterministic Open-Meteo payload. Day 0 = today, day 1 = tomorrow, day 2 = +2 days.
OPEN_METEO_PAYLOAD = {
    'daily': {
        'et0_fao_evapotranspiration': [4.5, 4.8, 4.2],
        'precipitation_sum': [0.0, 0.0, 0.0],
        'temperature_2m_max': [26.0, 28.0, 27.0],
        'shortwave_radiation_sum': [22.0, 24.0, 21.0],
    }
}


def _mock_handler(request: httpx.Request) -> httpx.Response:
    """Serve the canned weather payload and block every other outbound call."""
    url = str(request.url)
    if 'open-meteo.com' in url:
        return httpx.Response(200, json=OPEN_METEO_PAYLOAD)
    raise httpx.ConnectError(f'Outbound network access is blocked during tests: {url}')


@pytest.fixture(scope='session', autouse=True)
def _block_network():
    """Replace httpx.Client with a mock transport.

    The test suite must not depend on Open-Meteo (or any other host) being reachable:
    it makes the results deterministic, keeps CI fast and offline-safe.
    """
    real_client = httpx.Client

    def mocked_client(*args, **kwargs):
        kwargs.setdefault('transport', httpx.MockTransport(_mock_handler))
        return real_client(*args, **kwargs)

    patcher = pytest.MonkeyPatch()
    patcher.setattr(httpx, 'Client', mocked_client)
    yield
    patcher.undo()


@pytest.fixture(scope='session', autouse=True)
def _isolated_database():
    """Ensure the throwaway database exists and is seeded exactly once per session."""
    create_db_and_tables()
    yield


@pytest.fixture(autouse=True)
def _reset_rate_limiters():
    """Rate limiters are process-wide singletons - reset them between tests."""
    from src.core.rate_limit import api_limiter, auth_limiter, webhook_limiter
    for limiter in (auth_limiter, api_limiter, webhook_limiter):
        limiter.clear()
    yield
    for limiter in (auth_limiter, api_limiter, webhook_limiter):
        limiter.clear()


@pytest.fixture()
def client():
    return TestClient(app)


@pytest.fixture()
def admin_token(client):
    resp = client.post('/api/v1/auth/login', json={'email': ADMIN_EMAIL, 'password': ADMIN_PASSWORD})
    assert resp.status_code == 200, resp.text
    return resp.json()['access_token']


@pytest.fixture()
def admin_headers(admin_token):
    return {'Authorization': f'Bearer {admin_token}'}


@pytest.fixture()
def grower_headers(client):
    """Register a fresh grower and return its bearer header."""
    email = f'grower_{uuid.uuid4().hex[:10]}@terraflux.ag'
    resp = client.post(
        '/api/v1/auth/register',
        json={'email': email, 'password': 'growerpass123', 'full_name': 'Test Grower'}
    )
    assert resp.status_code == 200, resp.text
    return {'Authorization': f"Bearer {resp.json()['access_token']}"}


@pytest.fixture()
def first_field_id():
    with Session(engine) as session:
        field = session.exec(
            select(FieldModel).where(FieldModel.active == True).order_by(FieldModel.id)  # noqa: E712
        ).first()
        assert field is not None
        return field.id


@pytest.fixture()
def demo_chat_id():
    """Telegram chat_id of the first seeded demo field (used for bot webhook tests)."""
    with Session(engine) as session:
        field = session.exec(
            select(FieldModel)
            .where(FieldModel.farmer_chat_id.is_not(None))
            .order_by(FieldModel.id)
        ).first()
        assert field is not None, 'no field with a Telegram chat id in the test database'
        return field.farmer_chat_id


@pytest.fixture()
def telemetry_row_count():
    def _count():
        with Session(engine) as session:
            return len(session.exec(select(TelemetryReadingModel)).all())
    return _count


@pytest.fixture()
def sensor_moisture():
    def _read(field_id: int):
        with Session(engine) as session:
            sensor = session.exec(select(SensorModel).where(SensorModel.field_id == field_id)).first()
            return sensor.last_moisture_pct if sensor else None
    return _read
