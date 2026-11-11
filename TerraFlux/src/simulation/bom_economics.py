"""Hardware Bill of Materials (BOM) & Farmer ROI Economics Calculator."""
from dataclasses import dataclass
from typing import Dict, Any, List

# Labour assumptions for the ROI model (a grower walking the blocks during irrigation)
LABOR_HOURS_PER_BLOCK_WEEK = 1.5      # hours per block and week
IRRIGATION_SEASON_WEEKS = 20          # weeks with active irrigation
LABOR_COST_PER_HOUR_EUR = 28.0        # fully loaded hourly cost of vineyard labour

@dataclass
class BOMComponent:
    category: str
    item_name: str
    part_number_ref: str
    unit_cost_eur: float
    supplier_notes: str

@dataclass
class EconomicsReport:
    farm_area_ha: float
    probes_installed: int
    kit_hardware_capex_eur: float
    annual_subscription_eur: float
    total_year1_cost_eur: float
    
    baseline_water_use_m3_yr: float
    water_tariff_eur_m3: float
    pumping_energy_tariff_eur_kwh: float
    baseline_water_bill_eur_yr: float
    
    water_saved_pct: float
    water_saved_m3_yr: float
    annual_financial_savings_eur: float
    net_first_year_gain_eur: float
    payback_period_months: float
    three_year_roi_pct: float
    bom_breakdown: List[Dict[str, Any]]
    gateway_capex_eur: float = 0.0
    hardware_tier: str = 'diy'
    # Manufacturing cost of a single kit (sum of the BOM components).
    bom_unit_cost_eur: float = 0.0
    # Labour saved by not walking the blocks to check valves and pressure.
    labor_hours_saved_yr: float = 0.0
    labor_savings_eur_yr: float = 0.0
    water_savings_eur_yr: float = 0.0


class BOMEconomicsModel:
    """Calculates hardware feasibility, BOM margins, and farm financial ROI."""

    @classmethod
    def get_standard_bom(cls) -> List[BOMComponent]:
        """Hardware Bill of Materials for €89 retail target kit."""
        return [
            BOMComponent(
                category='Sensor Probe',
                item_name='Capacitive Soil Moisture + Temp Probe (RS485 Modbus / Analog)',
                part_number_ref='RS-WS-N01-TR / Dragino probe replacement',
                unit_cost_eur=16.50,
                supplier_notes='Epoxy sealed, 316 stainless steel prongs, 2m UV-resistant cable'
            ),
            BOMComponent(
                category='MCU & Radio',
                item_name='LoRaWAN Transceiver Node (EU868 / US915) with STM32/ESP32',
                part_number_ref='Seeed Wio-E5 / Ra-08H / ESP32-S3-LoRa',
                unit_cost_eur=12.20,
                supplier_notes='Ultra-low sleep current (<15uA), integrated IPEX antenna'
            ),
            BOMComponent(
                category='Power System',
                item_name='LiFePO4 18650 Battery (3.2V 1500mAh) + TP5000 Charge IC',
                part_number_ref='IFR18650-1500 / TP5000 Solar BMS',
                unit_cost_eur=4.80,
                supplier_notes='Wide temperature range (-20C to +60C), 2000+ cycle life'
            ),
            BOMComponent(
                category='Energy Harvesting',
                item_name='0.6W Monocrystalline Mini Solar Panel (5V 120mA)',
                part_number_ref='ETFE Laminated 80x60mm Panel',
                unit_cost_eur=3.40,
                supplier_notes='UV-resistant ETFE coating for 5+ years outdoor longevity'
            ),
            BOMComponent(
                category='Enclosure & Mount',
                item_name='IP67 Waterproof Enclosure + PG7 Gland + 60cm Fiberglass Ground Stake',
                part_number_ref='ABS IP67 Weatherproof Box + Ground Anchor',
                unit_cost_eur=8.50,
                supplier_notes='Pre-drilled with breathable waterproof vent ePTFE membrane'
            ),
            BOMComponent(
                category='Manufacturing & Assembly',
                item_name='PCB Fabrication, SMT Assembly, Conformal Coating, Testing & Packaging',
                part_number_ref='PCBA + Calibration Jig + Retail Eco Box',
                unit_cost_eur=9.60,
                supplier_notes='Silicone conformal coating against condensation'
            )
        ]

    @classmethod
    def evaluate_economics(
        cls,
        farm_area_ha: float = 20.0,
        probes_count: int = 3,
        water_tariff_eur_m3: float = 1.20, # €/m3 (typical in DACH/Southern Europe commercial irrigation)
        crop_type: str = 'grapevine',
        annual_sub_per_probe_eur: float = 60.0, # €5/month per probe
        include_gateway: bool = False,
        hardware_tier: str = 'diy'
    ) -> EconomicsReport:
        """Compute full financial ROI and payback timeline for the grower."""
        bom_list = cls.get_standard_bom()
        raw_bom_cost = sum(item.unit_cost_eur for item in bom_list) # ~€55 manufacturing BOM
        retail_kit_price = 195.0 if hardware_tier == 'cots' else 89.0
        gateway_capex = 185.0 if include_gateway else 0.0

        # Total capex
        hardware_capex = (probes_count * retail_kit_price) + gateway_capex
        annual_subscription = probes_count * annual_sub_per_probe_eur
        year1_cost = hardware_capex + annual_subscription

        # Supplemental irrigation baseline (m3 per hectare and year).
        # Grapevine in a warm DACH/Rhine/Burgenland block: 120-180 mm of supplemental
        # irrigation over the season = 450 m3/ha. The older 1,400 m3/ha figure was a
        # gross water demand, not supplemental irrigation, and inflated every ROI.
        # Maize / apple / olive are gross crop irrigation and kept as literature values.
        crop_water_intensities = {
            'grapevine': 450.0,   # m3/ha supplemental irrigation
            'maize': 3200.0,      # m3/ha gross crop irrigation
            'apple': 2600.0,      # m3/ha gross crop irrigation
            'olive': 1100.0       # m3/ha supplemental irrigation
        }
        baseline_m3_ha = crop_water_intensities.get(crop_type, 1500.0)
        total_baseline_m3 = farm_area_ha * baseline_m3_ha

        # Extraction and pumping cost per m3 (groundwater fees + energy, DACH reference)
        pumping_cost_m3 = 0.45
        effective_water_cost_m3 = water_tariff_eur_m3 + pumping_cost_m3
        baseline_annual_water_bill = total_baseline_m3 * effective_water_cost_m3

        # Water & power savings percentage from precision timing and rain suppression.
        # Proven field savings across empirical FAO RDI studies: 15% - 22%. The model
        # uses the conservative end of that band.
        water_saved_pct = 18.0
        water_saved_m3 = total_baseline_m3 * (water_saved_pct / 100.0)
        water_savings_eur = water_saved_m3 * effective_water_cost_m3

        # Labour saved: walking the blocks to check valves and pressure during the
        # irrigation season. One block per ~12.5 ha, 1.5 h per week over 20 weeks.
        blocks = max(1, round(farm_area_ha / 12.5))
        labor_hours_saved = round(blocks * LABOR_HOURS_PER_BLOCK_WEEK * IRRIGATION_SEASON_WEEKS, 1)
        labor_savings_eur = labor_hours_saved * LABOR_COST_PER_HOUR_EUR

        annual_financial_savings = water_savings_eur + labor_savings_eur

        net_first_year_gain = annual_financial_savings - year1_cost

        # Payback period (months into season)
        if annual_financial_savings > 0:
            payback_months = (year1_cost / annual_financial_savings) * 12.0
        else:
            payback_months = 999.0

        # 3-year ROI = (3 * annual_savings - (hardware_capex + 3 * annual_sub)) / (hardware_capex + 3 * annual_sub) * 100
        three_year_cost = hardware_capex + (3 * annual_subscription)
        three_year_savings = 3 * annual_financial_savings
        three_year_roi = ((three_year_savings - three_year_cost) / max(1.0, three_year_cost)) * 100.0

        bom_data = [
            {
                'category': c.category,
                'item_name': c.item_name,
                'part_number': c.part_number_ref,
                'unit_cost_eur': c.unit_cost_eur,
                'notes': c.supplier_notes
            }
            for c in bom_list
        ]

        return EconomicsReport(
            farm_area_ha=farm_area_ha,
            probes_installed=probes_count,
            kit_hardware_capex_eur=round(hardware_capex, 2),
            annual_subscription_eur=round(annual_subscription, 2),
            total_year1_cost_eur=round(year1_cost, 2),
            baseline_water_use_m3_yr=round(total_baseline_m3, 0),
            water_tariff_eur_m3=round(water_tariff_eur_m3, 2),
            pumping_energy_tariff_eur_kwh=round(pumping_cost_m3, 2),
            baseline_water_bill_eur_yr=round(baseline_annual_water_bill, 2),
            water_saved_pct=round(water_saved_pct, 1),
            water_saved_m3_yr=round(water_saved_m3, 0),
            annual_financial_savings_eur=round(annual_financial_savings, 2),
            net_first_year_gain_eur=round(net_first_year_gain, 2),
            payback_period_months=round(payback_months, 1),
            three_year_roi_pct=round(three_year_roi, 1),
            bom_breakdown=bom_data,
            gateway_capex_eur=round(gateway_capex, 2),
            hardware_tier=hardware_tier,
            bom_unit_cost_eur=round(raw_bom_cost, 2),
            labor_hours_saved_yr=labor_hours_saved,
            labor_savings_eur_yr=round(labor_savings_eur, 2),
            water_savings_eur_yr=round(water_savings_eur, 2)
        )
