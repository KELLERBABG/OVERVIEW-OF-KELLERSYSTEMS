// ============================================================================
// 7-Mode Priority Cascade Controller — ATA.canvas full specification
// ============================================================================

use crate::store_types::StoreType;
use crate::physics::{ThermalZone, compute_ttf, compute_state_vector};
use crate::bundesland::Bundesland;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ControllerMode {
    Emergency, ThermalGuard, GridBalancing,
    MpcPreCool, Arbitrage, IdleNormal, DualMarket,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ControllerOutput {
    pub compressor_on: bool,
    pub tes_charging: bool,
    pub vfd_speed: f64,
    pub mode: ControllerMode,
    pub setpoint_c: f64,
    pub ttf_sec: f64,
    pub thermal_integral: f64,
    pub droop_revenue_eur: f64,
    pub fc_earnings_eur: f64,
    pub state_changes: u64,
}

// ── ASCT Governor + Shed Timer ─────────────────────────
#[derive(Debug, Clone)]
pub struct AsctGovernor {
    pub last_state: bool,
    pub locked_state: bool,
    pub lock_until_hour: i64,
    pub state_changes: u64,
    pub shed_hours_continuous: f64,
}

impl AsctGovernor {
    pub fn new() -> Self {
        Self {
            last_state: false, locked_state: false, lock_until_hour: -999,
            state_changes: 0, shed_hours_continuous: 0.0,
        }
    }
    pub fn apply(&mut self, desired: bool, current_hour: i64, min_run: f64, min_off: f64) -> bool {
        if self.lock_until_hour >= current_hour { return self.locked_state; }
        if desired != self.last_state {
            let duration = if desired { min_run.ceil() as i64 } else { min_off.ceil() as i64 };
            self.last_state = desired;
            self.locked_state = desired;
            self.lock_until_hour = current_hour + duration.max(1);
            self.state_changes += 1;
            return desired;
        }
        desired
    }
    pub fn update_shed_timer(&mut self, compressor_on: bool) {
        if compressor_on { self.shed_hours_continuous = 0.0; }
        else { self.shed_hours_continuous += 1.0; }
    }
}

// ── MPC Solver ─────────────────────────────────────────
pub fn solve_mpc(
    lookahead: &[(f64, f64)],
    temp_start: f64, t_min: f64, t_max: f64,
    r_th: f64, c_th: f64, cooling_power_kw: f64,
) -> (f64, f64, f64, usize) {
    let horizon = lookahead.len();
    if horizon < 4 { return (0.0, 0.0, 0.0, 0); }
    let tau = r_th * c_th;
    if tau <= 0.0 { return (0.0, 0.0, 0.0, 0); }
    let mut best_start = 0usize;
    let mut best_dur = 1usize;
    let mut best_cost = f64::INFINITY;
    let mut worst_cost = 0.0;
    for start_delay in 0..horizon {
        let max_dur = (horizon - start_delay).min(8);
        for dur in 1..=max_dur {
            let mut total_cost = 0.0;
            let mut sim_temp = temp_start;
            let mut path_failed = false;
            for (h, &(price, ambient)) in lookahead.iter().enumerate().take(horizon) {
                let dt = 3600.0;
                let in_window = h >= start_delay && h < start_delay + dur;
                let compressor_on = in_window && sim_temp > t_min;
                if !compressor_on && sim_temp >= t_max { path_failed = true; break; }
                if compressor_on {
                    let cop = (0.6 * (sim_temp + 273.15) / ((ambient - sim_temp).abs().max(0.1))).clamp(0.5, 6.0);
                    let eff = ambient - cooling_power_kw * 1000.0 * cop * r_th;
                    sim_temp = eff + (sim_temp - eff) * (-dt / tau).exp();
                    total_cost += cooling_power_kw * price / 1000.0;
                } else {
                    sim_temp = ambient + (sim_temp - ambient) * (-dt / tau).exp();
                }
            }
            if !path_failed && sim_temp > t_min {
                let final_price = lookahead.last().unwrap().0.max(10.0);
                let hrs_left = ((sim_temp - t_min).max(0.0) / 2.0).max(1.0);
                total_cost += hrs_left * cooling_power_kw * final_price / 1000.0;
            }
            if path_failed { total_cost = f64::INFINITY; }
            if total_cost < best_cost {
                best_cost = total_cost;
                best_start = start_delay;
                best_dur = dur;
            }
            if total_cost > worst_cost && total_cost != f64::INFINITY { worst_cost = total_cost; }
        }
    }
    let confidence = if worst_cost > 0.0 && best_cost < worst_cost && best_cost != f64::INFINITY {
        (1.0 - best_cost / worst_cost).clamp(0.0, 1.0)
    } else { 0.0 };
    (best_start as f64, best_cost, confidence, best_dur)
}

// ── Helpers ────────────────────────────────────────────
pub fn compute_thermal_integral(temp_c: f64, t_target: f64, prev: f64, dt: f64, decay: f64) -> f64 {
    let excess = (temp_c - t_target).max(0.0);
    (prev + excess * dt) * decay.powf(dt)
}

pub fn compute_dynamic_thresholds(prices: &[f64], lo_pct: f64, hi_pct: f64) -> (f64, f64) {
    if prices.is_empty() { return (30.0, 80.0); }
    let mut s = prices.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let lo = (s.len() as f64 * lo_pct) as usize;
    let hi = (s.len() as f64 * hi_pct) as usize;
    (s[lo.min(s.len() - 1)], s[hi.min(s.len() - 1)])
}

#[allow(dead_code)]
pub fn retail_hour_params(t: &StoreType, h: usize) -> (f64, f64) {
    let hod = (h % 24) as i32;
    let is_sunday = ((h as i32 / 24 + 4) % 7) == 0;
    let is_closed = is_sunday || !(6..=21).contains(&hod);
    if is_closed && t.is_retail { (t.internal_heat_kw * 0.2, t.u_value * 0.5) }
    else { (t.internal_heat_kw, t.u_value) }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PriceRegime { Gorge, Buy, Neutral, Shed }

pub fn classify_price(price: f64, charge_threshold: f64, discharge_threshold: f64) -> PriceRegime {
    if price < 0.0 { PriceRegime::Gorge }
    else if price <= charge_threshold { PriceRegime::Buy }
    else if price >= discharge_threshold { PriceRegime::Shed }
    else { PriceRegime::Neutral }
}

fn compute_fcr_revenue(t: &StoreType, _h: usize, freq: f64, freq_nominal: f64) -> f64 {
    if !t.has_tes { return 0.0; }
    let deviation = (freq - freq_nominal).abs();
    if deviation < 0.01 { return 0.0; }
    let capacity_mw = t.cooling_power_kw / 1000.0;
    let fcr_capacity_price = 15.0;
    let response_fraction = if deviation > 0.03 { 0.8 } else { 0.3 };
    let event_factor = 0.5;
    capacity_mw * fcr_capacity_price * response_fraction * event_factor
}

fn thermal_runaway_check(t: &StoreType, zone: &ThermalZone, shed_hours: f64) -> bool {
    let q_accumulated_j = t.internal_heat_kw * 1000.0 * shed_hours * 3600.0;
    let temp_headroom = t.t_max - zone.temp_c;
    if temp_headroom <= 0.0 { return true; }
    let safe_buffer_j = zone.c_th * temp_headroom;
    q_accumulated_j >= safe_buffer_j * 0.80
}

pub fn is_peak_window(h: usize, bundesland: Bundesland) -> bool {
    bundesland.is_peak_window(h)
}

// ── MAIN DECISION — 7-mode priority cascade ────────────
#[allow(clippy::too_many_arguments)]
pub fn decide(
    t: &StoreType, zone: &ThermalZone, h: usize,
    price: f64, amb: f64, freq: f64,
    lookahead: &[(f64, f64)],
    asct: &mut AsctGovernor,
    thermal_integral: f64,
    price_lookahead_hours: &[f64],
    _flat_tariff: f64,
    _fc_price_eur_per_mw_per_h: f64,
    volatile: bool,  // NEW: remove all safety bounds when true
    bundesland: Bundesland,
) -> ControllerOutput {
    let freq_nominal = 50.0;
    let ttf = compute_ttf(zone, t.t_max);
    let state = compute_state_vector(zone, t);

    // Compute dynamic thresholds
    // Retail: use 30th/70th percentiles (wider neutral, less aggressive arbitrage)
    // Industrial: use 15th/85th (more aggressive)
    let (lo_pct, hi_pct) = if t.is_retail && !volatile { (0.30, 0.70) } else { (0.15, 0.85) };
    let (dyn_cheap, dyn_expensive) = compute_dynamic_thresholds(price_lookahead_hours, lo_pct, hi_pct);
    let (charge_threshold, discharge_threshold) = if t.has_tes {
        let n = price_lookahead_hours.len().min(12);
        let window = &price_lookahead_hours[..n];
        let avg: f64 = window.iter().sum::<f64>() / window.len() as f64;
        (avg * 0.80, avg * 1.20)
    } else { (dyn_cheap, dyn_expensive) };
    let regime = classify_price(price, charge_threshold, discharge_threshold);

    let continuous_shed_hours = asct.shed_hours_continuous;
    let fcr_revenue = compute_fcr_revenue(t, h, freq, freq_nominal);

    let is_hot_ambient = amb >= 28.0;
    let can_shed = t.max_shed_hours >= 1.0 && !is_hot_ambient;

    // Volatile mode: skip thermal runaway check and max_shed_hours cap
    // Let the system squeeze every last cent, even if it risks temperature excursion
    if !volatile
        && can_shed
        && (thermal_runaway_check(t, zone, continuous_shed_hours + 1.0)
            || continuous_shed_hours >= t.max_shed_hours)
    {
        return ControllerOutput {
            compressor_on: true, tes_charging: t.has_tes, vfd_speed: 1.0,
            mode: ControllerMode::ThermalGuard, setpoint_c: t.t_setpoint,
            ttf_sec: ttf, thermal_integral,
            droop_revenue_eur: 0.0, fc_earnings_eur: fcr_revenue,
            state_changes: asct.state_changes,
        };
    }

    // Priority 1: Emergency
    if ttf < 900.0 || state.temp_c >= t.t_max {
        return ControllerOutput {
            compressor_on: true, tes_charging: false, vfd_speed: 1.0,
            mode: ControllerMode::Emergency, setpoint_c: t.t_min,
            ttf_sec: ttf, thermal_integral,
            droop_revenue_eur: 0.0, fc_earnings_eur: fcr_revenue,
            state_changes: asct.state_changes,
        };
    }

    // Priority 2: Thermal Guard
    let integral_max = (t.t_max - t.t_setpoint).max(1.0) * 3.0;
    if thermal_integral > integral_max {
        return ControllerOutput {
            compressor_on: true, tes_charging: false, vfd_speed: 1.0,
            mode: ControllerMode::ThermalGuard, setpoint_c: t.t_setpoint,
            ttf_sec: ttf, thermal_integral,
            droop_revenue_eur: 0.0, fc_earnings_eur: fcr_revenue,
            state_changes: asct.state_changes,
        };
    }

    // Priority 3: Grid Balancing (TES only)
    if (freq - 50.0).abs() > 0.03 && t.has_tes && continuous_shed_hours < t.max_shed_hours * 0.5 {
        let wants_on = freq > 50.03;
        let gov = asct.apply(wants_on, h as i64, t.min_runtime_sec / 3600.0, t.min_offtime_sec / 3600.0);
        return ControllerOutput {
            compressor_on: gov, tes_charging: gov && t.has_tes, vfd_speed: 1.0,
            mode: ControllerMode::GridBalancing, setpoint_c: t.t_setpoint,
            ttf_sec: ttf, thermal_integral,
            droop_revenue_eur: 0.0, fc_earnings_eur: fcr_revenue,
            state_changes: asct.state_changes,
        };
    }

    // Priority 4: Dual-Market (TES only)
    if t.has_tes && regime == PriceRegime::Buy && fcr_revenue > 0.0 && continuous_shed_hours < 0.5 {
        let gov = asct.apply(true, h as i64, t.min_runtime_sec / 3600.0, t.min_offtime_sec / 3600.0);
        return ControllerOutput {
            compressor_on: gov, tes_charging: true, vfd_speed: 0.7,
            mode: ControllerMode::DualMarket, setpoint_c: t.t_setpoint,
            ttf_sec: ttf, thermal_integral,
            droop_revenue_eur: 0.0, fc_earnings_eur: fcr_revenue,
            state_changes: asct.state_changes,
        };
    }

    // Priority 5: MPC Pre-Cool
    let (mpc_start, _mpc_cost, mpc_conf, mpc_dur) = solve_mpc(
        lookahead, zone.temp_c, t.t_min, t.t_max,
        zone.r_th, zone.c_th, t.cooling_power_kw,
    );
    let start_h = mpc_start.round() as usize;
    let in_precool_window = start_h == 0 && mpc_dur > 0;
    if mpc_conf > 0.2
        && lookahead.len() >= 4
        && in_precool_window
        && zone.temp_c > t.t_min + 0.5
        && !lookahead.is_empty()
        && lookahead[0].0 <= charge_threshold
        && !is_peak_window(h, bundesland)
    {
        let gov = asct.apply(true, h as i64, t.min_runtime_sec / 3600.0, t.min_offtime_sec / 3600.0);
        return ControllerOutput {
            compressor_on: gov, tes_charging: t.has_tes, vfd_speed: 1.0,
            mode: ControllerMode::MpcPreCool, setpoint_c: t.t_min,
            ttf_sec: ttf, thermal_integral,
            droop_revenue_eur: 0.0, fc_earnings_eur: fcr_revenue,
            state_changes: asct.state_changes,
        };
    }

    // ── Priority 6: Energy-Neutral Arbitrage ─────────────
    let thermostat_wants_on = zone.temp_c > t.t_setpoint + 0.5;

    // For retail freezers: avoid deep shed to t_max — use "eco drift" instead.
    // Drift to setpoint+1°C instead of t_max. This avoids the punishing COP
    // recovery penalty that makes shed unprofitable for small freezers.
    let shed_setpoint = if t.is_retail { t.t_setpoint + 1.0 } else { t.t_max };

    // For retail: also use a much higher threshold for "gorge" — only at truly negative prices
    let gorge_price_threshold = if t.is_retail { -1.0 } else { 0.0 };

    match regime {
        PriceRegime::Gorge => {
            if can_shed {
                // Only pre-cool aggressively when truly being paid
                if price < gorge_price_threshold && !is_peak_window(h, bundesland) {
                    let wants = zone.temp_c > t.t_min + 0.5;
                    let gov = asct.apply(wants, h as i64, t.min_runtime_sec / 3600.0, t.min_offtime_sec / 3600.0);
                    ControllerOutput {
                        compressor_on: gov, tes_charging: gov && t.has_tes, vfd_speed: 1.0,
                        mode: ControllerMode::Arbitrage, setpoint_c: t.t_min,
                        ttf_sec: ttf, thermal_integral,
                        droop_revenue_eur: 0.0, fc_earnings_eur: fcr_revenue,
                        state_changes: asct.state_changes,
                    }
                } else {
                    // Mild gorge: normal thermostat (don't waste energy over-cooling)
                    let gov = asct.apply(thermostat_wants_on, h as i64, t.min_runtime_sec / 3600.0, t.min_offtime_sec / 3600.0);
                    ControllerOutput {
                        compressor_on: gov, tes_charging: t.has_tes, vfd_speed: 1.0,
                        mode: ControllerMode::IdleNormal, setpoint_c: t.t_setpoint,
                        ttf_sec: ttf, thermal_integral,
                        droop_revenue_eur: 0.0, fc_earnings_eur: fcr_revenue,
                        state_changes: asct.state_changes,
                    }
                }
            } else {
                let gov = asct.apply(thermostat_wants_on, h as i64, t.min_runtime_sec / 3600.0, t.min_offtime_sec / 3600.0);
                ControllerOutput {
                    compressor_on: gov, tes_charging: t.has_tes, vfd_speed: 1.0,
                    mode: ControllerMode::IdleNormal, setpoint_c: t.t_setpoint,
                    ttf_sec: ttf, thermal_integral,
                    droop_revenue_eur: 0.0, fc_earnings_eur: fcr_revenue,
                    state_changes: asct.state_changes,
                }
            }
        }
        PriceRegime::Buy | PriceRegime::Neutral => {
            // Normal thermostat: maintain setpoint
            let gov = asct.apply(thermostat_wants_on, h as i64, t.min_runtime_sec / 3600.0, t.min_offtime_sec / 3600.0);
            ControllerOutput {
                compressor_on: gov,
                tes_charging: t.has_tes && zone.charge_j < t.tes_capacity_kwh * 3_600_000.0 * 0.9,
                vfd_speed: 0.8,
                mode: ControllerMode::IdleNormal, setpoint_c: t.t_setpoint,
                ttf_sec: ttf, thermal_integral,
                droop_revenue_eur: 0.0, fc_earnings_eur: fcr_revenue,
                state_changes: asct.state_changes,
            }
        }
        PriceRegime::Shed => {
            if can_shed {
                // Smart shed: use eco-drift (not t_max) for retail to avoid recovery penalty.
                // The recovery from t_max (-18°C) back to setpoint (-22°C) costs more in
                // Carnot COP penalty than was saved by not running during the shed.
                let gov = asct.apply(false, h as i64, t.min_runtime_sec / 3600.0, t.min_offtime_sec / 3600.0);
                ControllerOutput {
                    compressor_on: gov, tes_charging: false, vfd_speed: 0.2,
                    mode: ControllerMode::Arbitrage, setpoint_c: shed_setpoint,
                    ttf_sec: ttf, thermal_integral,
                    droop_revenue_eur: 0.0, fc_earnings_eur: fcr_revenue,
                    state_changes: asct.state_changes,
                }
            } else {
                let gov = asct.apply(thermostat_wants_on, h as i64, t.min_runtime_sec / 3600.0, t.min_offtime_sec / 3600.0);
                ControllerOutput {
                    compressor_on: gov, tes_charging: false, vfd_speed: 0.5,
                    mode: ControllerMode::IdleNormal, setpoint_c: t.t_setpoint,
                    ttf_sec: ttf, thermal_integral,
                    droop_revenue_eur: 0.0, fc_earnings_eur: fcr_revenue,
                    state_changes: asct.state_changes,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::create_zone;
    use crate::store_types::find_type;

    #[test]
    fn asct_governor_locks_after_state_change() {
        let mut g = AsctGovernor::new();
        assert!(g.apply(true, 0, 15.0, 10.0));
        assert_eq!(g.state_changes, 1);
        // Locked until the min-run window expires: opposing command is ignored.
        assert!(g.apply(false, 5, 15.0, 10.0));
        assert_eq!(g.state_changes, 1);
        assert!(!g.apply(false, 16, 15.0, 10.0));
        assert_eq!(g.state_changes, 2);
    }

    #[test]
    fn mpc_flat_lookahead_stays_in_bounds() {
        // NOTE: unlike the gateway solver, the fleet solver's tail-cost
        // heuristic makes flat-price paths differ (earlier starts cool more →
        // smaller tail), so confidence is > 0 — the contract to test is that
        // outputs stay in valid ranges, not that confidence is zero.
        let (start, cost, conf, _dur) = solve_mpc(&[(50.0, 20.0); 12], -22.0, -25.0, -18.0, 0.001, 8e9, 200.0);
        assert!(cost.is_finite() && cost >= 0.0, "cost must be finite, got {cost}");
        assert!((0.0..=1.0).contains(&conf), "confidence must be in [0,1], got {conf}");
        assert!((0.0..12.0).contains(&start), "start must be within the horizon, got {start}");
    }

    #[test]
    fn mpc_prefers_early_cheap_window() {
        // Cheap first hour, expensive rest → optimal start is immediately.
        // Parameters chosen so the thermal time constant (~2 h) makes the cheap
        // hour actually charge the mass (expected: start=0, conf≈0.9).
        let lookahead = [vec![(10.0, 20.0)], std::iter::repeat_n((100.0, 20.0), 7).collect()].concat();
        let (start, _cost, conf, _dur) = solve_mpc(&lookahead, -20.0, -25.0, -18.0, 0.01, 7.2e5, 200.0);
        assert!(conf > 0.2, "cheap-first lookahead must have meaningful confidence, got {conf}");
        assert!(start < 2.0, "expected an early pre-cool start, got {start}");
    }

    #[test]
    fn mpc_short_horizon_returns_zero() {
        let (start, cost, conf, dur) = solve_mpc(&[(50.0, 20.0); 3], -22.0, -25.0, -18.0, 0.001, 8e9, 200.0);
        assert_eq!((start, cost, conf, dur), (0.0, 0.0, 0.0, 0));
    }

    #[test]
    fn price_regime_classification() {
        assert_eq!(classify_price(-5.0, 30.0, 80.0), PriceRegime::Gorge);
        assert_eq!(classify_price(30.0, 30.0, 80.0), PriceRegime::Buy);
        assert_eq!(classify_price(50.0, 30.0, 80.0), PriceRegime::Neutral);
        assert_eq!(classify_price(80.0, 30.0, 80.0), PriceRegime::Shed);
    }

    #[test]
    fn thermal_integral_accumulates_and_decays() {
        // Excess above setpoint accumulates...
        let i0 = compute_thermal_integral(-20.0, -22.0, 0.0, 1.0, 0.99);
        assert!(i0 > 0.0, "excess above setpoint must accumulate");
        // ...and no excess means only decay applies.
        let i1 = compute_thermal_integral(-22.0, -22.0, 100.0, 1.0, 0.99);
        assert!(i1 < 100.0 && i1 > 90.0, "decay-only window: got {i1}");
    }

    #[test]
    fn dynamic_thresholds_are_percentiles() {
        assert_eq!(compute_dynamic_thresholds(&[], 0.3, 0.7), (30.0, 80.0));
        let prices = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let (lo, hi) = compute_dynamic_thresholds(&prices, 0.2, 0.8);
        assert_eq!(lo, 20.0);
        assert_eq!(hi, 50.0);
    }

    #[test]
    fn retail_hours_reduce_internal_load_when_closed() {
        let aldi = find_type("aldi");
        let (q_open, u_open) = retail_hour_params(aldi, 12); // midday, open
        let (q_closed, u_closed) = retail_hour_params(aldi, 3); // 03:00, closed
        assert!((q_open - aldi.internal_heat_kw).abs() < 1e-9);
        assert!((u_open - aldi.u_value).abs() < 1e-9);
        assert!((q_closed - aldi.internal_heat_kw * 0.2).abs() < 1e-9);
        assert!((u_closed - aldi.u_value * 0.5).abs() < 1e-9);
        assert!(q_closed < q_open);
        // Sunday counts as closed regardless of hour.
        let (q_sunday, _) = retail_hour_params(aldi, 24 * 3 + 12); // h=84 → Sunday noon
        assert!((q_sunday - aldi.internal_heat_kw * 0.2).abs() < 1e-9, "Sunday must be closed");
    }

    #[test]
    fn emergency_when_temperature_at_limit() {
        let aldi = find_type("aldi");
        let mut z = create_zone(aldi, 25.0);
        z.temp_c = aldi.t_max;
        let mut asct = AsctGovernor::new();
        // Volatile mode skips the runaway guard so the true TTF emergency path fires.
        let out = decide(aldi, &z, 0, 50.0, 25.0, 50.0,
            &[(50.0, 20.0); 12], &mut asct, 0.0, &[50.0; 24], 63.0, 15.0, true, Bundesland::default());
        assert_eq!(out.mode, ControllerMode::Emergency);
        assert!(out.compressor_on);
        assert_eq!(out.vfd_speed, 1.0);
    }

    #[test]
    fn thermal_guard_on_exceeded_integral() {
        let aldi = find_type("aldi");
        let z = create_zone(aldi, 25.0);
        let mut asct = AsctGovernor::new();
        let integral_max = (aldi.t_max - aldi.t_setpoint).max(1.0) * 3.0;
        let out = decide(aldi, &z, 0, 50.0, 25.0, 50.0,
            &[(50.0, 20.0); 12], &mut asct, integral_max + 1.0, &[50.0; 24], 63.0, 15.0, false, Bundesland::default());
        assert_eq!(out.mode, ControllerMode::ThermalGuard);
        assert!(out.compressor_on);
    }

    #[test]
    fn grid_balancing_for_tes_sites_on_frequency_deviation() {
        let dc = find_type("datacenter");
        let z = create_zone(dc, 25.0);
        let mut asct = AsctGovernor::new();
        // 50.1 Hz overfrequency → load response wants the compressor ON.
        let out = decide(dc, &z, 0, 50.0, 25.0, 50.1,
            &[(50.0, 20.0); 12], &mut asct, 0.0, &[50.0; 24], 63.0, 15.0, false, Bundesland::default());
        assert_eq!(out.mode, ControllerMode::GridBalancing);
        assert!(out.compressor_on);
        assert!(out.tes_charging);
    }

    #[test]
    fn retail_shed_uses_eco_drift_setpoint() {
        let aldi = find_type("aldi");
        let z = create_zone(aldi, 25.0);
        let mut asct = AsctGovernor::new();
        let prices = vec![100.0, 110.0, 120.0, 130.0, 140.0, 150.0];
        // 200 €/MWh clears the 70th-percentile discharge threshold → shed.
        let out = decide(aldi, &z, 12, 200.0, 25.0, 50.0,
            &[(200.0, 20.0); 12], &mut asct, 0.0, &prices, 63.0, 15.0, false, Bundesland::default());
        assert_eq!(out.mode, ControllerMode::Arbitrage);
        assert!(!out.compressor_on);
        // Retail eco-drift: shed to setpoint + 1 °C, NOT t_max (avoids COP recovery penalty).
        assert!((out.setpoint_c - (aldi.t_setpoint + 1.0)).abs() < 1e-9);
        assert!((out.vfd_speed - 0.2).abs() < 1e-9);
    }

    #[test]
    fn neutral_price_yields_idle_normal() {
        let aldi = find_type("aldi");
        let z = create_zone(aldi, 25.0);
        let mut asct = AsctGovernor::new();
        let out = decide(aldi, &z, 12, 50.0, 25.0, 50.0,
            &[(50.0, 20.0); 12], &mut asct, 0.0, &[50.0; 24], 63.0, 15.0, false, Bundesland::default());
        assert_eq!(out.mode, ControllerMode::IdleNormal);
    }
}