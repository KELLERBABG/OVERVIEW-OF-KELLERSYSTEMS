//! Environmental & Barometric Flux Normalization

/// Corrects atmospheric pressure and thermal variations in muon flux.
/// Standard barometric coefficient beta_P = -0.15% / hPa.
pub struct FluxNormalizer {
    pub standard_pressure_hpa: f64,
    pub beta_p: f64, // -0.0015 per hPa
    pub alpha_t: f64,
    pub reference_temp_c: f64,
}

impl Default for FluxNormalizer {
    fn default() -> Self {
        Self {
            standard_pressure_hpa: 1013.25,
            beta_p: -0.0015,
            alpha_t: 0.0005,
            reference_temp_c: 15.0,
        }
    }
}

impl FluxNormalizer {
    /// Computes normalization scaling factor to apply to raw muon count rates.
    pub fn compute_scale_factor(&self, current_pressure_hpa: f64, current_temp_c: f64) -> f64 {
        let delta_p = current_pressure_hpa - self.standard_pressure_hpa;
        let delta_t = current_temp_c - self.reference_temp_c;

        let modulation = 1.0 + self.beta_p * delta_p + self.alpha_t * delta_t;
        if modulation > 0.5 {
            1.0 / modulation
        } else {
            1.0
        }
    }
}
