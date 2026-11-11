//! Enhanced control engine with Model Predictive Control and short-cycling protection.
//!
//! ## MPC (Model Predictive Control)
//! Looks ahead 6-12 hours in the price/weather dataset to determine optimal
//! pre-cooling strategy. Instead of reacting to the current price tick, it
//! solves J = Σ price(t) · power(t) over the horizon and chooses the
//! cheapest time-window to precool.
//!
//! ## Short-cycling Protection
//! Industrial compressors require minimum runtime (900s) and minimum
//! off-time (600s) to prevent oil return failure and motor burnout.
//! A state machine governor enforces these constraints. Only Emergency
//! override can bypass.
//!
//! ## Time Domain
//! ALL time-dependent operations use `current_sim_time_sec` (simulated clock),
//! NOT wall-clock time. This prevents the "time domain collision" where
//! governor locks last 900+ simulated hours instead of 900 simulated seconds.

use crate::config::Config;
use crate::sensor::SensorReading;
use serde::{Deserialize, Serialize};

/// Decision produced by the control engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlDecision {
    pub compressor_on: bool,
    /// VFD speed command (0.0..=1.0) when `control.vfd_enabled`; `None` in
    /// legacy on/off mode. Forced-cooling modes always command `Some(1.0)`.
    pub compressor_speed: Option<f64>,
    pub setpoint_c: f64,
    pub active_controller: ControllerMode,
    pub ttf_sec: f64,
    pub thermal_integral: f64,
    pub grid_deviation_hz: f64,
    pub virtual_price_eur_per_mwh: f64,
    /// MPC: optimal cooling start hour (relative to current)
    pub mpc_optimal_window_h: f64,
    /// MPC: optimal pre-cooling duration (hours)
    pub mpc_optimal_duration_h: f64,
    /// MPC: predicted cost of the chosen strategy (€)
    pub mpc_predicted_cost_eur: f64,
    /// MPC: confidence level (0..1)
    pub mpc_confidence: f64,
}

/// Which controller mode is active.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ControllerMode {
    Idle,
    Arbitrage,
    ThermalGuard,
    GridBalancing,
    Emergency,
    /// MPC-selected pre-cooling window
    MpcPreCool,
    /// VFD oil-return pulse: brief forced high-speed run so oil returns to
    /// the compressor sump on legacy racks during extended low-speed operation.
    OilReturnPulse,
}

/// The control engine state.
pub struct ControlEngine {
    thermal_integral: f64,
    last_integral_update: f64,
    virtual_price_eur_per_mwh: f64,
    config: Config,
    /// Emergency latches until the product cools below the reset band, so a
    /// single frame cannot silently drop the alarm while the product is critical.
    emergency_latched: bool,

    // ── Short-cycling governor state ──────────────────────
    comp_last_switch_time: f64,
    comp_last_state: bool,
    comp_lock_active: bool,
    comp_locked_state: bool,
    comp_lock_until: f64,

    // ── VFD oil-return pulse state ────────────────────────
    oil_return_last_pulse_at: f64,
    oil_return_pulse_until: f64,
    oil_return_pulse_count: u64,

    // ── Cargo Thermal Lag State ──────────────────────────
    cargo_temp_c: f64,
    cargo_last_update: f64,

    // ── VFD Dynamic Tracking State ───────────────────────
    vfd_integral: f64,
    vfd_last_time: f64,

    // ── MPC state ─────────────────────────────────────────
    mpc_lookahead: Vec<(f64, f64)>,  // (price_eur_per_mwh, ambient_temp_c)
    mpc_optimal_hour: f64,
    mpc_optimal_duration_h: f64,
    mpc_predicted_cost: f64,
    mpc_confidence: f64,

    // ── Slew Rate / Ramp Limiter State ────────────────────
    last_commanded_setpoint_c: f64,
    last_setpoint_time_sec: f64,
    has_initialized_slew: bool,
}

impl ControlEngine {
    pub fn new(config: Config) -> Self {
        let initial_temp = config.thermal.t_setpoint_normal;
        Self {
            thermal_integral: 0.0,
            last_integral_update: 0.0,
            virtual_price_eur_per_mwh: 50.0,
            config,
            emergency_latched: false,
            comp_last_switch_time: 0.0,
            comp_last_state: false,
            comp_lock_active: false,
            comp_locked_state: false,
            comp_lock_until: 0.0,
            oil_return_last_pulse_at: 0.0,
            oil_return_pulse_until: 0.0,
            oil_return_pulse_count: 0,
            cargo_temp_c: initial_temp,
            cargo_last_update: 0.0,
            vfd_integral: 0.0,
            vfd_last_time: 0.0,
            mpc_lookahead: Vec::new(),
            mpc_optimal_hour: 0.0,
            mpc_optimal_duration_h: 0.0,
            mpc_predicted_cost: 0.0,
            mpc_confidence: 0.0,
            last_commanded_setpoint_c: initial_temp,
            last_setpoint_time_sec: 0.0,
            has_initialized_slew: false,
        }
    }

    pub fn set_mpc_lookahead(&mut self, lookahead: Vec<(f64, f64)>) {
        self.mpc_lookahead = lookahead;
    }

    pub fn update_price(&mut self, price_eur_per_mwh: f64) {
        // Reject non-finite prices so the financial accumulator can't be poisoned.
        if price_eur_per_mwh.is_finite() {
            self.virtual_price_eur_per_mwh = price_eur_per_mwh;
        }
    }

    pub fn cargo_temp_c(&self) -> f64 {
        self.cargo_temp_c
    }

    pub fn vfd_integral(&self) -> f64 {
        self.vfd_integral
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Build the fail-safe decision for sensor/communication loss: the product
    /// can no longer be observed, so the only safe action is to keep cooling
    /// and raise the alarm. Used by the gateway when PLC data goes stale.
    pub fn fail_safe_decision(&self) -> ControlDecision {
        ControlDecision {
            compressor_on: true,
            setpoint_c: self.config.thermal.t_setpoint_min,
            active_controller: ControllerMode::Emergency,
            ttf_sec: 0.0,
            thermal_integral: self.thermal_integral,
            grid_deviation_hz: 0.0,
            virtual_price_eur_per_mwh: self.virtual_price_eur_per_mwh,
            mpc_optimal_window_h: self.mpc_optimal_hour,
            mpc_optimal_duration_h: self.mpc_optimal_duration_h,
            mpc_predicted_cost_eur: self.mpc_predicted_cost,
            mpc_confidence: self.mpc_confidence,
            compressor_speed: self.config.control.vfd_enabled.then_some(1.0),
        }
    }

    /// Apply a remotely-updated configuration to the RUNNING engine.
    /// (PUT /config used to return OK but only took effect after restart.)
    pub fn apply_config(&mut self, cfg: Config) {
        self.config = cfg;
    }

    // ── MPC Solver ─────────────────────────────────────────

    fn solve_mpc(&mut self, reading: &SensorReading) {
        let horizon = self.mpc_lookahead.len();
        if horizon < 4 {
            self.mpc_optimal_hour = 0.0;
            self.mpc_predicted_cost = f64::INFINITY;
            self.mpc_confidence = 0.0;
            return;
        }

        let r_th = self.config.thermal.r_th;
        let c_th = reading.estimated_c_th;
        let electrical_power_w = self.config.sensor.cooling_power_w;
        let tau = r_th * c_th;
        if tau <= 0.0 { return; }

        let t_start = reading.internal_temp_c;
        let t_target = self.config.thermal.t_setpoint_min;
        let t_max = self.config.thermal.t_max;
        let carnot_eta = 0.6;

        let mut best_start = 0usize;
        let mut best_duration = 0usize;
        let mut best_cost = f64::INFINITY;
        let mut worst_cost = 0.0;

        // Candidate search: evaluate (start_delay, duration_hours) combinations.
        // A duration of 0 means passive floating until thermal constraints force cooling.
        let max_duration = horizon.min(8);

        for start_delay in 0..horizon {
            for duration in 0..=max_duration.min(horizon - start_delay) {
                let mut total_cost = 0.0;
                let mut sim_temp = t_start;
                let mut path_failed = false;

                for h in 0..horizon {
                    let (price, ambient) = self.mpc_lookahead[h];
                    if !price.is_finite() || !ambient.is_finite() {
                        path_failed = true;
                        break;
                    }
                    let dt = 3600.0;

                    // Active pre-cool block if within [start_delay, start_delay + duration)
                    // and not already below minimum safe setpoint:
                    let is_in_precool_window = duration > 0
                        && h >= start_delay
                        && h < start_delay + duration;

                    let compressor_on = if is_in_precool_window {
                        sim_temp > t_target
                    } else if sim_temp >= t_max - 0.2 {
                        // Safety violation avoidance: must run if hitting upper boundary
                        true
                    } else {
                        // Otherwise coast / float during expensive periods
                        false
                    };

                    if !compressor_on && sim_temp >= t_max {
                        path_failed = true;
                        break;
                    }

                    if compressor_on {
                        let t_cold_k = sim_temp + 273.15;
                        let t_warm_k = ambient + 273.15;
                        let delta_t = (t_warm_k - t_cold_k).clamp(0.1, 100.0);
                        let cop = (carnot_eta * t_cold_k / delta_t).clamp(0.5, 6.0);
                        let thermal_power_w = electrical_power_w * cop;
                        let eff = ambient - thermal_power_w * r_th;
                        sim_temp = eff + (sim_temp - eff) * (-dt / tau).exp();
                        total_cost += (electrical_power_w / 1_000_000.0) * price;
                    } else {
                        sim_temp = ambient + (sim_temp - ambient) * (-dt / tau).exp();
                    }
                }

                // Terminal penalty for leaving the store above target
                if sim_temp > t_target && !path_failed {
                    let final_price = self.mpc_lookahead.last().unwrap().0.max(10.0);
                    let estimated_hours_left = (sim_temp - t_target).max(0.0) / 2.0;
                    total_cost += estimated_hours_left * (electrical_power_w / 1_000_000.0) * final_price;
                }

                if path_failed { total_cost = f64::INFINITY; }

                if total_cost < best_cost {
                    best_cost = total_cost;
                    best_start = start_delay;
                    best_duration = duration;
                }
                if total_cost != f64::INFINITY && total_cost > worst_cost {
                    worst_cost = total_cost;
                }
            }
        }

        self.mpc_confidence = if worst_cost > 0.0 && best_cost < worst_cost && best_cost != f64::INFINITY {
            (1.0 - best_cost / worst_cost).clamp(0.0, 1.0)
        } else { 0.0 };
        self.mpc_optimal_hour = best_start as f64;
        self.mpc_predicted_cost = best_cost;
        self.mpc_optimal_duration_h = best_duration as f64;
    }

    // ── Short-cycling governor (SIMULATED clock) ──────────
    // ALL time values are simulated seconds, NOT wall-clock seconds.
    // This prevents the "time domain collision" where 1 real second = 1 simulated hour.

    fn governor_lock(&mut self, desired_state: bool, sim_time_sec: f64) -> bool {
        if self.comp_lock_active {
            if sim_time_sec < self.comp_lock_until {
                return self.comp_locked_state;
            } else {
                self.comp_lock_active = false;
            }
        }
        if desired_state != self.comp_last_state {
            self.comp_last_switch_time = sim_time_sec;
            self.comp_last_state = desired_state;
            let duration = if desired_state { self.config.control.min_runtime_sec } else { self.config.control.min_off_time_sec };
            self.comp_lock_active = true;
            self.comp_locked_state = desired_state;
            self.comp_lock_until = sim_time_sec + duration;
            return self.comp_locked_state;
        }
        desired_state
    }

    pub fn governor_locked_until(&self) -> f64 { if self.comp_lock_active { (self.comp_lock_until - 0.0).max(0.0) } else { 0.0 } }
    pub fn governor_override(&self) -> Option<bool> { if self.comp_lock_active { Some(self.comp_locked_state) } else { None } }

    /// Schedule the periodic VFD oil-return pulse (hardware protection for
    /// legacy racks running at low speed). A pulse starts only when the ASCT
    /// governor would allow an ON transition (min-run/min-offtime locks are
    /// never violated); the speed override itself is applied in `decide()`.
    fn schedule_oil_return_pulse(&mut self, sim_time_sec: f64) {
        let every = self.config.control.vfd_oil_return_pulse_every_sec;
        if !self.config.control.vfd_enabled || every <= 0.0 {
            return;
        }
        // Already pulsing, or locked OFF by the governor → keep waiting.
        let locked_off = self.comp_lock_active && !self.comp_locked_state && sim_time_sec < self.comp_lock_until;
        if sim_time_sec < self.oil_return_pulse_until || locked_off {
            return;
        }
        if sim_time_sec - self.oil_return_last_pulse_at >= every {
            self.oil_return_last_pulse_at = sim_time_sec;
            self.oil_return_pulse_until = sim_time_sec + self.config.control.vfd_oil_return_pulse_duration_sec;
            self.oil_return_pulse_count = self.oil_return_pulse_count.saturating_add(1);
            log::info!(
                "🛢️ VFD oil-return pulse #{} — {:.0}s @ {:.0}% speed",
                self.oil_return_pulse_count,
                self.config.control.vfd_oil_return_pulse_duration_sec,
                self.config.control.vfd_oil_return_pulse_speed * 100.0,
            );
            crate::metrics::inc_oil_return_pulse();
        }
    }

    // ── Main decision loop ─────────────────────────────────

    pub fn decide(
        &mut self,
        reading: &SensorReading,
        current_sim_time_sec: f64,
    ) -> ControlDecision {
        // FAIL-SAFE: a non-finite sensor reading must never be interpreted as
        // "room is cold, keep compressor off" (NaN comparisons are all false).
        // Force cooling + Emergency so a bad sensor cannot silently endanger
        // the cold chain. Upstream gates should prevent this, this is the last line.
        if !reading.internal_temp_c.is_finite() || !reading.ambient_temp_c.is_finite() {
            log::error!("🚨 Sensor reading non-finite — FAIL-SAFE: forcing cooling + Emergency");
            return ControlDecision {
                compressor_on: true,
                setpoint_c: self.config.thermal.t_setpoint_min,
                active_controller: ControllerMode::Emergency,
                ttf_sec: 0.0,
                thermal_integral: self.thermal_integral,
                grid_deviation_hz: 0.0,
                virtual_price_eur_per_mwh: self.virtual_price_eur_per_mwh,
                mpc_optimal_window_h: self.mpc_optimal_hour,
                mpc_optimal_duration_h: self.mpc_optimal_duration_h,
                mpc_predicted_cost_eur: self.mpc_predicted_cost,
                mpc_confidence: self.mpc_confidence,
                compressor_speed: self.config.control.vfd_enabled.then_some(1.0),
            };
        }

        self.update_thermal_integral(reading, current_sim_time_sec);
        self.update_cargo_temp(reading.internal_temp_c, current_sim_time_sec);
        let ttf = self.compute_ttf(reading);
        let grid_dev = reading.grid_frequency_hz - self.config.control.freq_nominal;
        self.solve_mpc(reading);

        let mut decision = ControlDecision {
            compressor_on: false,
            setpoint_c: self.config.thermal.t_setpoint_normal,
            active_controller: ControllerMode::Idle,
            ttf_sec: ttf,
            thermal_integral: self.thermal_integral,
            grid_deviation_hz: grid_dev,
            virtual_price_eur_per_mwh: self.virtual_price_eur_per_mwh,
            mpc_optimal_window_h: self.mpc_optimal_hour,
            mpc_optimal_duration_h: self.mpc_optimal_duration_h,
            mpc_predicted_cost_eur: self.mpc_predicted_cost,
            mpc_confidence: self.mpc_confidence,
            compressor_speed: None,
        };

        // Priority 1: Emergency Override (bypasses governor, but records the
        // switch so the governor can't force OFF mid-emergency — anti-chatter).
        // Latches until the product recovers below the normal setpoint.
        let temp_at_limit = reading.internal_temp_c >= self.config.thermal.t_max;
        let cargo_at_limit = self.config.thermal.cargo_lag_enabled
            && self.cargo_temp_c >= self.config.thermal.cargo_t_max;

        if self.emergency_latched
            && reading.internal_temp_c <= self.config.thermal.t_setpoint_normal
            && (!self.config.thermal.cargo_lag_enabled || self.cargo_temp_c <= self.config.thermal.t_setpoint_normal)
        {
            self.emergency_latched = false; // recovered
        }
        let emergency = self.emergency_latched || ttf < self.config.control.ttf_critical_sec || temp_at_limit || cargo_at_limit;
        if emergency {
            self.emergency_latched = true;
            decision.compressor_on = true;
            decision.setpoint_c = self.config.thermal.t_setpoint_min;
            decision.active_controller = ControllerMode::Emergency;
            decision.ttf_sec = ttf;
            if self.config.control.vfd_enabled {
                decision.compressor_speed = Some(1.0);
            }
            // Keep the governor state consistent with reality (see governor docs):
            // record the ON transition + start a min-runtime lock so recovery
            // can't immediately force OFF (short-cycling / chatter).
            if !self.comp_last_state {
                self.comp_last_switch_time = current_sim_time_sec;
                self.comp_last_state = true;
                self.comp_lock_active = true;
                self.comp_locked_state = true;
                self.comp_lock_until = current_sim_time_sec + self.config.control.min_runtime_sec;
            }
            return decision;
        }

        // Schedule the periodic VFD oil-return pulse (hardware protection);
        // the actual speed override is applied after the governor merge.
        self.schedule_oil_return_pulse(current_sim_time_sec);

        // Freeze protection (food safety floor): if internal temperature is at or below
        // t_min (-24°C), immediately cut cooling OFF regardless of governor locks.
        // Over-freezing damages inventory.
        if reading.internal_temp_c <= self.config.thermal.t_min {
            decision.compressor_on = false;
            decision.compressor_speed = Some(0.0);
            decision.setpoint_c = self.config.thermal.t_setpoint_normal;
            decision.active_controller = ControllerMode::Idle;
            self.comp_last_state = false;
            self.comp_lock_active = false;
            return decision;
        }

        // Apply governor with SIMULATED clock
        let raw_decision = self.compute_raw_decision(reading, ttf, grid_dev, current_sim_time_sec);
        let raw_wanted_on = raw_decision.compressor_on;
        let governed_on = self.governor_lock(raw_wanted_on, current_sim_time_sec);
        decision.compressor_on = governed_on;

        if let Some(locked_state) = self.governor_override() {
            if locked_state {
                decision.setpoint_c = self.config.thermal.t_setpoint_min;
                if raw_wanted_on != governed_on {
                    // Governor forced ON against a raw OFF intent → flag it.
                    decision.active_controller = ControllerMode::ThermalGuard;
                } else {
                    // Governor allowed/kept the raw ON intent — preserve the
                    // raw mode (grid response, pre-cool, arbitrage, ...).
                    // (Pre-existing bug: this branch used to leave the mode at
                    // Idle whenever a fresh ON lock was set.)
                    decision.active_controller = raw_decision.active_controller;
                }
            }
        } else {
            decision.active_controller = raw_decision.active_controller;
            decision.setpoint_c = raw_decision.setpoint_c;
        }

        // Propagate the VFD speed from the raw decision and keep it consistent
        // with the governor's final on/off state: never command a 0-speed
        // register write while the thermostat/coil says ON (oil-return floor).
        decision.compressor_speed = match raw_decision.compressor_speed {
            Some(s) if decision.compressor_on => Some(s.max(0.15)),
            Some(_) => Some(0.0),
            None => decision.compressor_speed,
        };

        // Active oil-return pulse overrides economic modes (never safety modes
        // — Emergency/ThermalGuard return earlier — nor an active grid
        // response, which is a committed service).
        if current_sim_time_sec < self.oil_return_pulse_until
            && raw_decision.active_controller != ControllerMode::GridBalancing
        {
            decision.compressor_on = true;
            decision.setpoint_c = self.config.thermal.t_setpoint_normal;
            decision.active_controller = ControllerMode::OilReturnPulse;
            decision.compressor_speed = Some(self.config.control.vfd_oil_return_pulse_speed.clamp(0.5, 1.0));
        }

        // ── Slew Rate / Ramp Limiter ──
        // Smooth setpoint transitions to prevent thermal shock / liquid hammer (unless Emergency).
        if self.config.control.max_setpoint_slew_c_per_hour > 0.0
            && decision.active_controller != ControllerMode::Emergency
        {
            if !self.has_initialized_slew {
                self.has_initialized_slew = true;
                self.last_commanded_setpoint_c = self.config.thermal.t_setpoint_normal;
                self.last_setpoint_time_sec = current_sim_time_sec;
                decision.setpoint_c = self.config.thermal.t_setpoint_normal;
            } else {
                let dt = (current_sim_time_sec - self.last_setpoint_time_sec).max(0.0);
                let max_delta = (self.config.control.max_setpoint_slew_c_per_hour / 3600.0) * dt;
                let delta = decision.setpoint_c - self.last_commanded_setpoint_c;
                let clamped_delta = delta.clamp(-max_delta, max_delta);
                decision.setpoint_c = self.last_commanded_setpoint_c + clamped_delta;
                self.last_commanded_setpoint_c = decision.setpoint_c;
                self.last_setpoint_time_sec = current_sim_time_sec;
            }
        } else {
            self.last_commanded_setpoint_c = decision.setpoint_c;
            self.last_setpoint_time_sec = current_sim_time_sec;
        }

        decision
    }

    fn compute_raw_decision(
        &mut self, reading: &SensorReading, ttf: f64, grid_dev: f64, current_sim_time_sec: f64,
    ) -> ControlDecision {
        let mut decision = ControlDecision {
            compressor_on: false, setpoint_c: self.config.thermal.t_setpoint_normal,
            active_controller: ControllerMode::Idle, ttf_sec: ttf,
            thermal_integral: self.thermal_integral, grid_deviation_hz: grid_dev,
            virtual_price_eur_per_mwh: self.virtual_price_eur_per_mwh,
            mpc_optimal_window_h: self.mpc_optimal_hour,
            mpc_optimal_duration_h: self.mpc_optimal_duration_h,
            mpc_predicted_cost_eur: self.mpc_predicted_cost,
            mpc_confidence: self.mpc_confidence,
            compressor_speed: None,
        };

        if self.thermal_integral > self.config.control.thermal_integral_max {
            decision.compressor_on = true; decision.setpoint_c = self.config.thermal.t_setpoint_normal;
            decision.active_controller = ControllerMode::ThermalGuard;
            if self.config.control.vfd_enabled { decision.compressor_speed = Some(1.0); }
            return decision;
        }
        // ── Step 1: Determine Economic Baseline (Prio 2: Arbitrage / MPC / Thermostat) ──
        let mut baseline_on = self.normal_thermostat(reading);
        let mut baseline_setpoint = self.config.thermal.t_setpoint_normal;
        let mut baseline_mode = ControllerMode::Idle;

        // High ambient temperature derating:
        // Above derate_ambient_c_threshold (e.g. 28°C), transcritical CO2 and DX racks
        // experience extreme condensing pressures and reduced COP. Aggressive shedding
        // and pre-cooling are derated to protect compressor health and avoid high-pressure trips.
        let ambient_hot = self.config.control.derate_ambient_c_threshold > 0.0
            && reading.ambient_temp_c >= self.config.control.derate_ambient_c_threshold;

        // Intraday fast-path: when the LIVE price deviates from the day-ahead
        // forecast for the current hour beyond the configured threshold,
        // re-trigger the pre-cool / shed decision immediately instead of
        // waiting for the next MPC solve.
        let intraday_threshold = self.config.control.mpc_intraday_deviation_threshold_eur_per_mwh;
        if ambient_hot {
            // Keep normal thermostat cooling, no economic shedding or pre-cooling excursions
            baseline_on = self.normal_thermostat(reading);
            baseline_setpoint = self.config.thermal.t_setpoint_normal;
            baseline_mode = ControllerMode::Idle;
        } else if intraday_threshold > 0.0 && !self.mpc_lookahead.is_empty() {
            let forecast_now = self.mpc_lookahead[0].0;
            let deviation = self.virtual_price_eur_per_mwh - forecast_now;
            if deviation <= -intraday_threshold {
                baseline_on = true;
                baseline_setpoint = self.config.thermal.t_setpoint_min;
                baseline_mode = ControllerMode::MpcPreCool;
            } else if deviation >= intraday_threshold
                && reading.internal_temp_c < self.config.thermal.t_setpoint_normal
                && ttf > self.config.control.ttf_critical_sec * 2.0
            {
                baseline_on = false;
                baseline_setpoint = self.config.thermal.t_setpoint_max;
                baseline_mode = ControllerMode::Arbitrage;
            }
        } else if self.mpc_confidence > 0.2 && self.mpc_lookahead.len() >= 4 {
            let start_h = self.mpc_optimal_hour;
            let duration_h = self.mpc_optimal_duration_h;
            let in_window = start_h <= 0.5 && duration_h > 0.0;
            let price_acceptable = !self.mpc_lookahead.is_empty()
                && self.mpc_lookahead[0].0 <= self.config.control.mpc_cheap_threshold_eur_per_mwh;

            if (in_window || price_acceptable) && reading.internal_temp_c > self.config.thermal.t_setpoint_min {
                baseline_on = true;
                baseline_setpoint = self.config.thermal.t_setpoint_min;
                baseline_mode = ControllerMode::MpcPreCool;
            } else if start_h > 1.0 {
                // Optimal window is in the future: float with conservative setpoint to conserve chill
                baseline_on = self.normal_thermostat(reading);
                baseline_setpoint = self.config.thermal.t_setpoint_max;
                baseline_mode = ControllerMode::MpcPreCool;
            }
        } else if self.should_arbitrage(reading) {
            let (charge_th, shed_th) = self.price_thresholds();
            if self.virtual_price_eur_per_mwh < charge_th {
                baseline_on = true;
                baseline_setpoint = self.config.thermal.t_setpoint_min;
            } else if self.virtual_price_eur_per_mwh > shed_th {
                baseline_on = false;
                baseline_setpoint = self.config.thermal.t_setpoint_max;
            } else {
                baseline_on = reading.internal_temp_c > self.config.thermal.t_setpoint_normal;
            }
            baseline_mode = ControllerMode::Arbitrage;
        }

        // ── Step 2: Calculate Baseline VFD Speed (if VFD enabled) ──
        let baseline_speed = if self.config.control.vfd_enabled {
            if baseline_on {
                self.vfd_speed_for(reading, baseline_setpoint, current_sim_time_sec)
            } else {
                0.0
            }
        } else {
            if baseline_on { 1.0 } else { 0.0 }
        };

        // ── Step 3: Layer Grid Balancing (Prio 3: Frequency Response) ──
        // Only layer frequency response if grid frequency deviates beyond deadband.
        if grid_dev.abs() > self.config.control.grid_balancing_deadband_hz {
            let want_more_cooling = grid_dev > 0.0;
            let (_, shed_th) = self.price_thresholds();
            // Economic profitability check: if over-frequency (want more power),
            // ensure spot price isn't at peak-shed level unless price droop makes it net negative/cheap.
            let economic_ok_to_cool = !want_more_cooling
                || self.virtual_price_eur_per_mwh < shed_th
                || grid_dev > 0.10; // Strong over-frequency pays highest FCR bonus

            if economic_ok_to_cool {
                if self.config.control.vfd_enabled {
                    // Continuous Proportional Droop:
                    // Δf > 0 increases speed (soak excess power); Δf < 0 decreases speed (relieve grid).
                    // ±0.20 Hz deviation commands ±50% speed modulation around baseline.
                    let droop_speed_delta = (grid_dev / 0.20) * 0.50;
                    let modulated_speed = (baseline_speed + droop_speed_delta).clamp(0.0, 1.0);

                    // Physical safety bounds:
                    // If near t_min (-24°C), clamp upper speed to prevent over-chilling product.
                    // If near t_max (-18°C), ensure minimum speed to protect cold chain.
                    let safe_speed = if reading.internal_temp_c <= self.config.thermal.t_min {
                        modulated_speed.min(baseline_speed)
                    } else if reading.internal_temp_c >= self.config.thermal.t_max - 1.0 {
                        modulated_speed.max(0.60)
                    } else {
                        modulated_speed
                    };

                    let is_running = safe_speed >= 0.15;
                    decision.compressor_on = is_running;
                    decision.compressor_speed = Some(if is_running { safe_speed } else { 0.0 });
                    decision.setpoint_c = if want_more_cooling {
                        self.config.thermal.t_setpoint_min
                    } else {
                        self.config.thermal.t_setpoint_max
                    };
                    decision.active_controller = ControllerMode::GridBalancing;
                    return decision;
                } else {
                    // Discrete/Binary On-Off Mode:
                    let want_on = want_more_cooling;
                    let safe_want_on = if want_on && reading.internal_temp_c <= self.config.thermal.t_min {
                        false
                    } else if !want_on && reading.internal_temp_c >= self.config.thermal.t_max - 1.0 {
                        true
                    } else {
                        want_on
                    };

                    decision.compressor_on = safe_want_on;
                    decision.setpoint_c = if want_more_cooling {
                        self.config.thermal.t_setpoint_min
                    } else {
                        self.config.thermal.t_setpoint_max
                    };
                    decision.active_controller = ControllerMode::GridBalancing;
                    return decision;
                }
            }
        }

        // ── Step 4: Apply Baseline Decision if no Grid Balancing override ──
        decision.compressor_on = baseline_on;
        decision.setpoint_c = baseline_setpoint;
        decision.active_controller = baseline_mode;
        if self.config.control.vfd_enabled {
            decision.compressor_speed = Some(baseline_speed);
        }
        decision
    }

    /// Dynamic heat-balance + PI feedback VFD control law:
    /// - Feedforward term tracks the instantaneous heat load (wall leakage + internal heat).
    /// - Feedback PI term adjusts speed dynamically based on error from desired setpoint.
    ///
    /// Clamped between 0.15 (oil return floor) and 1.0 (rated compressor speed).
    fn vfd_speed_for(&mut self, reading: &SensorReading, target_setpoint_c: f64, sim_time_sec: f64) -> f64 {
        let r_th = self.config.thermal.r_th.max(0.001);
        let wall_leak_w = ((reading.ambient_temp_c - reading.internal_temp_c).max(0.0) / r_th).max(0.0);
        let internal_heat_w = self.config.sensor.heat_leak_w.max(0.0);
        let cooling_w = (self.config.sensor.cooling_power_w * 2.0).max(1.0);
        let feedforward = (wall_leak_w + internal_heat_w) / cooling_w;

        // PI feedback on temperature deviation:
        let err = reading.internal_temp_c - target_setpoint_c;
        let dt = (sim_time_sec - self.vfd_last_time).clamp(0.0, 3600.0);
        if dt > 0.0 && dt.is_finite() {
            self.vfd_integral += err * dt;
            // Anti-windup clamping on integral term
            self.vfd_integral = self.vfd_integral.clamp(-500.0, 500.0);
            self.vfd_last_time = sim_time_sec;
        }

        let p_term = self.config.control.vfd_kp * err;
        let i_term = self.config.control.vfd_ki * self.vfd_integral;
        let commanded_speed = feedforward + p_term + i_term;

        commanded_speed.clamp(0.15, 1.0)
    }

    fn update_thermal_integral(&mut self, reading: &SensorReading, sim_time_sec: f64) {
        let dt = (sim_time_sec - self.last_integral_update).clamp(0.0, 3600.0);
        if dt < 0.001 || !dt.is_finite() { return; }
        let excess = (reading.internal_temp_c - self.config.thermal.t_setpoint_normal).max(0.0);
        if !excess.is_finite() { return; }
        self.thermal_integral += excess * dt;
        // Time-consistent decay: 0.99^dt assumed an hourly cadence, but frames
        // advance ~3600 simulated seconds each. Normalize to hours so sub-1°C
        // excursions actually ACCUMULATE (was effectively memoryless at dt≈3600).
        self.thermal_integral *= 0.99_f64.powf(dt / 3600.0);
        self.thermal_integral = self.thermal_integral.clamp(0.0, self.config.control.thermal_integral_max * 2.0);
        self.last_integral_update = sim_time_sec;
    }

    /// Advance the cargo core temperature lag model.
    /// Cargo has high thermal mass coupled through cargo_r_th to internal air.
    fn update_cargo_temp(&mut self, air_temp: f64, sim_time_sec: f64) {
        if !self.config.thermal.cargo_lag_enabled {
            self.cargo_temp_c = air_temp;
            self.cargo_last_update = sim_time_sec;
            return;
        }
        let dt = (sim_time_sec - self.cargo_last_update).clamp(0.0, 7200.0);
        if dt < 0.001 || !dt.is_finite() {
            return;
        }
        let cargo_tau = self.config.thermal.cargo_r_th * self.config.thermal.cargo_c_th;
        if cargo_tau > 0.0 && cargo_tau.is_finite() && air_temp.is_finite() {
            self.cargo_temp_c = air_temp + (self.cargo_temp_c - air_temp) * (-dt / cargo_tau).exp();
        }
        self.cargo_last_update = sim_time_sec;
    }

    pub fn cargo_temperature_c(&self) -> f64 {
        self.cargo_temp_c
    }

    fn compute_ttf(&self, reading: &SensorReading) -> f64 {
        let t_max = self.config.thermal.t_max;
        let delta = t_max - reading.internal_temp_c;
        // Already at/above the limit: failure is IMMEDIATE even if the compressor
        // claims to be running (catches a running-but-failing refrigeration plant).
        if !delta.is_finite() { return f64::INFINITY; }
        if delta <= 0.0 { return 0.0; }
        if reading.compressor_on { return f64::INFINITY; }
        // Use the RLS-adaptive capacitance, not the static config value: the
        // safety layer must protect the physics the estimator actually learned.
        let tau = self.config.thermal.r_th * reading.estimated_c_th;
        if !tau.is_finite() || tau <= 0.0 { return f64::INFINITY; }
        let rate = (reading.ambient_temp_c - reading.internal_temp_c) / tau;
        // NaN rate must never reach the comparison below (NaN < x is false →
        // Emergency would silently never fire). Fail towards INFINITY (safe).
        if !rate.is_finite() || rate <= 0.0 { return f64::INFINITY; }
        delta / rate
    }

    /// Compute effective (charge_threshold, shed_threshold) for arbitrage.
    /// Uses lookahead percentiles if enabled and data is available,
    /// otherwise falls back to static 30.0 / 80.0 EUR/MWh.
    pub fn price_thresholds(&self) -> (f64, f64) {
        if self.config.control.dynamic_price_thresholds_enabled && !self.mpc_lookahead.is_empty() {
            let mut prices: Vec<f64> = self.mpc_lookahead
                .iter()
                .map(|(p, _)| *p)
                .filter(|p| p.is_finite())
                .collect();
            if !prices.is_empty() {
                prices.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let lo_idx = ((prices.len() as f64 * self.config.control.dynamic_price_charge_percentile).floor() as usize)
                    .min(prices.len() - 1);
                let hi_idx = ((prices.len() as f64 * self.config.control.dynamic_price_shed_percentile).ceil() as usize)
                    .min(prices.len() - 1);
                let lo_thresh = prices[lo_idx];
                let hi_thresh = prices[hi_idx];
                if lo_thresh < hi_thresh {
                    return (lo_thresh, hi_thresh);
                }
            }
        }
        (30.0, 80.0)
    }

    fn should_arbitrage(&self, reading: &SensorReading) -> bool {
        if reading.internal_temp_c >= self.config.thermal.t_max || reading.internal_temp_c <= self.config.thermal.t_min { return false; }
        let (charge_th, shed_th) = self.price_thresholds();
        self.virtual_price_eur_per_mwh < charge_th || self.virtual_price_eur_per_mwh > shed_th
    }

    fn normal_thermostat(&self, reading: &SensorReading) -> bool {
        let t = reading.internal_temp_c; let target = self.config.thermal.t_setpoint_normal;
        if reading.compressor_on { t > target - 1.0 } else { t >= target + 1.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn make_reading(temp: f64, ambient: f64, compressor_on: bool, freq: f64) -> SensorReading {
        SensorReading {
            timestamp: chrono::Utc::now(),
            internal_temp_c: temp, ambient_temp_c: ambient,
            compressor_on, power_consumption_w: 0.0, grid_frequency_hz: freq,
            estimated_c_th: 1_000_000.0,
        }
    }

    #[test]
    fn vfd_disabled_keeps_on_off_semantics() {
        // Default config (vfd_enabled = false) must be byte-for-byte legacy
        // on/off: no speed command, coil semantics untouched.
        let config = Config::default();
        let mut engine = ControlEngine::new(config);
        let reading = make_reading(-19.0, 25.0, false, 50.0);
        let decision = engine.decide(&reading, 0.0);
        assert_eq!(decision.compressor_speed, None, "on/off mode must not emit a speed");
    }

    #[test]
    fn vfd_enabled_modulates_idle_speed() {
        let mut config = Config::default();
        config.control.vfd_enabled = true;
        config.thermal.r_th = 0.05;
        config.sensor.cooling_power_w = 50_000.0;
        config.sensor.heat_leak_w = 7_937.5;
        let mut engine = ControlEngine::new(config.clone());

        // Thermostat ON (temp above the +1 K deadband): speed must be a
        // modulated heat-balance value in the safe [0.15, 1.0] band.
        let reading = make_reading(-19.0, 25.0, false, 50.0);
        let decision = engine.decide(&reading, 0.0);
        assert_eq!(decision.active_controller, ControllerMode::Idle);
        match decision.compressor_speed {
            Some(s) => assert!((0.15..=1.0).contains(&s), "modulated speed {s} out of range"),
            None => panic!("vfd_enabled must emit a speed"),
        }

        // Thermostat satisfied: VFD idles at 0.0 (like a coil OFF). Fresh
        // engine so the min-runtime governor lock from the ON case can't hold
        // the compressor on.
        let mut engine_cold = ControlEngine::new(config);
        let reading_cold = make_reading(-20.0, -22.0, false, 50.0);
        let decision_cold = engine_cold.decide(&reading_cold, 0.0);
        assert_eq!(decision_cold.compressor_speed, Some(0.0));
    }

    #[test]
    fn vfd_emergency_forces_full_speed() {
        let mut config = Config::default();
        config.control.vfd_enabled = true;
        let t_max = config.thermal.t_max;
        let mut engine = ControlEngine::new(config);
        // At t_max the emergency path fires → forced cooling at 100% speed.
        let reading = make_reading(t_max, 25.0, false, 50.0);
        let decision = engine.decide(&reading, 0.0);
        assert_eq!(decision.active_controller, ControllerMode::Emergency);
        assert_eq!(decision.compressor_speed, Some(1.0));
    }

    #[test]
    fn oil_return_pulse_forces_full_speed_on_interval() {
        let mut config = Config::default();
        config.control.vfd_enabled = true;
        config.control.vfd_oil_return_pulse_every_sec = 100.0;
        config.control.vfd_oil_return_pulse_duration_sec = 30.0;
        config.control.vfd_oil_return_pulse_speed = 1.0;
        let mut engine = ControlEngine::new(config.clone());
        // Room satisfied (would normally idle) — once the interval elapses the
        // pulse still forces a brief full-speed run.
        let reading = make_reading(-20.0, -22.0, false, 50.0);
        let decision = engine.decide(&reading, 100.0);
        assert_eq!(decision.active_controller, ControllerMode::OilReturnPulse);
        assert!(decision.compressor_on);
        assert_eq!(decision.compressor_speed, Some(1.0));

        // Before the interval elapses no pulse fires.
        let mut engine2 = ControlEngine::new(config);
        let decision2 = engine2.decide(&reading, 50.0);
        assert_ne!(decision2.active_controller, ControllerMode::OilReturnPulse);
    }

    #[test]
    fn oil_return_pulse_respects_governor_off_lock() {
        let mut config = Config::default();
        config.control.vfd_enabled = true;
        config.control.vfd_oil_return_pulse_every_sec = 100.0;
        let mut engine = ControlEngine::new(config);
        // Force a governor OFF lock spanning t=1000 (min-offtime: 901→1501).
        engine.governor_lock(true, 0.0);
        engine.governor_lock(false, 901.0);
        let reading = make_reading(-20.0, -22.0, false, 50.0);
        let d_locked = engine.decide(&reading, 1000.0);
        assert_ne!(d_locked.active_controller, ControllerMode::OilReturnPulse,
            "pulse must not start inside a governor off-lock");
        // After the lock expires the pulse is allowed.
        let d_free = engine.decide(&reading, 1502.0);
        assert_eq!(d_free.active_controller, ControllerMode::OilReturnPulse);
    }

    #[test]
    fn oil_return_pulse_does_not_override_grid_balancing() {
        let mut config = Config::default();
        config.control.vfd_enabled = true;
        config.control.vfd_oil_return_pulse_every_sec = 100.0;
        let mut engine = ControlEngine::new(config);
        // 50.2 Hz triggers a grid response at the same moment the pulse is due —
        // the committed grid response must win.
        let reading = make_reading(-20.0, -22.0, false, 50.2);
        let decision = engine.decide(&reading, 100.0);
        assert_eq!(decision.active_controller, ControllerMode::GridBalancing,
            "an active grid response must not be overridden by the oil pulse");
    }

    #[test]
    fn intraday_fast_path_precools_on_cheap_deviation() {
        let mut config = Config::default();
        config.control.mpc_intraday_deviation_threshold_eur_per_mwh = 30.0;
        config.control.max_setpoint_slew_c_per_hour = 0.0;
        let t_min = config.thermal.t_min;
        let mut engine = ControlEngine::new(config);
        engine.set_mpc_lookahead(vec![(100.0, 25.0); 6]); // forecast 100 €/MWh now
        engine.update_price(10.0); // live price 10 → deviation −90 → charge now
        let reading = make_reading(-19.0, 25.0, false, 50.0);
        let decision = engine.decide(&reading, 0.0);
        assert_eq!(decision.active_controller, ControllerMode::MpcPreCool);
        assert!(decision.compressor_on);
        assert_eq!(decision.setpoint_c, t_min);
    }

    #[test]
    fn intraday_fast_path_sheds_on_expensive_deviation() {
        let mut config = Config::default();
        config.control.mpc_intraday_deviation_threshold_eur_per_mwh = 30.0;
        config.control.max_setpoint_slew_c_per_hour = 0.0;
        config.thermal.c_th = 4_000_000.0; // larger mass → ample TTF margin
        let t_max = config.thermal.t_max;
        let mut engine = ControlEngine::new(config);
        engine.set_mpc_lookahead(vec![(10.0, 25.0); 6]); // forecast 10 €/MWh now
        engine.update_price(100.0); // live price 100 → deviation +90 → shed
        let reading = make_reading(-22.0, 25.0, false, 50.0); // below setpoint → safe to shed
        let decision = engine.decide(&reading, 0.0);
        assert_eq!(decision.active_controller, ControllerMode::Arbitrage);
        assert!(!decision.compressor_on);
        assert_eq!(decision.setpoint_c, t_max);
    }

    #[test]
    fn intraday_fast_path_disabled_by_default() {
        let config = Config::default(); // threshold 0 → feature off
        let mut engine = ControlEngine::new(config);
        engine.set_mpc_lookahead(vec![(100.0, 25.0); 6]);
        let reading = make_reading(-19.0, 25.0, false, 50.0);
        let decision = engine.decide(&reading, 0.0);
        assert_eq!(decision.active_controller, ControllerMode::Idle);
    }

    #[test]
    fn test_governor_min_on_time() {
        let config = Config::default();
        let mut engine = ControlEngine::new(config);
        // Start at sim_time = 0, turn ON
        let result = engine.governor_lock(true, 0.0);
        assert!(result, "Request ON from OFF should be allowed");

        // Immediately request OFF at sim_time = 1 — should be blocked (900s min runtime)
        let blocked = engine.governor_lock(false, 1.0);
        assert!(blocked, "Governor should block OFF within min runtime (stays ON)");

        // After 900 simulated seconds, should be free to change
        let freed = engine.governor_lock(false, 901.0);
        assert!(!freed, "After min runtime expires, OFF should be allowed");
    }

    #[test]
    fn test_governor_min_off_time() {
        let config = Config::default();
        let mut engine = ControlEngine::new(config);
        // Turn ON at sim_time=0, OFF at sim_time=901 (after min runtime expires)
        engine.governor_lock(true, 0.0);
        engine.governor_lock(false, 901.0); // allowed

        // Immediately request ON at sim_time=902 — blocked by min_off_time (600s)
        let blocked = engine.governor_lock(true, 902.0);
        assert!(!blocked, "Governor should block ON within min off-time (stays OFF)");

        // After 600 more simulated seconds, should be free
        let freed = engine.governor_lock(true, 1502.0);
        assert!(freed, "After min off-time expires, ON should be allowed");
    }

    #[test]
    fn test_mpc_solve_finds_cheapest_window() {
        let config = Config::default();
        let mut engine = ControlEngine::new(config);
        let reading = make_reading(-20.0, 25.0, false, 50.0);
        let lookahead: Vec<(f64, f64)> = vec![
            (100.0, 25.0), (90.0, 25.0), (10.0, 25.0), (15.0, 25.0), (80.0, 25.0), (85.0, 25.0),
        ];
        engine.set_mpc_lookahead(lookahead);
        engine.solve_mpc(&reading);
        let best = engine.mpc_optimal_hour as usize;
        assert!(best <= 2, "MPC should pick early window (0-2), got {}", best);
        assert!(engine.mpc_predicted_cost.is_finite());
    }

    #[test]
    fn test_mpc_confidence_high() {
        let config = Config::default();
        let mut engine = ControlEngine::new(config);
        let reading = make_reading(-20.0, 25.0, false, 50.0);
        let lookahead: Vec<(f64, f64)> = (0..12).map(|i| {
            if (3..=6).contains(&i) { (10.0, 25.0) } else { (100.0, 25.0) }
        }).collect();
        engine.set_mpc_lookahead(lookahead);
        engine.solve_mpc(&reading);
        assert!(engine.mpc_confidence >= 0.4);
    }

    #[test]
    fn test_normal_operation() {
        let config = Config::default();
        let mut engine = ControlEngine::new(config);
        let reading = make_reading(-20.0, 25.0, false, 50.0);
        let decision = engine.decide(&reading, 0.0);
        assert!(decision.active_controller != ControllerMode::Emergency);
    }

    #[test]
    fn test_emergency_ttf() {
        let config = Config::default();
        let mut engine = ControlEngine::new(config);
        let reading = make_reading(-17.0, 40.0, false, 50.0);
        let decision = engine.decide(&reading, 0.0);
        assert!(decision.active_controller == ControllerMode::Emergency || decision.active_controller == ControllerMode::ThermalGuard);
    }

    #[test]
    fn test_grid_balancing() {
        let config = Config::default();
        let mut engine = ControlEngine::new(config);
        let reading = make_reading(-20.0, 25.0, false, 49.8);
        let decision = engine.decide(&reading, 0.0);
        assert_eq!(decision.active_controller, ControllerMode::GridBalancing);
        assert!(!decision.compressor_on);
    }

    #[test]
    fn test_nan_reading_fails_safe() {
        // A NaN sensor reading must NEVER be interpreted as "cold, stay off".
        let config = Config::default();
        let mut engine = ControlEngine::new(config);
        let reading = make_reading(f64::NAN, 25.0, false, 50.0);
        let decision = engine.decide(&reading, 0.0);
        assert_eq!(decision.active_controller, ControllerMode::Emergency);
        assert!(decision.compressor_on, "fail-safe must force cooling ON");
        assert_eq!(decision.ttf_sec, 0.0);
    }

    #[test]
    fn test_nan_ttf_never_disables_emergency() {
        // TTF must be finite-guarded: NaN rate can't compare false and skip Emergency.
        let config = Config::default();
        let mut engine = ControlEngine::new(config);
        // ambient NaN → rate NaN → old code returned NaN ttf → no emergency.
        let reading = make_reading(-17.5, f64::NAN, false, 50.0);
        let decision = engine.decide(&reading, 0.0);
        assert!(decision.active_controller == ControllerMode::Emergency,
            "expected fail-safe Emergency for NaN ambient, got {:?}", decision.active_controller);
    }

    #[test]
    fn test_ttf_zero_at_tmax_even_when_compressor_on() {
        // A running-but-failing plant (temp at limit while "on") must trip
        // Emergency, not report TTF=inf forever.
        let config = Config::default();
        let mut engine = ControlEngine::new(config);
        let reading = make_reading(-18.0, 30.0, true, 50.0); // at t_max, compressor claims ON
        assert_eq!(engine.compute_ttf(&reading), 0.0);
        let decision = engine.decide(&reading, 0.0);
        assert_eq!(decision.active_controller, ControllerMode::Emergency);
    }

    #[test]
    fn test_emergency_latches_until_recovery() {
        let config = Config::default();
        let mut engine = ControlEngine::new(config);
        // Critical temp while OFF → Emergency
        let d1 = engine.decide(&make_reading(-17.5, 30.0, false, 50.0), 0.0);
        assert_eq!(d1.active_controller, ControllerMode::Emergency);
        // Next frame compressor reports ON (previous command) but still critical:
        // Emergency must persist (old code dropped it because TTF returned inf).
        let d2 = engine.decide(&make_reading(-17.8, 30.0, true, 50.0), 1.0);
        assert_eq!(d2.active_controller, ControllerMode::Emergency,
            "Emergency must latch until the product recovers below setpoint");
        // Recovered below normal setpoint → latch released.
        let d3 = engine.decide(&make_reading(-21.0, 25.0, true, 50.0), 2.0);
        assert_ne!(d3.active_controller, ControllerMode::Emergency);
    }

    #[test]
    fn test_grid_balancing_respects_thermal_limits() {
        // Under-frequency shed request but product near t_max: must NOT shed.
        let mut engine = ControlEngine::new(Config::default());
        let near_limit = engine.decide(&make_reading(-18.5, 30.0, false, 49.8), 0.0);
        assert!(near_limit.compressor_on,
            "grid shed must be blocked near the thermal limit");
        // Over-frequency cool request but product already below t_min: must NOT cool further.
        let mut engine = ControlEngine::new(Config::default());
        let over_cold = engine.decide(&make_reading(-25.0, 30.0, false, 50.2), 0.0);
        assert!(!over_cold.compressor_on,
            "grid cool must be blocked below t_min");
    }

    #[test]
    fn test_update_price_rejects_nan() {
        let config = Config::default();
        let mut engine = ControlEngine::new(config);
        engine.update_price(42.0);
        engine.update_price(f64::NAN); // must be ignored
        assert_eq!(engine.virtual_price_eur_per_mwh, 42.0);
    }

    #[test]
    fn test_dynamic_price_thresholds() {
        let mut config = Config::default();
        config.control.dynamic_price_thresholds_enabled = true;
        config.control.dynamic_price_charge_percentile = 0.20;
        config.control.dynamic_price_shed_percentile = 0.80;

        let mut engine = ControlEngine::new(config);
        // Default when no lookahead: falls back to (30.0, 80.0)
        assert_eq!(engine.price_thresholds(), (30.0, 80.0));

        // Lookahead with a distribution from 50 to 150 EUR/MWh
        let lookahead = vec![
            (50.0, 20.0), (60.0, 20.0), (70.0, 20.0), (80.0, 20.0),
            (90.0, 20.0), (100.0, 20.0), (110.0, 20.0), (120.0, 20.0),
            (130.0, 20.0), (150.0, 20.0),
        ];
        engine.set_mpc_lookahead(lookahead);
        let (lo, hi) = engine.price_thresholds();
        assert!(lo > 30.0, "lo threshold should adapt dynamically above 30, got {lo}");
        assert!(hi > 100.0, "hi threshold should adapt dynamically above 100, got {hi}");

        // Now test arbitrage decision with dynamic threshold
        engine.update_price(lo - 5.0); // Cheaper than dynamic lo
        let reading = make_reading(-20.0, 20.0, false, 50.0);
        assert!(engine.should_arbitrage(&reading));
        let decision = engine.decide(&reading, 0.0);
        assert_eq!(decision.active_controller, ControllerMode::Arbitrage);
        assert!(decision.compressor_on);
    }

    #[test]
    fn test_cargo_thermal_lag_and_recovery() {
        let mut config = Config::default();
        config.thermal.cargo_lag_enabled = true;
        config.thermal.cargo_r_th = 0.01;
        config.thermal.cargo_c_th = 1000.0; // Small tau = 10s for fast test
        config.thermal.cargo_t_max = -18.0;
        config.thermal.t_setpoint_normal = -20.0;

        let mut engine = ControlEngine::new(config);
        assert_eq!(engine.cargo_temp_c, -20.0);

        // Warm air temperature of -17.0°C for 20 seconds.
        // Cargo starts at -20.0°C and warms exponentially towards -17.0°C.
        let warm_reading = make_reading(-17.0, 25.0, false, 50.0);
        let _ = engine.decide(&warm_reading, 0.0);
        let d2 = engine.decide(&warm_reading, 20.0);

        // After 20s (2 time constants), cargo has warmed to ~ -17.4°C >= cargo_t_max (-18.0°C).
        assert!(engine.cargo_temp_c > -18.0, "cargo should have warmed above -18°C, got {}", engine.cargo_temp_c);
        assert_eq!(d2.active_controller, ControllerMode::Emergency);
        assert!(engine.emergency_latched);

        // Now air drops quickly to cold setpoint (-21.0°C) after 1 second, but cargo is still > -20.0°C.
        let cold_air_reading = make_reading(-21.0, 25.0, true, 50.0);
        let d3 = engine.decide(&cold_air_reading, 21.0);
        // Even though air is <= normal setpoint, cargo is still warm: emergency must remain latched!
        assert!(engine.cargo_temp_c > -20.0);
        assert_eq!(d3.active_controller, ControllerMode::Emergency,
            "emergency must remain latched until cargo core also recovers");

        // Allow cargo sufficient time (e.g. +40s) in cold air to recover below -20.0°C
        let d4 = engine.decide(&cold_air_reading, 61.0);
        assert!(engine.cargo_temp_c <= -20.0, "cargo should recover below -20.0°C, got {}", engine.cargo_temp_c);
        assert!(!engine.emergency_latched, "emergency latch should be released once cargo recovers");
        assert_ne!(d4.active_controller, ControllerMode::Emergency);
    }

    #[test]
    fn test_vfd_pi_closed_loop_modulation() {
        let mut config = Config::default();
        config.control.vfd_enabled = true;
        config.control.vfd_kp = 0.05;
        config.control.vfd_ki = 0.01;
        config.thermal.t_setpoint_normal = -20.0;

        let mut engine = ControlEngine::new(config);

        // Reading is 1°C warm (-19.0°C vs setpoint -20.0°C)
        let reading = make_reading(-19.0, 20.0, true, 50.0);

        let d1 = engine.decide(&reading, 0.0);
        let speed_initial = d1.compressor_speed.expect("VFD speed should be present");
        assert_eq!(engine.vfd_integral, 0.0);

        // Advance by 50 seconds: error (+1.0°C) accumulates dt * 1.0 = 50.0 into integral
        let d2 = engine.decide(&reading, 50.0);
        let speed_after_integral = d2.compressor_speed.expect("VFD speed should be present");
        assert!((engine.vfd_integral - 50.0).abs() < 1e-4, "integral should be 50.0, got {}", engine.vfd_integral);
        assert!(speed_after_integral > speed_initial,
            "PI feedback with positive integral must increase compressor speed (initial: {}, modulated: {})",
            speed_initial, speed_after_integral
        );

        // Anti-windup test: large time jump must be clamped to anti-windup bounds [-500, 500]
        let _ = engine.decide(&reading, 100_000.0);
        assert!(engine.vfd_integral <= 500.0, "integral should clamp to anti-windup ceiling");
    }

    #[test]
    fn test_co_optimized_grid_balancing_with_vfd_droop() {
        let mut config = Config::default();
        config.control.vfd_enabled = true;
        config.thermal.t_setpoint_normal = -20.0;
        config.thermal.t_min = -24.0;
        config.thermal.t_max = -18.0;

        let mut engine = ControlEngine::new(config);

        // Scenario: Room is cold at -23.0°C. Electricity is in normal range.
        // Grid surges with +0.12 Hz over-frequency (excess power).
        let cold_surplus_reading = make_reading(-23.0, 20.0, false, 50.12);
        let decision = engine.decide(&cold_surplus_reading, 0.0);

        // System must seamlessly provide balancing without violating food safety
        assert_eq!(decision.active_controller, ControllerMode::GridBalancing);
        assert!(decision.compressor_on);
        let speed = decision.compressor_speed.expect("VFD should command modulated speed");
        assert!(speed >= 0.25, "over-frequency must modulate speed upward, got {}", speed);
    }

    #[test]
    fn test_setpoint_slew_rate_limiting() {
        let mut config = Config::default();
        config.thermal.t_setpoint_normal = -20.0;
        config.thermal.t_setpoint_min = -24.0;
        config.control.max_setpoint_slew_c_per_hour = 6.0; // max 1.0 °C per 600s
        let mut engine = ControlEngine::new(config);

        // Precool price trigger
        engine.update_price(10.0); // cheap price triggers setpoint to t_setpoint_min (-24.0)
        let reading = make_reading(-20.0, 20.0, false, 50.0);

        // Step 1 at t = 0
        let d1 = engine.decide(&reading, 0.0);
        assert_eq!(d1.setpoint_c, -20.0); // initial

        // Step 2 at t = 300s (5 minutes): max slew is (6.0 / 3600) * 300 = 0.5 °C
        let d2 = engine.decide(&reading, 300.0);
        assert!((d2.setpoint_c - (-20.5)).abs() < 1e-4, "setpoint should be slewed to -20.5, got {}", d2.setpoint_c);

        // Step 3 at t = 600s (10 minutes from start): max slew total is 1.0 °C -> -21.0
        let d3 = engine.decide(&reading, 600.0);
        assert!((d3.setpoint_c - (-21.0)).abs() < 1e-4, "setpoint should be slewed to -21.0, got {}", d3.setpoint_c);
    }

    #[test]
    fn test_high_ambient_temperature_derating() {
        let mut config = Config::default();
        config.thermal.t_setpoint_normal = -20.0;
        config.control.derate_ambient_c_threshold = 28.0;
        let mut engine = ControlEngine::new(config);

        // Very high spot price (normally triggers shedding)
        engine.update_price(120.0);

        // Case 1: Normal ambient (25°C < 28°C threshold) -> Arbitrage shed allowed
        let normal_amb_reading = make_reading(-21.0, 25.0, false, 50.0);
        let d_normal = engine.decide(&normal_amb_reading, 0.0);
        assert_eq!(d_normal.active_controller, ControllerMode::Arbitrage);
        assert!(!d_normal.compressor_on, "should shed during high price at normal ambient");

        // Case 2: High ambient (32°C >= 28°C threshold) -> Shedding suppressed to protect compressor
        let mut config_hot = Config::default();
        config_hot.thermal.t_setpoint_normal = -20.0;
        config_hot.control.derate_ambient_c_threshold = 28.0;
        let mut engine_hot = ControlEngine::new(config_hot);
        engine_hot.update_price(120.0);
        let hot_amb_reading = make_reading(-19.5, 32.0, true, 50.0);
        let d_hot = engine_hot.decide(&hot_amb_reading, 0.0);
        assert_ne!(d_hot.active_controller, ControllerMode::Arbitrage, "shedding should be suppressed during high ambient");
        assert!(d_hot.compressor_on, "compressor should run to maintain safe cooling in high heat");
    }
}