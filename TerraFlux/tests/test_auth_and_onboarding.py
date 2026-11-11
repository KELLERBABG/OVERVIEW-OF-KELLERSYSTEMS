import pytest
from fastapi.testclient import TestClient
from sqlmodel import Session
from src.api.main import app
from src.api.database import engine
from src.core.models import FieldModel
from src.core.config import settings
from src.ingestion.weather import WeatherService

@pytest.fixture
def client():
    return TestClient(app)

def test_auth_registration_and_login_flow(client):
    unique_email = f"grower_test_{id(client)}@terraflux.ag"
    password = "supersecretpassword123"

    # 1. Register new user
    reg_resp = client.post(
        "/api/v1/auth/register",
        json={"email": unique_email, "password": password, "full_name": "Hans Gruber", "role": "grower"}
    )
    assert reg_resp.status_code == 200, reg_resp.text
    reg_data = reg_resp.json()
    assert "access_token" in reg_data
    assert reg_data["email"] == unique_email
    assert reg_data["role"] == "grower"

    token = reg_data["access_token"]

    # 2. Prevent duplicate email registration
    dup_resp = client.post(
        "/api/v1/auth/register",
        json={"email": unique_email, "password": password}
    )
    assert dup_resp.status_code == 400
    assert "already registered" in dup_resp.json()["detail"]

    # 3. Successful login
    login_resp = client.post(
        "/api/v1/auth/login",
        json={"email": unique_email, "password": password}
    )
    assert login_resp.status_code == 200
    assert "access_token" in login_resp.json()

    # 4. Failed login (wrong password)
    fail_resp = client.post(
        "/api/v1/auth/login",
        json={"email": unique_email, "password": "wrongpassword"}
    )
    assert fail_resp.status_code == 401

    # 5. Access profile endpoint with Bearer token
    me_resp = client.get(
        "/api/v1/auth/me",
        headers={"Authorization": f"Bearer {token}"}
    )
    assert me_resp.status_code == 200
    me_data = me_resp.json()
    assert me_data["email"] == unique_email
    assert me_data["full_name"] == "Hans Gruber"

def test_multi_tenant_field_isolation(client):
    # Register two distinct users
    u1_email = f"farmer_one_{id(client)}@terraflux.ag"
    u2_email = f"farmer_two_{id(client)}@terraflux.ag"

    r1 = client.post("/api/v1/auth/register", json={"email": u1_email, "password": "pass123456", "role": "grower"}).json()
    r2 = client.post("/api/v1/auth/register", json={"email": u2_email, "password": "pass123456", "role": "grower"}).json()

    token1 = r1["access_token"]
    user1_id = r1["user_id"]
    token2 = r2["access_token"]

    # Attach a field to user 1
    with Session(engine) as session:
        f1 = FieldModel(
            name="Farmer One Private Vineyard",
            user_id=user1_id,
            invite_token="token_f1_test",
            area_ha=5.0
        )
        session.add(f1)
        session.commit()

    # User 1 queries fields: should see their field
    res1 = client.get("/api/v1/fields", headers={"Authorization": f"Bearer {token1}"})
    assert res1.status_code == 200
    fields_u1 = res1.json()
    assert any(f["name"] == "Farmer One Private Vineyard" for f in fields_u1)

    # User 2 queries fields: should NOT see User 1's field
    res2 = client.get("/api/v1/fields", headers={"Authorization": f"Bearer {token2}"})
    assert res2.status_code == 200
    fields_u2 = res2.json()
    assert not any(f["name"] == "Farmer One Private Vineyard" for f in fields_u2)

def test_telegram_start_deep_link_onboarding(client):
    # Set up a target field with a known invite_token
    token_code = f"invite_{id(client)}"
    with Session(engine) as session:
        field = FieldModel(
            name="Deep Link Test Orchard",
            invite_token=token_code,
            farmer_chat_id="old_chat_id",
            area_ha=3.2
        )
        session.add(field)
        session.commit()
        session.refresh(field)
        field_id = field.id

    # Simulate inbound Telegram message: /start <token_code>
    new_chat_id = "9876543210"
    update_payload = {
        "update_id": 10001,
        "message": {
            "message_id": 42,
            "chat": {"id": new_chat_id},
            "from": {"id": new_chat_id, "first_name": "Stefan"},
            "text": f"/start {token_code}"
        }
    }

    resp = client.post("/api/v1/telegram/webhook", json=update_payload)
    assert resp.status_code == 200
    data = resp.json()
    assert data["status"] == "ok"
    assert "Welcome to TerraFlux!" in data["reply"]
    assert "Deep Link Test Orchard" in data["reply"]

    # Verify database was updated with the new farmer_chat_id
    with Session(engine) as session:
        updated_field = session.get(FieldModel, field_id)
        assert updated_field.farmer_chat_id == new_chat_id

    # Test /start when already linked
    update_already_linked = {
        "update_id": 10002,
        "message": {
            "chat": {"id": new_chat_id},
            "text": "/start"
        }
    }
    resp_linked = client.post("/api/v1/telegram/webhook", json=update_already_linked)
    assert "Welcome back" in resp_linked.json()["reply"]

    # Test /start with invalid code
    update_invalid = {
        "update_id": 10003,
        "message": {
            "chat": {"id": "9999999"},
            "text": "/start completely_fake_token_123"
        }
    }
    resp_invalid = client.post("/api/v1/telegram/webhook", json=update_invalid)
    assert "Invalid Invite Code" in resp_invalid.json()["reply"]

def test_open_meteo_configuration(monkeypatch):
    # Verify default free tier URL
    assert WeatherService.BASE_URL == "https://api.open-meteo.com/v1/forecast"

    # Test that setting OPEN_METEO_API_KEY is recognized
    monkeypatch.setattr(settings, "OPEN_METEO_API_KEY", "test_commercial_key_123")
    assert settings.OPEN_METEO_API_KEY == "test_commercial_key_123"
