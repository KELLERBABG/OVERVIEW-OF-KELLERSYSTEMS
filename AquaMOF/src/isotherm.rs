//! Sorbent Isotherm models for MOF-303 and MOF-801 water harvesting.

use serde::{Deserialize, Serialize};

/// Type of Metal-Organic Framework sorbent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MofType {
    Mof303,
    Mof801,
    Custom,
}

/// Sorbent isotherm parameter set based on Dual-Site Langmuir-Freundlich (DSLF).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsothermParams {
    pub mof_type: MofType,
    /// Maximum water saturation capacity (g H2O / g MOF).
    pub q_sat: f64,
    /// Site 1 affinity parameter.
    pub b1: f64,
    /// Heterogeneity factor site 1 (steepness of step).
    pub n1: f64,
    /// Site 2 affinity parameter.
    pub b2: f64,
    /// Heterogeneity factor site 2.
    pub n2: f64,
    /// Desorption onset temperature (°C).
    pub t_desorp_onset_c: f64,
}

impl IsothermParams {
    /// Baseline MOF-303 [Al(OH)(HPDC)] calibrated from Yaghi et al. (2018/2020)
    pub fn mof303() -> Self {
        Self {
            mof_type: MofType::Mof303,
            q_sat: 0.48, // 480 g/kg at saturation
            b1: 7.2,
            n1: 5.0,     // Steep S-shaped step at 12-18% RH
            b2: 1.5,
            n2: 2.0,
            t_desorp_onset_c: 65.0,
        }
    }

    /// Baseline MOF-801 [Zr6O4(OH)4(fumarate)6]
    pub fn mof801() -> Self {
        Self {
            mof_type: MofType::Mof801,
            q_sat: 0.35, // 350 g/kg
            b1: 5.0,
            n1: 3.5,
            b2: 1.0,
            n2: 1.5,
            t_desorp_onset_c: 75.0,
        }
    }

    /// Calculate equilibrium loading q* (g H2O / g MOF) given Relative Humidity (0.0 to 1.0) and Temperature (°C).
    pub fn equilibrium_uptake(&self, rh: f64, temp_c: f64) -> f64 {
        let rh_clamped = rh.clamp(0.0, 1.0);
        
        let temp_factor = (-0.025 * (temp_c - 25.0).max(0.0)).exp();
        
        let term1 = (self.b1 * rh_clamped).powf(self.n1) / (1.0 + (self.b1 * rh_clamped).powf(self.n1));
        let term2 = (self.b2 * rh_clamped).powf(self.n2) / (1.0 + (self.b2 * rh_clamped).powf(self.n2));
        
        let q_eq = self.q_sat * (0.90 * term1 + 0.10 * term2) * temp_factor;
        q_eq.clamp(0.0, self.q_sat)
    }

    /// Desorption residual loading at high temperature under solar heating.
    pub fn desorption_residual_loading(&self, temp_c: f64, chamber_rh: f64) -> f64 {
        if temp_c <= self.t_desorp_onset_c {
            return self.equilibrium_uptake(chamber_rh, temp_c);
        }
        
        let delta_t = temp_c - self.t_desorp_onset_c;
        let desorption_fraction = (1.0 - (-delta_t / 15.0).exp()).min(0.95);
        let base_q = self.equilibrium_uptake(chamber_rh, self.t_desorp_onset_c);
        
        base_q * (1.0 - desorption_fraction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mof303_uptake_at_arid_rh() {
        let params = IsothermParams::mof303();
        // At 20% RH and 25°C, MOF-303 should adsorb >= 0.30 g/g
        let uptake_20 = params.equilibrium_uptake(0.20, 25.0);
        assert!(uptake_20 >= 0.30, "Uptake at 20% RH was {:.3}", uptake_20);

        // At 5% RH, uptake is small
        let uptake_5 = params.equilibrium_uptake(0.05, 25.0);
        assert!(uptake_5 < 0.15, "Uptake at 5% RH was {:.3}", uptake_5);

        // At 85°C, desorption residual is low
        let res = params.desorption_residual_loading(85.0, 0.40);
        assert!(res < 0.08, "Residual at 85°C was {:.3}", res);
    }
}
