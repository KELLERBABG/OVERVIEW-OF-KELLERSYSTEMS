//! Sensor module: simulated and real hardware abstraction.
//!
//! Provides:
//! - `SensorReading` — snapshot of system state
//! - `SensorHardware` — trait for physical/ simulated sensors
//! - `SimulatedSensors` — in-memory thermal physics with compressor control

use crate::thermal::ThermalState;
use chrono::{DateTime, Utc};

/// Physical plausibility bounds for temperatures (°C). Anything outside this
/// range is treated as a sensor fault, never as a genuine reading.
pub const TEMP_PLAUSIBLE_MIN: f64 = -80.0;
pub const TEMP_PLAUSIBLE_MAX: f64 = 80.0;

/// True when a temperature is finite and within physical plausibility bounds.
/// Central validation gate for every sensor boundary (sim, live-data, PLC).
pub fn is_plausible_temp(temp_c: f64) -> bool {
    temp_c.is_finite() && (TEMP_PLAUSIBLE_MIN..=TEMP_PLAUSIBLE_MAX).contains(&temp_c)
}

/// A single sensor reading snapshot.
#[derive(Debug, Clone)]
pub struct SensorReading {
    pub timestamp: DateTime<Utc>,
    pub internal_temp_c: f64,
    pub ambient_temp_c: f64,
    pub compressor_on: bool,
    pub power_consumption_w: f64,
    pub grid_frequency_hz: f64,
    /// RLS-estimated thermal capacitance (J/K) — adapts to changing warehouse load.
    pub estimated_c_th: f64,
}

/// Trait abstracting over simulated and real hardware sensors.
pub trait SensorHardware: Send {
    /// Read the current sensor state.
    fn read(&mut self) -> SensorReading;
    /// Set compressor on/off.
    fn set_compressor(&mut self, on: bool);
    /// Check if compressor is currently running.
    fn compressor_state(&self) -> bool;
}

/// Simulated sensors — physics advances only when `simulate_step()` called.
pub struct SimulatedSensors {
    thermal: ThermalState,
    compressor_on: bool,
    /// Electrical power draw of compressors (for display and financial tracking)
    electrical_power_w: f64,
    /// Thermal cooling power = electrical × COP (for physics simulation)
    #[allow(dead_code)] // kept for display/tuning
    thermal_power_w: f64,
    #[allow(dead_code)]
    heat_leak_w: f64,
    grid_frequency_hz: f64,
    manual_freq_mode: bool,
    // For RLS: track previous state when compressor was OFF
    last_off_temp: Option<f64>,
    last_off_time: Option<f64>,
    // Ambient spread across the OFF window (RLS variance guard).
    off_ambient_min: Option<f64>,
    off_ambient_max: Option<f64>,
    /// Max allowed ambient swing (°C) over an OFF window before skipping the
    /// C_th update; 0.0 disables the guard.
    rls_ambient_variance_max_k: f64,
}

impl SimulatedSensors {
    /// Fixed-arity constructor (call sites: main.rs + tests). Parameter count
    /// is deliberate — each value is a distinct physical/config input.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        initial_temp_c: f64,
        ambient_temp_c: f64,
        r_th: f64,
        c_th: f64,
        electrical_power_w: f64,
        thermal_power_w: f64,
        heat_leak_w: f64,
        rls_ambient_variance_max_k: f64,
    ) -> Self {
        Self {
            thermal: ThermalState::new(initial_temp_c, ambient_temp_c, r_th, c_th),
            compressor_on: false,
            electrical_power_w,
            thermal_power_w,
            heat_leak_w,
            grid_frequency_hz: 50.0,
            manual_freq_mode: false,
            last_off_temp: None,
            last_off_time: None,
            off_ambient_min: None,
            off_ambient_max: None,
            rls_ambient_variance_max_k,
        }
    }

    /// Set ambient temperature from external data (e.g. simulation dataset).
    /// Rejects implausible values (NaN/Inf/absurd) so one bad weather value
    /// can never poison the thermal model permanently.
    pub fn set_ambient_temp(&mut self, temp_c: f64) {
        if is_plausible_temp(temp_c) {
            self.thermal.ambient_temp_c = temp_c;
        } else {
            log::warn!("🌡️ Rejected implausible ambient temperature: {}", temp_c);
        }
    }

    /// Override the grid frequency (simulation scenarios).
    pub fn override_grid_frequency(&mut self, freq_hz: f64) {
        self.grid_frequency_hz = freq_hz;
        self.manual_freq_mode = true;
    }

    /// Step the thermal physics by a given number of simulated seconds.
    /// `sim_step_sec` = duration of this step in simulated seconds.
    /// `current_sim_time_sec` = absolute simulated time (for RLS and governor).
    /// Passes the RAW ELECTRICAL power (not pre-multiplied by COP) to ThermalState,
    /// which applies dynamic Carnot COP internally. This avoids double-counting COP.
    pub fn simulate_step(&mut self, sim_step_sec: f64, current_sim_time_sec: f64) {
        // Safety: never let a non-finite internal temperature reach the physics.
        if !self.thermal.temperature_c.is_finite() {
            log::error!("🌡️ Internal temperature became non-finite — re-seeding to safe default");
            self.thermal.temperature_c = -20.0;
        }
        let dt = sim_step_sec.clamp(1.0, 7200.0);
        // Pass ELECTRICAL power — ThermalState multiplies by dynamic COP internally
        let power = if self.compressor_on { self.electrical_power_w } else { 0.0 };
        self.thermal.step(dt, self.compressor_on, power);

        // RLS update: when compressor is OFF, track temperature decay
        // Uses simulated clock to prevent time-domain collision (wall clock vs sim clock).
        if !self.compressor_on {
            // Ambient-variance guard: track the ambient spread across the OFF
            // window so a door-opening / loading-event spike cannot corrupt the
            // C_th estimate from a single distorted decay observation.
            let amb = self.thermal.ambient_temp_c;
            self.off_ambient_min = Some(self.off_ambient_min.map_or(amb, |m: f64| m.min(amb)));
            self.off_ambient_max = Some(self.off_ambient_max.map_or(amb, |m: f64| m.max(amb)));
            match (self.last_off_temp, self.last_off_time) {
                (Some(prev_temp), Some(prev_time)) => {
                    let dt_obs = current_sim_time_sec - prev_time;
                    if dt_obs >= 300.0 { // At least 5 simulated minutes between observations
                        let swing = self.off_ambient_max.unwrap_or(amb) - self.off_ambient_min.unwrap_or(amb);
                        let gate = self.rls_ambient_variance_max_k;
                        if gate > 0.0 && swing > gate {
                            log::info!("⏭️ RLS C_th update skipped: ambient swung {:.1} K over OFF window (> {:.1} K threshold)", swing, gate);
                        } else {
                            self.thermal.rls_update(dt_obs, prev_temp, self.thermal.temperature_c, amb);
                        }
                        self.last_off_temp = Some(self.thermal.temperature_c);
                        self.last_off_time = Some(current_sim_time_sec);
                        self.off_ambient_min = Some(amb);
                        self.off_ambient_max = Some(amb);
                    }
                }
                _ => {
                    self.last_off_temp = Some(self.thermal.temperature_c);
                    self.last_off_time = Some(current_sim_time_sec);
                    self.off_ambient_min = Some(amb);
                    self.off_ambient_max = Some(amb);
                }
            }
        } else {
            // Reset RLS tracking when compressor turns on
            self.last_off_temp = None;
            self.last_off_time = None;
            self.off_ambient_min = None;
            self.off_ambient_max = None;
        }
    }

    /// Get the current internal temperature (read-only).
    pub fn internal_temp(&self) -> f64 {
        self.thermal.temperature_c
    }
}

/// Build a `SensorReading` from raw hardware values, applying the plausibility
/// gate. Returns `None` when the reading is invalid (caller must fail safe).
pub fn reading_from_hardware(
    internal_temp_c: f64,
    ambient_temp_c: f64,
    compressor_on: bool,
    grid_frequency_hz: f64,
    estimated_c_th: f64,
) -> Option<SensorReading> {
    if !is_plausible_temp(internal_temp_c) || !is_plausible_temp(ambient_temp_c) {
        return None;
    }
    Some(SensorReading {
        timestamp: Utc::now(),
        internal_temp_c,
        ambient_temp_c,
        compressor_on,
        power_consumption_w: if compressor_on { 0.0 } else { 10.0 },
        grid_frequency_hz,
        estimated_c_th,
    })
}

impl SensorHardware for SimulatedSensors {
    fn read(&mut self) -> SensorReading {
        SensorReading {
            timestamp: Utc::now(),
            internal_temp_c: self.thermal.temperature_c,
            ambient_temp_c: self.thermal.ambient_temp_c,
            compressor_on: self.compressor_on,
            power_consumption_w: if self.compressor_on { self.electrical_power_w } else { 10.0 },
            grid_frequency_hz: self.grid_frequency_hz,
            estimated_c_th: self.thermal.estimated_c_th, // RLS-adapted C_th for MPC
        }
    }

    fn set_compressor(&mut self, on: bool) {
        self.compressor_on = on;
    }

    fn compressor_state(&self) -> bool {
        self.compressor_on
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sim() -> SimulatedSensors {
        SimulatedSensors::new(-20.0, 25.0, 0.05, 1_000_000.0, 5000.0, 5000.0 * 2.5, 500.0, 3.0)
    }

    #[test]
    fn test_simulated_read() {
        let mut sim = make_sim();
        let reading = sim.read();
        assert!((reading.internal_temp_c - (-20.0)).abs() < 1.0);
        assert!(!reading.compressor_on);
    }

    #[test]
    fn test_simulated_cooling() {
        let mut sim = make_sim();
        sim.set_compressor(true);
        sim.simulate_step(3600.0, 3600.0);
        let reading = sim.read();
        assert!(reading.compressor_on);
        assert!(reading.internal_temp_c < -20.0);
    }

    #[test]
    fn test_frequency_override() {
        let mut sim = make_sim();
        sim.override_grid_frequency(49.5);
        let reading = sim.read();
        assert_eq!(reading.grid_frequency_hz, 49.5);
    }

    #[test]
    fn test_set_ambient_temp_rejects_nan() {
        let mut sim = make_sim();
        sim.set_ambient_temp(f64::NAN);
        assert!(sim.read().ambient_temp_c.is_finite());
        // Plausible values still pass through.
        sim.set_ambient_temp(30.0);
        assert_eq!(sim.read().ambient_temp_c, 30.0);
    }

    /// Drive two compressor-OFF RLS observation windows (300 s apart), returning
    /// the C_th estimate afterwards. `gate` is the ambient-variance threshold.
    fn run_off_window_observation(gate: f64, ambient_profile: &[(f64, f64)]) -> f64 {
        let mut sim = SimulatedSensors::new(-20.0, 25.0, 0.05, 1_000_000.0, 5000.0, 5000.0 * 2.5, 500.0, gate);
        let mut t = 0.0;
        for (dt_sec, ambient) in ambient_profile {
            sim.set_ambient_temp(*ambient);
            sim.simulate_step(*dt_sec, t + *dt_sec);
            t += *dt_sec;
        }
        sim.thermal.estimated_c_th
    }

    #[test]
    fn rls_guard_skips_update_when_ambient_swings_over_window() {
        // Stable ambient (25.0 → 25.5) over the OFF window: update applies.
        let stable = run_off_window_observation(3.0, &[(100.0, 25.0), (300.0, 25.5)]);
        assert_ne!(stable, 1_000_000.0, "C_th estimate must update on a stable window");

        // Violent ambient swing (25.0 → 35.0) over the window: update is SKIPPED,
        // the estimate stays at its initial value.
        let swung = run_off_window_observation(3.0, &[(100.0, 25.0), (300.0, 35.0)]);
        assert_eq!(swung, 1_000_000.0, "C_th must not update when ambient swings beyond the guard");
    }

    #[test]
    fn rls_guard_can_be_disabled() {
        // gate = 0.0 disables the guard: the same swing now updates the estimate.
        let swung = run_off_window_observation(0.0, &[(100.0, 25.0), (300.0, 35.0)]);
        assert_ne!(swung, 1_000_000.0, "guard disabled (0.0) must allow the update");
    }
}