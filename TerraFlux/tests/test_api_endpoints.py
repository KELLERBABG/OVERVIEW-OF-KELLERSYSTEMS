import pytest
from fastapi.testclient import TestClient
from src.api.main import app

@pytest.fixture
def client():
    return TestClient(app)

def test_health_and_readiness_probes(client):
    resp_health = client.get('/healthz')
    assert resp_health.status_code == 200
    assert resp_health.json()['status'] == 'healthy'
    assert 'X-Request-ID' in resp_health.headers

    resp_ready = client.get('/readyz')
    assert resp_ready.status_code == 200
    assert resp_ready.json()['status'] == 'ready'

def test_list_fields_pagination(client, admin_headers):
    response = client.get('/api/v1/fields?limit=1&offset=0', headers=admin_headers)
    assert response.status_code == 200
    fields = response.json()
    assert len(fields) == 1
    assert 'soil_status' in fields[0]

def test_advisory_evaluation(client, admin_headers):
    response = client.post('/api/v1/advisory/evaluate/1', headers=admin_headers)
    assert response.status_code == 200
    data = response.json()
    assert 'decision' in data
    assert 'whatsapp_message' in data

def test_whatsapp_webhook_incoming_done(client):
    response = client.post('/api/v1/whatsapp/webhook', data={'From': '+4915123456789', 'Body': 'DONE'})
    assert response.status_code == 200
    assert 'reply' in response.json()
    assert 'Logged' in response.json()['reply']

def test_simulation_run(client, admin_headers):
    response = client.post('/api/v1/simulation/run?field_id=1&days=7&scenario=drought', headers=admin_headers)
    assert response.status_code == 200
    data = response.json()
    assert len(data['timeline']) > 0
    assert 'final_status' in data

def test_economics_evaluate(client):
    response = client.post('/api/v1/economics/evaluate?farm_area_ha=25&probes_count=4&water_tariff_eur_m3=1.50')
    assert response.status_code == 200
    data = response.json()
    assert data['payback_period_months'] > 0
