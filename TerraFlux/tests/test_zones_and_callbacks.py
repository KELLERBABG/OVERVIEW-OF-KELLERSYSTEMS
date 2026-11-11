"""Tests for zone level advisories and the scoped Telegram callback payloads."""
from sqlmodel import Session, select

from src.api.database import engine
from src.bot.formatter import WhatsAppFormatter
from src.bot.handler import _parse_action
from src.core.advisor import AgronomicAdvisor, WeatherForecastSummary
from src.core.models import FieldModel, IrrigationEventModel, SensorModel


def _weather(**overrides):
    base = dict(
        et0_today_mm=4.8, et0_tomorrow_mm=5.0, rain_next_24h_mm=0.0,
        rain_next_48h_mm=0.0, rain_next_72h_mm=0.0,
        max_temp_tomorrow_c=28.0, solar_radiation_mj=22.0
    )
    base.update(overrides)
    return WeatherForecastSummary(**base)


def _field(**overrides):
    base = dict(name='Zoned Vineyard', crop_key='grapevine', growth_stage='mid_season',
                soil_key='sandy_loam', drip_emitter_spacing_m=0.5, drip_emitter_lph=2.0,
                drip_line_spacing_m=2.0, irrigation_efficiency=0.90)
    base.update(overrides)
    return FieldModel(**base)


def _make_zone_field(phone: str = '+4915123456789') -> int:
    """Create a block with two zoned probes: A1 dry, A2 moist."""
    import uuid
    suffix = uuid.uuid4().hex[:8].upper()
    with Session(engine) as session:
        field = FieldModel(name='Zone Test Block', crop_key='grapevine', growth_stage='mid_season',
                           soil_key='sandy_loam', area_ha=4.0, farmer_phone=phone)
        session.add(field)
        session.commit()
        session.refresh(field)
        field_id = field.id
        session.add(SensorModel(deveui=f'ZONEPROBE{suffix}1', name='Probe A1', field_id=field_id,
                                zone='A1', valve_id='V1', last_moisture_pct=15.0))
        session.add(SensorModel(deveui=f'ZONEPROBE{suffix}2', name='Probe A2', field_id=field_id,
                                zone='A2', valve_id='V2', last_moisture_pct=21.0))
        session.commit()
    return field_id


def test_action_parsing():
    assert _parse_action('DONE|3|A1') == ('DONE', 3, 'A1')
    assert _parse_action('DONE') == ('DONE', None, None)
    assert _parse_action('STATUS|12|') == ('STATUS', 12, None)
    assert _parse_action('WATER 15MM|4|B2') == ('WATER 15MM', 4, 'B2')


def test_critical_zone_is_the_driest_one():
    field_id = _make_zone_field()
    with Session(engine) as session:
        field = session.get(FieldModel, field_id)
        sensors = session.exec(select(SensorModel).where(SensorModel.field_id == field_id)).all()
        critical = AgronomicAdvisor.select_critical_zone(field, sensors)
        assert critical is not None
        sensor, zone, valve_id, moisture, _hours = critical
        assert zone == 'A1'
        assert valve_id == 'V1'
        assert moisture == 15.0


def test_advisory_targets_the_critical_zone(client, admin_headers):
    field_id = _make_zone_field()
    resp = client.post(f'/api/v1/advisory/evaluate/{field_id}', headers=admin_headers)
    assert resp.status_code == 200, resp.text
    decision = resp.json()['decision']
    assert decision['zone'] == 'A1'
    assert decision['valve_id'] == 'V1'
    assert decision['should_irrigate'] is True

    zones = resp.json()['zones']
    assert [z['zone'] for z in zones] == ['A1', 'A2']
    assert zones[0]['is_stressed'] is True
    assert zones[1]['is_stressed'] is False


def test_telegram_buttons_carry_the_scope():
    field = _field()
    decision = AgronomicAdvisor.evaluate_field(field, 15.0, _weather(), zone='A1', valve_id='V1')
    text, keyboard = WhatsAppFormatter.format_telegram_advisory(field, decision, _weather())
    buttons = keyboard['inline_keyboard']
    assert buttons[0][0]['callback_data'] == f'DONE|{field.id}|A1'
    assert buttons[1][0]['callback_data'] == f'DELAY 1D|{field.id}|A1'
    assert buttons[1][1]['callback_data'] == f'STATUS|{field.id}|A1'
    assert 'zone A1' in text
    assert 'valve V1' in text


def test_scoped_callback_logs_the_zone(client):
    """A button press must be logged against the named block and zone."""
    field_id = _make_zone_field()
    payload = f'DONE|{field_id}|A1'
    resp = client.post('/api/v1/whatsapp/webhook', data={'From': '+4915123456789', 'Body': payload})
    assert resp.status_code == 200

    with Session(engine) as session:
        event = session.exec(
            select(IrrigationEventModel)
            .where(IrrigationEventModel.field_id == field_id)
            .order_by(IrrigationEventModel.timestamp.desc())
        ).first()
        assert event is not None
        assert event.zone == 'A1'


def test_scoped_callback_for_a_foreign_block_is_refused(client):
    """A grower may not confirm an action for a block that is not theirs."""
    other_field_id = _make_zone_field(phone='+436601234567')
    resp = client.post('/api/v1/whatsapp/webhook', data={'From': '+4915123456789', 'Body': f'DONE|{other_field_id}|A1'})
    assert resp.status_code == 200
    assert 'Unknown Block' in resp.json()['reply']

    with Session(engine) as session:
        events = session.exec(
            select(IrrigationEventModel).where(IrrigationEventModel.field_id == other_field_id)
        ).all()
        assert events == []


def test_plain_done_still_works(client):
    resp = client.post('/api/v1/whatsapp/webhook', data={'From': '+4915123456789', 'Body': 'DONE'})
    assert resp.status_code == 200
    assert 'Logged' in resp.json()['reply']
