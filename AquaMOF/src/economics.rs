//! Economic Bill of Materials (BOM) and Scaling Calculator.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BomCostBreakdown {
    pub scale_units: usize,
    pub mof_cost_eur: f64,
    pub heat_exchanger_eur: f64,
    pub polycarbonate_enclosure_eur: f64,
    pub condenser_tank_eur: f64,
    pub instrumentation_ble_eur: f64,
    pub assembly_labor_eur: f64,
    pub total_bom_eur: f64,
    pub retail_price_eur: f64,
    pub gross_margin_percent: f64,
}

pub struct SizingCalculator;

impl SizingCalculator {
    pub fn calculate_bom(scale_units: usize, mof_mass_kg_per_panel: f64) -> BomCostBreakdown {
        let scale_factor = match scale_units {
            1..=10 => 1.0,
            11..=100 => 0.75,
            101..=500 => 0.45,
            _ => 0.35,
        };

        let mof_price_per_kg = 120.0 * scale_factor;
        let mof_cost = mof_mass_kg_per_panel * mof_price_per_kg;
        let heat_exchanger = 38.0 * scale_factor;
        let enclosure = 28.0 * scale_factor;
        let condenser_tank = 18.0 * scale_factor;
        let ble = 12.0 * scale_factor;
        let assembly = 15.0 * scale_factor;

        let total_bom = mof_cost + heat_exchanger + enclosure + condenser_tank + ble + assembly;
        let retail_price = 299.0;
        let margin = ((retail_price - total_bom) / retail_price) * 100.0;

        BomCostBreakdown {
            scale_units,
            mof_cost_eur: mof_cost,
            heat_exchanger_eur: heat_exchanger,
            polycarbonate_enclosure_eur: enclosure,
            condenser_tank_eur: condenser_tank,
            instrumentation_ble_eur: ble,
            assembly_labor_eur: assembly,
            total_bom_eur: total_bom,
            retail_price_eur: retail_price,
            gross_margin_percent: margin,
        }
    }
}
