"""Two-Way WhatsApp Webhook Command Handler & Dispatcher."""
import re
from datetime import datetime, timezone
from typing import Dict, Any, Tuple, Optional
from sqlmodel import Session, select
from ..core.models import FieldModel, SensorModel, IrrigationEventModel, AdvisoryAlertModel
from ..core.water_balance import WaterBalanceEngine
from ..core.logger import logger
from .formatter import WhatsAppFormatter
from .i18n import normalize_language, translate

def _describe_soil_status(field: FieldModel, sensor: Optional[SensorModel]) -> str:
    """Human readable soil status based on the FAO-56 threshold for this field."""
    lang = normalize_language(getattr(field, 'language', 'en') or 'en')
    if not sensor:
        return translate('state.no_data', lang)
    status = WaterBalanceEngine.analyze_soil_status(
        current_moisture_pct=sensor.last_moisture_pct,
        crop_key=field.crop_key,
        growth_stage=field.growth_stage,
        soil_key=field.soil_key
    )
    threshold = f"{status.threshold_moisture_pct:.1f}"
    if status.is_stressed:
        return translate('state.stressed', lang, threshold=threshold)
    if status.current_moisture_pct <= status.threshold_moisture_pct + 1.5:
        return translate('state.approaching', lang, threshold=threshold)
    return translate('state.optimal', lang, threshold=threshold)

def _parse_action(raw: str) -> Tuple[str, Optional[int], Optional[str]]:
    """Split an inbound command into action, optional field id and optional zone.

    Telegram inline buttons carry the scope of the advice:
    ``ACTION|field_id|zone`` (e.g. ``DONE|3|A1``). Plain text commands such as
    ``DONE`` or ``WATER 15MM`` have no scope.
    """
    parts = raw.strip().split('|')
    action = parts[0].strip().upper()
    field_id = None
    zone = None
    if len(parts) > 1 and parts[1].strip().isdigit():
        field_id = int(parts[1].strip())
    if len(parts) > 2 and parts[2].strip():
        zone = parts[2].strip().upper()
    return action, field_id, zone


class WhatsAppCommandHandler:
    """Parses incoming WhatsApp replies from growers and executes actions."""

    @classmethod
    def handle_incoming_message(
        cls,
        session: Session,
        from_number: str,
        message_body: str
    ) -> str:
        """Processes inbound message and returns text reply."""
        from datetime import timedelta
        clean_text = message_body.strip().upper()
        action, payload_field_id, payload_zone = _parse_action(message_body)
        
        # Check for Telegram /start onboarding and deep-link token
        if clean_text.startswith("/START") or clean_text.startswith("START"):
            start_parts = message_body.strip().split()
            token = start_parts[1].strip() if len(start_parts) > 1 else None

            if token:
                target_field = session.exec(select(FieldModel).where(FieldModel.invite_token == token)).first()
                if target_field:
                    target_field.farmer_chat_id = str(from_number)
                    session.add(target_field)
                    session.commit()
                    session.refresh(target_field)
                    logger.info(f"[TELEGRAM ONBOARDING] Successfully linked chat_id {from_number} to field '{target_field.name}' (id={target_field.id}) via token '{token}'")
                    return (
                        f"🎉 *Welcome to TerraFlux!*\n\n"
                        f"Your Telegram account has been linked to *{target_field.name}* ({target_field.location_name}).\n\n"
                        f"🌿 *Crop:* {target_field.crop_key.capitalize()} ({target_field.growth_stage})\n"
                        f"📍 *Area:* {target_field.area_ha} ha\n"
                        f"💧 *Target Emitters:* {target_field.drip_emitter_lph} L/h @ {target_field.drip_emitter_spacing_m}m\n\n"
                        f"You will automatically receive daily irrigation advisories and alerts here.\n"
                        f"Type *STATUS* to check live soil moisture, or *HELP* for command options."
                    )
                else:
                    return (
                        f"⚠️ *Invalid Invite Code:* No farm field was found matching code '{token}'.\n"
                        f"Please check your Telegram invite link or consult the TerraFlux dashboard."
                    )
            else:
                # /start without token: check if already linked
                existing = session.exec(
                    select(FieldModel).where(
                        (FieldModel.farmer_chat_id == str(from_number)) | (FieldModel.farmer_phone == str(from_number))
                    )
                ).first()
                if existing:
                    return (
                        f"👋 *Welcome back, {existing.farmer_name}!* You are connected to *{existing.name}*.\n\n"
                        f"Type *STATUS* for current sensor readings, *DONE* to confirm irrigation, or *HELP* for command options."
                    )
                else:
                    return (
                        "🌱 *Welcome to TerraFlux (FieldSense Engine)!*\n\n"
                        "To link your Telegram account to your farm, please open the private invite link from your dashboard or send:\n"
                        "`/start <your_invite_token>`"
                    )

        # Look up all fields matching this farmer's phone number or Telegram chat_id
        statement = select(FieldModel).where(
            (FieldModel.farmer_phone == str(from_number)) | (FieldModel.farmer_chat_id == str(from_number))
        )
        matching_fields = session.exec(statement).all()
        
        if not matching_fields:
            logger.warning(f"[SECURITY] Message from unregistered number / user: {from_number}")
            return (
                f"⚠️ *Unregistered Number:* The phone number / chat ID {from_number} is not associated with any active TerraFlux farm.\n"
                "Please contact your agricultural manager or register this phone in your account."
            )

        # Select target field: an inline button names the block explicitly, otherwise
        # fall back to a name keyword or the block with the most recent pending alert.
        field = matching_fields[0]
        if payload_field_id is not None:
            scoped = next((f for f in matching_fields if f.id == payload_field_id), None)
            if scoped is None:
                logger.warning(
                    f"[SECURITY] Sender {from_number} tried to act on field {payload_field_id} "
                    f"which does not belong to them"
                )
                return (
                    "⚠️ *Unknown Block:* This action refers to a farm block that is not linked "
                    "to your account. Please contact your agricultural manager."
                )
            field = scoped
        else:
            for f in matching_fields:
                # check if field name or keyword in text
                first_word = f.name.split()[0].upper()
                if first_word in clean_text:
                    field = f
                    break
            else:
                # Look for pending alert
                field_ids = [f.id for f in matching_fields]
                pending_alert = session.exec(
                    select(AdvisoryAlertModel)
                    .where(AdvisoryAlertModel.field_id.in_(field_ids))
                    .where(AdvisoryAlertModel.status == 'PENDING')
                    .order_by(AdvisoryAlertModel.timestamp.desc())
                ).first()
                if pending_alert:
                    field = next((f for f in matching_fields if f.id == pending_alert.field_id), matching_fields[0])

        # Zone aware probe selection: an inline button may name the zone.
        zone_sensors = session.exec(
            select(SensorModel).where(SensorModel.field_id == field.id)
        ).all()
        sensor = None
        if payload_zone:
            sensor = next((s for s in zone_sensors if (s.zone or '').upper() == payload_zone), None)
        if sensor is None and zone_sensors:
            sensor = zone_sensors[0]

        # 1. Handle HELP
        if "HELP" in action or "INFO" in action:
            return WhatsAppFormatter.format_help(normalize_language(getattr(field, 'language', 'en')))

        # 2. Handle STATUS
        if "STATUS" in action or "CHECK" in action:
            if len(matching_fields) > 1 and "ALL" in action:
                lines = [f"📊 *TerraFlux Status Summary ({len(matching_fields)} Fields)*"]
                for f in matching_fields:
                    s = session.exec(select(SensorModel).where(SensorModel.field_id == f.id)).first()
                    m = f"{s.last_moisture_pct:.1f}%" if s else "N/A"
                    lines.append(f"• *{f.name}:* {m} VWC")
                return "\n".join(lines)

            status_text = _describe_soil_status(field, sensor)
            if sensor:
                return WhatsAppFormatter.format_status_reply(field, sensor, status_text)
            return f"Field {field.name}: active, but no sensor attached yet."

        # Helper to compute physical VWC increase from a NET depth that reached the root zone.
        # The efficiency factor is NOT applied here: the depth is already the net water
        # that infiltrated, so only the root-zone conversion applies.
        def calculate_added_vwc(net_depth_mm: float) -> float:
            crop_params = WaterBalanceEngine.get_crop_params(field.crop_key)
            zr = float(crop_params.get("root_depth_m", 1.0))
            # 1 mm over Zr meters: (1 / (1000 * Zr)) * 100 % VWC
            return net_depth_mm * (100.0 / (1000.0 * max(0.2, zr)))

        # 3. Handle DONE / YES / OK
        if action in ("DONE", "YES", "OK", "IRRIGATED", "FINISHED", "FERTIG"):
            alert = session.exec(
                select(AdvisoryAlertModel)
                .where(AdvisoryAlertModel.field_id == field.id)
                .order_by(AdvisoryAlertModel.timestamp.desc())
            ).first()
            # When the button named a zone, prefer the advisory for that zone.
            if payload_zone:
                zone_alert = session.exec(
                    select(AdvisoryAlertModel)
                    .where(AdvisoryAlertModel.field_id == field.id)
                    .where(AdvisoryAlertModel.zone == payload_zone)
                    .order_by(AdvisoryAlertModel.timestamp.desc())
                ).first()
                if zone_alert:
                    alert = zone_alert

            # The advisory stores the NET root-zone depth; the gross volume the grower
            # had to run is derived from it and kept in the audit notes.
            # A confirmation without a pending irrigation advisory (for example the
            # grower watered by hand) falls back to a documented default depth instead
            # of logging a meaningless 0 mm event.
            use_advisory = bool(alert and alert.recommended_depth_mm and alert.recommended_depth_mm > 0)
            if use_advisory:
                net_depth = alert.recommended_depth_mm
                duration = alert.recommended_duration_min
                basis = 'confirmed against advisory'
            else:
                net_depth = 10.0
                duration = WaterBalanceEngine.calculate_irrigation_duration_min(
                    net_depth, field.application_rate_mm_h
                )
                basis = 'confirmed without a pending advisory - default depth applied'
            gross_depth = WaterBalanceEngine.calculate_gross_depth_mm(
                net_depth_mm=net_depth,
                irrigation_efficiency=field.irrigation_efficiency
            )

            event = IrrigationEventModel(
                field_id=field.id,
                timestamp=datetime.now(timezone.utc),
                depth_applied_mm=net_depth,
                duration_minutes=duration,
                zone=payload_zone,
                source="whatsapp_reply",
                notes=(f"Confirmed by grower via chat | net {net_depth:.1f}mm, "
                       f"gross {gross_depth:.1f}mm at {field.irrigation_efficiency:.0%} efficiency | {basis}")
            )
            session.add(event)

            if alert:
                alert.status = "ACCEPTED"
                session.add(alert)

            # Only update sensor reading directly if it is a simulated probe
            # Real hardware probes will report true soil moisture via LoRaWAN uplink
            if sensor and sensor.is_simulated:
                soil_params = WaterBalanceEngine.get_soil_params(field.soil_key)
                fc = float(soil_params.get("field_capacity_pct", 22.0))
                sensor.last_moisture_pct = min(fc, sensor.last_moisture_pct + calculate_added_vwc(net_depth))
                session.add(sensor)

            session.commit()
            logger.info(f"[IRRIGATION LOGGED] Field: {field.name} | Net Depth: {net_depth}mm via chat")
            return WhatsAppFormatter.format_confirmation_reply(
                field.name, net_depth, duration,
                language=normalize_language(getattr(field, 'language', 'en'))
            )

        # 4. Handle DELAY / SNOOZE / PAUSE
        # NOTE: this branch is intentionally evaluated BEFORE the WATER command.
        # The old WATER regex accepted an optional keyword and matched bare digits,
        # so "DELAY 1D" / "PAUSE 3D" were misread as a 1mm / 3mm irrigation event
        # and the field was never actually paused.
        if re.search(r'\b(DELAY|PAUSE|SNOOZE|SPÄTER)\b', action):
            days_match = re.search(r'(\d+)\s*(?:D|DAY|DAYS|TAGE)?', action)
            pause_days = int(days_match.group(1)) if days_match else 1
            pause_days = min(14, max(1, pause_days))

            field.paused_until = datetime.now(timezone.utc) + timedelta(days=pause_days)
            session.add(field)

            # Update latest alert to SNOOZED if pending
            alert = session.exec(
                select(AdvisoryAlertModel)
                .where(AdvisoryAlertModel.field_id == field.id)
                .order_by(AdvisoryAlertModel.timestamp.desc())
            ).first()
            if alert and alert.status == 'PENDING':
                alert.status = 'SNOOZED'
                session.add(alert)

            session.commit()
            logger.info(f"[ADVISORY POSTPONED] Field: {field.name} paused for {pause_days} day(s)")
            return WhatsAppFormatter.format_delay_reply(
                field.name, pause_days, field.paused_until.strftime('%b %d'),
                language=normalize_language(getattr(field, 'language', 'en'))
            )

        # 5. Handle custom WATER <depth> (e.g. "WATER 15MM", "GEWÄSSERT 15", "15MM").
        # A keyword OR an explicit MM unit is mandatory - a bare number must never be
        # interpreted as a watering depth.
        water_match = re.fullmatch(
            r'(?:(?:WATER|WASSER|GEWÄSSERT)\s*(?P<kw>\d+(?:[.,]\d+)?)\s*MM?|(?P<unit>\d+(?:[.,]\d+)?)\s*MM)',
            action
        )
        if water_match:
            raw_depth = water_match.group('kw') or water_match.group('unit')
            gross_depth = float(raw_depth.replace(',', '.'))
            if gross_depth > 0:
                # A grower reports what they ran (GROSS). Only the efficient share
                # reaches the root zone, so the net depth is what we log and model.
                net_depth = WaterBalanceEngine.calculate_net_depth_mm(
                    gross_depth_mm=gross_depth,
                    irrigation_efficiency=field.irrigation_efficiency
                )
                duration = WaterBalanceEngine.calculate_irrigation_duration_min(
                    net_depth, field.application_rate_mm_h
                )

                event = IrrigationEventModel(
                    field_id=field.id,
                    timestamp=datetime.now(timezone.utc),
                    depth_applied_mm=net_depth,
                    duration_minutes=duration,
                    zone=payload_zone,
                    source="whatsapp_custom_command",
                    notes=(f"Custom logged depth: gross {gross_depth:.1f}mm, "
                           f"net {net_depth:.1f}mm at {field.irrigation_efficiency:.0%} efficiency")
                )
                session.add(event)

                if sensor and sensor.is_simulated:
                    soil_params = WaterBalanceEngine.get_soil_params(field.soil_key)
                    fc = float(soil_params.get("field_capacity_pct", 22.0))
                    sensor.last_moisture_pct = min(fc, sensor.last_moisture_pct + calculate_added_vwc(net_depth))
                    session.add(sensor)

                session.commit()
                logger.info(f"[CUSTOM WATER LOGGED] Field: {field.name} | Gross: {gross_depth}mm | Net: {net_depth:.1f}mm")
                return WhatsAppFormatter.format_confirmation_reply(
                    field.name, net_depth, duration,
                    language=normalize_language(getattr(field, 'language', 'en'))
                )

        # Default fallback
        return (
            f"👋 Hi {field.farmer_name}! Received: \"{message_body}\".\n"
            "Reply *DONE* to confirm watering, *STATUS* for soil levels, *DELAY 1D* to postpone, or *HELP* for command list."
        )

    @classmethod
    def handle_telegram_update(cls, session: Session, update: Dict[str, Any]) -> Dict[str, Any]:
        """Processes Telegram webhook update (message or callback_query) and returns response payload."""
        from .telegram_client import TelegramBotClient
        telegram_client = TelegramBotClient()

        # 1. Handle Inline Keyboard Button Click
        if "callback_query" in update:
            cb = update["callback_query"]
            cb_id = cb.get("id", "")
            chat_id = str(cb.get("message", {}).get("chat", {}).get("id"))
            action = cb.get("data", "")

            # Acknowledge callback query
            if cb_id:
                telegram_client.answer_callback_query(cb_id, text=f"Recorded: {action}")

            reply_text = cls.handle_incoming_message(session, chat_id, action)
            dispatch_result = telegram_client.send_message(chat_id, reply_text)
            return {
                "status": "ok",
                "type": "callback_query",
                "action": action,
                "reply": reply_text,
                "dispatch": dispatch_result
            }

        # 2. Handle Direct Text Message
        if "message" in update:
            msg = update["message"]
            chat_id = str(msg.get("chat", {}).get("id"))
            text = msg.get("text", "")

            reply_text = cls.handle_incoming_message(session, chat_id, text)
            dispatch_result = telegram_client.send_message(chat_id, reply_text)
            return {
                "status": "ok",
                "type": "message",
                "reply": reply_text,
                "dispatch": dispatch_result
            }

        return {"status": "ignored", "reason": "No supported message or callback_query in update"}
