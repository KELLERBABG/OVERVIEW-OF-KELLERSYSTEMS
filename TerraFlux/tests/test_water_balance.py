from src.core.water_balance import WaterBalanceEngine

def test_kc_lookup():
    kc_grape = WaterBalanceEngine.get_crop_kc('grapevine', 'mid_season')
    assert kc_grape == 0.75
    
    kc_maize = WaterBalanceEngine.get_crop_kc('maize', 'mid_season')
    assert kc_maize == 1.15

def test_crop_etc_calculation():
    # ET0 = 5.0 mm, grapevine mid-season (Kc = 0.75) -> ETc = 3.75 mm
    etc = WaterBalanceEngine.calculate_crop_etc(5.0, 'grapevine', 'mid_season')
    assert round(etc, 2) == 3.75

def test_soil_water_status_analysis():
    # Sandy Loam: FC = 22%, WP = 10%. Grapevine: Zr = 1.2m, p = 0.45
    # TAW = 1000 * (0.22 - 0.10) * 1.2 = 144.0 mm
    # RAW = 0.45 * 144.0 = 64.8 mm
    # Threshold moisture % = 22 - (0.45 * 12) = 16.6%
    status = WaterBalanceEngine.analyze_soil_status(
        current_moisture_pct=15.0,
        crop_key='grapevine',
        growth_stage='mid_season',
        soil_key='sandy_loam'
    )
    assert status.field_capacity_pct == 22.0
    assert status.wilting_point_pct == 10.0
    assert status.taw_mm == 144.0
    assert status.raw_mm == 64.8
    assert status.threshold_moisture_pct == 16.6
    assert status.is_stressed is True
    assert status.stress_index > 0.0

def test_irrigation_duration_conversion():
    # Target depth 12mm with application rate 4.0 mm/h -> 3 hours = 180 min
    mins = WaterBalanceEngine.calculate_irrigation_duration_min(12.0, 4.0)
    assert mins == 180

def test_gross_net_depth_conversion_round_trip():
    """The farmer runs a GROSS volume; only the efficient share reaches the root zone."""
    gross = WaterBalanceEngine.calculate_gross_depth_mm(net_depth_mm=12.0, irrigation_efficiency=0.90)
    assert round(gross, 2) == 13.33

    net = WaterBalanceEngine.calculate_net_depth_mm(gross_depth_mm=gross, irrigation_efficiency=0.90)
    assert round(net, 2) == 12.0

    # A missing/zero efficiency must not explode or silently invert the conversion
    assert WaterBalanceEngine.calculate_gross_depth_mm(10.0, irrigation_efficiency=0) > 10.0
    assert WaterBalanceEngine.calculate_net_depth_mm(10.0, irrigation_efficiency=0) > 0
