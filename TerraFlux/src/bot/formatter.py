"""WhatsApp / Telegram message formatting for TerraFlux.

All grower facing wording lives in the message catalogue (``src/bot/i18n.py``), so the
same advisory can be delivered in the language of the farm. WhatsApp renders markdown,
Telegram renders the same text as HTML.
"""
from typing import Any, Dict, Tuple

from ..core.models import FieldModel, SensorModel
from ..core.advisor import AdvisoryDecision, WeatherForecastSummary
from .i18n import normalize_language, to_telegram_html, translate


class WhatsAppFormatter:
    """Creates concise, farmer-friendly messages."""

    @staticmethod
    def _language(field) -> str:
        return normalize_language(getattr(field, 'language', 'en') or 'en')

    @staticmethod
    def _scope(field, decision) -> str:
        """Human readable scope of the advice (zone and/or valve)."""
        parts = []
        if decision.zone:
            parts.append(f"zone {decision.zone}")
        if decision.valve_id:
            parts.append(f"valve {decision.valve_id}")
        return f" ({', '.join(parts)})" if parts else ""

    @staticmethod
    def _callback(action: str, field, decision) -> str:
        """Callback payload that names the field (and zone) explicitly.

        Format: ``ACTION|field_id|zone``. The command handler resolves the block from
        the payload instead of guessing which of the grower's blocks it is.
        """
        return f"{action}|{field.id}|{decision.zone or ''}"

    @staticmethod
    def _duration_text(duration_minutes: int) -> str:
        hours = duration_minutes // 60
        mins = duration_minutes % 60
        return f"{hours}h {mins}m" if hours > 0 else f"{mins}m"

    @classmethod
    def _advisory_context(cls, field, decision, weather) -> Dict[str, Any]:
        status = decision.soil_status
        return {
            'field': field.name,
            'scope': cls._scope(field, decision),
            'crop': field.crop_key.capitalize(),
            'area': field.area_ha,
            'moisture': f"{status.current_moisture_pct:.1f}",
            'threshold': f"{status.threshold_moisture_pct:.1f}",
            'et0': f"{weather.et0_tomorrow_mm:.1f}",
            'rain': f"{weather.rain_next_48h_mm:.1f}",
            'gross': f"{decision.gross_depth_mm:.1f}",
            'net': f"{decision.target_depth_mm:.1f}",
            'time': decision.optimal_start_time,
            'duration': cls._duration_text(decision.duration_minutes),
            'reason': decision.reason
        }

    @classmethod
    def format_daily_advisory(
        cls,
        field: FieldModel,
        decision: AdvisoryDecision,
        weather: WeatherForecastSummary
    ) -> str:
        lang = cls._language(field)
        context = cls._advisory_context(field, decision, weather)
        key = 'advisory.irrigate' if decision.should_irrigate else 'advisory.no_water'
        return translate(key, lang, **context)

    @classmethod
    def format_status_reply(
        cls,
        field: FieldModel,
        sensor: SensorModel,
        status_info: str
    ) -> str:
        lang = cls._language(field)
        return translate(
            'status', lang,
            field=field.name,
            moisture=f"{sensor.last_moisture_pct:.1f}",
            temp=f"{sensor.last_temp_c:.1f}",
            battery=f"{sensor.battery_volts:.2f}",
            uplink=sensor.last_seen.strftime('%H:%M'),
            state=status_info
        )

    @classmethod
    def format_confirmation_reply(cls, field_name: str, depth_mm: float, duration_min: int, language: str = 'en') -> str:
        return translate(
            'confirm', language,
            depth=f"{depth_mm:.1f}",
            field=field_name,
            minutes=duration_min
        )

    @classmethod
    def format_delay_reply(cls, field_name: str, days: int, date_str: str, language: str = 'en') -> str:
        return translate(
            'delay', language,
            field=field_name,
            days=days,
            date=date_str
        )

    @classmethod
    def format_help(cls, language: str = 'en') -> str:
        return translate('help', language)

    @classmethod
    def format_digest(cls, field: FieldModel, summary: Dict[str, Any]) -> str:
        lang = cls._language(field)
        return translate(
            'digest', lang,
            field=field.name,
            moisture=f"{summary.get('moisture_pct', 0):.1f}",
            threshold=f"{summary.get('threshold_pct', 0):.1f}",
            applied=f"{summary.get('applied_net_depth_mm', 0):.1f}",
            irrigations=summary.get('irrigations', 0),
            et0=f"{summary.get('et0_mm', 0):.1f}",
            rain=f"{summary.get('rain_mm', 0):.1f}"
        )

    @classmethod
    def format_escalation(cls, field: FieldModel, summary: Dict[str, Any]) -> str:
        lang = cls._language(field)
        return translate(
            'escalation', lang,
            field=field.name,
            age=int(summary.get('age_hours', 0)),
            reason=summary.get('reason', ''),
            moisture=f"{summary.get('moisture_pct', 0):.1f}",
            threshold=f"{summary.get('threshold_pct', 0):.1f}",
            action=summary.get('action', '')
        )

    # -------------------------------------------------------------
    # Telegram specific formatting
    # -------------------------------------------------------------

    @classmethod
    def format_telegram_advisory(
        cls,
        field: FieldModel,
        decision: AdvisoryDecision,
        weather: WeatherForecastSummary
    ) -> Tuple[str, dict]:
        """Returns HTML text and inline_keyboard reply_markup for Telegram."""
        lang = cls._language(field)
        context = cls._advisory_context(field, decision, weather)

        if decision.should_irrigate:
            key = 'advisory.irrigate'
            keyboard = {
                "inline_keyboard": [
                    [{"text": f"✅ Confirm {decision.gross_depth_mm:.1f}mm Done", "callback_data": cls._callback('DONE', field, decision)}],
                    [
                        {"text": "⏳ Delay 24h", "callback_data": cls._callback('DELAY 1D', field, decision)},
                        {"text": "📊 Live Status", "callback_data": cls._callback('STATUS', field, decision)}
                    ]
                ]
            }
        else:
            key = 'advisory.no_water'
            keyboard = {
                "inline_keyboard": [
                    [
                        {"text": "📊 Live Status", "callback_data": cls._callback('STATUS', field, decision)},
                        {"text": "❓ Help", "callback_data": "HELP"}
                    ]
                ]
            }

        return to_telegram_html(translate(key, lang, **context)), keyboard

    @classmethod
    def format_telegram_status(
        cls,
        field: FieldModel,
        sensor: SensorModel,
        status_info: str
    ) -> Tuple[str, dict]:
        """Returns HTML status message and inline keyboard."""
        lang = cls._language(field)
        text = to_telegram_html(translate(
            'status', lang,
            field=field.name,
            moisture=f"{sensor.last_moisture_pct:.1f}",
            temp=f"{sensor.last_temp_c:.1f}",
            battery=f"{sensor.battery_volts:.2f}",
            uplink=sensor.last_seen.strftime('%H:%M'),
            state=status_info
        ))
        keyboard = {
            "inline_keyboard": [
                [
                    {"text": "💧 Water 10mm", "callback_data": "WATER 10mm"},
                    {"text": "💧 Water 15mm", "callback_data": "WATER 15mm"}
                ],
                [
                    {"text": "⏳ Delay 24h", "callback_data": "DELAY 1D"},
                    {"text": "❓ Help", "callback_data": "HELP"}
                ]
            ]
        }
        return text, keyboard


# Alias for unified bot formatting
TelegramFormatter = WhatsAppFormatter
