from src.simulation.bom_economics import BOMEconomicsModel

def test_bom_breakdown_cost():
    bom = BOMEconomicsModel.get_standard_bom()
    assert len(bom) >= 5
    total_mfg_cost = sum(c.unit_cost_eur for c in bom)
    # Total manufacturing BOM is around €55, allowing retail at €89
    assert 45.0 <= total_mfg_cost <= 65.0

def test_farmer_roi_calculation():
    # 20ha vineyard, 3 probes, €1.20/m3 water
    report = BOMEconomicsModel.evaluate_economics(
        farm_area_ha=20.0,
        probes_count=3,
        water_tariff_eur_m3=1.20,
        crop_type='grapevine'
    )
    assert report.kit_hardware_capex_eur == 3 * 89.0  # €267
    # Supplemental irrigation baseline of 450 m3/ha (not the 1,400 m3/ha gross demand)
    assert report.baseline_water_use_m3_yr == 20.0 * 450.0
    assert report.water_saved_m3_yr == 1620.0           # 18 % of 9,000 m3
    assert report.water_savings_eur_yr > 2000.0
    # Labour saved by not walking the blocks during the irrigation season
    assert report.labor_hours_saved_yr > 0
    assert report.labor_savings_eur_yr > 1000.0
    assert report.annual_financial_savings_eur > 3000.0
    assert report.payback_period_months < 3.0            # Payback in under 3 months
    assert report.three_year_roi_pct > 500.0


def test_water_savings_stay_inside_the_documented_band():
    """The model must not promise more than the FAO RDI literature supports."""
    report = BOMEconomicsModel.evaluate_economics(farm_area_ha=20.0, probes_count=3)
    assert 15.0 <= report.water_saved_pct <= 22.0
    assert report.water_saved_pct == 18.0


def test_cots_tier_and_gateway_are_priced():
    diy = BOMEconomicsModel.evaluate_economics(farm_area_ha=20.0, probes_count=3)
    cots = BOMEconomicsModel.evaluate_economics(farm_area_ha=20.0, probes_count=3, hardware_tier='cots')
    assert cots.kit_hardware_capex_eur > diy.kit_hardware_capex_eur
    assert diy.gateway_capex_eur == 0.0

    with_gateway = BOMEconomicsModel.evaluate_economics(farm_area_ha=20.0, probes_count=3, include_gateway=True)
    assert with_gateway.gateway_capex_eur == 185.0
    assert with_gateway.kit_hardware_capex_eur > diy.kit_hardware_capex_eur
