//! End-to-end verification-chain integration tests.
//!
//! Exercises the full pipeline from the development plan: control-law
//! command → snapshot environment → execution gate with horizon prediction
//! → dynamics integration → hash-chained flight-recorder spec.

use plane_os::constraints::PathConstraints;
use plane_os::controller::PilotInput;
use plane_os::dynamics::{AircraftConfig, FlightState, TerrainModel, Vec3};
use plane_os::envelope::EnvelopeHazards;
use plane_os::formula::VarNS;
use plane_os::gate::{ExecutionGate, StandardCases};
use plane_os::simulator::PlaneSimulator;
use plane_os::solver::{free_variables, namespaced_env, CheckResult, SMTBackend, StubSolver};
use plane_os::state::{build_snapshot, ActuatorCommand};

/// A normal cruise flight must never be blocked by the gate.
#[test]
fn cruise_flight_is_approved_end_to_end() {
    let initial = FlightState::level(Vec3::new(0.0, 0.0, 3000.0), 150.0, 0.0);
    let mut sim = PlaneSimulator::new(
        initial,
        AircraftConfig::default(),
        TerrainModel::sea_level(),
    );
    for _ in 0..20 {
        let report = sim.step(PilotInput::default());
        assert!(
            report.approved,
            "cruise cycle blocked: {:?}",
            report.blocked_reason
        );
    }
    assert_eq!(sim.cycle, 20);
    assert!(sim.spec_valid());
    assert_eq!(sim.spec.records.len(), 20);
}

/// An engine-limit violation must block the command.
#[test]
fn engine_overheat_blocks_command() {
    let initial = FlightState::level(Vec3::new(0.0, 0.0, 3000.0), 150.0, 0.0);
    let mut sim = PlaneSimulator::new(
        initial,
        AircraftConfig::default(),
        TerrainModel::sea_level(),
    );
    // Over-temperature left EGT.
    sim.egt_debug_override(0, 960.0);
    let report = sim.step(PilotInput::default());
    assert!(report.blocked());
    assert!(report.blocked_reason == Some("thrust_hazard"));
}

/// Terrain conflict must block the command.
#[test]
fn terrain_conflict_blocks_command() {
    let initial = FlightState::level(Vec3::new(0.0, 0.0, 50.0), 150.0, 0.0);
    let mut sim = PlaneSimulator::new(
        initial,
        AircraftConfig::default(),
        TerrainModel::sea_level(),
    );
    let report = sim.step(PilotInput::default());
    assert!(report.blocked());
    assert_eq!(report.blocked_reason, Some("envelope_escape"));
}

/// The horizon expansion proves safety across the predicted path.
#[test]
fn horizon_expansion_blocks_future_overspeed() {
    let pc = PathConstraints { horizon: 3 };
    let base = EnvelopeHazards::envelope_escape();
    let expanded = pc.expand_over_horizon(&base);

    let mut env = namespaced_env(
        VarNS::State,
        &[
            ("ias_knots", 250.0),
            ("alpha_deg", 4.0),
            ("nz", 1.0),
            ("mach", 0.6),
            ("altitude", 3000.0),
            ("terrain_clearance", 3000.0),
        ],
    );
    for (k, v) in [
        ("mag.envelope.vne_knots", 340.0),
        ("mag.envelope.vs_clean_knots", 120.0),
        ("mag.envelope.alpha_max_deg", 14.0),
        ("mag.envelope.nz_max", 2.5),
        ("mag.envelope.nz_min", -1.0),
        ("mag.envelope.mmo", 0.82),
        ("mag.envelope.max_altitude", 13000.0),
        ("mag.envelope.min_terrain_clearance", 150.0),
    ] {
        env.insert(k.to_string(), v);
    }

    let base_state = env.clone();
    for step in 1..=3 {
        env.extend(PathConstraints::horizon_env(&base_state, step));
    }

    // All safe -> unsat.
    assert_eq!(
        StubSolver::new().check(&expanded, &env),
        CheckResult::Unsat
    );

    // Future overspeed -> sat.
    env.insert("his.ias_knots_2".to_string(), 350.0);
    assert_eq!(
        StubSolver::new().check(&expanded, &env),
        CheckResult::Sat
    );
}

/// Taint audit blocks a command on a failed channel.
#[test]
fn taint_failure_blocks_command() {
    let initial = FlightState::level(Vec3::new(0.0, 0.0, 3000.0), 150.0, 0.0);
    let mut sim = PlaneSimulator::new(
        initial,
        AircraftConfig::default(),
        TerrainModel::sea_level(),
    );
    // Fail the elevator channel (simulate IRU loss).
    sim.sensors_debug_fail_elevator();
    let report = sim.step(PilotInput::new(0.5, 0.0, 0.0, 0.3));
    assert!(
        report.blocked(),
        "failed elevator + pitch command must be blocked by the gate"
    );
}

/// Free-variable extraction covers the canonical hazard set.
#[test]
fn hazard_formulas_have_complete_variable_set() {
    for case in StandardCases::all() {
        let vars = free_variables(&case.formula);
        // Every hazard must reference at least one state var.
        assert!(
            vars.iter().any(|v| v.starts_with("state.")),
            "{} has no state vars: {:?}",
            case.name,
            vars
        );
        // Every hazard must have descriptions to make recorder specs
        // human-auditable.
        assert!(!case.description.is_empty());
    }
}

/// Snapshot environment round-trips through the gate with zero `Unknown`.
#[test]
fn snapshot_env_decides_all_standard_cases() {
    let initial = FlightState::level(Vec3::new(0.0, 0.0, 3000.0), 150.0, 0.0);
    let snap = build_snapshot(
        &initial,
        ActuatorCommand::new(0.3, 0.1, 0.0, 0.0),
        &AircraftConfig::default(),
        &TerrainModel::sea_level(),
    );
    let env = snap.env();
    let gate = ExecutionGate::stub(StandardCases::all());
    for case in &gate.cases {
        let result = StubSolver::new().check(&case.formula, &env);
        assert_eq!(
            result,
            CheckResult::Unsat,
            "{} should be decidable UNSAT on a healthy snapshot (got {})",
            case.name,
            result
        );
    }
}