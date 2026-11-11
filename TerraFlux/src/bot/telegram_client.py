"""Telegram Bot API Integration Client for TerraFlux."""
import httpx
from typing import Optional, Dict, Any
from ..core.config import settings
from ..core.logger import logger

class TelegramBotClient:
    """Production Telegram Bot API client with simulated sandbox fallback."""

    def __init__(self, token: Optional[str] = None):
        self.token = token or settings.TELEGRAM_BOT_TOKEN
        self.base_url = f"https://api.telegram.org/bot{self.token}" if self.token else None

    def send_message(
        self,
        chat_id: str | int,
        text: str,
        reply_markup: Optional[Dict[str, Any]] = None,
        parse_mode: str = "HTML"
    ) -> Dict[str, Any]:
        """Send message via Telegram Bot API or log in simulated mock mode."""
        str_chat_id = str(chat_id)

        if not self.token:
            logger.info(f"[SIMULATED TELEGRAM] ChatID: {str_chat_id} | Text: {text[:60]}...")
            return {
                "status": "simulated",
                "chat_id": str_chat_id,
                "text": text,
                "reply_markup": reply_markup,
                "mock": True
            }

        url = f"{self.base_url}/sendMessage"
        payload: Dict[str, Any] = {
            "chat_id": str_chat_id,
            "text": text,
            "parse_mode": parse_mode
        }
        if reply_markup:
            payload["reply_markup"] = reply_markup

        try:
            with httpx.Client(timeout=8.0) as client:
                resp = client.post(url, json=payload)
                if resp.status_code == 200:
                    data = resp.json()
                    msg_id = data.get("result", {}).get("message_id")
                    logger.info(f"[TELEGRAM DISPATCH SUCCESS] Msg ID: {msg_id} -> Chat: {str_chat_id}")
                    return {"status": "sent", "message_id": msg_id, "mock": False}
                logger.error(f"[TELEGRAM ERROR] {resp.status_code}: {resp.text}")
                return {"status": "error", "code": resp.status_code, "detail": resp.text, "mock": False}
        except Exception as e:
            logger.exception(f"[TELEGRAM EXCEPTION] {e}")
            return {"status": "exception", "error": str(e), "mock": True}

    def answer_callback_query(
        self,
        callback_query_id: str,
        text: Optional[str] = None,
        show_alert: bool = False
    ) -> Dict[str, Any]:
        """Acknowledge an incoming inline keyboard button press."""
        if not self.token:
            return {"status": "simulated", "callback_query_id": callback_query_id, "mock": True}

        url = f"{self.base_url}/answerCallbackQuery"
        payload: Dict[str, Any] = {"callback_query_id": callback_query_id, "show_alert": show_alert}
        if text:
            payload["text"] = text

        try:
            with httpx.Client(timeout=5.0) as client:
                resp = client.post(url, json=payload)
                return {"status": "ok" if resp.status_code == 200 else "error"}
        except Exception as e:
            logger.warning(f"[TELEGRAM CALLBACK ANSWER ERROR] {e}")
            return {"status": "exception", "error": str(e)}

    def set_webhook(self, webhook_url: str, secret_token: Optional[str] = None) -> Dict[str, Any]:
        """Configure the Telegram webhook for real-time inbound updates."""
        if not self.token:
            return {"status": "simulated", "url": webhook_url, "mock": True}

        url = f"{self.base_url}/setWebhook"
        payload: Dict[str, Any] = {"url": webhook_url}
        if secret_token or settings.TELEGRAM_WEBHOOK_SECRET:
            payload["secret_token"] = secret_token or settings.TELEGRAM_WEBHOOK_SECRET

        with httpx.Client(timeout=10.0) as client:
            resp = client.post(url, json=payload)
            return resp.json()
