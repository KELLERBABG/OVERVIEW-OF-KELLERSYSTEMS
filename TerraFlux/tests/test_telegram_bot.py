import pytest
from fastapi.testclient import TestClient
from src.api.main import app
from src.core.config import settings
from src.bot.telegram_client import TelegramBotClient

@pytest.fixture
def client():
    return TestClient(app)

def test_telegram_client_mock_dispatch():
    """Verify TelegramBotClient operates safely without real credentials in sandbox."""
    client = TelegramBotClient(token=None)
    res = client.send_message("123456789", "Test Advisory Message", reply_markup={"inline_keyboard": []})
    assert res["status"] == "simulated"
    assert res["mock"] is True
    assert res["chat_id"] == "123456789"

    cb_res = client.answer_callback_query("query_123", text="Acknowledged")
    assert cb_res["status"] == "simulated"

def test_telegram_webhook_incoming_message(client, demo_chat_id):
    """Verify standard message handling via /api/v1/telegram/webhook."""
    payload = {
        "update_id": 1001,
        "message": {
            "message_id": 501,
            "chat": {"id": demo_chat_id},
            "from": {"id": demo_chat_id, "first_name": "Stefan"},
            "text": "STATUS"
        }
    }
    response = client.post("/api/v1/telegram/webhook", json=payload)
    assert response.status_code == 200
    data = response.json()
    assert data["status"] == "ok"
    assert data["type"] == "message"
    assert "Live Status" in data["reply"]

def test_telegram_webhook_incoming_callback_done(client, demo_chat_id):
    """Verify inline button callback query handling (e.g. DONE button pressed)."""
    payload = {
        "update_id": 1002,
        "callback_query": {
            "id": "cb_query_999",
            "from": {"id": demo_chat_id, "first_name": "Stefan"},
            "message": {
                "message_id": 502,
                "chat": {"id": demo_chat_id}
            },
            "data": "DONE"
        }
    }
    response = client.post("/api/v1/telegram/webhook", json=payload)
    assert response.status_code == 200
    data = response.json()
    assert data["status"] == "ok"
    assert data["type"] == "callback_query"
    assert data["action"] == "DONE"
    assert "Logged" in data["reply"]

def test_telegram_webhook_secret_token_auth(client, demo_chat_id):
    """Verify secret token authentication header verification."""
    original_secret = settings.TELEGRAM_WEBHOOK_SECRET
    original_enforce = settings.ENFORCE_WEBHOOK_AUTH
    try:
        settings.TELEGRAM_WEBHOOK_SECRET = "secret-telegram-token-456"
        settings.ENFORCE_WEBHOOK_AUTH = True

        payload = {
            "update_id": 1003,
            "message": {
                "chat": {"id": demo_chat_id},
                "text": "HELP"
            }
        }

        # 1. Reject without token
        resp_unauth = client.post("/api/v1/telegram/webhook", json=payload)
        assert resp_unauth.status_code == 401

        # 2. Reject with invalid token
        resp_bad = client.post(
            "/api/v1/telegram/webhook",
            json=payload,
            headers={"X-Telegram-Bot-Api-Secret-Token": "wrong-token"}
        )
        assert resp_bad.status_code == 401

        # 3. Accept with valid token
        resp_ok = client.post(
            "/api/v1/telegram/webhook",
            json=payload,
            headers={"X-Telegram-Bot-Api-Secret-Token": "secret-telegram-token-456"}
        )
        assert resp_ok.status_code == 200
        assert "Commands" in resp_ok.json()["reply"]
    finally:
        settings.TELEGRAM_WEBHOOK_SECRET = original_secret
        settings.ENFORCE_WEBHOOK_AUTH = original_enforce
