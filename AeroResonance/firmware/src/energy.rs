//! Energy model: supercapacitor store, task-tier mapping, duty-cycle planning.
//!
//! Mirrors the simulation's PMIC/capacitor model (`eh_sim_core::physics`,
//! `simulation::subsystems::PlasmaWakeEnergyBuffering`) so predictions made on
//! the desktop transfer to the field node.

use alloc::vec::Vec;

use esim_core::scheduler::{TaskSchedulerConfig, TaskTier};

/// `f64::sqrt` is not available on bare-metal targets (no std); use libm.
#[inline]
fn sqrt(x: f64) -> f64 {
    libm::sqrt(x)
}

/// Tier thresholds — same shape as the simulator's `SimConfig` tier fields.
#[derive(Debug, Clone, Copy)]
pub struct TierConfig {
    pub v_cap_tier0_min: f64,
    pub v_cap_tier1_min: f64,
    pub v_cap_tier2_min: f64,
    pub v_cap_tier3_min: f64,
}

impl Default for TierConfig {
    fn default() -> Self {
        // Same defaults as `SimConfig` (simulation/src/config.rs).
        Self {
            v_cap_tier0_min: 2.5,
            v_cap_tier1_min: 2.8,
            v_cap_tier2_min: 3.1,
            v_cap_tier3_min: 3.25,
        }
    }
}

/// Energy in a capacitor: E = ½·C·V²  [J].
pub fn cap_energy_j(capacitance_f: f64, v_cap: f64) -> f64 {
    0.5 * capacitance_f * v_cap.max(0.0) * v_cap.max(0.0)
}

/// Voltage for a given stored energy: V = √(2E/C)  [V].
pub fn cap_voltage_for_energy_j(capacitance_f: f64, energy_j: f64) -> f64 {
    sqrt(2.0 * energy_j.max(0.0) / capacitance_f.max(1e-12))
}

/// Wire code for a tier (0..=4), matching the telemetry decoder in
/// `telemetry::decode_payload` (0 → T-1 … 4 → T3). `TaskTier`'s discriminants
/// start at -1, so a plain cast would wrap.
pub fn tier_code(tier: TaskTier) -> u8 {
    (tier as i32 + 1) as u8
}

/// Map a measured supercapacitor voltage to the device's task tier.
///
/// Uses the exact same `eh_sim_core::scheduler` logic as the simulator, so the
/// desktop run and the field node agree on when each task may run.
pub fn node_tier(v_cap: f64, tiers: &TierConfig) -> TaskTier {
    let cfg = TaskSchedulerConfig {
        tier0_energy_j: 0.0,
        tier1_energy_j: 0.0,
        tier2_energy_j: 0.0,
        tier3_energy_j: 0.0,
        v_cap_tier0_min: tiers.v_cap_tier0_min,
        v_cap_tier1_min: tiers.v_cap_tier1_min,
        v_cap_tier2_min: tiers.v_cap_tier2_min,
        v_cap_tier3_min: tiers.v_cap_tier3_min,
    };
    cfg.highest_tier(v_cap)
}

/// Average-power and burst parameters of the node.
#[derive(Debug, Clone, Copy)]
pub struct LoadProfile {
    /// Average current draw while awake, at 3.3 V [A] (ESP32-C3 radio on).
    pub active_current_a: f64,
    /// Average current draw while sleeping (light sleep), at 3.3 V [A].
    pub sleep_current_a: f64,
    /// Length of one awake burst (sample + transmit) [s].
    pub burst_time_s: f64,
}

impl Default for LoadProfile {
    fn default() -> Self {
        // Measured-ish figures for ESP32-C3 (see hardware/POWER_BUDGET.md):
        // ~80 mA active with WiFi TX bursts, ~0.8 mA light sleep.
        Self {
            active_current_a: 0.080,
            sleep_current_a: 0.0008,
            burst_time_s: 5.0,
        }
    }
}

impl LoadProfile {
    /// Energy spent in one awake burst [J] (at 3.3 V).
    pub fn burst_energy_j(&self) -> f64 {
        3.3 * self.active_current_a * self.burst_time_s
    }
    /// Continuous draw while asleep [W].
    pub fn sleep_power_w(&self) -> f64 {
        3.3 * self.sleep_current_a
    }
}

/// A suggested wake-up schedule computed from measured harvest and load.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DutyPlan {
    /// Suggested interval between wake-ups [s].
    pub interval_s: f64,
    /// Estimated average system power over the cycle [W].
    pub avg_power_w: f64,
    /// Energy available in the store right now [J].
    pub store_energy_j: f64,
    /// Estimated time the store can run with zero harvest [s].
    pub autonomy_s: f64,
}

/// Duty-cycle planner: given the current store state and the expected average
/// harvest, propose a conservative wake interval so the store never drains.
///
/// `harvest_avg_w` is the *measured* 24 h average from the node's own PV log
/// (start conservative, e.g. 0.02 W for a 2 W panel in poor conditions).
pub fn plan_duty(
    store_energy_j: f64,
    harvest_avg_w: f64,
    load: &LoadProfile,
    min_interval_s: f64,
    max_interval_s: f64,
) -> DutyPlan {
    let burst = load.burst_energy_j();
    let sleep_w = load.sleep_power_w();
    // Time to replenish exactly one burst's worth of energy at the current
    // harvest rate. If harvest is (near) zero, fall back to the max interval.
    let replenish_s = if harvest_avg_w > 1e-9 {
        burst / harvest_avg_w
    } else {
        f64::INFINITY
    };
    let interval_s = replenish_s.clamp(min_interval_s, max_interval_s);

    let cycle_s = interval_s;
    let avg_power_w = burst / cycle_s + sleep_w;
    let autonomy_s = if sleep_w + burst / cycle_s > 1e-12 {
        store_energy_j / avg_power_w
    } else {
        f64::INFINITY
    };

    DutyPlan {
        interval_s,
        avg_power_w,
        store_energy_j,
        autonomy_s,
    }
}

/// Rough daily harvest estimate for PV sizing sanity checks.
///
/// `peak_sun_hours` ≈ 3 (northern Europe, worst month) … 5 (central Europe);
/// `system_efficiency` accounts for charge controller + storage + regulator
/// losses (≈ 0.5 conservative, ≈ 0.7 optimistic).
pub fn estimate_daily_harvest_wh(panel_w: f64, peak_sun_hours: f64, system_efficiency: f64) -> f64 {
    panel_w * peak_sun_hours * system_efficiency
}

/// One measurement cycle of the node — the payload of a telemetry frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CycleReading {
    pub v_cap: f64,
    pub temp_c: f64,
    pub humidity_pct: f64,
    pub pressure_hpa: f64,
    pub pv_voltage_v: f64,
    pub elf_mv: f64,
    pub piezo_mv: f64,
    pub adc_entropy_bytes: u8,
    pub harvest_mw: f64,
}

/// A plan for one wake cycle: what tier, how long to sleep, what to report.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CyclePlan {
    pub tier: TaskTier,
    pub duty: DutyPlan,
    pub reading: CycleReading,
}

impl CyclePlan {
    /// Human-readable one-line status for local logging.
    pub fn to_log_line(&self) -> alloc::string::String {
        use alloc::format;
        format!(
            "tier={} v_cap={:.3}V autonomy={:.0}s interval={:.0}s E={:.1}J pv={:.3}mW elf={:.1}mV piezo={:.1}mV",
            self.tier.name(),
            self.reading.v_cap,
            self.duty.autonomy_s,
            self.duty.interval_s,
            self.duty.store_energy_j,
            self.reading.harvest_mw,
            self.reading.elf_mv,
            self.reading.piezo_mv,
        )
    }
}

/// Assemble the full per-cycle plan from live readings (the firmware's
/// `loop` body, made pure and testable).
pub fn build_cycle_plan(
    v_cap: f64,
    capacitance_f: f64,
    harvest_avg_w: f64,
    reading: CycleReading,
    tiers: &TierConfig,
    load: &LoadProfile,
) -> CyclePlan {
    let store_j = cap_energy_j(capacitance_f, v_cap);
    let duty = plan_duty(store_j, harvest_avg_w, load, 30.0, 3600.0);
    let tier = node_tier(v_cap, tiers);
    CyclePlan {
        tier,
        duty,
        reading,
    }
}

// ---------- helpers used by tests ----------
#[allow(dead_code)]
fn _tier_names() -> Vec<&'static str> {
    (0..4).map(|_| "tier").collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const C_F: f64 = 25.0; // 2 × 50 F series ≈ 25 F

    #[test]
    fn cap_energy_roundtrip() {
        let e = cap_energy_j(C_F, 5.0);
        let v = cap_voltage_for_energy_j(C_F, e);
        assert!((v - 5.0).abs() < 1e-9);
    }

    #[test]
    fn tier_drops_with_voltage() {
        let t = TierConfig::default();
        assert_eq!(node_tier(3.4, &t).name(), "T3-Low");
        assert_eq!(node_tier(3.0, &t).name(), "T1-High");
        assert_eq!(node_tier(2.4, &t).name(), "T-1-DeepHibernate");
    }

    #[test]
    fn duty_plan_never_drains_store() {
        // Poor harvest: 0.02 W average (2 W panel, very cloudy).
        let store_j = cap_energy_j(C_F, 4.0);
        let plan = plan_duty(store_j, 0.02, &LoadProfile::default(), 30.0, 3600.0);
        assert!(plan.interval_s >= 30.0 && plan.interval_s <= 3600.0);
        // Average draw must not exceed what harvest + store can sustain.
        assert!(plan.avg_power_w > 0.0);
        assert!(plan.autonomy_s > 0.0);
    }

    #[test]
    fn no_harvest_falls_back_to_max_interval() {
        let store_j = cap_energy_j(C_F, 4.0);
        let plan = plan_duty(store_j, 0.0, &LoadProfile::default(), 30.0, 3600.0);
        assert_eq!(plan.interval_s, 3600.0);
    }

    #[test]
    fn tier_code_mapping_matches_decoder() {
        let t = TierConfig::default();
        // TaskTier discriminants start at -1; the wire code must be 0..=4
        // (telemetry::decode_payload maps 0→T-1 … 4→T3).
        assert_eq!(tier_code(node_tier(3.4, &t)), 4); // T3
        assert_eq!(tier_code(node_tier(3.0, &t)), 2); // T1
        assert_eq!(tier_code(node_tier(2.4, &t)), 0); // T-1
    }

    #[test]
    fn daily_harvest_sanity() {
        // 2 W panel × 4 h × 0.5 efficiency ≈ 4 Wh/day — enough for the node
        // (≈ 0.065 Wh/day, see hardware/POWER_BUDGET.md).
        let wh = estimate_daily_harvest_wh(2.0, 4.0, 0.5);
        assert!((wh - 4.0).abs() < 1e-9);
        assert!(wh > 0.065);
    }

    #[test]
    fn cycle_plan_is_deterministic() {
        let t = TierConfig::default();
        let r = CycleReading {
            v_cap: 4.0,
            temp_c: 21.5,
            humidity_pct: 44.0,
            pressure_hpa: 1013.0,
            pv_voltage_v: 6.0,
            elf_mv: 0.12,
            piezo_mv: 1.8,
            adc_entropy_bytes: 8,
            harvest_mw: 25.0,
        };
        let a = build_cycle_plan(4.0, C_F, 0.025, r, &t, &LoadProfile::default());
        let b = build_cycle_plan(4.0, C_F, 0.025, r, &t, &LoadProfile::default());
        assert_eq!(a, b);
        assert!(a.duty.store_energy_j > 0.0);
    }
}
