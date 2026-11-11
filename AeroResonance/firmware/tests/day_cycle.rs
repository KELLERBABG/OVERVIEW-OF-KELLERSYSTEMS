//! Simulated-day test: run the firmware's cycle-planning loop for 24 h of
//! simulated time against a day/night harvest profile and check the node
//! (a) never drains below the PMIC cutoff, (b) reaches a high tier by day,
//! and (c) reports continuously. No hardware required — the loop body is the
//! same `build_cycle_plan` the ESP32-C3 calls.

use ehar_firmware_core::{
    build_cycle_plan, cap_energy_j, cap_voltage_for_energy_j, count_transitions, ColdStartHysteresis,
    ColdStartState, CycleReading, HysteresisConfig, LoadProfile, TierConfig,
};

const C_F: f64 = 25.0; // 2 × 50 F series
const DAY_S: f64 = 86_400.0;

/// Day/night harvest model [W]: 0.08 W in daylight (06–18 h), 0.01 W at night.
/// ≈ 1.08 Wh/day — comfortably above the v2 load (≈ 0.31 Wh/day).
fn harvest_at(t: f64) -> f64 {
    let hour = (t / 3600.0) % 24.0;
    if (6.0..18.0).contains(&hour) {
        0.08
    } else {
        0.010
    }
}

fn reading(v_cap: f64) -> CycleReading {
    CycleReading {
        v_cap,
        temp_c: 20.0,
        humidity_pct: 45.0,
        pressure_hpa: 1013.0,
        pv_voltage_v: 5.5,
        elf_mv: 0.1,
        piezo_mv: 0.5,
        adc_entropy_bytes: 8,
        harvest_mw: harvest_at(0.0) * 1000.0,
    }
}

#[test]
fn node_survives_24h_with_daynight_harvest() {
    let tiers = TierConfig::default();
    let load = LoadProfile::default(); // v2 target profile (light-sleep)
    let mut v_cap = 5.0;
    let mut t = 0.0;
    let mut min_v = f64::MAX;
    let mut max_tier = i32::MIN;
    let mut cycles = 0u64;
    let mut drained = false;

    while t < DAY_S {
        let plan = build_cycle_plan(v_cap, C_F, harvest_at(t), reading(v_cap), &tiers, &load);
        let interval = plan.duty.interval_s.max(1.0);
        let e_before = cap_energy_j(C_F, v_cap);
        let harvest_j = harvest_at(t) * interval;
        let spend_j = plan.duty.avg_power_w * interval;
        let e_after = (e_before - spend_j + harvest_j).max(0.0);
        v_cap = cap_voltage_for_energy_j(C_F, e_after);
        if v_cap <= 0.0 {
            drained = true;
            break;
        }
        min_v = min_v.min(v_cap);
        max_tier = max_tier.max(plan.tier as i32);
        cycles += 1;
        t += interval;
    }

    assert!(!drained, "node drained below 0 V during the simulated day");
    assert!(min_v >= 3.0, "node fell below PMIC cutoff (3.0 V): min {min_v:.3} V");
    assert!(max_tier >= 3, "node never reached tier 3 (T3-Low) by day: max {max_tier}");
    assert!(cycles >= 24, "expected ≥ 24 report cycles, got {cycles}");
    assert!(v_cap >= 3.0, "node ended the day drained: {v_cap:.3} V");
}

#[test]
fn no_harvest_throttles_and_degrades_to_lowest_tier() {
    // Zero sun (dead panel): with the v2 load profile the planner throttles
    // to the max interval (hourly), so a full 25 F store lasts ~22 h before
    // the tier collapses. Assert the throttle *and* the eventual T-1 within
    // a 30 h window — the honest behavior, matching the power budget.
    let tiers = TierConfig::default();
    let load = LoadProfile::default();
    let mut v_cap = 5.0;
    let mut t = 0.0;
    let mut saw_hibernate = false;
    let mut max_interval_seen: f64 = 0.0;
    let mut cycles = 0u64;

    while t < 30.0 * 3600.0 && v_cap > 0.0 {
        let plan = build_cycle_plan(v_cap, C_F, 0.0, reading(v_cap), &tiers, &load);
        let interval = plan.duty.interval_s;
        max_interval_seen = max_interval_seen.max(interval);
        let e_after = (cap_energy_j(C_F, v_cap) - plan.duty.avg_power_w * interval).max(0.0);
        v_cap = cap_voltage_for_energy_j(C_F, e_after);
        if plan.tier as i32 == -1 {
            saw_hibernate = true;
        }
        cycles += 1;
        t += interval;
    }

    assert_eq!(max_interval_seen, 3600.0, "planner must throttle to max interval with no harvest");
    assert!(saw_hibernate, "no-harvest run never reached T-1 deep-hibernate");
    assert!(cycles > 0);
}

// ─────────────────────────────────────────────────────────────────────────────
// Cold-start hysteresis: zero oscillation chattering during dawn/dusk ramps
// ─────────────────────────────────────────────────────────────────────────────

/// Simulate a dawn ramp: V_cap rises slowly from 1.0 V to 4.0 V over 6 hours
/// with small sinusoidal noise (±0.02 V) mimicking low-irradiance PV ripple.
/// The hysteresis machine must transition monotonically upward with exactly
/// one transition per boundary — zero chattering.
#[test]
fn dawn_ramp_zero_chatter() {
    let cfg = HysteresisConfig::default();
    let mut machine = ColdStartHysteresis::new(cfg);

    // 6-hour ramp: 1.0 V → 4.0 V, sampled every 30 s (720 samples).
    let n_samples = 720;
    let v_start = 1.0_f64;
    let v_end = 4.0_f64;
    let noise_amp = 0.02_f64; // ±20 mV ripple

    let mut transitions = 0usize;
    let mut prev_state = machine.state;

    for i in 0..n_samples {
        let frac = (i as f64) / (n_samples - 1) as f64;
        let base = v_start + (v_end - v_start) * frac;
        // Sinusoidal ripple at ~0.5 Hz equivalent (one period per 12 samples).
        let noise = noise_amp * ((i as f64 * std::f64::consts::PI / 6.0).sin());
        let v = base + noise;

        machine.step(v);
        if machine.state != prev_state {
            // Verify monotonic upward: rank must increase by exactly 1.
            assert!(
                machine.state.rank() == prev_state.rank() + 1,
                "non-adjacent transition at sample {i}: {:?} → {:?}",
                prev_state,
                machine.state
            );
            transitions += 1;
            prev_state = machine.state;
        }
    }

    // From DeadStorage(0) to ActiveTelemetry(3): exactly 3 upward transitions.
    assert_eq!(
        transitions, 3,
        "expected exactly 3 transitions (Dead→Cold→Survival→Active), got {transitions}"
    );
    assert_eq!(
        machine.state,
        ColdStartState::ActiveTelemetry,
        "dawn ramp must end in ActiveTelemetry"
    );
}

/// Simulate a dusk ramp: V_cap falls slowly from 4.0 V to 1.0 V over 6 hours
/// with small sinusoidal noise (±0.02 V). The hysteresis machine must
/// transition monotonically downward with exactly one transition per
/// boundary — zero chattering.
#[test]
fn dusk_ramp_zero_chatter() {
    let cfg = HysteresisConfig::default();
    let mut machine = ColdStartHysteresis::with_state(
        ColdStartState::ActiveTelemetry,
        cfg,
    );

    let n_samples = 720;
    let v_start = 4.0_f64;
    let v_end = 1.0_f64;
    let noise_amp = 0.02_f64;

    let mut transitions = 0usize;
    let mut prev_state = machine.state;

    for i in 0..n_samples {
        let frac = (i as f64) / (n_samples - 1) as f64;
        let base = v_start + (v_end - v_start) * frac;
        let noise = noise_amp * ((i as f64 * std::f64::consts::PI / 6.0).sin());
        let v = base + noise;

        machine.step(v);
        if machine.state != prev_state {
            // Verify monotonic downward: rank must decrease by exactly 1.
            assert!(
                machine.state.rank() == prev_state.rank() - 1,
                "non-adjacent transition at sample {i}: {:?} → {:?}",
                prev_state,
                machine.state
            );
            transitions += 1;
            prev_state = machine.state;
        }
    }

    // From ActiveTelemetry(3) to DeadStorage(0): exactly 3 downward transitions.
    assert_eq!(
        transitions, 3,
        "expected exactly 3 transitions (Active→Survival→Cold→Dead), got {transitions}"
    );
    assert_eq!(
        machine.state,
        ColdStartState::DeadStorage,
        "dusk ramp must end in DeadStorage"
    );
}

/// Full dawn-then-dusk cycle: V_cap rises then falls back. The hysteresis
/// machine must NOT chatter at the boundaries — total transitions over the
/// full cycle should be exactly 6 (3 up + 3 down), not more.
#[test]
fn full_dawn_dusk_cycle_no_chatter() {
    let cfg = HysteresisConfig::default();
    let mut machine = ColdStartHysteresis::new(cfg);

    // Triangle wave: 1.0 → 4.0 → 1.0 over 12 hours, 30 s samples (1440 total).
    let n_half = 720;
    let noise_amp = 0.02_f64;
    let mut transitions = 0usize;
    let mut prev_state = machine.state;

    for i in 0..(2 * n_half) {
        let v: f64 = if i < n_half {
            // Rising: 1.0 → 4.0
            1.0 + 3.0 * (i as f64 / (n_half - 1) as f64)
        } else {
            // Falling: 4.0 → 1.0
            4.0 - 3.0 * ((i - n_half) as f64 / (n_half - 1) as f64)
        };
        let noise = noise_amp * ((i as f64 * std::f64::consts::PI / 6.0).sin());
        machine.step(v + noise);

        if machine.state != prev_state {
            transitions += 1;
            prev_state = machine.state;
        }
    }

    // Exactly 3 up + 3 down = 6 total. No chattering.
    assert_eq!(
        transitions, 6,
        "full dawn/dusk cycle must have exactly 6 transitions (no chatter), got {transitions}"
    );
}

/// Verify that the hysteresis bands actually prevent chattering: if V_cap
/// oscillates within the hysteresis gap (e.g., 2.50–2.85 V while in
/// SurvivalSenseOnly), the state must NOT change.
#[test]
fn hysteresis_band_prevents_boundary_oscillation() {
    let cfg = HysteresisConfig::default();

    // Case 1: In SurvivalSenseOnly, oscillate between 2.50 and 2.84 V.
    // Must stay in SurvivalSenseOnly (downward threshold is 2.10, upward is 2.85).
    let mut m = ColdStartHysteresis::with_state(ColdStartState::SurvivalSenseOnly, cfg);
    for _ in 0..100 {
        m.step(2.67); // mid-band
        m.step(2.51); // near lower edge but above 2.10
        m.step(2.84); // near upper edge but below 2.85
    }
    assert_eq!(
        m.state,
        ColdStartState::SurvivalSenseOnly,
        "state must not change while oscillating within hysteresis band"
    );

    // Case 2: In ActiveTelemetry, oscillate between 2.50 and 3.29 V.
    // Must stay in ActiveTelemetry (downward threshold is 2.50).
    let mut m2 = ColdStartHysteresis::with_state(ColdStartState::ActiveTelemetry, cfg);
    for _ in 0..100 {
        m2.step(2.90);
        m2.step(2.51); // just above 2.50 downward threshold
        m2.step(3.29); // just below 3.30 upward (irrelevant, already active)
    }
    assert_eq!(
        m2.state,
        ColdStartState::ActiveTelemetry,
        "state must not change while oscillating within ActiveTelemetry hysteresis band"
    );
}

/// Use `run_hysteresis_trace` + `count_transitions` to verify a noisy dawn
/// ramp produces zero extra transitions.
#[test]
fn trace_api_dawn_ramp_zero_extra_transitions() {
    let cfg = HysteresisConfig::default();
    let n = 500;
    let mut voltages = alloc::vec::Vec::with_capacity(n);

    for i in 0..n {
        let frac = i as f64 / (n - 1) as f64;
        let base = 1.0 + 3.0 * frac; // 1.0 → 4.0
        let noise = 0.02 * ((i as f64 * std::f64::consts::PI / 5.0).sin());
        voltages.push(base + noise);
    }

    let trace = ehar_firmware_core::run_hysteresis_trace(&cfg, &voltages);
    let transitions = count_transitions(&trace);

    // From DeadStorage to ActiveTelemetry: exactly 3 transitions.
    assert_eq!(transitions, 3, "trace API must show exactly 3 transitions for clean dawn ramp");
    assert_eq!(*trace.last().unwrap(), ColdStartState::ActiveTelemetry);
}
