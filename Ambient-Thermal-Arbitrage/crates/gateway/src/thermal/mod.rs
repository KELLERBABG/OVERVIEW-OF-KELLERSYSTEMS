//! Enhanced thermodynamic model with dynamic COP and parameter estimation.
//!
//! ## Key Improvements vs MVP:
//! 1. **Dynamic COP**: Carnot efficiency model — COP drops as temperature delta increases
//! 2. **RLS Parameter Estimation**: Recursive Least Squares for tracking C_th changes
//! 3. **TTF with confidence bounds**: Incorporates parameter uncertainty

use serde::{Deserialize, Serialize};

/// Enhanced thermal state with dynamic COP and adaptive parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalState {
    /// Current internal temperature (°C).
    pub temperature_c: f64,
    /// Ambient (outside) temperature (°C).
    pub ambient_temp_c: f64,
    /// Thermal resistance (K/W).
    pub r_th: f64,
    /// Thermal capacitance (J/K) — may be updated by RLS estimator.
    pub c_th: f64,
    /// Carnot efficiency factor (0.0-1.0), e.g. 0.6 for a good system.
    pub carnot_efficiency: f64,
    /// Maximum electrical cooling power (W).
    pub max_electrical_power_w: f64,
    /// Timestamp of last update (seconds since epoch).
    pub last_update: f64,
    // ── RLS Estimator State ──────────────────────────────
    /// Estimated C_th from RLS (adaptive).
    pub estimated_c_th: f64,
    /// RLS covariance (uncertainty estimate).
    pub rls_p: f64,
    /// RLS forgetting factor (0.95-0.99). Lower = adapts faster.
    pub rls_lambda: f64,
}

impl ThermalState {
    pub fn new(
        temperature_c: f64, ambient_temp_c: f64,
        r_th: f64, c_th: f64,
    ) -> Self {
        Self {
            temperature_c, ambient_temp_c, r_th, c_th,
            carnot_efficiency: 0.6, // 60% of Carnot is realistic
            max_electrical_power_w: 50_000.0,
            last_update: Self::now(),
            estimated_c_th: c_th,
            rls_p: 1e6, // High initial uncertainty
            rls_lambda: 0.98, // moderate forgetting
        }
    }

    /// Dynamic COP based on Carnot cycle efficiency.
    ///
    /// COP_carnot = T_cold / (T_warm - T_cold)   [temperatures in Kelvin]
    /// COP_real = η_C * COP_carnot
    ///
    /// When the room is very cold (-24°C) and ambient is warm (35°C),
    /// COP drops significantly. At -18°C with 20°C ambient, COP is higher.
    pub fn dynamic_cop(&self) -> f64 {
        // Fail-safe: non-finite temperatures must not propagate (NaN comparisons
        // are always false, so NaN would silently poison every downstream calc).
        if !self.temperature_c.is_finite() || !self.ambient_temp_c.is_finite() {
            return 3.0; // mid-range COP — keeps the model finite and conservative
        }
        let t_cold_k = self.temperature_c + 273.15;
        let t_warm_k = self.ambient_temp_c + 273.15;
        let delta_t = (t_warm_k - t_cold_k).clamp(0.1, 100.0); // avoid division by zero / absurd deltas

        // Carnot COP = T_cold / ΔT
        let cop_carnot = t_cold_k / delta_t;

        // Real COP = η * COP_carnot
        let cop_real = self.carnot_efficiency * cop_carnot;

        // Clamp to realistic range (minimum 0.5, maximum 6.0)
        cop_real.clamp(0.5, 6.0)
    }

    /// Effective thermal cooling power at current conditions (W).
    /// P_thermal = P_electrical * COP(T_amb, T_int)
    pub fn effective_cooling_power_w(&self) -> f64 {
        self.max_electrical_power_w * self.dynamic_cop()
    }

    /// Time constant τ = R_th * C_th.
    /// Uses the estimated (adaptive) C_th when available.
    pub fn time_constant(&self) -> f64 {
        self.r_th * self.estimated_c_th
    }

    // ── RLS Parameter Estimation ─────────────────────────
    //
    // We observe: dT/dt = (T_amb - T_int) / (R * C_th)
    // For cooling: dT/dt = (T_eff - T_int) / (R * C_th)
    //
    // The system identification watches the temperature decay curve
    // when the compressors are OFF to estimate τ = R * C_th.
    //
    // Since R_th is known from building geometry, we solve for C_th.
    //
    // Recursive Least Squares update:
    //   θ_hat(t) = θ_hat(t-1) + K(t) * [y(t) - φ(t) * θ_hat(t-1)]
    //   K(t) = P(t-1) * φ(t) / [λ + φ²(t) * P(t-1)]
    //   P(t) = [1 - K(t) * φ(t)] * P(t-1) / λ

    /// Update the C_th estimate using RLS from a temperature observation.
    /// Call this when compressor is OFF for at least 2 observation periods.
    ///
    /// Args:
    ///   dt_sec: time since last observation
    ///   temp_before: temperature at start of interval
    ///   temp_after: temperature at end of interval
    ///   ambient: ambient temperature during interval
    pub fn rls_update(&mut self, dt_sec: f64, temp_before: f64, temp_after: f64, ambient: f64) {
        if dt_sec < 60.0 || !dt_sec.is_finite() { return; } // Need at least 1 minute of data

        // Fail-safe: reject any non-finite observation. NaN comparisons are
        // always false, so the old `num / den <= 0.0` guard alone is bypassable
        // and would slam the estimate to its 10x clamp (see hardening notes).
        if !temp_before.is_finite() || !temp_after.is_finite() || !ambient.is_finite() {
            return;
        }
        if !self.r_th.is_finite() || self.r_th <= 0.0 { return; }

        let num = temp_after - ambient;
        let den = temp_before - ambient;
        if den.abs() < 0.1 || num / den <= 0.0 { return; }

        let y = (num / den).ln(); // observed log ratio
        if !y.is_finite() { return; }
        let phi = -dt_sec / self.r_th; // regressor

        // RLS update on θ = 1/C_th
        let theta_old = 1.0 / self.estimated_c_th;
        let prediction = phi * theta_old;
        let error = y - prediction;

        // Kalman gain
        let phi_t = phi; // scalar case
        let k = self.rls_p * phi_t / (self.rls_lambda + phi_t * self.rls_p * phi_t);

        // Update estimate
        let theta_new = theta_old + k * error;

        // Update covariance
        self.rls_p = (1.0 - k * phi_t) * self.rls_p / self.rls_lambda;

        // Clamp covariance
        self.rls_p = self.rls_p.clamp(1.0, 1e10);

        // Convert back to C_th and clamp to physical range
        let c_th_estimated = 1.0 / theta_new.abs().max(1e-10);
        self.estimated_c_th = c_th_estimated.clamp(self.c_th * 0.1, self.c_th * 10.0);
    }

    /// Compute C_th uncertainty as coefficient of variation.
    pub fn c_th_uncertainty(&self) -> f64 {
        (self.rls_p / self.estimated_c_th.powi(2)).sqrt().min(1.0)
    }

    // ── Core Physics ─────────────────────────────────────

    /// Compute the maximum safe time (s) the system can be off before reaching T_limit.
    pub fn max_safe_downtime(&self, t_limit: f64) -> f64 {
        let tau = self.time_constant();
        let num = t_limit - self.ambient_temp_c;
        let den = self.temperature_c - self.ambient_temp_c;
        if den.abs() < 1e-9 || num / den <= 0.0 { return 0.0; }
        let ratio = num / den;
        if ratio <= 0.0 || ratio >= 1.0 { return 0.0; }
        (-tau * ratio.ln()).max(0.0)
    }

    /// Step the thermal model: T(t+dt) given a constant ambient.
    /// Uses dynamic COP — the cooling power varies with temperature delta.
    pub fn step(&mut self, dt_sec: f64, cooling: bool, electrical_power_w: f64) {
        // Fail-safe against non-finite input: a single NaN/Inf ambient or model
        // constant must never brick the state permanently (NaN propagates and
        // comparison-based guards are defeated). Re-seed from a safe value.
        if !self.ambient_temp_c.is_finite() || !self.r_th.is_finite() || !self.c_th.is_finite() {
            if !self.temperature_c.is_finite() {
                self.temperature_c = -20.0; // safe cold-chain default
            }
            return;
        }
        if !self.temperature_c.is_finite() {
            self.temperature_c = self.ambient_temp_c;
        }
        let tau = self.time_constant();
        if !tau.is_finite() || tau <= 0.0 { return; }

        // For cooling, the effective ambient is shifted by P * R_th,
        // but now P_thermal = P_electrical * COP(T_amb, T_int) varies dynamically.
        // We'll use a midpoint approximation for accuracy over large dt.
        let effective_ambient = if cooling {
            // At the start of the step, compute COP at current temperature
            let cop_start = self.dynamic_cop();
            let thermal_power_start = electrical_power_w * cop_start;

            // Estimate temperature at midpoint using a half-step
            let factor_half = (-dt_sec * 0.5 / tau).exp();
            let eff_start = self.ambient_temp_c - thermal_power_start * self.r_th;
            let temp_mid = eff_start + (self.temperature_c - eff_start) * factor_half;

            // Compute COP at midpoint temperature (better approximation)
            let temp_old = self.temperature_c;
            self.temperature_c = temp_mid; // temporarily set to midpoint
            let cop_mid = self.dynamic_cop();
            self.temperature_c = temp_old; // restore

            let thermal_power_mid = electrical_power_w * cop_mid;
            self.ambient_temp_c - thermal_power_mid * self.r_th
        } else {
            self.ambient_temp_c
        };

        let factor = (-dt_sec / tau).exp();
        self.temperature_c = (effective_ambient + (self.temperature_c - effective_ambient) * factor).clamp(-35.0, 50.0);
        self.last_update = Self::now();
    }

    /// Current rate of temperature change (°C/s).
    pub fn d_t_dt(&self, cooling: bool, electrical_power_w: f64) -> f64 {
        if !self.temperature_c.is_finite() || !self.ambient_temp_c.is_finite() { return 0.0; }
        let tau = self.time_constant();
        if !tau.is_finite() || tau <= 0.0 { return 0.0; }
        let effective_ambient = if cooling {
            let cop = self.dynamic_cop();
            self.ambient_temp_c - electrical_power_w * cop * self.r_th
        } else {
            self.ambient_temp_c
        };
        (effective_ambient - self.temperature_c) / tau
    }

    /// Predict temperature at a future time (without mutating).
    pub fn predict_temp(&self, dt_sec: f64, cooling: bool, electrical_power_w: f64) -> f64 {
        if !self.temperature_c.is_finite() || !self.ambient_temp_c.is_finite() { return self.temperature_c; }
        let tau = self.time_constant();
        if !tau.is_finite() || tau <= 0.0 { return self.temperature_c; }
        let effective_ambient = if cooling {
            let cop = self.dynamic_cop();
            self.ambient_temp_c - electrical_power_w * cop * self.r_th
        } else {
            self.ambient_temp_c
        };
        effective_ambient + (self.temperature_c - effective_ambient) * (-dt_sec / tau).exp()
    }

    /// TTF with confidence bounds (accounts for C_th uncertainty).
    /// Returns (ttf_lower, ttf_estimate, ttf_upper).
    pub fn time_to_failure_bounds(&self, t_limit: f64, cooling: bool, electrical_power_w: f64) -> (f64, f64, f64) {
        let rate = self.d_t_dt(cooling, electrical_power_w);
        if rate.abs() < 1e-12 { return (f64::INFINITY, f64::INFINITY, f64::INFINITY); }

        let delta = if cooling {
            self.temperature_c - t_limit
        } else {
            t_limit - self.temperature_c
        };
        if delta <= 0.0 { return (0.0, 0.0, 0.0); }

        let base_ttf = if rate > 0.0 { delta / rate } else { delta / rate.abs() };
        let uncertainty = self.c_th_uncertainty();

        // Lower bound: C_th smaller → faster temperature change → shorter TTF
        // Upper bound: C_th larger → slower change → longer TTF
        let lower = base_ttf / (1.0 + uncertainty);
        let upper = base_ttf / (1.0 - uncertainty).max(0.01);

        (lower, base_ttf, upper)
    }

    fn now() -> f64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_cop_decreases_with_delta() {
        // At small delta (warm ambient, warm room): high COP
        let warm = ThermalState {
            temperature_c: -18.0, ambient_temp_c: 5.0,
            r_th: 0.05, c_th: 1_000_000.0,
            carnot_efficiency: 0.6, max_electrical_power_w: 50_000.0,
            last_update: 0.0, estimated_c_th: 1_000_000.0,
            rls_p: 1e6, rls_lambda: 0.98,
        };

        // At large delta (cold room, hot ambient): low COP
        let cold = ThermalState {
            temperature_c: -24.0, ambient_temp_c: 35.0,
            r_th: 0.05, c_th: 1_000_000.0,
            carnot_efficiency: 0.6, max_electrical_power_w: 50_000.0,
            last_update: 0.0, estimated_c_th: 1_000_000.0,
            rls_p: 1e6, rls_lambda: 0.98,
        };

        let cop_warm = warm.dynamic_cop();
        let cop_cold = cold.dynamic_cop();

        assert!(cop_warm > cop_cold,
            "COP should decrease at larger temperature deltas: warm={:.2}, cold={:.2}",
            cop_warm, cop_cold);
        assert!((0.5..=6.0).contains(&cop_warm),
            "COP out of range: {}", cop_warm);
        assert!((0.5..=6.0).contains(&cop_cold),
            "COP out of range: {}", cop_cold);
    }

    #[test]
    fn test_dynamic_cop_typical_values() {
        // NRW winter: room at -20°C, ambient 2°C → COP should be high
        let winter = ThermalState {
            temperature_c: -20.0, ambient_temp_c: 2.0,
            r_th: 0.05, c_th: 1_000_000.0,
            carnot_efficiency: 0.6, max_electrical_power_w: 50_000.0,
            last_update: 0.0, estimated_c_th: 1_000_000.0,
            rls_p: 1e6, rls_lambda: 0.98,
        };
        let cop = winter.dynamic_cop();
        // At -20°C / 2°C: COP_carnot = 253/22 = 11.5, real ≈ 0.6*11.5 ≈ 6.9 → clamped to 6.0
        assert!((cop - 6.0).abs() < 0.01, "Winter COP should be near 6.0: {}", cop);
    }

    #[test]
    fn test_rls_estimator_converges() {
        let mut s = ThermalState::new(-20.0, 25.0, 0.05, 1_000_000.0);
        let true_c_th = 800_000.0; // different from initial estimate
        s.estimated_c_th = 500_000.0; // wrong initial estimate

        // Simulate several observations with the true C_th
        let mut temp = -20.0;
        let ambient = 25.0;
        let dt = 3600.0;

        for _ in 0..10 {
            let exp_arg: f64 = -dt / (0.05 * true_c_th);
            let temp_next = ambient + (temp - ambient) * exp_arg.exp();
            s.rls_update(dt, temp, temp_next, ambient);
            temp = temp_next;
        }

        // Should have moved towards true value
        let error = ((s.estimated_c_th - true_c_th) / true_c_th).abs();
        assert!(error < 0.3,
            "RLS should converge: estimated={:.0}, true={:.0}, err={:.2}",
            s.estimated_c_th, true_c_th, error);
    }

    #[test]
    fn test_ttf_bounds() {
        let s = ThermalState::new(-20.0, 25.0, 0.05, 1_000_000.0);
        let (lower, base, upper) = s.time_to_failure_bounds(-18.0, false, 0.0);
        assert!(lower.is_finite());
        assert!(base.is_finite());
        assert!(upper.is_finite());
        assert!(lower <= base, "Lower bound should be <= base: {:.0} vs {:.0}", lower, base);
        assert!(base <= upper, "Base should be <= upper: {:.0} vs {:.0}", base, upper);
    }

    #[test]
    fn test_step_rejects_nan_ambient_without_poisoning() {
        let mut s = ThermalState::new(-20.0, 25.0, 0.05, 1_000_000.0);
        s.ambient_temp_c = f64::NAN;
        s.step(3600.0, false, 0.0);
        assert!(s.temperature_c.is_finite(), "temperature must stay finite after NaN ambient");
        // Model must recover and behave normally once good data returns.
        s.ambient_temp_c = 25.0;
        s.step(3600.0, false, 0.0);
        assert!(s.temperature_c.is_finite());
        assert!(s.temperature_c > -20.0, "should warm toward ambient, got {}", s.temperature_c);
    }

    #[test]
    fn test_step_re_seeds_non_finite_internal_temp() {
        let mut s = ThermalState::new(-20.0, 25.0, 0.05, 1_000_000.0);
        s.temperature_c = f64::NAN;
        s.step(3600.0, false, 0.0);
        assert!(s.temperature_c.is_finite(), "NaN internal temp must be re-seeded");
    }

    #[test]
    fn test_rls_rejects_nan_observation() {
        let mut s = ThermalState::new(-20.0, 25.0, 0.05, 1_000_000.0);
        s.rls_update(3600.0, -20.0, -19.5, f64::NAN);
        assert_eq!(s.estimated_c_th, 1_000_000.0, "NaN sample must not move the estimate");
        assert!(s.rls_p.is_finite(), "covariance must stay finite");
    }
}