//! # Thermal Inertia Coupling
//!
//! Dynamic compute scheduling based on MCU temperature and material heat
//! capacity model. Matches local compute load to the thermal dissipation
//! capacity of the surrounding concrete/asphalt structure.
//!
//! The key insight: concrete has significant thermal mass. Short bursts of
//! compute generate heat that is absorbed by the surrounding material. By
//! modeling this thermal inertia, we can safely run compute bursts without
//! exceeding junction temperature limits.

use crate::types::ThermalState;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum safe junction temperature (millidegrees Celsius).
pub const MAX_JUNCTION_MC: i32 = 85_000;

/// Thermal shutdown temperature (millidegrees Celsius).
pub const THERMAL_SHUTDOWN_MC: i32 = 100_000;

/// Typical concrete thermal diffusivity (mm²/s).
pub const CONCRETE_DIFFUSIVITY_MM2_S: f32 = 0.75;

/// Ambient temperature assumption if no sensor (millidegrees Celsius).
pub const DEFAULT_AMBIENT_MC: i32 = 25_000;

// ---------------------------------------------------------------------------
// Thermal Model
// ---------------------------------------------------------------------------

/// Models the thermal behavior of a compute node embedded in structural material.
#[derive(Debug, Clone)]
pub struct ThermalModel {
    /// Core temperature (millidegrees Celsius).
    core_mc: i32,
    /// Ambient temperature estimate.
    ambient_mc: i32,
    /// Thermal resistance, junction-to-ambient (K/W).
    r_ja_kw: f32,
    /// Time constant (τ = R * C) in seconds (C = thermal capacitance, J/K).
    tau_sec: f32,
    /// Last update timestamp.
    last_update: crate::types::TimeMs,
}

impl ThermalModel {
    pub fn new(r_ja_kw: f32, c_th_jk: f32) -> Self {
        let tau = r_ja_kw * c_th_jk;
        Self {
            core_mc: 25_000, // Start at ambient (25°C)
            ambient_mc: DEFAULT_AMBIENT_MC,
            r_ja_kw,
            tau_sec: tau,
            last_update: crate::types::TimeMs::now(),
        }
    }

    /// Force the core temperature (used by tests and thermal fault injection).
    pub fn set_core_temperature(&mut self, core_mc: i32) {
        self.core_mc = core_mc;
        self.last_update = crate::types::TimeMs::now();
    }

    /// Update with a measured core temperature.
    pub fn update_measured(&mut self, core_mc: i32) {
        self.core_mc = core_mc;
        self.last_update = crate::types::TimeMs::now();
    }

    /// Update ambient temperature estimate.
    pub fn set_ambient(&mut self, ambient_mc: i32) {
        self.ambient_mc = ambient_mc;
    }

    /// Predict temperature rise from a compute burst.
    ///
    /// `power_mw`: Estimated power consumption of the compute burst.
    /// `duration_ms`: Duration of the burst.
    pub fn predict_temperature_rise(&self, power_mw: f32, duration_ms: u32) -> i32 {
        // ΔT = P * R * (1 - e^(-t/τ))
        let t_sec = duration_ms as f32 / 1000.0;
        let rise = power_mw / 1000.0 * self.r_ja_kw * (1.0 - libm::expf(-t_sec / self.tau_sec));
        (rise * 1000.0) as i32 // Convert to millidegrees
    }

    /// Estimate current core temperature given power dissipation.
    ///
    /// Uses the cooling curve: T(t) = T_ambient + (T_initial - T_ambient) * e^(-t/τ)
    pub fn estimate_core_temperature(&self, power_mw: f32) -> i32 {
        let t_sec = (crate::types::TimeMs::now().0 - self.last_update.0) as f32 / 1000.0;
        let cooling = (self.core_mc - self.ambient_mc) as f32 * libm::expf(-t_sec / self.tau_sec);
        let heating = power_mw / 1000.0 * self.r_ja_kw;
        self.ambient_mc + cooling as i32 + (heating * 1000.0) as i32
    }

    /// Maximum safe compute duty cycle given current state.
    ///
    /// Returns a percentage (0-100).
    pub fn max_duty_cycle(&self, power_mw: f32) -> u8 {
        if self.core_mc >= THERMAL_SHUTDOWN_MC {
            return 0;
        }

        // How much headroom do we have?
        let headroom = MAX_JUNCTION_MC - self.core_mc;
        if headroom <= 0 {
            return 0;
        }

        // For a given duty cycle D, the steady-state temp rise is:
        // ΔT = P * D * R
        // Solve for D: D = ΔT / (P * R)
        let p_r = (power_mw / 1000.0 * self.r_ja_kw * 1000.0) as i32; // in millidegrees
        if p_r <= 0 {
            return 100;
        }

        let duty = (headroom as f32 / p_r as f32 * 100.0) as u8;
        duty.min(100)
    }

    /// Get the current thermal state for reporting.
    pub fn get_state(&self, power_mw: f32) -> ThermalState {
        let estimated = self.estimate_core_temperature(power_mw);
        ThermalState {
            core_mc: self.core_mc.max(estimated),
            ambient_mc: self.ambient_mc,
            capacity_pct: {
                let used =
                    (self.core_mc as f32 - self.ambient_mc as f32) / MAX_JUNCTION_MC as f32 * 100.0;
                used.clamp(0.0, 100.0) as u8
            },
            recommended_duty_pct: self.max_duty_cycle(power_mw),
        }
    }
}

// ---------------------------------------------------------------------------
// Thermal-Aware Scheduler
// ---------------------------------------------------------------------------

/// Schedules compute tasks respecting thermal constraints.
#[derive(Debug, Clone)]
pub struct ThermalScheduler {
    /// Queue of pending operations with their estimated energy cost.
    pending_ops: heapless::Vec<ScheduledOp, 16>,
    model: ThermalModel,
}

/// A scheduled compute operation.
#[derive(Debug, Clone)]
pub struct ScheduledOp {
    /// Description of the operation.
    pub name: &'static str,
    /// Estimated power consumption (mW).
    pub power_mw: f32,
    /// Estimated duration (ms).
    pub duration_ms: u32,
    /// Priority (higher = more important).
    pub priority: u8,
}

impl ThermalScheduler {
    pub fn new(model: ThermalModel) -> Self {
        Self {
            pending_ops: heapless::Vec::new(),
            model,
        }
    }

    /// Schedule an operation.
    ///
    /// The unit error type just signals "queue full" — callers only check
    /// `is_ok()`.
    #[allow(clippy::result_unit_err)]
    pub fn schedule(&mut self, op: ScheduledOp) -> Result<(), ()> {
        self.pending_ops.push(op).map_err(|_| ())
    }

    /// Execute the next operation if thermal budget allows.
    ///
    /// Returns `Ok(Some(op))` if an op was executed, `Ok(None)` if queue is empty,
    /// `Err(thermal_limit)` if thermal budget is exceeded.
    pub fn execute_next<F>(&mut self, mut exec_fn: F) -> Result<Option<ScheduledOp>, u8>
    where
        F: FnMut(&ScheduledOp),
    {
        // Pick the highest-priority op with a linear scan. The queue is tiny
        // (≤16) and slice sorting is unavailable in no_std on current Rust
        // (sort moved to alloc), so a scan is the embedded-friendly choice.
        let best = self
            .pending_ops
            .iter()
            .enumerate()
            .max_by_key(|(_, op)| op.priority)
            .map(|(i, _)| i);

        let op = match best {
            Some(i) => self.pending_ops.remove(i),
            None => return Ok(None),
        };

        let duty = self.model.max_duty_cycle(op.power_mw);
        if duty == 0 {
            return Err(duty);
        }

        // Simulate thermal rise
        let rise = self
            .model
            .predict_temperature_rise(op.power_mw, op.duration_ms);
        if self.model.core_mc + rise > MAX_JUNCTION_MC {
            // Re-queue and return error
            self.pending_ops.push(op).ok();
            return Err(0);
        }

        exec_fn(&op);
        self.model.core_mc += rise;
        Ok(Some(op))
    }

    /// Number of pending operations.
    pub fn pending_count(&self) -> usize {
        self.pending_ops.len()
    }

    /// Get a reference to the thermal model.
    pub fn model(&self) -> &ThermalModel {
        &self.model
    }

    /// Get a mutable reference to the thermal model.
    pub fn model_mut(&mut self) -> &mut ThermalModel {
        &mut self.model
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thermal_model_initialization() {
        let model = ThermalModel::new(50.0, 10.0);
        assert!(model.core_mc > 0);
        assert_eq!(model.ambient_mc, DEFAULT_AMBIENT_MC);
    }

    #[test]
    fn test_temperature_rise_prediction() {
        let model = ThermalModel::new(50.0, 10.0);
        // 100 mW for 1 second should cause measurable rise
        let rise = model.predict_temperature_rise(100.0, 1000);
        assert!(rise > 0);
        // Longer duration = more rise
        let rise_long = model.predict_temperature_rise(100.0, 5000);
        assert!(rise_long >= rise);
    }

    #[test]
    fn test_max_duty_cycle() {
        let model = ThermalModel::new(50.0, 10.0);
        // At ambient, should allow > 0 duty cycle
        let duty = model.max_duty_cycle(100.0);
        assert!(duty > 0);
        assert!(duty <= 100);
    }

    #[test]
    fn test_thermal_scheduler() {
        let model = ThermalModel::new(50.0, 10.0);
        let mut scheduler = ThermalScheduler::new(model);

        let op = ScheduledOp {
            name: "sensor_read",
            power_mw: 50.0,
            duration_ms: 10,
            priority: 1,
        };

        let op2 = ScheduledOp {
            name: "radio_tx",
            power_mw: 200.0,
            duration_ms: 5,
            priority: 5,
        };

        assert!(scheduler.schedule(op).is_ok());
        assert!(scheduler.schedule(op2).is_ok());
        assert_eq!(scheduler.pending_count(), 2);

        // Execute should pick highest priority (radio_tx)
        let executed = scheduler.execute_next(|_| {}).ok();
        assert!(executed.is_some());
        assert_eq!(scheduler.pending_count(), 1);
    }

    #[test]
    fn test_max_duty_at_thermal_limit() {
        let mut model = ThermalModel::new(50.0, 10.0);
        model.core_mc = MAX_JUNCTION_MC; // Already at limit
        assert_eq!(model.max_duty_cycle(100.0), 0);
    }
}
