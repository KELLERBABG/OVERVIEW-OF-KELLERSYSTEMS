"""Twilio WhatsApp API Integration Client and Cryptographic Signature Validator."""
import hmac
import hashlib
import base64
import httpx
from typing import Optional, Dict, Any
from ..core.config import settings
from ..core.logger import logger

class TwilioWhatsAppClient:
    """Production Twilio WhatsApp API client with cryptographic verification."""

    def __init__(
        self,
        account_sid: Optional[str] = None,
        auth_token: Optional[str] = None,
        from_whatsapp_number: Optional[str] = None
    ):
        self.account_sid = account_sid or settings.TWILIO_ACCOUNT_SID
        self.auth_token = auth_token or settings.TWILIO_AUTH_TOKEN
        self.from_number = from_whatsapp_number or settings.TWILIO_WHATSAPP_FROM

    def send_message(
        self,
        to_phone: str,
        body: str,
        content_sid: Optional[str] = None,
        content_variables: Optional[Dict[str, str]] = None
    ) -> Dict[str, Any]:
        """Send WhatsApp message via Twilio or log in mock mode."""
        import json
        formatted_to = to_phone if to_phone.startswith('whatsapp:') else f'whatsapp:{to_phone}'
        target_content_sid = content_sid or settings.TWILIO_ADVISORY_CONTENT_SID

        if not self.account_sid or not self.auth_token:
            logger.info(f"[SIMULATED WHATSAPP] To: {formatted_to} | Body: {body[:60]}...")
            return {
                'status': 'simulated',
                'to': formatted_to,
                'from': self.from_number,
                'body': body,
                'content_sid': target_content_sid,
                'mock': True
            }

        url = f'https://api.twilio.com/2010-04-01/Accounts/{self.account_sid}/Messages.json'
        data = {
            'From': self.from_number,
            'To': formatted_to,
        }
        if target_content_sid:
            data['ContentSid'] = target_content_sid
            if content_variables:
                data['ContentVariables'] = json.dumps(content_variables)
        else:
            data['Body'] = body

        try:
            with httpx.Client(timeout=8.0) as client:
                resp = client.post(url, data=data, auth=(self.account_sid, self.auth_token))
                if resp.status_code in (200, 201):
                    logger.info(f"[TWILIO DISPATCH SUCCESS] SID: {resp.json().get('sid')}")
                    return {'status': 'sent', 'sid': resp.json().get('sid'), 'mock': False}
                logger.error(f"[TWILIO ERROR] {resp.status_code}: {resp.text}")
                return {'status': 'error', 'code': resp.status_code, 'detail': resp.text, 'mock': False}
        except Exception as e:
            logger.exception(f"[TWILIO EXCEPTION] {e}")
            return {'status': 'exception', 'error': str(e), 'mock': True}

    @classmethod
    def validate_signature(cls, full_url: str, params: Dict[str, Any], signature: Optional[str]) -> bool:
        """
        Cryptographic validation of X-Twilio-Signature using HMAC-SHA1.
        Sorts POST parameters alphabetically by key and computes HMAC.
        """
        auth_token = settings.TWILIO_AUTH_TOKEN
        if not auth_token or not signature:
            # If no auth token is configured and we are in dev/simulation, allow
            if not settings.ENFORCE_WEBHOOK_AUTH and not settings.is_production:
                return True
            return False

        # Build payload string: full_url + sorted param key-value pairs
        s = full_url
        for k in sorted(params.keys()):
            s += f"{k}{params[k]}"

        # Compute HMAC-SHA1
        computed_mac = hmac.new(
            auth_token.encode('utf-8'),
            s.encode('utf-8'),
            hashlib.sha1
        ).digest()
        computed_signature = base64.b64encode(computed_mac).decode('utf-8').strip()

        return hmac.compare_digest(computed_signature, signature.strip())
