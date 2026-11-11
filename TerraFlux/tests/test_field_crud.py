"""Tests for the field / probe lifecycle API (create, update, zones, binding, removal)."""
import pytest
from fastapi.testclient import TestClient
from sqlmodel import Session, select

from src.api.main import app
from src.api.database import engine
from src.core.models import SensorModel, TelemetryReadingModel


@pytest.fixture()
def grower_client(grower_headers):
    """A client that authenticates every request as the grower."""
    client = TestClient(app)
    client.headers.update(grower_headers)
    return client


def test_create_field_assigns_the_caller(grower_client):
    resp = grower_client.post(
        '/api/v1/fields',
        json={
            'name': 'New Block',
            'area_ha': 3.5,
            'crop_key': 'apple',
            'growth_stage': 'mid_season',
            'soil_key': 'silt_loam',
            'timezone': 'Europe/Vienna',
            'daily_alert_hour': 6
        }
    )
    assert resp.status_code == 201, resp.text
    data = resp.json()
    assert data['name'] == 'New Block'
    assert data['invite_token']
    assert 't.me/TerraFluxBot' in data['telegram_deep_link']


def test_create_field_rejects_unknown_agronomy(grower_client):
    resp = grower_client.post('/api/v1/fields', json={'name': 'Bad Crop', 'crop_key': 'banana'})
    assert resp.status_code == 400
    assert 'crop' in resp.json()['detail'].lower()

    resp = grower_client.post('/api/v1/fields', json={'name': 'Bad Stage', 'crop_key': 'grapevine', 'growth_stage': 'winter'})
    assert resp.status_code == 400

    resp = grower_client.post('/api/v1/fields', json={'name': 'Bad Soil', 'soil_key': 'lava'})
    assert resp.status_code == 400


def test_create_field_requires_authentication(client):
    resp = client.post('/api/v1/fields', json={'name': 'Anonymous'})
    assert resp.status_code == 401


def test_update_and_deactivate_field(grower_client):
    created = grower_client.post('/api/v1/fields', json={'name': 'Editable Block', 'area_ha': 2.0}).json()
    field_id = created['id']

    resp = grower_client.patch(f'/api/v1/fields/{field_id}', json={'area_ha': 6.5, 'daily_alert_hour': 5})
    assert resp.status_code == 200
    assert 'area_ha' in resp.json()['updated']

    resp = grower_client.get(f'/api/v1/fields/{field_id}')
    assert resp.status_code == 200
    assert resp.json()['area_ha'] == 6.5
    assert resp.json()['daily_alert_hour'] == 5

    resp = grower_client.delete(f'/api/v1/fields/{field_id}')
    assert resp.status_code == 200
    assert resp.json()['active'] is False

    # A deactivated block no longer shows up in the listing
    resp = grower_client.get('/api/v1/fields')
    assert all(f['id'] != field_id for f in resp.json())


def test_update_rejects_invalid_efficiency(grower_client):
    created = grower_client.post('/api/v1/fields', json={'name': 'Efficiency Block'}).json()
    resp = grower_client.patch(f"/api/v1/fields/{created['id']}", json={'irrigation_efficiency': 3.0})
    assert resp.status_code == 400


def test_probe_registration_and_zone_mapping(grower_client):
    created = grower_client.post('/api/v1/fields', json={'name': 'Zoned Block'}).json()
    field_id = created['id']

    for zone, deveui in (('A1', 'TESTSENSOR0001'), ('A2', 'TESTSENSOR0002')):
        resp = grower_client.post(
            f'/api/v1/fields/{field_id}/sensors',
            json={'deveui': deveui, 'name': f'Probe {zone}', 'zone': zone}
        )
        assert resp.status_code == 201, resp.text
        assert resp.json()['zone'] == zone

    resp = grower_client.get(f'/api/v1/fields/{field_id}/sensors')
    assert resp.status_code == 200
    zones = resp.json()['zones']
    assert [z['zone'] for z in zones] == ['A1', 'A2']
    assert all('threshold_pct' in z for z in zones)

    # The field detail exposes the same zone map
    resp = grower_client.get(f'/api/v1/fields/{field_id}')
    assert len(resp.json()['zones']) == 2


def test_duplicate_deveui_is_rejected(grower_client):
    created = grower_client.post('/api/v1/fields', json={'name': 'Duplicate Probe Block'}).json()
    payload = {'deveui': 'DUPLICATEPROBE1', 'zone': 'A1'}
    assert grower_client.post(f"/api/v1/fields/{created['id']}/sensors", json=payload).status_code == 201
    resp = grower_client.post(f"/api/v1/fields/{created['id']}/sensors", json=payload)
    assert resp.status_code == 409


def test_orphan_probe_can_be_bound(grower_client):
    """A probe auto-registered by a TTN uplink (no field) can be bound later."""
    created = grower_client.post('/api/v1/fields', json={'name': 'Binding Target'}).json()

    # Simulate the TTN auto-registration path
    with Session(engine) as session:
        orphan = SensorModel(deveui='ORPHANPROBE01', name='Orphan Probe')
        session.add(orphan)
        session.commit()
        session.refresh(orphan)
        orphan_id = orphan.id

    resp = grower_client.post(f'/api/v1/sensors/{orphan_id}/bind', json={'field_id': created['id'], 'zone': 'B2'})
    assert resp.status_code == 200
    assert resp.json()['field_id'] == created['id']
    assert resp.json()['zone'] == 'B2'


def test_probe_removal_deletes_its_telemetry(grower_client):
    created = grower_client.post('/api/v1/fields', json={'name': 'Removal Block'}).json()
    sensor = grower_client.post(
        f"/api/v1/fields/{created['id']}/sensors",
        json={'deveui': 'REMOVABLEPROBE1'}
    ).json()

    with Session(engine) as session:
        session.add(TelemetryReadingModel(
            sensor_id=sensor['id'], soil_moisture_pct=20.0, soil_temp_c=19.0, battery_volts=3.6
        ))
        session.commit()

    resp = grower_client.delete(f"/api/v1/sensors/{sensor['id']}")
    assert resp.status_code == 200
    assert resp.json()['status'] == 'removed'

    with Session(engine) as session:
        assert session.get(SensorModel, sensor['id']) is None
        readings = session.exec(
            select(TelemetryReadingModel).where(TelemetryReadingModel.sensor_id == sensor['id'])
        ).all()
        assert readings == []


def test_grower_cannot_touch_another_tenants_field(grower_client):
    created = grower_client.post('/api/v1/fields', json={'name': 'Owned By Someone Else'}).json()
    field_id = created['id']

    # The owner can see it
    assert grower_client.get(f'/api/v1/fields/{field_id}').status_code == 200

    # Another tenant cannot
    other = grower_client.post('/api/v1/auth/register', json={
        'email': f'intruder_{id(created)}@terraflux.ag', 'password': 'intruder2026'
    }).json()
    other_client = TestClient(app)
    other_client.headers.update({'Authorization': f"Bearer {other['access_token']}"})

    assert other_client.get(f'/api/v1/fields/{field_id}').status_code == 403
    assert other_client.patch(f'/api/v1/fields/{field_id}', json={'area_ha': 1}).status_code == 403
    assert other_client.delete(f'/api/v1/fields/{field_id}').status_code == 403
    assert other_client.post(f'/api/v1/fields/{field_id}/sensors', json={'deveui': 'SNEAKY00001'}).status_code == 403
