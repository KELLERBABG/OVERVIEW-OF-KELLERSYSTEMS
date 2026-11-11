"""Automated Background Scheduler for Daily Agronomic Advisory Dispatches."""
import asyncio
import os
import socket
import uuid
from typing import Optional
from datetime import datetime, timezone, timedelta
from sqlmodel import Session, select
from .database import engine
from ..core.models import FieldModel, SensorModel, AdvisoryAlertModel, SchedulerLockModel
from ..core.models import DispatchLogModel, IrrigationEventModel
from ..core.advisor import AgronomicAdvisor, hours_since_reading
from ..core.time_utils import as_utc, hours_between
from ..core.water_balance import WaterBalanceEngine
from ..ingestion.weather import WeatherService
from ..bot.formatter import WhatsAppFormatter
from ..bot.i18n import to_telegram_html
from ..bot.dispatch import AdvisoryDispatcher
from ..actuation import build_plan, execute_plan
from ..bot.twilio_client import TwilioWhatsAppClient
from ..bot.telegram_client import TelegramBotClient
from ..core.logger import logger
from ..core.config import settings

DISPATCH_LOCK_NAME = 'daily_advisory_dispatch'
DISPATCH_LOCK_TTL_SECONDS = 300

class DailyAdvisoryScheduler:
    """Runs a background loop that evaluates fields and dispatches daily advisories."""

    _task: asyncio.Task = None
    _running: bool = False
    _holder_id: str = f"{socket.gethostname()}:{os.getpid()}:{uuid.uuid4().hex[:8]}"

    @classmethod
    async def start(cls):
        """Start the background scheduler task."""
        if not settings.AUTO_SCHEDULER_ENABLED:
            logger.info("[SCHEDULER] Auto-scheduler is disabled by configuration.")
            return

        cls._running = True
        cls._task = asyncio.create_task(cls._scheduler_loop())
        logger.info("[SCHEDULER] Background Daily Advisory Scheduler started.")

    @classmethod
    async def stop(cls):
        """Stop the background scheduler cleanly."""
        cls._running = False
        if cls._task:
            cls._task.cancel()
            try:
                await cls._task
            except asyncio.CancelledError:
                pass
        logger.info("[SCHEDULER] Background Daily Advisory Scheduler stopped.")

    @classmethod
    async def _scheduler_loop(cls):
        """Loop running every 60 seconds, checking for pending hourly field dispatches in their local timezones."""
        last_checked_hour = -1

        while cls._running:
            try:
                now_utc = datetime.now(timezone.utc)
                current_hour = now_utc.hour

                if current_hour != last_checked_hour:
                    logger.info(f"[SCHEDULER TICK] Checking active fields for hour {current_hour}:00 UTC (timezone-aware)")
                    # The dispatch does blocking DB work and outbound HTTP calls, so it must
                    # not run on the event loop - otherwise every API request waits for it.
                    await asyncio.to_thread(cls.run_hourly_dispatch, target_hour=None)
                    # Weekly digest and escalation check run once per hour as well.
                    await asyncio.to_thread(cls.run_weekly_digest)
                    await asyncio.to_thread(cls.run_escalation_check)
                    last_checked_hour = current_hour

                await asyncio.sleep(60)
            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.exception(f"[SCHEDULER LOOP ERROR] {e}")
                await asyncio.sleep(60)

    @classmethod
    def _acquire_dispatch_lock(cls, session: Session, ttl_seconds: int = DISPATCH_LOCK_TTL_SECONDS) -> bool:
        """Acquire the cross-process dispatch lease.

        Only one API worker may send grower notifications per tick. The lease is
        stored in the database so it works across processes and containers.
        """
        now = datetime.now(timezone.utc)
        lock = session.exec(
            select(SchedulerLockModel).where(SchedulerLockModel.name == DISPATCH_LOCK_NAME)
        ).first()

        if lock is None:
            lock = SchedulerLockModel(
                name=DISPATCH_LOCK_NAME,
                holder=cls._holder_id,
                acquired_at=now,
                expires_at=now + timedelta(seconds=ttl_seconds)
            )
            session.add(lock)
            try:
                session.commit()
                return True
            except Exception:
                # Another worker inserted the lock first.
                session.rollback()
                return False

        expires_at = lock.expires_at
        if expires_at.tzinfo is None:
            expires_at = expires_at.replace(tzinfo=timezone.utc)
        if expires_at > now:
            return False  # a live lease is held by another worker

        lock.holder = cls._holder_id
        lock.acquired_at = now
        lock.expires_at = now + timedelta(seconds=ttl_seconds)
        session.add(lock)
        try:
            session.commit()
            return True
        except Exception:
            session.rollback()
            return False

    @classmethod
    def _release_dispatch_lock(cls, session: Session) -> None:
        """Release the lease so the next worker can dispatch."""
        lock = session.exec(
            select(SchedulerLockModel).where(SchedulerLockModel.name == DISPATCH_LOCK_NAME)
        ).first()
        if lock and lock.holder == cls._holder_id:
            lock.expires_at = datetime.now(timezone.utc)
            session.add(lock)
            try:
                session.commit()
            except Exception:
                session.rollback()

    @classmethod
    def run_hourly_dispatch(cls, target_hour: Optional[int] = None, force: bool = False) -> int:
        """Evaluate and dispatch alerts for all active fields in their local farm timezones.
        
        If target_hour is provided (e.g. testing), evaluates fields configured for that hour.
        If target_hour is None (live background scheduler), evaluates each field against its local wall-clock hour.
        """
        dispatched_count = 0
        twilio = TwilioWhatsAppClient()
        telegram = TelegramBotClient()

        now_utc = datetime.now(timezone.utc)

        with Session(engine) as session:
            # Leader election: only one worker may dispatch per tick.
            if not force:
                if not cls._acquire_dispatch_lock(session):
                    logger.info("[SCHEDULER SKIPPED] Another worker holds the dispatch lease")
                    return 0
            try:
                dispatched_count = cls._dispatch_fields(
                    session=session,
                    target_hour=target_hour,
                    force=force,
                    now_utc=now_utc,
                    twilio=twilio,
                    telegram=telegram
                )
            finally:
                if not force:
                    cls._release_dispatch_lock(session)

        return dispatched_count

    @classmethod
    def _in_quiet_hours(cls, field: FieldModel, local_now) -> bool:
        """True when the local time is inside the configured quiet hours."""
        start = getattr(field, 'quiet_hours_start', None)
        end = getattr(field, 'quiet_hours_end', None)
        if not start or not end:
            return False
        try:
            start_h, start_m = [int(part) for part in str(start).split(':')[:2]]
            end_h, end_m = [int(part) for part in str(end).split(':')[:2]]
        except (TypeError, ValueError):
            return False
        current = local_now.hour * 60 + local_now.minute
        start_minutes = start_h * 60 + start_m
        end_minutes = end_h * 60 + end_m
        if start_minutes <= end_minutes:
            return start_minutes <= current < end_minutes
        # The window crosses midnight (e.g. 22:00 -> 06:00)
        return current >= start_minutes or current < end_minutes

    @classmethod
    def run_weekly_digest(cls) -> int:
        """Send the weekly summary to every block that enabled the digest."""
        from zoneinfo import ZoneInfo

        sent_count = 0
        twilio = TwilioWhatsAppClient()
        telegram = TelegramBotClient()

        with Session(engine) as session:
            fields = session.exec(
                select(FieldModel).where(FieldModel.active).where(FieldModel.digest_enabled)
            ).all()

            for field in fields:
                try:
                    try:
                        tz = ZoneInfo(field.timezone or 'Europe/Berlin')
                    except Exception:
                        tz = ZoneInfo('Europe/Berlin')
                    local_now = datetime.now(tz)
                    if local_now.weekday() != int(field.digest_day or 0) or local_now.hour != int(field.digest_hour or 8):
                        continue

                    # One digest per week - guard with the dispatch log
                    today_start = local_now.replace(hour=0, minute=0, second=0, microsecond=0)
                    already_sent = session.exec(
                        select(DispatchLogModel)
                        .where(DispatchLogModel.field_id == field.id)
                        .where(DispatchLogModel.channel == 'digest')
                        .where(DispatchLogModel.created_at >= today_start)
                    ).first()
                    if already_sent:
                        continue

                    summary = cls._build_digest_summary(session, field)
                    message = WhatsAppFormatter.format_digest(field, summary)

                    if field.farmer_chat_id:
                        tg_text = to_telegram_html(message)
                        telegram.send_message(field.farmer_chat_id, tg_text)
                    if field.farmer_phone:
                        twilio.send_message(field.farmer_phone, message)

                    session.add(DispatchLogModel(
                        field_id=field.id, channel='digest', target=field.farmer_phone or '',
                        status='sent', attempts=1, detail=f'weekly digest {today_start.date()}'
                    ))
                    session.commit()
                    sent_count += 1
                    logger.info(f"[DIGEST] Weekly summary sent for '{field.name}'")
                except Exception as e:
                    logger.exception(f"[DIGEST FAILED] Field: {field.name} | Error: {e}")
                    session.rollback()

        return sent_count

    @classmethod
    def _build_digest_summary(cls, session: Session, field: FieldModel) -> dict:
        """Collect the numbers of the last seven days."""
        since = datetime.now(timezone.utc) - timedelta(days=7)
        events = session.exec(
            select(IrrigationEventModel)
            .where(IrrigationEventModel.field_id == field.id)
            .where(IrrigationEventModel.timestamp >= since)
        ).all()
        alerts = session.exec(
            select(AdvisoryAlertModel)
            .where(AdvisoryAlertModel.field_id == field.id)
            .where(AdvisoryAlertModel.timestamp >= since)
        ).all()
        sensor = session.exec(select(SensorModel).where(SensorModel.field_id == field.id)).first()
        moisture = sensor.last_moisture_pct if sensor else 0.0
        status = WaterBalanceEngine.analyze_soil_status(
            current_moisture_pct=moisture,
            crop_key=field.crop_key,
            growth_stage=field.growth_stage,
            soil_key=field.soil_key
        )
        return {
            'moisture_pct': moisture,
            'threshold_pct': status.threshold_moisture_pct,
            'applied_net_depth_mm': round(sum(e.depth_applied_mm for e in events), 1),
            'irrigations': len(events),
            'et0_mm': round(sum(a.forecast_et0_mm for a in alerts), 1),
            'rain_mm': round(sum(a.forecast_rain_mm for a in alerts), 1)
        }

    @classmethod
    def run_escalation_check(cls) -> int:
        """Notify the escalation contact about critical advisories that stay unconfirmed."""
        escalated_count = 0
        twilio = TwilioWhatsAppClient()

        with Session(engine) as session:
            fields = session.exec(
                select(FieldModel)
                .where(FieldModel.active)
                .where(FieldModel.escalation_phone.is_not(None))
            ).all()

            now = datetime.now(timezone.utc)
            for field in fields:
                try:
                    age_limit = now - timedelta(hours=settings.ESCALATION_AFTER_HOURS)
                    alerts = session.exec(
                        select(AdvisoryAlertModel)
                        .where(AdvisoryAlertModel.field_id == field.id)
                        .where(AdvisoryAlertModel.status == 'PENDING')
                        .where(AdvisoryAlertModel.urgency == 'CRITICAL')
                        .where(AdvisoryAlertModel.timestamp <= age_limit)
                        .order_by(AdvisoryAlertModel.timestamp.desc())
                    ).all()

                    for alert in alerts:
                        already = session.exec(
                            select(DispatchLogModel)
                            .where(DispatchLogModel.alert_id == alert.id)
                            .where(DispatchLogModel.channel == 'escalation')
                        ).first()
                        if already:
                            continue

                        sensor = session.exec(select(SensorModel).where(SensorModel.field_id == field.id)).first()
                        moisture = sensor.last_moisture_pct if sensor else 0.0
                        status = WaterBalanceEngine.analyze_soil_status(
                            current_moisture_pct=moisture,
                            crop_key=field.crop_key,
                            growth_stage=field.growth_stage,
                            soil_key=field.soil_key
                        )
                        age_hours = int(hours_between(now, alert.timestamp))
                        summary = {
                            'age_hours': age_hours,
                            'reason': 'critical moisture deficit, recommendation not confirmed',
                            'moisture_pct': moisture,
                            'threshold_pct': status.threshold_moisture_pct,
                            'action': f'Water {alert.recommended_gross_mm:.1f} mm at {alert.recommended_start_time}'
                        }
                        message = WhatsAppFormatter.format_escalation(field, summary)
                        outcome = twilio.send_message(field.escalation_phone, message)

                        alert.status = 'ESCALATED'
                        session.add(alert)
                        session.add(DispatchLogModel(
                            field_id=field.id, alert_id=alert.id, channel='escalation',
                            target=field.escalation_phone, status=outcome.get('status', 'sent'),
                            attempts=1, detail=f'escalated after {age_hours} h'
                        ))
                        session.commit()
                        escalated_count += 1
                        logger.info(
                            f"[ESCALATION] Field '{field.name}' escalated to {field.escalation_phone} "
                            f"after {age_hours} h"
                        )
                except Exception as e:
                    logger.exception(f"[ESCALATION FAILED] Field: {field.name} | Error: {e}")
                    session.rollback()

        return escalated_count

    @classmethod
    def _dispatch_fields(
        cls,
        session: Session,
        target_hour: Optional[int],
        force: bool,
        now_utc: datetime,
        twilio: TwilioWhatsAppClient,
        telegram: TelegramBotClient
    ) -> int:
        """Evaluate every eligible field and dispatch its advisory. Returns the count."""
        from zoneinfo import ZoneInfo

        dispatched_count = 0
        statement = select(FieldModel).where(FieldModel.active)
        if target_hour is not None:
            statement = statement.where(FieldModel.daily_alert_hour == target_hour)
        fields = session.exec(statement).all()

        for field in fields:
            try:
                # When running in live background mode (target_hour is None), check field's local wall-clock hour
                if target_hour is None:
                    try:
                        tz = ZoneInfo(field.timezone or 'Europe/Berlin')
                        local_now = datetime.now(tz)
                    except Exception:
                        local_now = now_utc
                    if local_now.hour != field.daily_alert_hour:
                        continue
                    # Quiet hours: the grower asked for silence in this window.
                    if cls._in_quiet_hours(field, local_now):
                        logger.info(f"[SCHEDULER SKIPPED] Field '{field.name}' is in quiet hours")
                        continue
                # Skip if paused
                if not force and field.paused_until and as_utc(field.paused_until) > now_utc:
                    logger.info(f"[SCHEDULER SKIPPED] Field: {field.name} is paused until {field.paused_until}")
                    continue

                # Idempotency guard: prevent duplicate dispatch if already evaluated within the past 20 hours
                if not force:
                    cutoff_time = now_utc - timedelta(hours=20)
                    existing_recent_alert = session.exec(
                        select(AdvisoryAlertModel)
                        .where(AdvisoryAlertModel.field_id == field.id)
                        .where(AdvisoryAlertModel.timestamp >= cutoff_time)
                    ).first()
                    if existing_recent_alert:
                        logger.info(f"[SCHEDULER IDEMPOTENT SKIP] Field: {field.name} already dispatched recently")
                        continue

                sensors = session.exec(select(SensorModel).where(SensorModel.field_id == field.id)).all()
                sensor = sensors[0] if sensors else None
                current_moisture = sensor.last_moisture_pct if sensor else 22.0
                stale_hours = hours_since_reading(sensor)
                zone = None
                valve_id = None

                # Zone level advisory: the driest zone of the block decides.
                critical = AgronomicAdvisor.select_critical_zone(field, sensors)
                if critical:
                    _, zone, valve_id, current_moisture, stale_hours = critical

                weather = WeatherService.get_forecast(field.latitude, field.longitude)
                decision = AgronomicAdvisor.evaluate_field(
                    field, current_moisture, weather,
                    hours_since_reading=stale_hours,
                    zone=zone,
                    valve_id=valve_id
                )
                message_text = WhatsAppFormatter.format_daily_advisory(field, decision, weather)

                alert = AdvisoryAlertModel(
                    field_id=field.id,
                    timestamp=datetime.now(timezone.utc),
                    soil_moisture_pct=current_moisture,
                    threshold_pct=decision.soil_status.threshold_moisture_pct,
                    forecast_et0_mm=weather.et0_tomorrow_mm,
                    forecast_rain_mm=weather.rain_next_48h_mm,
                    recommended_depth_mm=decision.target_depth_mm,
                    recommended_gross_mm=decision.gross_depth_mm,
                    recommended_duration_min=decision.duration_minutes,
                    recommended_start_time=decision.optimal_start_time,
                    zone=decision.zone,
                    urgency=decision.urgency,
                    status='PENDING' if decision.should_irrigate else 'AUTO_LOGGED',
                    message_text=message_text
                )
                session.add(alert)
                session.commit()
                session.refresh(alert)

                # Deliver through every configured channel; every attempt is persisted
                # and transient failures are retried.
                dispatch_result = AdvisoryDispatcher.dispatch(
                    session=session, field=field, alert=alert, decision=decision, weather=weather
                )

                # Optional valve actuation (off by default - the controller is simulated
                # unless VALVE_CONTROLLER_URL is configured).
                if settings.AUTO_ACTUATION_ENABLED:
                    plan = build_plan(field, decision)
                    if plan:
                        execute_plan(session, plan, alert)
                    else:
                        logger.info(f"[SCHEDULER] No valve plan for '{field.name}' - nothing to actuate")

                dispatched_count += 1
                logger.info(
                    f"[SCHEDULED DISPATCH] Field: {field.name} -> Phone: {field.farmer_phone} | "
                    f"Chat: {field.farmer_chat_id} | attempts: {dispatch_result.attempts} | "
                    f"failures: {dispatch_result.failures}"
                )
            except Exception as e:
                logger.exception(f"[SCHEDULED DISPATCH FAILED] Field: {field.name} | Error: {e}")

        return dispatched_count

