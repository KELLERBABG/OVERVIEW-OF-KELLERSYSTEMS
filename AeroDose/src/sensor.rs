//! Sensirion SPS30 Optical Particle Sensor Driver & Calibration.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sps30Reading {
    pub pm1_0_ug_m3: f64,
    pub pm2_5_ug_m3: f64,
    pub pm4_0_ug_m3: f64,
    pub pm10_0_ug_m3: f64,
    pub typical_particle_size_um: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sps30Calibrator {
    pub slope_correction: f64,
    pub zero_offset_ug_m3: f64,
    pub rh_humidity_correction_factor: f64,
}

impl Sps30Calibrator {
    pub fn new() -> Self {
        Self {
            slope_correction: 1.02,
            zero_offset_ug_m3: 0.5,
            rh_humidity_correction_factor: 1.0,
        }
    }

    /// Apply hygroscopic growth and laser calibration correction:
    /// C_corr = (C_raw * slope + offset) / (1 + kappa * (RH / (1 - RH)))
    pub fn calibrate(&self, raw: &Sps30Reading, ambient_rh_pct: f64) -> f64 {
        let rh_fraction = (ambient_rh_pct / 100.0).clamp(0.0, 0.95);
        
        // Kappa-Kohler hygroscopic growth factor for wildfire smoke
        let hygroscopic_growth = 1.0 + 0.25 * (rh_fraction / (1.0 - rh_fraction)).min(3.0);
        
        let raw_adjusted = raw.pm2_5_ug_m3 * self.slope_correction + self.zero_offset_ug_m3;
        (raw_adjusted / hygroscopic_growth).max(0.0)
    }
}
