"""Tests for i18n, quiet hours, the weekly digest and the escalation path."""
from datetime import datetime, timedelta, timezone
from zoneinfo import ZoneInfo

from sqlmodel import Session, select

from src.api.database import engine
from src.api.scheduler import DailyAdvisoryScheduler
from src.bot.i18n import normalize_language, to_telegram_html, translate
from src.core.models import (
    AdvisoryAlertModel,
    DispatchLogModel,
    FieldModel,
    IrrigationEventModel,
    SensorModel,
)


def _field(**overrides):
    base = dict(name='i18n Field', crop_key='grapevine', growth_stage='mid_season',
                soil_key='sandy_loam', farmer_phone='+4915123456789', farmer_chat_id='900000001')
    base.update(overrides)
    return FieldModel(**base)


# --------------------------------------------------------------------------------------
# i18n
# --------------------------------------------------------------------------------------

def test_language_normalisation():
    assert normalize_language('de-DE') == 'de'
    assert normalize_language('DE') == 'de'
    assert normalize_language('fr') == 'en'
    assert normalize_language('') == 'en'
    assert normalize_language(None) == 'en'


def test_translate_falls_back_to_english():
    assert 'Bodenfeuchte' in translate('advisory.irrigate', 'de', field='X', scope='', crop='Wein',
                                        area=1, moisture='20', threshold='16', et0='4', rain='0',
                                        gross='12', net='11', time='06:00', duration='1h', reason='')
    # Unknown language -> English catalogue
    text = translate('advisory.no_water', 'fr', field='X', scope='', crop='Grape', area=1,
                      moisture='20', threshold='16', et0='4', rain='0', reason='ok')
    assert 'NO WATERING NEEDED' in text


def test_translate_returns_the_key_when_unknown():
    assert translate('does.not.exist', 'en') == 'does.not.exist'


def test_telegram_html_conversion():
    html = to_telegram_html('*Bold* and _italic_ and <script>')
    assert '<b>Bold</b>' in html
    assert '<i>italic</i>' in html
    assert '&lt;script&gt;' in html
    assert '<script>' not in html


def test_advisory_is_rendered_in_the_field_language():
    from src.bot.formatter import WhatsAppFormatter
    from src.core.advisor import AgronomicAdvisor, WeatherForecastSummary

    weather = WeatherForecastSummary(
        et0_today_mm=4.0, et0_tomorrow_mm=5.0, rain_next_24h_mm=0.0,
        rain_next_48h_mm=0.0, rain_next_72h_mm=0.0,
        max_temp_tomorrow_c=25.0, solar_radiation_mj=20.0
    )
    german = _field(language='de')
    decision = AgronomicAdvisor.evaluate_field(german, 14.0, weather)
    message = WhatsAppFormatter.format_daily_advisory(german, decision, weather)
    assert 'AKTION' in message
    assert 'Tropfschlauch' in message

    english = _field(language='en')
    decision_en = AgronomicAdvisor.evaluate_field(english, 14.0, weather)
    message_en = WhatsAppFormatter.format_daily_advisory(english, decision_en, weather)
    assert 'ACTION' in message_en


def test_telegram_advisory_is_html():
    from src.bot.formatter import WhatsAppFormatter
    from src.core.advisor import AgronomicAdvisor, WeatherForecastSummary

    weather = WeatherForecastSummary(
        et0_today_mm=4.0, et0_tomorrow_mm=5.0, rain_next_24h_mm=0.0,
        rain_next_48h_mm=0.0, rain_next_72h_mm=0.0,
        max_temp_tomorrow_c=25.0, solar_radiation_mj=20.0
    )
    field = _field(language='en')
    decision = AgronomicAdvisor.evaluate_field(field, 14.0, weather)
    text, keyboard = WhatsAppFormatter.format_telegram_advisory(field, decision, weather)
    assert '<b>' in text
    assert '&' not in text.replace('&amp;', '')  # no raw ampersands
    assert keyboard['inline_keyboard'][0][0]['callback_data'].startswith('DONE|')


# --------------------------------------------------------------------------------------
# Quiet hours
# --------------------------------------------------------------------------------------

def test_quiet_hours_window():
    field = _field(quiet_hours_start='22:00', quiet_hours_end='06:00')
    assert DailyAdvisoryScheduler._in_quiet_hours(field, datetime(2026, 1, 1, 23, 0)) is True
    assert DailyAdvisoryScheduler._in_quiet_hours(field, datetime(2026, 1, 1, 3, 0)) is True
    assert DailyAdvisoryScheduler._in_quiet_hours(field, datetime(2026, 1, 1, 12, 0)) is False


def test_quiet_hours_same_day_window():
    field = _field(quiet_hours_start='09:00', quiet_hours_end='17:00')
    assert DailyAdvisoryScheduler._in_quiet_hours(field, datetime(2026, 1, 1, 12, 0)) is True
    assert DailyAdvisoryScheduler._in_quiet_hours(field, datetime(2026, 1, 1, 8, 0)) is False
    assert DailyAdvisoryScheduler._in_quiet_hours(field, datetime(2026, 1, 1, 18, 0)) is False


def test_no_quiet_hours_configured():
    field = _field()
    assert DailyAdvisoryScheduler._in_quiet_hours(field, datetime(2026, 1, 1, 3, 0)) is False


# --------------------------------------------------------------------------------------
# Weekly digest
# --------------------------------------------------------------------------------------

def test_weekly_digest_is_sent_once_per_week(client, monkeypatch):
    # A block that is due for a digest right now
    tz = ZoneInfo('Europe/Berlin')
    now_local = datetime.now(tz)
    with Session(engine) as session:
        field = FieldModel(name='Digest Field', crop_key='grapevine', growth_stage='mid_season',
                           soil_key='sandy_loam', farmer_phone='+4990000001', farmer_chat_id='900009001',
                           digest_enabled=True, digest_day=now_local.weekday(), digest_hour=now_local.hour)
        session.add(field)
        session.commit()
        session.refresh(field)
        field_id = field.id
        session.add(SensorModel(deveui='DIGESTPROBE01', field_id=field_id, last_moisture_pct=18.0))
        session.add(IrrigationEventModel(field_id=field_id, timestamp=datetime.now(timezone.utc) - timedelta(days=1),
                                         depth_applied_mm=12.0, duration_minutes=120, source='whatsapp_reply'))
        session.commit()

    sent = DailyAdvisoryScheduler.run_weekly_digest()
    assert sent >= 1

    with Session(engine) as session:
        digests = session.exec(
            select(DispatchLogModel).where(DispatchLogModel.field_id == field_id)
            .where(DispatchLogModel.channel == 'digest')
        ).all()
        assert len(digests) == 1
        assert 'weekly digest' in (digests[0].detail or '')

    # A second run in the same hour must not send again
    assert DailyAdvisoryScheduler.run_weekly_digest() == 0


def test_digest_summary_collects_the_week():
    with Session(engine) as session:
        field = FieldModel(name='Digest Summary Field', crop_key='grapevine', growth_stage='mid_season',
                           soil_key='sandy_loam', digest_enabled=True)
        session.add(field)
        session.commit()
        session.refresh(field)
        session.add(SensorModel(deveui='DIGESTSUM01', field_id=field.id, last_moisture_pct=17.5))
        session.add(IrrigationEventModel(field_id=field.id, timestamp=datetime.now(timezone.utc),
                                         depth_applied_mm=10.0, duration_minutes=60, source='whatsapp_reply'))
        session.add(AdvisoryAlertModel(field_id=field.id, timestamp=datetime.now(timezone.utc),
                                       soil_moisture_pct=17.5, threshold_pct=16.6, forecast_et0_mm=4.0,
                                       forecast_rain_mm=1.0, recommended_depth_mm=10.0,
                                       recommended_duration_min=60, status='ACCEPTED', message_text='x'))
        session.commit()

        summary = DailyAdvisoryScheduler._build_digest_summary(session, field)
        assert summary['applied_net_depth_mm'] == 10.0
        assert summary['irrigations'] == 1
        assert summary['et0_mm'] == 4.0
        assert summary['rain_mm'] == 1.0
        assert summary['moisture_pct'] == 17.5


# --------------------------------------------------------------------------------------
# Escalation
# --------------------------------------------------------------------------------------

def test_unconfirmed_critical_advisory_is_escalated(client, monkeypatch):
    from src.core.config import settings
    monkeypatch.setattr(settings, 'ESCALATION_AFTER_HOURS', 6)

    with Session(engine) as session:
        field = FieldModel(name='Escalation Field', crop_key='grapevine', growth_stage='mid_season',
                           soil_key='sandy_loam', farmer_phone='+4990000002',
                           escalation_phone='+4990000003')
        session.add(field)
        session.commit()
        session.refresh(field)
        field_id = field.id
        session.add(SensorModel(deveui='ESCALATIONPROBE1', field_id=field_id, last_moisture_pct=11.0))
        session.add(AdvisoryAlertModel(
            field_id=field_id,
            timestamp=datetime.now(timezone.utc) - timedelta(hours=10),
            soil_moisture_pct=11.0, threshold_pct=16.6, forecast_et0_mm=5.0, forecast_rain_mm=0.0,
            recommended_depth_mm=25.0, recommended_gross_mm=27.5, recommended_duration_min=300,
            recommended_start_time='06:00', urgency='CRITICAL', status='PENDING', message_text='critical'
        ))
        session.commit()

    escalated = DailyAdvisoryScheduler.run_escalation_check()
    assert escalated >= 1

    with Session(engine) as session:
        alert = session.exec(
            select(AdvisoryAlertModel).where(AdvisoryAlertModel.field_id == field_id)
        ).first()
        assert alert.status == 'ESCALATED'
        logs = session.exec(
            select(DispatchLogModel).where(DispatchLogModel.channel == 'escalation')
        ).all()
        assert len(logs) >= 1
        assert logs[-1].target == '+4990000003'

    # A second run must not escalate the same alert again
    assert DailyAdvisoryScheduler.run_escalation_check() == 0


def test_acknowledged_alert_is_not_escalated(client, monkeypatch):
    from src.core.config import settings
    monkeypatch.setattr(settings, 'ESCALATION_AFTER_HOURS', 6)

    with Session(engine) as session:
        field = FieldModel(name='Escalation Ack Field', crop_key='grapevine', growth_stage='mid_season',
                           soil_key='sandy_loam', escalation_phone='+4990000004')
        session.add(field)
        session.commit()
        session.refresh(field)
        session.add(AdvisoryAlertModel(
            field_id=field.id, timestamp=datetime.now(timezone.utc) - timedelta(hours=10),
            soil_moisture_pct=11.0, threshold_pct=16.6, forecast_et0_mm=5.0, forecast_rain_mm=0.0,
            recommended_depth_mm=25.0, recommended_duration_min=300,
            urgency='CRITICAL', status='ACCEPTED', message_text='done'
        ))
        session.commit()

    assert DailyAdvisoryScheduler.run_escalation_check() == 0
