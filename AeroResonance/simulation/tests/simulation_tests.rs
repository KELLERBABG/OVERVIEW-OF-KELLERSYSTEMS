//! Integration tests for the Atmospheric Resonance simulator.

use atmospheric_resonance_sim::config::SimConfig;
use atmospheric_resonance_sim::simulation::{run_simulation, EnergyRecord};
use std::path::Path;

/// Quick configuration for tests so runs complete fast (avoids the default
/// 3600 s @ 0.001 s = 3.6M-step run).
fn short_cfg() -> SimConfig {
    // 0.05 s step over 50 s = 1000 steps — fast for tests.
    SimConfig {
        dt: 0.05,
        total_time: 50.0,
        ..SimConfig::default()
    }
}

/// The default configuration should run and report all 11 subsystems.
#[test]
fn default_run_reports_all_subsystems() {
    let cfg = short_cfg();
    let (records, summaries) = run_simulation(&cfg, Some(42));
    assert!(!records.is_empty());
    // All 11 documented harvesters present in the summary.
    assert_eq!(summaries.len(), 11, "expected 11 subsystems");
    let names: Vec<&str> = summaries.iter().map(|s| s.sub_name.as_str()).collect();
    for expected in [
        "IonosphericPlasmaCoupling",
        "NearVerticalSkywaveInductor",
        "MaxwellianFieldRelaxation",
        "AcousticAtmosphericTransduction",
        "NonLinearWaveModulation",
        "CyclostationaryFeatureFiltering",
        "PlasmaWakeEnergyBuffering",
        "ResonantSubstrateAdaptation",
        "ThermodynamicCapacityGradient",
        "StaticInvariantDischarge",
        "SolarPhotovoltaic",
    ] {
        assert!(names.contains(&expected), "missing subsystem: {expected}");
    }
    // Column sanity: energy and power are finite.
    for s in &summaries {
        assert!(s.total_energy_j.is_finite());
        assert!(s.avg_power_w.is_finite());
    }
}

/// Every bundled scenario JSON must deserialize and run without panicking.
#[test]
fn all_scenarios_run() {
    for f in [
        "50x50mm_config.json",
        "severe_weather.json",
        "severe_weather_mitigated.json",
    ] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(f);
        let contents = std::fs::read_to_string(&path).expect("read scenario");
        let cfg: SimConfig = serde_json::from_str(&contents)
            .unwrap_or_else(|e| panic!("{f} failed to deserialize: {e}"));
        // Keep the run short for the test.
        let mut short = cfg.clone();
        short.total_time = 50.0;
        short.dt = 0.01;
        let (_records, summaries) = run_simulation(&short, Some(1));
        assert_eq!(summaries.len(), 11, "{f} should produce 11 summary rows");
    }
}

/// A malformed-missing-field config must fall back to defaults (serde default),
/// not panic during deserialization.
#[test]
fn missing_fields_default_to_defaults() {
    let minimal = r#"{"dt":0.05,"total_time":10,"output_interval":1}"#;
    let cfg: SimConfig = serde_json::from_str(minimal).expect("minimal config should load");
    assert_eq!(cfg.tower_height_m, 30.0); // from Default
    let (_records, summaries) = run_simulation(&cfg, Some(2));
    assert_eq!(summaries.len(), 11);
}

/// Garbage / type-mismatched JSON is rejected at parse time, never a panic.
#[test]
fn malformed_json_is_rejected_not_panicked() {
    for bad in [
        r#"{ this is not json"#,
        r#"{"dt":"not-a-number"}"#,
        r#"{"total_time":}"#,
    ] {
        assert!(
            serde_json::from_str::<SimConfig>(bad).is_err(),
            "expected {bad:?} to fail cleanly"
        );
    }
}

/// Same seed ⇒ identical results (bit-deterministic).
#[test]
fn same_seed_is_deterministic() {
    let cfg = short_cfg();
    let (_, a) = run_simulation(&cfg, Some(99));
    let (_, b) = run_simulation(&cfg, Some(99));
    assert_eq!(format!("{:?}", a), format!("{:?}", b));
}

/// Different seed ⇒ (almost certainly) different results.
#[test]
fn different_seed_differs() {
    let cfg = short_cfg();
    let (_, a) = run_simulation(&cfg, Some(1));
    let (_, b) = run_simulation(&cfg, Some(2));
    let fa: f64 = a.iter().map(|s: &EnergyRecord| s.total_energy_j).sum();
    let fb: f64 = b.iter().map(|s: &EnergyRecord| s.total_energy_j).sum();
    assert!(
        (fa - fb).abs() > 1e-12,
        "seeds 1 vs 2 should not give identical totals"
    );
}
