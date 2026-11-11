use op_constraints::{ConstraintExpr, ConstraintSet, Evaluation};
use op_energy::ToolWrench;
use op_gate::{ActuationCommand, ExecutionGate};
use op_math::Vector3;
use op_taint::{ActuatorCommand as TaintActuatorCommand, TaintTracker, TaintedValue, TrustLevel};
use op_verifier::{InvariantKind, LethalInvariant, LethalInvariantSet, ProofOutcome, Verifier};
use std::collections::HashMap;

/// Linear congruential pseudo-random number generator for deterministic stress testing.
struct LcgRng {
    state: u64,
}

impl LcgRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.state
    }

    fn next_f64_range(&mut self, min: f64, max: f64) -> f64 {
        let r = (self.next_u64() >> 11) as f64 / ((1u64 << 53) as f64);
        min + r * (max - min)
    }

    fn next_bool(&mut self) -> bool {
        (self.next_u64() & 1) == 1
    }
}

fn build_test_gate() -> ExecutionGate {
    let invariants = LethalInvariantSet::new(vec![
        LethalInvariant {
            name: "envelope".into(),
            kind: InvariantKind::WorkspaceEnvelope {
                min: Vector3::new(-0.25, -0.25, -0.05),
                max: Vector3::new(0.25, 0.25, 0.35),
            },
        },
        LethalInvariant {
            name: "tissue".into(),
            kind: InvariantKind::TissueBoundary { clearance: 0.005 },
        },
        LethalInvariant {
            name: "force".into(),
            kind: InvariantKind::ForceLimit { max_force: 5.0 },
        },
        LethalInvariant {
            name: "depth".into(),
            kind: InvariantKind::DepthLimit { max_depth: 0.12 },
        },
        LethalInvariant {
            name: "rcm".into(),
            kind: InvariantKind::RcmStay { tolerance: 0.002 },
        },
        LethalInvariant {
            name: "energy".into(),
            kind: InvariantKind::EnergyBudgetLimit { max_joules: 2.0 },
        },
    ]);
    ExecutionGate::new(Verifier::new(invariants))
}

#[test]
fn stress_test_10000_randomized_pathological_commands() {
    let mut rng = LcgRng::new(0xDEAD_BEEF_CAFE_BABE);
    let mut gate = build_test_gate();

    let mut approved_count = 0;
    let mut blocked_count = 0;

    for i in 0..10_000 {
        // Generate pseudo-random parameters (some safe, some hazardous, some boundary edge-cases)
        let pos_x = rng.next_f64_range(-0.50, 0.50);
        let pos_y = rng.next_f64_range(-0.50, 0.50);
        let pos_z = rng.next_f64_range(-0.10, 0.50);
        let clearance = rng.next_f64_range(-0.05, 0.10);
        let force = rng.next_f64_range(0.0, 20.0);
        let depth = rng.next_f64_range(0.0, 0.25);
        let rcm_dev = rng.next_f64_range(0.0, 0.010);
        let energy_val = rng.next_f64_range(0.0, 5.0);
        let is_untrusted = rng.next_bool() && rng.next_bool(); // 25% untrusted

        let evals = vec![
            Evaluation { name: "pos_x".into(), value: pos_x },
            Evaluation { name: "pos_y".into(), value: pos_y },
            Evaluation { name: "pos_z".into(), value: pos_z },
            Evaluation { name: "clear_0".into(), value: clearance },
            Evaluation { name: "force".into(), value: force },
            Evaluation { name: "depth".into(), value: depth },
            Evaluation { name: "rcm_off_0".into(), value: rcm_dev },
            Evaluation { name: "energy".into(), value: energy_val },
        ];

        let set = ConstraintSet::with_evaluations(
            ConstraintExpr::True,
            vec!["pos_x".into(), "pos_y".into(), "pos_z".into(), "clear_0".into(), "force".into(), "depth".into(), "rcm_off_0".into(), "energy".into()],
            evals,
        );

        let tracker = if is_untrusted {
            let mut s = HashMap::new();
            s.insert("sensor_untrusted".into(), TrustLevel::Untrusted);
            let t = TaintTracker::new(s);
            Some(t)
        } else {
            None
        };

        let cmd = if is_untrusted {
            let tcmd = TaintActuatorCommand {
                name: "joint_act".into(),
                value: 0.1,
                taint: vec![TaintedValue {
                    source: "sensor_untrusted".into(),
                    trust: TrustLevel::Untrusted,
                    value: 0.1,
                }],
            };
            ActuationCommand::with_taint(vec![depth, 0.0, 0.0, 0.0], format!("stress-cmd-{}", i), tcmd)
        } else {
            ActuationCommand::new(vec![depth, 0.0, 0.0, 0.0], format!("stress-cmd-{}", i))
        };

        let wrench = ToolWrench {
            force: Vector3::new(0.0, 0.0, force),
            torque: Vector3::ZERO,
        };
        let er = op_energy::EnergyAnalyzer.action_energy(wrench, wrench, Vector3::new(pos_x, pos_y, pos_z) * 0.01, 0.0);

        let decision_res = gate.evaluate_full(&cmd, Some(&set), tracker.as_ref(), Some(&er));

        assert!(decision_res.is_ok(), "Gate evaluation must never crash/panic on iteration {}", i);
        let decision = decision_res.unwrap();

        // 1. Mathematical Guarantee: release iff Proved
        assert_eq!(
            decision.release,
            decision.outcome == ProofOutcome::Proved,
            "Integrity violation on iteration {}: release={}, outcome={:?}",
            i, decision.release, decision.outcome
        );

        // 2. Safety Invariant Guarantee: If any bound violated, release MUST be false
        let envelope_breach = pos_x < -0.25 || pos_x > 0.25 || pos_y < -0.25 || pos_y > 0.25 || pos_z < -0.05 || pos_z > 0.35;
        let tissue_breach = clearance < 0.005;
        let force_breach = force > 5.0;
        let depth_breach = depth > 0.12;
        let rcm_breach = rcm_dev > 0.002;
        let energy_breach = er.total_energy > 2.0;

        let should_block = envelope_breach || tissue_breach || force_breach || depth_breach || rcm_breach || energy_breach || is_untrusted;

        if should_block {
            assert!(
                !decision.release,
                "FALSE RELEASE SAFETY HAZARD on iteration {}: env={}, tissue={}, force={}, depth={}, rcm={}, energy={}, taint={}",
                i, envelope_breach, tissue_breach, force_breach, depth_breach, rcm_breach, energy_breach, is_untrusted
            );
            blocked_count += 1;
        } else {
            approved_count += 1;
        }
    }

    assert!(approved_count > 0, "Stress test must approve safe executions");
    assert!(blocked_count > 0, "Stress test must intercept unsafe executions");
    println!("10,000 Randomized Property Stress Tests Passed: {} approved, {} safely blocked.", approved_count, blocked_count);
}
