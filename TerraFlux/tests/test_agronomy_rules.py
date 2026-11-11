"""Tests for the agronomy rules added in P2: RDI, infiltration limit and rain/ET integration."""
import pytest

from src.core.advisor import AgronomicAdvisor, WeatherForecastSummary
from src.core.models import FieldModel
from src.core.water_balance import WaterBalanceEngine


def _weather(**overrides):
    base = dict(
        et0_today_mm=4.8,
        et0_tomorrow_mm=5.0,
        rain_next_24h_mm=0.0,
        rain_next_48h_mm=0.0,
        rain_next_72h_mm=0.0,
        max_temp_tomorrow_c=28.0,
        solar_radiation_mj=22.0
    )
    base.update(overrides)
    return WeatherForecastSummary(**base)


def _field(**overrides):
    base = dict(
        name='Test Vineyard',
        crop_key='grapevine',
        growth_stage='mid_season',
        soil_key='sandy_loam',
        drip_emitter_spacing_m=0.5,
        drip_emitter_lph=2.0,
        drip_line_spacing_m=2.0,
        irrigation_efficiency=0.90
    )
    base.update(overrides)
    return FieldModel(**base)


# --------------------------------------------------------------------------------------
# #17 Regulated Deficit Irrigation
# --------------------------------------------------------------------------------------

def test_rdi_lowers_the_threshold_in_the_late_season():
    mid_status = WaterBalanceEngine.analyze_soil_status(17.0, 'grapevine', 'mid_season', 'sandy_loam')
    late_status = WaterBalanceEngine.analyze_soil_status(17.0, 'grapevine', 'late_season', 'sandy_loam',
                                                          depletion_fraction_override=0.60)
    # RDI allows the crop to use more of the readily available water before it is stressed
    assert late_status.threshold_moisture_pct < mid_status.threshold_moisture_pct


def test_rdi_is_announced_in_the_advisory_reason():
    decision = AgronomicAdvisor.evaluate_field(_field(growth_stage='late_season'), 17.5, _weather())
    assert decision.should_irrigate is False
    assert 'RDI' in decision.reason
    assert 'Regulated Deficit Irrigation' in decision.reason


def test_no_rdi_in_the_mid_season():
    decision = AgronomicAdvisor.evaluate_field(_field(growth_stage='mid_season'), 17.5, _weather())
    assert 'RDI' not in decision.reason


def test_crops_without_deficit_strategy_are_unaffected():
    decision = AgronomicAdvisor.evaluate_field(
        _field(crop_key='maize', growth_stage='late_season', soil_key='silt_loam'), 17.5, _weather()
    )
    assert 'RDI' not in decision.reason


# --------------------------------------------------------------------------------------
# #17 Soil infiltration limit
# --------------------------------------------------------------------------------------

def test_recommendation_is_capped_by_the_infiltration_rate():
    # Clay loam infiltrates at 10 mm/h -> at most 60 mm in a 6 h window
    field = _field(soil_key='clay_loam')
    decision = AgronomicAdvisor.evaluate_field(field, 4.0, _weather())
    assert decision.should_irrigate is True
    assert decision.target_depth_mm <= 60.0
    assert 'Split over' in decision.reason


def test_run_time_never_exceeds_the_irrigation_window():
    """A valve run has to be actionable - the recommendation is split into cycles."""
    from src.core.advisor import MAX_IRRIGATION_WINDOW_HOURS
    field = _field()  # drip: 2.0 mm/h gross, 1.8 mm/h net
    decision = AgronomicAdvisor.evaluate_field(field, 4.0, _weather())
    assert decision.should_irrigate is True
    assert decision.duration_minutes <= MAX_IRRIGATION_WINDOW_HOURS * 60
    assert 'Split over' in decision.reason


def test_small_deficit_fits_into_one_cycle():
    # A modern drip (8 L/h over 0.5 m2 = 16 mm/h gross) delivers the deficit in one run
    field = _field(drip_emitter_lph=8.0, drip_emitter_spacing_m=0.5, drip_line_spacing_m=1.0)
    decision = AgronomicAdvisor.evaluate_field(field, 16.5, _weather())
    assert decision.should_irrigate is True
    assert 'Split over' not in decision.reason
    assert decision.duration_minutes <= 360


def test_runoff_is_flagged_when_the_application_rate_exceeds_infiltration():
    # 6 L/h over 0.5 m2 = 12 mm/h gross on a clay loam (10 mm/h infiltration)
    field = _field(soil_key='clay_loam', drip_emitter_lph=6.0, drip_emitter_spacing_m=0.5, drip_line_spacing_m=1.0)
    decision = AgronomicAdvisor.evaluate_field(field, 15.0, _weather())
    assert 'runoff' in decision.reason.lower()


def test_no_runoff_warning_when_the_rate_is_below_infiltration():
    decision = AgronomicAdvisor.evaluate_field(_field(), 15.0, _weather())
    assert 'runoff' not in decision.reason.lower()


# --------------------------------------------------------------------------------------
# #18 Rain / ET integration
# --------------------------------------------------------------------------------------

def test_stale_reading_is_corrected_for_crop_demand():
    # 12 h of crop demand dries the root zone
    decision = AgronomicAdvisor.evaluate_field(_field(), 18.0, _weather(), hours_since_reading=12)
    assert 'corrected for 12 h' in decision.reason
    assert decision.soil_status.current_moisture_pct < 18.0


def test_rain_offsets_the_crop_demand():
    dry = AgronomicAdvisor.evaluate_field(_field(), 18.0, _weather(rain_next_24h_mm=0.0), hours_since_reading=12)
    wet = AgronomicAdvisor.evaluate_field(_field(), 18.0, _weather(rain_next_24h_mm=8.0), hours_since_reading=12)
    assert wet.soil_status.current_moisture_pct > dry.soil_status.current_moisture_pct


def test_fresh_readings_are_not_corrected():
    decision = AgronomicAdvisor.evaluate_field(_field(), 18.0, _weather(), hours_since_reading=0)
    assert decision.soil_status.current_moisture_pct == 18.0
    assert 'corrected for' not in decision.reason


def test_moisture_adjustment_helper():
    # 3.6 mm/day ETc over 12 h = 1.8 mm -> 0.15 % VWC for a 1.2 m root zone
    adjusted = WaterBalanceEngine.adjust_moisture_for_weather(
        current_moisture_pct=20.0, hours_since_reading=12, etc_day_mm=3.6,
        rain_mm=0.0, root_depth_m=1.2
    )
    assert adjusted == pytest.approx(19.85, abs=0.01)

    # Rain cancels the loss
    wet = WaterBalanceEngine.adjust_moisture_for_weather(
        current_moisture_pct=20.0, hours_since_reading=12, etc_day_mm=3.6,
        rain_mm=2.0, root_depth_m=1.2
    )
    assert wet > adjusted


def test_rain_covering_the_deficit_suppresses_irrigation():
    # A dry soil with a deficit of ~25 mm and 25 mm of rain in the next 72 h
    decision = AgronomicAdvisor.evaluate_field(
        _field(), 19.5, _weather(rain_next_24h_mm=5.0, rain_next_48h_mm=12.0, rain_next_72h_mm=25.0)
    )
    assert decision.should_irrigate is False
    assert 'Suppressed' in decision.reason
    assert '% of the deficit' in decision.reason


def test_dry_weather_still_irrigates():
    decision = AgronomicAdvisor.evaluate_field(_field(), 14.0, _weather())
    assert decision.should_irrigate is True
