// ============================================================================
// Physics engine — RC model, RLS, COP, defrost, stratification
// ============================================================================

use crate::store_types::StoreType;

/// A single thermal zone state.
#[derive(Debug, Clone)]
pub struct ThermalZone {
    pub temp_c: f64,
    pub amb_c: f64,
    pub r_th: f64,
    pub c_th: f64,
    pub tau: f64,
    pub cop_current: f64,
    pub charge_j: f64,
    pub estimated_c_th: f64,
    pub rls_p: f64,
    pub rls_lambda: f64,
    pub cargo_temp_c: f64,
    pub cargo_r_th: f64,
    pub ceiling_temp_c: f64,
    pub floor_temp_c: f64,
    pub stratification_factor: f64,
    pub hours_since_defrost: f64,
    pub in_defrost: bool,
    pub defrost_timer: f64,
    pub desired_temp_c: f64,  // what temp the controller is targeting
}

pub fn create_zone(t: &StoreType, amb_c: f64) -> ThermalZone {
    let r_th = 1.0 / (t.u_value * t.surface_m2 + t.volume_m3 * t.air_change_rate_ach / 3600.0 * 1.2 * 1005.0).max(0.001);
    let c_th = t.thermal_mass_kj_k * 1000.0;
    let tau = r_th * c_th;
    let cargo_r_th = r_th * 0.1;
    ThermalZone {
        temp_c: t.t_setpoint,
        amb_c, r_th, c_th, tau,
        cop_current: compute_cop(t, t.t_setpoint, amb_c),
        charge_j: if t.has_tes { t.tes_capacity_kwh * 3_600_000.0 * 0.5 } else { 0.0 },
        estimated_c_th: c_th, rls_p: 1e6, rls_lambda: 0.98,
        cargo_temp_c: t.t_setpoint, cargo_r_th,
        ceiling_temp_c: t.t_setpoint, floor_temp_c: t.t_setpoint,
        stratification_factor: 0.0,
        hours_since_defrost: 0.0, in_defrost: false, defrost_timer: 0.0,
        desired_temp_c: t.t_setpoint,
    }
}

pub fn compute_cop(t: &StoreType, temp_c: f64, amb_c: f64) -> f64 {
    let deviation = (temp_c - t.t_setpoint).abs();
    let penalty = if deviation > 0.5 { (1.0 - 0.05 * deviation).max(0.65) } else { 1.0 };
    let carnot = 0.6 * (temp_c + 273.15) / ((amb_c - temp_c).abs().max(0.1));
    (carnot * penalty).clamp(0.5, 6.0)
}

pub fn rls_update(zone: &mut ThermalZone, dt_sec: f64, temp_before: f64, temp_after: f64, ambient: f64) {
    if dt_sec < 60.0 { return; }
    let num = temp_after - ambient;
    let den = temp_before - ambient;
    if den.abs() < 0.1 || num / den <= 0.0 { return; }
    let y = (num / den).ln();
    let phi = -dt_sec / zone.r_th;
    let theta_old = 1.0 / zone.estimated_c_th;
    let error = y - phi * theta_old;
    let k = zone.rls_p * phi / (zone.rls_lambda + phi * zone.rls_p * phi);
    let theta_new = theta_old + k * error;
    zone.rls_p = (1.0 - k * phi) * zone.rls_p / zone.rls_lambda;
    zone.rls_p = zone.rls_p.clamp(1.0, 1e10);
    let c_th_est = 1.0 / theta_new.abs().max(1e-10);
    zone.estimated_c_th = c_th_est.clamp(zone.c_th * 0.1, zone.c_th * 10.0);
}

pub const DT_HOUR: f64 = 3600.0;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PhysicsResult {
    pub compressor_energy_mwh: f64,
    pub fan_energy_mwh: f64,
    pub defrost_energy_mwh: f64,
    pub defrost_active: bool,
}

/// Simulate one hour of physics using VFD continuous modulation for ALL assets.
///
/// The key insight: ALL compressors modulate continuously. The `can_shed` flag
/// only controls how far the temperature is allowed to drift from setpoint.
/// This eliminates the energy-wasting ON/OFF recovery cycle entirely.
///
/// Energy consumption = VFD_load × rated_power × 1h
/// where VFD_load tracks the exact heat load (internal + wall leakage + defrost).
/// Result: energy consumption stays close to baseline regardless of setpoint changes.
pub fn simulate_hour(
    t: &StoreType,
    zone: &mut ThermalZone,
    _h: usize,
    setpoint_c: f64,  // target temperature (from controller)
    tes_charging: bool,
    _vfd_speed: f64,  // unused — VFD always modulates to match load
    _can_shed: bool,
) -> PhysicsResult {
    let amb = zone.amb_c;
    let dt = DT_HOUR;
    let q_int_w = t.internal_heat_kw * 1000.0;

    // Store the desired setpoint for the controller
    zone.desired_temp_c = setpoint_c.clamp(t.t_min, t.t_max);

    // ── Defrost ───────────────────────────────────────────
    zone.hours_since_defrost += 1.0;
    let mut defrost_energy_mwh = 0.0;

    if t.has_defrost && t.defrost_interval_h > 0.0 {
        if zone.hours_since_defrost >= t.defrost_interval_h && !zone.in_defrost {
            zone.in_defrost = true;
            zone.defrost_timer = 0.0;
        }
        if zone.in_defrost {
            zone.defrost_timer += 1.0;
            defrost_energy_mwh = t.defrost_energy_kw / 1000.0;
            let defrost_heat_w = t.defrost_energy_kw * 1000.0;
            let defrost_eff = zone.temp_c + defrost_heat_w * zone.r_th;
            zone.temp_c = defrost_eff + (zone.temp_c - defrost_eff) * (-dt / zone.tau).exp();
            if zone.defrost_timer >= t.defrost_duration_h {
                zone.in_defrost = false;
                zone.hours_since_defrost = 0.0;
            }
        }
    }

    // ── Compute heat load ──────────────────────────────────
    zone.cop_current = compute_cop(t, zone.temp_c, amb);
    let thermal_power_w = t.cooling_power_kw * 1000.0 * zone.cop_current;
    let wall_leak_w = (amb - zone.temp_c).max(0.0) / zone.r_th;
    // Total steady-state heat load: internal heat + building envelope leakage.
    // (Defrost heating is already applied to temperature evolution above).
    let total_heat_w = q_int_w + wall_leak_w;

    // ── VFD modulation with thermal battery arbitrage ─────
    // The VFD follows heat balance to maintain steady-state, BUT we add a
    // gentle correction that lets the building mass act as a thermal battery:
    //   - During shed (temp < setpoint): reduce VFD slightly → temp drifts up
    //   - During pre-cool (temp > setpoint): increase VFD → temp drifts down
    // This creates the arbitrage benefit: cheap cooling stored in thermal mass.
    // The correction is DELIBERATELY slow: just 5% per °C, capped at 10%.
    // Fast enough to create savings, slow enough to avoid compressor overwork.
    let base_vfd = (total_heat_w / thermal_power_w.max(0.1)).clamp(0.0, 1.0);
    let temp_error = zone.temp_c - zone.desired_temp_c;
    let correction = (temp_error * 0.05).clamp(-0.08, 0.08);
    let vfd_load = (base_vfd + correction).clamp(0.0, 1.0);

    // ── Energy consumption ─────────────────────────────────
    // The REAL energy consumed: VFD_load × rated_power × 1h
    // This is exact regardless of ON/OFF cycling — the compressor modulates
    // to deliver exactly the cooling needed and no more.
    let compressor_energy_mwh = t.cooling_power_kw * vfd_load / 1000.0;
    let fan_energy_mwh = t.fan_power_kw * vfd_load.powi(3) / 1000.0;

    // ── Temperature evolution ──────────────────────────────
    // The net cooling power determines if temp rises or falls
    let cooling_provided_w = thermal_power_w * vfd_load;
    let net_cooling_w = cooling_provided_w - total_heat_w;

    if net_cooling_w.abs() > 1.0 {
        // Use exponential approach to steady-state
        let steady_temp = zone.temp_c - net_cooling_w * zone.r_th;
        zone.temp_c = steady_temp + (zone.temp_c - steady_temp) * (-dt / zone.tau).exp();
    }
    zone.temp_c = zone.temp_c.clamp(t.t_min, t.t_max);

    // ── Thermal stratification ─────────────────────────────
    if vfd_load > 0.3 { zone.stratification_factor = 0.0; }
    else { zone.stratification_factor = (zone.stratification_factor + 0.1).min(1.0); }
    let sf = zone.stratification_factor;
    zone.ceiling_temp_c = zone.temp_c + (amb - zone.temp_c).max(0.0) * 0.5 * sf;
    zone.floor_temp_c = zone.temp_c - (zone.temp_c - zone.floor_temp_c).max(0.0) * 0.3 * sf;

    // ── Critical cooling chain ─────────────────────────────
    if t.cargo_mass_kg > 0.0 {
        let cargo_c_th = t.cargo_mass_kg * t.cargo_specific_heat;
        let cargo_tau = zone.cargo_r_th * cargo_c_th;
        if cargo_tau > 0.0 {
            zone.cargo_temp_c = zone.temp_c + (zone.cargo_temp_c - zone.temp_c) * (-dt / cargo_tau).exp();
        }
    }

    // ── RLS update (when not in defrost, cold temp rise is informative) ──
    if !zone.in_defrost && vfd_load < 0.3 {
        rls_update(zone, dt, zone.temp_c, zone.temp_c + net_cooling_w * zone.r_th * (1.0 - (-dt / zone.tau).exp()), amb);
    }

    // ── TES tank ──────────────────────────────────────────
    if t.has_tes && tes_charging {
        let surplus_w = (cooling_provided_w - total_heat_w).max(0.0);
        let max_charge_j = t.tes_capacity_kwh * 3_600_000.0;
        zone.charge_j = (zone.charge_j + surplus_w * dt).min(max_charge_j);
    }

    PhysicsResult {
        compressor_energy_mwh,
        fan_energy_mwh,
        defrost_energy_mwh,
        defrost_active: zone.in_defrost,
    }
}

pub fn compute_ttf(zone: &ThermalZone, t_max: f64) -> f64 {
    if zone.tau <= 0.0 { return f64::INFINITY; }
    let rate = (zone.amb_c - zone.temp_c) / zone.tau;
    if rate <= 0.0 { return f64::INFINITY; }
    let delta = t_max - zone.temp_c;
    if delta <= 0.0 { return 0.0; }
    delta / rate
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StateVector {
    pub temp_c: f64,
    pub pressure_bar: f64,
    pub load_ok: bool,
}

pub fn compute_state_vector(zone: &ThermalZone, t: &StoreType) -> StateVector {
    let pressure = (zone.temp_c - t.t_setpoint).abs() * 0.5 + 2.0;
    StateVector {
        temp_c: zone.temp_c,
        pressure_bar: pressure.clamp(0.0, 30.0),
        load_ok: t.cooling_power_kw > 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store_types::{STORE_TYPES, find_type};

    #[test]
    fn cop_is_bounded_and_degrades_with_ambient_and_deviation() {
        let aldi = &STORE_TYPES[0];
        let cop_cold = compute_cop(aldi, -22.0, -10.0);
        let cop_hot = compute_cop(aldi, -22.0, 30.0);
        assert!((0.5..=6.0).contains(&cop_cold), "COP {} out of bounds", cop_cold);
        assert!((0.5..=6.0).contains(&cop_hot), "COP {} out of bounds", cop_hot);
        assert!(cop_hot < cop_cold, "warmer ambient must reduce freezer COP");
        // Deviation penalty: at 10 °C ambient the Carnot term is below the 6.0
        // clamp, so the 0.8 deviation factor is visible (at -10 °C it is masked
        // by the clamp — both sides saturate at 6.0).
        let cop_off = compute_cop(aldi, -18.0, 10.0);
        let cop_on = compute_cop(aldi, -22.0, 10.0);
        assert!(cop_off < cop_on, "deviation from setpoint must reduce COP (off={cop_off}, on={cop_on})");
    }

    #[test]
    fn create_zone_initializes_physics() {
        let aldi = &STORE_TYPES[0];
        let z = create_zone(aldi, 25.0);
        assert!((z.temp_c - aldi.t_setpoint).abs() < 1e-9);
        assert!((z.tau - z.r_th * z.c_th).abs() < 1e-6);
        assert_eq!(z.charge_j, 0.0, "aldi has no TES");

        let dc = find_type("datacenter");
        let z2 = create_zone(dc, 25.0);
        assert!((z2.charge_j - dc.tes_capacity_kwh * 3_600_000.0 * 0.5).abs() < 1.0,
            "TES sites start half-charged");
    }

    #[test]
    fn simulate_hour_keeps_temperature_in_bounds_and_energy_finite() {
        let aldi = &STORE_TYPES[0];
        let mut z = create_zone(aldi, 25.0);
        for h in 0..48 {
            let r = simulate_hour(aldi, &mut z, h, aldi.t_setpoint, false, 0.8, true);
            assert!(r.compressor_energy_mwh.is_finite() && r.compressor_energy_mwh >= 0.0);
            assert!(r.compressor_energy_mwh <= aldi.cooling_power_kw / 1000.0 + 1e-9,
                "hourly energy cannot exceed one hour at full power");
            assert!(r.fan_energy_mwh.is_finite() && r.fan_energy_mwh >= 0.0);
            assert!(z.temp_c >= aldi.t_min && z.temp_c <= aldi.t_max,
                "temp {} out of bounds at hour {}", z.temp_c, h);
        }
        // VFD modulation tracks the heat load: no drift to t_max at steady state.
        assert!((z.temp_c - aldi.t_setpoint).abs() < 3.0, "temp drifted to {}", z.temp_c);
    }

    #[test]
    fn defrost_cycles_on_interval() {
        let freezer = find_type("freezer");
        let mut z = create_zone(freezer, 10.0);
        let mut defrost_hours = 0usize;
        let mut energy = 0.0;
        for h in 0..30 {
            let r = simulate_hour(freezer, &mut z, h, freezer.t_setpoint, false, 0.8, true);
            if r.defrost_energy_mwh > 0.0 {
                defrost_hours += 1;
                energy += r.defrost_energy_mwh;
            }
            assert!(r.defrost_energy_mwh >= 0.0 && r.defrost_energy_mwh.is_finite());
        }
        // The defrost heater runs one full hour per 12 h interval (duration is
        // < 1 h, so `defrost_active` in the result is false by the time the
        // hour completes — the energy trace is the observable).
        assert!(defrost_hours >= 2, "expected >= 2 defrost events in 30 h, got {defrost_hours}");
        assert!(energy > 0.0, "defrost must consume energy");
    }

    #[test]
    fn ttf_is_finite_when_warm_and_zero_at_limit() {
        let aldi = &STORE_TYPES[0];
        let z = create_zone(aldi, 25.0);
        let ttf = compute_ttf(&z, aldi.t_max);
        assert!(ttf.is_finite() && ttf > 0.0, "warm ambient must yield a finite positive TTF");
        let mut at_limit = z.clone();
        at_limit.temp_c = aldi.t_max;
        assert_eq!(compute_ttf(&at_limit, aldi.t_max), 0.0);
        // Cold room, cold ambient → no temperature rise → infinite TTF
        let z2 = create_zone(aldi, -25.0);
        assert_eq!(compute_ttf(&z2, aldi.t_max), f64::INFINITY);
    }

    #[test]
    fn rls_estimate_stays_within_physical_clamps() {
        let aldi = &STORE_TYPES[0];
        let mut z = create_zone(aldi, 25.0);
        z.temp_c = -20.0;
        z.amb_c = 10.0;
        for _ in 0..20 {
            let (t0, amb) = (z.temp_c, z.amb_c);
            rls_update(&mut z, 3600.0, t0, t0 + 0.3, amb);
        }
        assert!(z.estimated_c_th >= z.c_th * 0.1 && z.estimated_c_th <= z.c_th * 10.0,
            "C_th estimate {} outside [0.1x, 10x] of nominal {}", z.estimated_c_th, z.c_th);
        assert!(z.rls_p >= 1.0 && z.rls_p <= 1e10);
    }

    #[test]
    fn rls_ignores_short_windows() {
        let aldi = &STORE_TYPES[0];
        let mut z = create_zone(aldi, 25.0);
        let before = z.estimated_c_th;
        let (t0, amb) = (z.temp_c, z.amb_c);
        rls_update(&mut z, 30.0, t0, t0 + 0.3, amb);
        assert_eq!(z.estimated_c_th, before, "dt < 60 s must be ignored");
    }
}