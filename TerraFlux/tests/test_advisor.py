import pytest
from src.core.models import FieldModel
from src.core.advisor import AgronomicAdvisor, WeatherForecastSummary

@pytest.fixture
def sample_field():
    return FieldModel(
        name='Test Vineyard',
        crop_key='grapevine',
        growth_stage='mid_season',
        soil_key='sandy_loam',
        drip_emitter_spacing_m=0.5,
        drip_emitter_lph=2.0,
        drip_line_spacing_m=2.0,
        irrigation_efficiency=0.90
    )

def test_advisor_triggers_irrigation_when_depleted(sample_field):
    # Current moisture = 14% (below RAW threshold of 16.6%), rain = 0
    weather = WeatherForecastSummary(
        et0_today_mm=4.8,
        et0_tomorrow_mm=5.0,
        rain_next_24h_mm=0.0,
        rain_next_48h_mm=0.0,
        max_temp_tomorrow_c=28.0,
        solar_radiation_mj=22.0
    )
    decision = AgronomicAdvisor.evaluate_field(sample_field, 14.0, weather)
    assert decision.should_irrigate is True
    assert decision.target_depth_mm > 0.0
    assert decision.duration_minutes > 0
    assert decision.optimal_start_time in ('06:00', '07:00')

def test_advisor_reports_gross_and_net_depth(sample_field):
    """The advice must carry both the net refill and the gross valve setting."""
    weather = WeatherForecastSummary(
        et0_today_mm=4.8,
        et0_tomorrow_mm=5.0,
        rain_next_24h_mm=0.0,
        rain_next_48h_mm=0.0,
        max_temp_tomorrow_c=28.0,
        solar_radiation_mj=22.0
    )
    decision = AgronomicAdvisor.evaluate_field(sample_field, 14.0, weather)
    # 90% application efficiency -> the gross depth is larger than the net depth
    assert decision.gross_depth_mm > decision.target_depth_mm
    expected_gross = decision.target_depth_mm / sample_field.irrigation_efficiency
    assert round(decision.gross_depth_mm, 1) == round(expected_gross, 1)

    # Duration must be consistent with the net depth at the net application rate
    expected_minutes = int(round((decision.target_depth_mm / sample_field.application_rate_mm_h) * 60))
    assert decision.duration_minutes == expected_minutes

def test_advisor_rain_suppression(sample_field):
    # Moisture is slightly low (15%), but heavy rain (15mm) is forecast in 48h
    weather = WeatherForecastSummary(
        et0_today_mm=4.0,
        et0_tomorrow_mm=3.0,
        rain_next_24h_mm=10.0,
        rain_next_48h_mm=15.0,
        max_temp_tomorrow_c=22.0,
        solar_radiation_mj=14.0
    )
    decision = AgronomicAdvisor.evaluate_field(sample_field, 15.0, weather)
    assert decision.should_irrigate is False
    assert decision.rain_suppression is True
    assert 'rain' in decision.reason.lower()
