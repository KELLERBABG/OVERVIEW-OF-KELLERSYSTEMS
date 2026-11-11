"""Cryptographic Security, Password Hashing (PBKDF2-HMAC-SHA256) & Stateless Token Authentication."""
import hmac
import json
import base64
import hashlib
import secrets
from datetime import datetime, timezone, timedelta
from typing import Optional, Dict, Any
from .config import settings

PBKDF2_ITERATIONS = 120_000
SALT_BYTES = 16

def hash_password(plain_password: str) -> str:
    """Hash password using PBKDF2-HMAC-SHA256 with cryptographically secure random salt."""
    salt = secrets.token_bytes(SALT_BYTES)
    dk = hashlib.pbkdf2_hmac('sha256', plain_password.encode('utf-8'), salt, PBKDF2_ITERATIONS)
    salt_b64 = base64.b64encode(salt).decode('ascii')
    dk_b64 = base64.b64encode(dk).decode('ascii')
    return f"pbkdf2:sha256:{PBKDF2_ITERATIONS}${salt_b64}${dk_b64}"

def verify_password(plain_password: str, hashed_password: str) -> bool:
    """Verify password against stored PBKDF2 hash using constant-time comparison."""
    try:
        parts = hashed_password.split('$')
        if len(parts) != 3:
            return False
        header, salt_b64, dk_b64 = parts
        salt = base64.b64decode(salt_b64.encode('ascii'))
        expected_dk = base64.b64decode(dk_b64.encode('ascii'))
        iterations = int(header.split(':')[-1])

        computed_dk = hashlib.pbkdf2_hmac('sha256', plain_password.encode('utf-8'), salt, iterations)
        return hmac.compare_digest(computed_dk, expected_dk)
    except Exception:
        return False

def create_access_token(
    user_id: int,
    email: str,
    role: str = 'grower',
    expires_in_hours: int = 72,
    token_version: int = 1
) -> str:
    """Generate a signed URL-safe bearer token with expiration and HMAC-SHA256 signature.

    The token carries the user's `token_version`. Bumping that counter on the user
    invalidates every token issued before the change (server-side revocation).
    """
    now = datetime.now(timezone.utc)
    expires_at = now + timedelta(hours=expires_in_hours)
    
    payload = {
        'sub': user_id,
        'email': email,
        'role': role,
        'ver': token_version,
        'iat': int(now.timestamp()),
        'exp': int(expires_at.timestamp())
    }
    
    payload_json = json.dumps(payload, separators=(',', ':')).encode('utf-8')
    payload_b64 = base64.urlsafe_b64encode(payload_json).decode('ascii').rstrip('=')
    
    secret_key = settings.SECRET_KEY.encode('utf-8')
    sig = hmac.new(secret_key, payload_b64.encode('ascii'), hashlib.sha256).digest()
    sig_b64 = base64.urlsafe_b64encode(sig).decode('ascii').rstrip('=')
    
    return f"{payload_b64}.{sig_b64}"

def decode_access_token(token: str) -> Optional[Dict[str, Any]]:
    """Verify cryptographic signature and expiration of bearer token."""
    try:
        parts = token.strip().split('.')
        if len(parts) != 2:
            return None
        payload_b64, sig_b64 = parts
        
        # Verify signature
        secret_key = settings.SECRET_KEY.encode('utf-8')
        computed_sig = hmac.new(secret_key, payload_b64.encode('ascii'), hashlib.sha256).digest()
        computed_sig_b64 = base64.urlsafe_b64encode(computed_sig).decode('ascii').rstrip('=')
        
        if not hmac.compare_digest(sig_b64, computed_sig_b64):
            return None
        
        # Restore padding for payload
        rem = len(payload_b64) % 4
        if rem > 0:
            payload_b64 += '=' * (4 - rem)
            
        payload_bytes = base64.urlsafe_b64decode(payload_b64.encode('ascii'))
        payload = json.loads(payload_bytes.decode('utf-8'))
        
        # Check expiration
        now_ts = int(datetime.now(timezone.utc).timestamp())
        if payload.get('exp', 0) < now_ts:
            return None
            
        return payload
    except Exception:
        return None
