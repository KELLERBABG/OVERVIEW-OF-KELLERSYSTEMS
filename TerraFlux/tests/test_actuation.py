"""Tests for the actuation layer (valve plans, simulated controller, audit trail)."""
from sqlmodel import Session, select

from src.api.database import engine
from src.core.advisor import AgronomicAdvisor, WeatherForecastSummary
from src.core.models import ActuationEventModel, FieldModel, SensorModel
from src.actuation import build_plan, execute_plan, get_valve_controller
from src.actuation.controller import HttpValveController, SimulatedValveController


def _weather(**overrides):
    base = dict(et0_today_mm=4.8, et0_tomorrow_mm=5.0, rain_next_24h_mm=0.0,
                rain_next_48h_mm=0.0, rain_next_72h_mm=0.0,
                max_temp_tomorrow_c=28.0, solar_radiation_mj=22.0)
    base.update(overrides)
    return WeatherForecastSummary(**base)


def _field(**overrides):
    base = dict(name='Actuation Vineyard', crop_key='grapevine', growth_stage='mid_season',
                soil_key='sandy_loam', drip_emitter_spacing_m=0.5, drip_emitter_lph=2.0,
                drip_line_spacing_m=2.0, irrigation_efficiency=0.90)
    base.update(overrides)
    return FieldModel(**base)


def test_plan_is_built_from_a_decision():
    field = _field()
    decision = AgronomicAdvisor.evaluate_field(field, 14.0, _weather(), zone='B2', valve_id='V4')
    plan = build_plan(field, decision)
    assert plan is not None
    assert plan.zone == 'B2'
    assert plan.valve_id == 'V4'
    assert plan.duration_minutes == decision.duration_minutes
    assert plan.gross_depth_mm == decision.gross_depth_mm
    assert plan.net_depth_mm == decision.target_depth_mm
    assert plan.start_time in ('06:00', '07:00')
    assert plan.to_dict()['valve_id'] == 'V4'


def test_no_plan_when_nothing_has_to_be_done():
    field = _field()
    decision = AgronomicAdvisor.evaluate_field(field, 21.0, _weather())
    assert build_plan(field, decision) is None


def test_simulated_controller_is_the_default():
    controller = get_valve_controller()
    assert isinstance(controller, SimulatedValveController)


def test_http_controller_is_used_when_configured(monkeypatch):
    from src.core.config import settings
    monkeypatch.setattr(settings, 'VALVE_CONTROLLER_URL', 'http://valve.local')
    controller = get_valve_controller()
    assert isinstance(controller, HttpValveController)


def test_execute_plan_records_an_audit_event():
    field = _field()
    decision = AgronomicAdvisor.evaluate_field(field, 14.0, _weather(), zone='A1', valve_id='V1')
    plan = build_plan(field, decision)

    with Session(engine) as session:
        result = execute_plan(session, plan)
        assert result['status'] == 'simulated'
        events = session.exec(select(ActuationEventModel)).all()
        assert len(events) >= 1
        latest = events[-1]
        assert latest.status == 'simulated'
        assert latest.zone == 'A1'
        assert latest.valve_id == 'V1'
        assert latest.gross_depth_mm == plan.gross_depth_mm


def test_actuation_endpoints(client, admin_headers):
    # A block that clearly needs water
    created = client.post('/api/v1/fields', headers=admin_headers, json={'name': 'Actuation API Block'}).json()
    field_id = created['id']

    # Attach a dry probe so the engine asks for water
    import uuid
    with Session(engine) as session:
        session.add(SensorModel(
            deveui=f'ACTPROBE{uuid.uuid4().hex[:8].upper()}',
            name='Actuation Probe', field_id=field_id, zone='A1', valve_id='V1',
            last_moisture_pct=14.0
        ))
        session.commit()

    resp = client.post(f'/api/v1/fields/{field_id}/actuation/plan', headers=admin_headers)
    assert resp.status_code == 200, resp.text
    data = resp.json()
    assert 'plan' in data
    assert data['action_required'] is True
    assert data['plan']['duration_minutes'] >= 0
    assert data['plan']['zone'] == 'A1'

    resp = client.get(f'/api/v1/fields/{field_id}/actuation', headers=admin_headers)
    assert resp.status_code == 200
    assert resp.json()['controller'] == 'simulated'
    assert len(resp.json()['events']) >= 1


def test_actuation_requires_authentication(client):
    resp = client.post('/api/v1/fields/1/actuation/plan')
    assert resp.status_code == 401
