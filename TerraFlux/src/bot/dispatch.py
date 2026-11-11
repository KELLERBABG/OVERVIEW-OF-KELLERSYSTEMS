"""Centralised advisory dispatch: retry, persistence and an audit trail.

Every outbound message attempt is written to the ``dispatch_logs`` table, so a
failed delivery to a grower is visible in the API instead of disappearing into the
log file. The scheduler and the manual "Dispatch Advisory" button both go through
this service, which keeps the behaviour identical.
"""
import time
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional

from sqlmodel import Session

from ..core.advisor import AdvisoryDecision, WeatherForecastSummary
from ..core.logger import logger
from ..core.models import AdvisoryAlertModel, DispatchLogModel, FieldModel
from .formatter import WhatsAppFormatter
from .telegram_client import TelegramBotClient
from .twilio_client import TwilioWhatsAppClient


@dataclass
class DispatchResult:
    """Outcome of a dispatch across all configured channels."""
    telegram: Dict[str, Any] = field(default_factory=dict)
    whatsapp: Dict[str, Any] = field(default_factory=dict)
    attempts: int = 0
    failures: int = 0
    skipped: List[str] = field(default_factory=list)

    @property
    def delivered(self) -> bool:
        """True when no channel failed (skipped channels do not count as failures)."""
        return self.failures == 0

    def to_dict(self) -> Dict[str, Any]:
        return {
            'telegram': self.telegram,
            'whatsapp': self.whatsapp,
            'attempts': self.attempts,
            'failures': self.failures,
            'skipped': self.skipped,
            'delivered': self.delivered
        }


class AdvisoryDispatcher:
    """Sends an advisory to every configured channel, with retry and persistence."""

    MAX_ATTEMPTS = 3
    RETRY_BACKOFF_SECONDS = 2

    @classmethod
    def dispatch(
        cls,
        session: Session,
        field: FieldModel,
        alert: Optional[AdvisoryAlertModel],
        decision: AdvisoryDecision,
        weather: WeatherForecastSummary
    ) -> DispatchResult:
        result = DispatchResult()
        whatsapp_text = WhatsAppFormatter.format_daily_advisory(field, decision, weather)

        # --- Telegram (primary, zero fee) ---
        if field.farmer_chat_id:
            tg_text, tg_markup = WhatsAppFormatter.format_telegram_advisory(field, decision, weather)
            outcome, attempts = cls._send_with_retry(
                lambda: TelegramBotClient().send_message(
                    field.farmer_chat_id, tg_text, reply_markup=tg_markup
                )
            )
            result.telegram = outcome
            result.attempts += attempts
            if outcome.get('status') in ('error', 'exception'):
                result.failures += 1
            cls._log(session, field, alert, 'telegram', field.farmer_chat_id, outcome, attempts)
        else:
            result.skipped.append('telegram')
            cls._log(session, field, alert, 'telegram', '', {'status': 'skipped', 'reason': 'no_farmer_chat_id'}, 0)

        # --- WhatsApp (legacy fallback) ---
        if field.farmer_phone:
            outcome, attempts = cls._send_with_retry(
                lambda: TwilioWhatsAppClient().send_message(field.farmer_phone, whatsapp_text)
            )
            result.whatsapp = outcome
            result.attempts += attempts
            if outcome.get('status') in ('error', 'exception'):
                result.failures += 1
            cls._log(session, field, alert, 'whatsapp', field.farmer_phone, outcome, attempts)
        else:
            result.skipped.append('whatsapp')
            cls._log(session, field, alert, 'whatsapp', '', {'status': 'skipped', 'reason': 'no_farmer_phone'}, 0)

        return result

    @classmethod
    def _send_with_retry(cls, send) -> tuple:
        """Run a send function, retrying transient failures.

        Returns ``(outcome, attempts)``. Only 'error' and 'exception' outcomes are
        retried - a 'skipped' or 'simulated' result is a valid, final answer.
        """
        outcome: Dict[str, Any] = {'status': 'unknown'}
        attempts = 0
        for attempt in range(1, cls.MAX_ATTEMPTS + 1):
            attempts = attempt
            try:
                outcome = send() or {'status': 'unknown'}
            except Exception as exc:  # transport level failure
                outcome = {'status': 'exception', 'error': str(exc)}
            if outcome.get('status') not in ('error', 'exception'):
                return outcome, attempts
            if attempt < cls.MAX_ATTEMPTS:
                logger.warning(f"[DISPATCH] Attempt {attempt} failed ({outcome.get('status')}) - retrying")
                time.sleep(cls.RETRY_BACKOFF_SECONDS * attempt)
        logger.error(f"[DISPATCH] Giving up after {attempts} attempts: {outcome}")
        return outcome, attempts

    @classmethod
    def _log(
        cls,
        session: Session,
        field: FieldModel,
        alert: Optional[AdvisoryAlertModel],
        channel: str,
        target: str,
        outcome: Dict[str, Any],
        attempts: int
    ) -> None:
        """Persist one delivery attempt."""
        status = outcome.get('status', 'unknown')
        if status == 'unknown':
            status = 'failed'
        detail = None
        if status in ('error', 'exception'):
            detail = str(outcome.get('detail') or outcome.get('error') or 'unknown error')[:500]
        elif status == 'skipped':
            detail = str(outcome.get('reason') or 'skipped')[:200]

        entry = DispatchLogModel(
            field_id=field.id,
            alert_id=alert.id if alert else None,
            channel=channel,
            target=target,
            status=status,
            detail=detail,
            attempts=attempts
        )
        session.add(entry)
        try:
            session.commit()
        except Exception as exc:
            session.rollback()
            logger.error(f"[DISPATCH] Could not persist the dispatch log entry: {exc}")
