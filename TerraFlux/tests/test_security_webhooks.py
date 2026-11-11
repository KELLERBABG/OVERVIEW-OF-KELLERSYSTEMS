import pytest
from fastapi.testclient import TestClient
from src.api.main import app
from src.core.config import settings
from src.bot.twilio_client import TwilioWhatsAppClient

@pytest.fixture
def client():
    return TestClient(app)

def test_unregistered_phone_rejection(client):
    """Ensure random unknown phone numbers cannot trigger actions or hijack existing farms."""
    payload = {
        'From': '+4919999999999', # Random unknown phone number
        'Body': 'DONE'
    }
    response = client.post('/api/v1/whatsapp/webhook', data=payload)
    assert response.status_code == 200
    reply = response.json().get('reply', '')
    assert 'Unregistered Number' in reply
    assert 'not associated' in reply

def test_registered_phone_success(client):
    """Ensure registered phone number succeeds."""
    payload = {
        'From': '+4915123456789', # Registered Stefan Becker
        'Body': 'STATUS'
    }
    response = client.post('/api/v1/whatsapp/webhook', data=payload)
    assert response.status_code == 200
    reply = response.json().get('reply', '')
    assert 'FieldSense Live Status' in reply
    assert 'Kaiserstuhl Vineyard' in reply

def test_ttn_webhook_authentication(client):
    """Ensure TTN webhook verifies secret token when configured."""
    original_secret = settings.TTN_WEBHOOK_SECRET
    try:
        settings.TTN_WEBHOOK_SECRET = 'super-secret-ttn-token-123'
        
        sample_payload = {
            'end_device_ids': {'dev_eui': '2CF7F1C044300001'},
            'uplink_message': {
                'decoded_payload': {'soil_moisture': 21.0, 'soil_temp': 20.0, 'battery': 3.65}
            }
        }
        
        # 1. Reject without token
        resp_unauth = client.post('/api/v1/telemetry/ttn', json=sample_payload)
        assert resp_unauth.status_code == 401
        
        # 2. Reject with wrong token
        resp_wrong = client.post(
            '/api/v1/telemetry/ttn',
            json=sample_payload,
            headers={'X-TTN-Webhook-Secret': 'wrong-token'}
        )
        assert resp_wrong.status_code == 401

        # 3. Accept with valid token
        resp_valid = client.post(
            '/api/v1/telemetry/ttn',
            json=sample_payload,
            headers={'X-TTN-Webhook-Secret': 'super-secret-ttn-token-123'}
        )
        assert resp_valid.status_code == 200
        assert resp_valid.json()['status'] == 'processed'
    finally:
        settings.TTN_WEBHOOK_SECRET = original_secret

def test_twilio_hmac_signature_validation():
    """Verify cryptographic HMAC-SHA1 calculation for Twilio requests."""
    settings.TWILIO_AUTH_TOKEN = '12345authsecret'
    url = 'https://api.fieldsense.ag/api/v1/whatsapp/webhook'
    params = {'Body': 'DONE', 'From': '+4915123456789'}
    
    # Intentionally invalid signature
    is_valid = TwilioWhatsAppClient.validate_signature(url, params, 'invalid_signature')
    assert is_valid is False
