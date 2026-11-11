//! Dynamic adsorption & desorption kinetics (LDF model).

use crate::isotherm::IsothermParams;
use serde::{Deserialize, Serialize};

/// Dynamic state of the MOF sorbent bed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedState {
    /// Mass of dry MOF in kg.
    pub mof_mass_kg: f64,
    /// Current water loading (g H2O / g MOF).
    pub current_loading: f64,
    /// Bed temperature in °C.
    pub bed_temp_c: f64,
    /// Mass of adsorbed water in kg.
    pub water_mass_kg: f64,
}

impl BedState {
    pub fn new(mof_mass_kg: f64, initial_temp_c: f64) -> Self {
        Self {
            mof_mass_kg,
            current_loading: 0.02,
            bed_temp_c: initial_temp_c,
            water_mass_kg: mof_mass_kg * 0.02,
        }
    }

    /// Update loading using Linear Driving Force (LDF) mass transfer equation:
    /// dq/dt = k_ldf * (q* - q)
    pub fn step_adsorption(&mut self, params: &IsothermParams, ambient_rh: f64, ambient_temp_c: f64, dt_seconds: f64) {
        self.bed_temp_c = ambient_temp_c;
        let q_eq = params.equilibrium_uptake(ambient_rh, ambient_temp_c);
        let k_ldf = 0.0008; // Mass transfer coefficient in s^-1
        
        let delta_q = k_ldf * (q_eq - self.current_loading) * dt_seconds;
        self.current_loading = (self.current_loading + delta_q).clamp(0.0, params.q_sat);
        self.water_mass_kg = self.mof_mass_kg * self.current_loading;
    }

    /// Step desorption under solar thermal irradiance:
    /// returns mass of released vapor in kg during this step.
    pub fn step_desorption(&mut self, params: &IsothermParams, solar_irradiance_w_m2: f64, ambient_temp_c: f64, dt_seconds: f64) -> f64 {
        // Solar thermal balance: T_bed rises based on solar flux and heat loss
        let panel_area_m2 = 1.0;
        let absorptivity = 0.92;
        let u_loss = 6.5; // W/(m2*K) overall heat loss coefficient
        
        let q_solar_w = solar_irradiance_w_m2 * panel_area_m2 * absorptivity;
        let t_target = ambient_temp_c + (q_solar_w / (u_loss * panel_area_m2)).min(65.0);
        
        // Thermal relaxation of the bed
        let tau_thermal = 900.0; // 15 min thermal time constant
        let dt_rel = dt_seconds / tau_thermal;
        self.bed_temp_c += (t_target - self.bed_temp_c) * (1.0 - (-dt_rel).exp());
        
        // Desorption kinetics
        let q_target = params.desorption_residual_loading(self.bed_temp_c, 0.60);
        let k_desorp = 0.0015; // Fast desorption rate
        
        if self.current_loading > q_target {
            let delta_q = k_desorp * (self.current_loading - q_target) * dt_seconds;
            let released_water_kg = self.mof_mass_kg * delta_q;
            self.current_loading -= delta_q;
            self.water_mass_kg = self.mof_mass_kg * self.current_loading;
            released_water_kg
        } else {
            0.0
        }
    }
}
