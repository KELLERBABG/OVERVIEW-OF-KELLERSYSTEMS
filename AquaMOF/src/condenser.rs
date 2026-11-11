//! Condensation and Psychrometric water yield recovery modeling.

use serde::{Deserialize, Serialize};

/// Condenser specifications and collection metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condenser {
    /// Condenser heat exchange efficiency (0.0 to 1.0).
    pub efficiency: f64,
    /// Ambient condenser temperature (°C).
    pub condenser_temp_c: f64,
    /// Accumulated liquid water yield in Liters.
    pub accumulated_liters: f64,
}

impl Condenser {
    pub fn new(efficiency: f64) -> Self {
        Self {
            efficiency,
            condenser_temp_c: 25.0,
            accumulated_liters: 0.0,
        }
    }

    /// Calculate dew point (°C) using Magnus-Tetens formula.
    pub fn dew_point_c(temp_c: f64, rh: f64) -> f64 {
        let a = 17.27;
        let b = 237.7;
        let alpha = ((a * temp_c) / (b + temp_c)) + rh.max(0.001).ln();
        (b * alpha) / (a - alpha)
    }

    /// Process desorbed vapor: condense to liquid water yield.
    pub fn process_vapor(&mut self, vapor_kg: f64, bed_temp_c: f64, ambient_temp_c: f64) -> f64 {
        self.condenser_temp_c = ambient_temp_c;
        if vapor_kg <= 0.0 {
            return 0.0;
        }

        // Condensation effectiveness increases with temperature difference (T_bed - T_amb)
        let delta_t = (bed_temp_c - ambient_temp_c).max(0.0);
        let thermal_factor = (delta_t / 40.0).clamp(0.5, 0.98);
        
        let condensed_kg = vapor_kg * self.efficiency * thermal_factor;
        let liters = condensed_kg; // 1 kg water = 1 Liter
        self.accumulated_liters += liters;
        liters
    }
}
