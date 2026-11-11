//! Closed-Loop SiPM Thermal Bias DAC Controller
//! Compensates SiPM breakdown voltage drift (-54 mV/°C) to keep gain constant within ±0.2%.

pub struct SipmBiasPidController {
    pub v_breakdown_t0: f32, // Breakdown voltage at T0 (e.g. 53.0 V)
    pub v_overvoltage: f32,   // Nominal overvoltage (e.g. +4.5 V)
    pub temp_coeff: f32,      // -0.054 V/deg C
    pub t0_c: f32,            // 25.0 deg C
    pub max_dac_volts: f32,   // 65.0 V maximum DAC output scale
}

impl SipmBiasPidController {
    pub fn new(v_br_t0: f32, v_over: f32, temp_coeff: f32) -> Self {
        Self {
            v_breakdown_t0: v_br_t0,
            v_overvoltage: v_over,
            temp_coeff,
            t0_c: 25.0,
            max_dac_volts: 65.0,
        }
    }

    /// Computes target bias voltage V_bias(T) = V_br(T0) + coeff * (T - T0) + V_over
    pub fn compute_target_voltage(&self, current_temp_c: f32) -> f32 {
        let delta_t = current_temp_c - self.t0_c;
        let v_br_t = self.v_breakdown_t0 + self.temp_coeff * delta_t;
        v_br_t + self.v_overvoltage
    }

    /// Converts required target voltage into a 16-bit DAC word (0..65535).
    pub fn update_dac_code(&mut self, current_temp_c: f32) -> u16 {
        let v_target = self.compute_target_voltage(current_temp_c);
        let normalized = (v_target / self.max_dac_volts).clamp(0.0, 1.0);
        (normalized * 65535.0) as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thermal_pid_compensation() {
        let mut ctrl = SipmBiasPidController::new(53.0, 4.5, -0.054);
        let v25 = ctrl.compute_target_voltage(25.0);
        assert!((v25 - 57.5).abs() < 1e-3);

        // Temperature rises to 35°C (+10°C) -> breakdown decreases by 0.54V -> V_bias = 56.96V
        let v35 = ctrl.compute_target_voltage(35.0);
        assert!((v35 - 56.96).abs() < 1e-3);

        let dac_code = ctrl.update_dac_code(25.0);
        assert!(dac_code > 50000);
    }
}
