//! AquaMOF Harvester — Core Library.

pub mod isotherm;
pub mod kinetics;
pub mod condenser;
pub mod economics;
pub mod telemetry;

pub use isotherm::{IsothermParams, MofType};
pub use kinetics::BedState;
pub use condenser::Condenser;
pub use economics::{BomCostBreakdown, SizingCalculator};
pub use telemetry::HarvesterTelemetry;

/// Complete 24-Hour Day-Night Cycle Harvester Simulator.
#[derive(Debug, Clone)]
pub struct AquaMofSimulator {
    pub params: IsothermParams,
    pub bed: BedState,
    pub condenser: Condenser,
}

impl AquaMofSimulator {
    pub fn new(mof_type: MofType, mof_mass_kg: f64) -> Self {
        let params = match mof_type {
            MofType::Mof303 => IsothermParams::mof303(),
            MofType::Mof801 => IsothermParams::mof801(),
            MofType::Custom => IsothermParams::mof303(),
        };

        Self {
            params,
            bed: BedState::new(mof_mass_kg, 20.0),
            condenser: Condenser::new(0.92),
        }
    }

    /// Simulate 24-hour diurnal cycle:
    /// - 12 hours night adsorption (e.g. 20°C, 25% RH)
    /// - 12 hours day solar desorption (e.g. 35°C, 800 W/m2 solar flux)
    pub fn run_daily_cycle(&mut self, night_rh: f64, night_temp_c: f64, peak_solar_w_m2: f64, day_temp_c: f64) -> f64 {
        // Phase 1: Night Adsorption (12h = 43200s, dt = 600s)
        for _ in 0..72 {
            self.bed.step_adsorption(&self.params, night_rh, night_temp_c, 600.0);
        }

        // Phase 2: Day Solar Desorption (12h, sinusoidal solar curve)
        for i in 0..72 {
            let hour_fraction = (i as f64) / 72.0;
            let solar = (hour_fraction * std::f64::consts::PI).sin().max(0.0) * peak_solar_w_m2;
            let vapor_kg = self.bed.step_desorption(&self.params, solar, day_temp_c, 600.0);
            self.condenser.process_vapor(vapor_kg, self.bed.bed_temp_c, day_temp_c);
        }

        self.condenser.accumulated_liters
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_daily_cycle_yield() {
        let mut sim = AquaMofSimulator::new(MofType::Mof303, 1.0); // 1 kg MOF
        // Arid condition: 25% night RH, 850 W/m2 solar day
        let yield_liters = sim.run_daily_cycle(0.25, 18.0, 850.0, 32.0);
        
        assert!(yield_liters >= 0.20, "Yield was {:.3} L", yield_liters);
    }

    #[test]
    fn test_bom_economics() {
        let bom_1 = SizingCalculator::calculate_bom(1, 1.0);
        let bom_500 = SizingCalculator::calculate_bom(500, 1.0);

        assert!(bom_500.total_bom_eur < bom_1.total_bom_eur);
        assert!(bom_500.total_bom_eur <= 125.0, "BOM at 500 units was {:.2}", bom_500.total_bom_eur);
        assert!(bom_500.gross_margin_percent >= 55.0);
    }
}
