"""Centralized 12-Factor Application Configuration and Environment Validation."""
from typing import List, Optional
from pydantic import Field
from pydantic_settings import BaseSettings, SettingsConfigDict

class AppSettings(BaseSettings):
    """Production settings loaded from environment or .env file."""
    model_config = SettingsConfigDict(
        env_file='.env',
        env_file_encoding='utf-8',
        extra='ignore'
    )

    # Core Environment
    ENVIRONMENT: str = Field(default='development', description='development, staging, or production')
    SECRET_KEY: str = Field(default='fieldsense-dev-insecure-secret-key-change-me', description='API Secret')
    DATABASE_URL: str = Field(default='sqlite:///fieldsense.db', description='Database connection string')

    # Security & CORS
    CORS_ORIGINS: List[str] = Field(default=['*'], description='Allowed origins for CORS')
    TTN_WEBHOOK_SECRET: Optional[str] = Field(default=None, description='Pre-shared secret for TTN IoT uplinks')
    ENFORCE_WEBHOOK_AUTH: bool = Field(default=False, description='Strictly enforce Twilio & TTN signatures')

    # Twilio WhatsApp Configuration (Legacy / Alternative)
    TWILIO_ACCOUNT_SID: Optional[str] = Field(default=None, description='Twilio Account SID')
    TWILIO_AUTH_TOKEN: Optional[str] = Field(default=None, description='Twilio Auth Token')
    TWILIO_WHATSAPP_FROM: str = Field(default='whatsapp:+14155238886', description='Twilio WhatsApp sender')
    TWILIO_ADVISORY_CONTENT_SID: Optional[str] = Field(default=None, description='Pre-approved WhatsApp Utility Content SID for 24h window')

    # Telegram Bot Configuration (Recommended / Zero-Fee)
    TELEGRAM_BOT_TOKEN: Optional[str] = Field(default=None, description='Telegram Bot Token from @BotFather')
    TELEGRAM_WEBHOOK_SECRET: Optional[str] = Field(default=None, description='Secret token for Telegram webhook verification')

    # Agronomy & External Services
    OPEN_METEO_API_KEY: Optional[str] = Field(default=None, description='Optional commercial API key for Open-Meteo')
    WEATHER_CACHE_TTL_SECONDS: int = Field(default=14400, description='Weather forecast cache TTL (4h)')
    AUTO_SCHEDULER_ENABLED: bool = Field(default=True, description='Enable background daily advisory scheduler')
    LOG_LEVEL: str = Field(default='INFO', description='Logging level')

    # Authentication hardening
    PASSWORD_MIN_LENGTH: int = Field(default=8, description='Minimum password length for registration')
    LOGIN_MAX_FAILURES: int = Field(default=5, description='Failed logins before the account is locked')
    LOGIN_LOCK_SECONDS: int = Field(default=900, description='Account lockout duration after too many failures')
    LOGIN_RATE_LIMIT_PER_MINUTE: int = Field(default=20, description='Login attempts per IP per minute')
    REGISTER_RATE_LIMIT_PER_HOUR: int = Field(default=10, description='Registrations per IP per hour')
    API_RATE_LIMIT_PER_MINUTE: int = Field(default=120, description='Authenticated API calls per user per minute')
    WEBHOOK_RATE_LIMIT_PER_MINUTE: int = Field(default=600, description='Webhook calls per IP per minute')

    # Actuation (valve control)
    VALVE_CONTROLLER_URL: Optional[str] = Field(
        default=None,
        description='Base URL of the valve gateway. Leave empty to keep the simulated controller.'
    )
    VALVE_CONTROLLER_TOKEN: Optional[str] = Field(default=None, description='Bearer token for the valve gateway')
    AUTO_ACTUATION_ENABLED: bool = Field(
        default=False,
        description='Let the scheduler open the valve automatically after the advisory. Off by default.'
    )
    ESCALATION_AFTER_HOURS: int = Field(
        default=6,
        description='Hours after which an unconfirmed critical advisory is escalated to the second contact.'
    )

    @property
    def is_production(self) -> bool:
        return self.ENVIRONMENT.lower() == 'production'

# Singleton instance
settings = AppSettings()
