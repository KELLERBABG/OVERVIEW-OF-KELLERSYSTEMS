//! # op-sim — 3D simulator + CLI
//!
//! Top-level application for the OP-Roboter stack. Provides a mock surgical
//! arm (RCM-style parallelogram kinematics), a 3D workspace with tissue
//! boundaries, path planning, constraint generation, verification, and the
//! trustless execution gate — all wired together for the simulator CLI.
//!
//! > **Research tooling only — not a deployable medical device.**

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use op_constraints::PathConstraintGenerator;
use op_energy::{EnergyAnalyzer, ToolWrench};
use op_gate::{ActuationCommand, ExecutionGate, GateDecision};
use op_kinematics::{
    Chain, DhLink, ForwardKinematics, InverseKinematics, Jacobian, JointState, ToolPose,
};
use op_math::{deg_to_rad, Scalar, Vector3};
use op_taint::{ActuatorCommand as TaintActuatorCommand, TaintTracker, TaintedValue, TrustLevel};
use op_trajectory::{MotionPath, RcmConstraint, TissueBoundary, Volume, Waypoint, Workspace};
use op_verifier::{InvariantKind, LethalInvariant, LethalInvariantSet, ProofOutcome, Verifier};
use std::collections::HashMap;

/// A configured simulator scenario.
#[derive(Debug, Clone)]
pub struct Simulator {
    /// Surgical arm chain (RCM-style).
    pub arm: Chain,
    /// Workspace with tissue boundaries.
    pub workspace: Workspace,
    /// RCM (trocar) constraint.
    pub rcm: RcmConstraint,
    /// Current joint state.
    pub state: JointState,
    /// Force limit (N).
    pub force_limit: Scalar,
    /// Depth limit (m).
    pub depth_limit: Scalar,
}

/// Outcome of running a named surgical scenario.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ScenarioResult {
    /// Scenario name.
    pub name: String,
    /// Description of the clinical scenario.
    pub description: String,
    /// Whether the gate released the physical command.
    pub released: bool,
    /// Formal verification outcome.
    pub outcome: ProofOutcome,
    /// Gate decision explanation.
    pub message: String,
    /// Invariant violations detected (if blocked).
    pub violations: Vec<String>,
}

/// Real-time control loop execution latency profile.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BenchmarkProfile {
    /// Number of full verification cycles measured.
    pub iterations: usize,
    /// Mean total latency per cycle in microseconds (µs).
    pub mean_us: f64,
    /// 50th percentile (median) latency in microseconds (µs).
    pub p50_us: f64,
    /// 95th percentile latency in microseconds (µs).
    pub p95_us: f64,
    /// 99th percentile latency in microseconds (µs).
    pub p99_us: f64,
    /// Maximum latency (worst-case jitter) in microseconds (µs).
    pub max_us: f64,
    /// Whether all measured cycles completed within the safety timing budget (< 5000 µs).
    pub hard_realtime_compliant: bool,
}

impl Simulator {
    /// Build the default laparoscopic tool-insertion scenario.
    pub fn default_scenario() -> Self {
        let arm = rcm_arm();
        let state = vec![0.0, 0.0, 0.0, 0.0];
        // Compute the home tool pose so the trocar and test targets are
        // guaranteed to lie on the tool-shaft axis.
        let home = arm.forward(&state).expect("home pose");
        let axis = home.orientation.transform_vector(Vector3::Z);
        // RCM point 0.1 m behind the tip, on the shaft (within shaft_length).
        let rcm_point = home.position + axis * 0.1;
        let workspace = Workspace::new(
            Volume::new(
                Vector3::new(-0.30, -0.30, -0.05),
                Vector3::new(0.30, 0.30, 0.40),
            )
            .expect("valid volume"),
            vec![TissueBoundary::new(Vector3::new(0.0, 0.0, 0.15), Vector3::Z)],
        );
        let rcm = RcmConstraint::new(rcm_point, 0.002, 0.30);
        Self {
            arm,
            workspace,
            rcm,
            state,
            force_limit: 5.0,
            depth_limit: 0.12,
        }
    }

    /// Compute the current tool pose.
    pub fn tool_pose(&self) -> Result<ToolPose, op_kinematics::DhParamError> {
        self.arm.forward(&self.state)
    }

    /// Compute the Jacobian and singularity report at the current state.
    pub fn jacobian_report(
        &self,
        tol: Scalar,
    ) -> Result<op_kinematics::SingularityReport, op_kinematics::DhParamError> {
        Jacobian::analyze(&self.arm, &self.state, tol)
    }

    /// Build a plan from the current pose to a target position, then generate
    /// the boolean constraint set for it.
    pub fn plan_to(
        &self,
        target: Vector3,
    ) -> Result<(MotionPath, op_constraints::ConstraintSet), String> {
        self.plan_to_with_force(target, Some(1.0))
    }

    /// Plan motion to target position with a specified intended force.
    pub fn plan_to_with_force(
        &self,
        target: Vector3,
        force: Option<Scalar>,
    ) -> Result<(MotionPath, op_constraints::ConstraintSet), String> {
        let current = self.tool_pose().map_err(|e| e.to_string())?;
        let q0 = self.state.clone();
        let target_pose = ToolPose {
            position: target,
            orientation: current.orientation,
        };
        let ik = self
            .arm
            .inverse(&target_pose, &q0, 1e-4)
            .map_err(|e| e.to_string())?;
        if !ik.converged {
            return Err(format!(
                "IK did not converge (final error {:.6} m)",
                ik.position_error
            ));
        }
        let end_pose = self.arm.forward(&ik.q).map_err(|e| e.to_string())?;
        let start_wp = Waypoint {
            pose: current.orientation,
            force,
        };
        let end_wp = Waypoint {
            pose: end_pose.orientation,
            force,
        };
        let path = MotionPath::new(start_wp, end_wp);
        let gen = PathConstraintGenerator::new(
            self.workspace.clone(),
            Some(self.rcm),
            self.force_limit,
            self.depth_limit,
        );
        let set = gen.generate(&path, 8).map_err(|e| e.to_string())?;
        Ok((path, set))
    }

    /// Build the trustless execution gate for this scenario.
    pub fn build_gate(&self) -> ExecutionGate {
        let invariants = LethalInvariantSet::new(vec![
            LethalInvariant {
                name: "workspace_envelope".into(),
                kind: InvariantKind::WorkspaceEnvelope {
                    min: self.workspace.reachable.min,
                    max: self.workspace.reachable.max,
                },
            },
            LethalInvariant {
                name: "tissue_boundary".into(),
                kind: InvariantKind::TissueBoundary { clearance: 0.005 },
            },
            LethalInvariant {
                name: "force_limit".into(),
                kind: InvariantKind::ForceLimit {
                    max_force: self.force_limit,
                },
            },
            LethalInvariant {
                name: "depth_limit".into(),
                kind: InvariantKind::DepthLimit {
                    max_depth: self.depth_limit,
                },
            },
            LethalInvariant {
                name: "rcm_stay".into(),
                kind: InvariantKind::RcmStay {
                    tolerance: self.rcm.tolerance,
                },
            },
            LethalInvariant {
                name: "energy_budget".into(),
                kind: InvariantKind::EnergyBudgetLimit { max_joules: 2.0 },
            },
        ]);
        ExecutionGate::new(Verifier::new(invariants))
    }

    /// Check the RCM constraint at the current tool pose.
    pub fn check_rcm(&self) -> Result<(), op_trajectory::RcmError> {
        let pose = self
            .tool_pose()
            .map_err(|_| op_trajectory::RcmError::DegenerateAxis)?;
        let axis = pose.orientation.transform_vector(Vector3::Z);
        self.rcm.check(pose.position, axis)
    }

    /// Run a predefined clinical scenario and evaluate it through the proof gate.
    pub fn run_scenario(&self, scenario_name: &str) -> Result<ScenarioResult, String> {
        let home = self.tool_pose().map_err(|e| e.to_string())?;
        let axis = home.orientation.transform_vector(Vector3::Z);
        let mut gate = self.build_gate();

        match scenario_name {
            "safe-insertion" => {
                let target = home.position + axis * 0.04;
                let (_path, set) = self.plan_to_with_force(target, Some(1.5))?;
                let wrench = ToolWrench {
                    force: Vector3::new(0.0, 0.0, 1.5),
                    torque: Vector3::ZERO,
                };
                let er = EnergyAnalyzer.action_energy(wrench, wrench, axis * 0.04, 0.0);
                let cmd = ActuationCommand::new(self.state.clone(), "Nominal Safe Trocar Insertion");
                let decision = gate
                    .evaluate_full(&cmd, Some(&set), None, Some(&er))
                    .map_err(|e| e.to_string())?;
                Ok(ScenarioResult {
                    name: scenario_name.to_string(),
                    description: "Nominal trocar insertion along shaft axis within safe margins"
                        .into(),
                    released: decision.release,
                    outcome: decision.outcome,
                    message: decision.message,
                    violations: decision.violations.iter().map(|v| v.detail.clone()).collect(),
                })
            }
            "tissue-breach" => {
                // Target penetrating beyond protected tissue boundary at z = 0.15 (e.g. at z = 0.10)
                let target = Vector3::new(home.position.x, home.position.y, 0.10);
                let (_path, set) = self.plan_to_with_force(target, Some(1.0))?;
                let cmd = ActuationCommand::new(self.state.clone(), "Unsafe Tissue Plane Penetration");
                let decision = gate
                    .evaluate_full(&cmd, Some(&set), None, None)
                    .map_err(|e| e.to_string())?;
                Ok(ScenarioResult {
                    name: scenario_name.to_string(),
                    description: "Target trajectory penetrates protected anatomical tissue plane".into(),
                    released: decision.release,
                    outcome: decision.outcome,
                    message: decision.message,
                    violations: decision.violations.iter().map(|v| v.detail.clone()).collect(),
                })
            }
            "force-overload" => {
                // Command with excessive contact force (12.0 N vs 5.0 N limit)
                let target = home.position + axis * 0.02;
                let (_path, set) = self.plan_to_with_force(target, Some(12.0))?;
                let wrench = ToolWrench {
                    force: Vector3::new(0.0, 0.0, 12.0),
                    torque: Vector3::ZERO,
                };
                let er = EnergyAnalyzer.action_energy(wrench, wrench, axis * 0.02, 0.0);
                let cmd = ActuationCommand::new(self.state.clone(), "Excessive Contact Force Action");
                let decision = gate
                    .evaluate_full(&cmd, Some(&set), None, Some(&er))
                    .map_err(|e| e.to_string())?;
                Ok(ScenarioResult {
                    name: scenario_name.to_string(),
                    description: "Tool tip contact force exceeds safe tissue pressure threshold".into(),
                    released: decision.release,
                    outcome: decision.outcome,
                    message: decision.message,
                    violations: decision.violations.iter().map(|v| v.detail.clone()).collect(),
                })
            }
            "rcm-deviation" => {
                // Off-axis translation that pulls tool shaft sideways out of trocar pivot
                let target = home.position + Vector3::new(0.05, 0.05, 0.02);
                let (_path, set) = self.plan_to_with_force(target, Some(1.0))?;
                let cmd = ActuationCommand::new(self.state.clone(), "Trocar Off-Axis Translation");
                let decision = gate
                    .evaluate_full(&cmd, Some(&set), None, None)
                    .map_err(|e| e.to_string())?;
                Ok(ScenarioResult {
                    name: scenario_name.to_string(),
                    description: "Tool motion violates remote-center-of-motion trocar alignment".into(),
                    released: decision.release,
                    outcome: decision.outcome,
                    message: decision.message,
                    violations: decision.violations.iter().map(|v| v.detail.clone()).collect(),
                })
            }
            "tainted-sensor" => {
                let mut sources = HashMap::new();
                sources.insert("untrusted_vision_feed".into(), TrustLevel::Untrusted);
                let tracker = TaintTracker::new(sources);
                let taint_cmd = TaintActuatorCommand {
                    name: "vision_guided_pose".into(),
                    value: 0.05,
                    taint: vec![TaintedValue {
                        source: "untrusted_vision_feed".into(),
                        trust: TrustLevel::Untrusted,
                        value: 0.05,
                    }],
                };
                let cmd = ActuationCommand::with_taint(
                    self.state.clone(),
                    "Vision Guided Actuation (Unsanitized)",
                    taint_cmd,
                );
                let target = home.position + axis * 0.02;
                let (_, set) = self.plan_to_with_force(target, Some(1.0))?;
                let decision = gate
                    .evaluate_full(&cmd, Some(&set), Some(&tracker), None)
                    .map_err(|e| e.to_string())?;
                Ok(ScenarioResult {
                    name: scenario_name.to_string(),
                    description: "Command depends on unverified sensor feed without sanitizer approval".into(),
                    released: decision.release,
                    outcome: decision.outcome,
                    message: decision.message,
                    violations: decision.violations.iter().map(|v| v.detail.clone()).collect(),
                })
            }
            "patient-motion" => {
                // Diaphragmatic motion: swept tissue boundary expanded toward tool (by 0.18 m)
                let swept_boundary = self.workspace.boundaries[0].expanded(0.18);
                let mut swept_ws = self.workspace.clone();
                swept_ws.boundaries = vec![swept_boundary];
                let gen = PathConstraintGenerator::new(
                    swept_ws,
                    Some(self.rcm),
                    self.force_limit,
                    self.depth_limit,
                );
                let target = home.position + axis * 0.04;
                let start_wp = Waypoint {
                    pose: home.orientation,
                    force: Some(1.0),
                };
                let end_wp = Waypoint {
                    pose: op_math::Isometry3::new(home.orientation.rotation, target),
                    force: Some(1.0),
                };
                let path = MotionPath::new(start_wp, end_wp);
                let set = gen.generate(&path, 8).map_err(|e| e.to_string())?;
                let cmd = ActuationCommand::new(self.state.clone(), "Insertion during Patient Motion");
                let decision = gate
                    .evaluate_full(&cmd, Some(&set), None, None)
                    .map_err(|e| e.to_string())?;
                Ok(ScenarioResult {
                    name: scenario_name.to_string(),
                    description: "Dynamic tissue shift sweeps anatomical boundary into trajectory".into(),
                    released: decision.release,
                    outcome: decision.outcome,
                    message: decision.message,
                    violations: decision.violations.iter().map(|v| v.detail.clone()).collect(),
                })
            }
            _ => Err(format!(
                "Unknown scenario '{}'. Valid scenarios: safe-insertion, tissue-breach, force-overload, rcm-deviation, tainted-sensor, patient-motion",
                scenario_name
            )),
        }
    }

    /// Run real-time latency profiling over N execution gate cycles.
    pub fn run_latency_benchmark(&self, iterations: usize) -> Result<BenchmarkProfile, String> {
        if iterations == 0 {
            return Err("Benchmark requires at least 1 iteration".into());
        }
        let home = self.tool_pose().map_err(|e| e.to_string())?;
        let axis = home.orientation.transform_vector(Vector3::Z);
        let target = home.position + axis * 0.03;
        let mut gate = self.build_gate();

        let mut latencies_us = Vec::with_capacity(iterations);
        let wrench = ToolWrench {
            force: Vector3::new(0.0, 0.0, 1.0),
            torque: Vector3::ZERO,
        };

        for _ in 0..iterations {
            let t0 = std::time::Instant::now();
            let (_path, set) = self.plan_to_with_force(target, Some(1.0))?;
            let er = EnergyAnalyzer.action_energy(wrench, wrench, axis * 0.03, 0.0);
            let cmd = ActuationCommand::new(self.state.clone(), "realtime benchmark cycle");
            let decision = gate
                .evaluate_full(&cmd, Some(&set), None, Some(&er))
                .map_err(|e| e.to_string())?;
            if !decision.release {
                return Err("Benchmark cycle unexpectedly failed safety proof".into());
            }
            let elapsed = t0.elapsed().as_secs_f64() * 1_000_000.0;
            latencies_us.push(elapsed);
        }

        latencies_us.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let total: f64 = latencies_us.iter().sum();
        let mean_us = total / iterations as f64;
        let p50_us = latencies_us[iterations / 2];
        let p95_us = latencies_us[(iterations * 95) / 100];
        let p99_us = latencies_us[(iterations * 99) / 100];
        let max_us = *latencies_us.last().unwrap_or(&0.0);

        Ok(BenchmarkProfile {
            iterations,
            mean_us,
            p50_us,
            p95_us,
            p99_us,
            max_us,
            hard_realtime_compliant: max_us < 5000.0,
        })
    }
}

/// Build an RCM-style 4-DoF arm approximating a parallelogram laparoscopic
/// manipulator: yaw, pitch, insertion (prismatic), tool roll.
pub fn rcm_arm() -> Chain {
    Chain {
        links: vec![
            DhLink::revolute(0.0, 0.30, 0.0, deg_to_rad(-90.0)),
            DhLink::revolute(0.0, 0.0, 0.25, 0.0),
            DhLink::prismatic(0.0, 0.10, 0.0, 0.0),
            DhLink::revolute(0.0, 0.0, 0.0, 0.0),
        ],
    }
}

/// Render an ASCII schematic of the surgical workspace and robot arm.
pub fn render_ascii_workspace(sim: &Simulator) -> String {
    let pose = sim.tool_pose().map(|p| p.position).unwrap_or(Vector3::ZERO);
    let rcm = sim.rcm.point;
    let b_z = sim.workspace.boundaries.first().map(|b| b.point.z).unwrap_or(0.15);

    format!(
r#"
       [ROBOT BASE (0, 0, 0)]
              ||
       [J0: Shoulder Yaw]
              ||
       [J1: Elbow Pitch] === [J2: Prismatic Insertion] === [J3: Tool Roll]
                                  \\
                                   \\ (Tool Shaft)
                         ======== (O) Trocar RCM [{:.3}, {:.3}, {:.3}]
                                     \
                                      \
                                       [*] Tool Tip [{:.3}, {:.3}, {:.3}]
  ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
       PROTECTED TISSUE BOUNDARY (Z = {:.3} m)
  ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
"#,
        rcm.x, rcm.y, rcm.z, pose.x, pose.y, pose.z, b_z
    )
}

/// Render a textual 3D overview of the scenario report.
pub fn render_summary(sim: &Simulator) -> String {
    let pose = sim.tool_pose().map(|p| p.position).unwrap_or(Vector3::ZERO);
    let singular_report = sim.jacobian_report(1e-6);
    let (singular, cond, min_sv) = match singular_report {
        Ok(r) => (r.is_singular, r.condition_number, r.min_singular_value),
        Err(_) => (false, 1.0, 1.0),
    };
    let rcm_ok = sim.check_rcm().is_ok();
    let mut gate = sim.build_gate();
    let cmd = ActuationCommand::new(sim.state.clone(), "simulated actuation");
    let decision = gate.evaluate(&cmd).unwrap_or(GateDecision {
        release: false,
        outcome: ProofOutcome::Unknown,
        message: "gate error".into(),
        violations: Vec::new(),
    });

    let mut out = String::new();
    out.push_str(&render_ascii_workspace(sim));
    out.push_str(&format!(
        "--- Telemetry & Safety Status ---\n\
         • Tool tip position: ({:.3}, {:.3}, {:.3}) m\n\
         • Kinematic singularity: {} (min singular value: {:.4}, condition: {:.2})\n\
         • Trocar RCM constraint: {}\n\
         • Execution gate decision: release={} ({})\n\
         • Verification proof outcome: {:?}",
        pose.x,
        pose.y,
        pose.z,
        if singular { "AT RISK (Singular)" } else { "Nominal (Full Rank)" },
        min_sv,
        cond,
        if rcm_ok { "SATISFIED (Trocar-aligned)" } else { "VIOLATED (Off-axis)" },
        decision.release,
        decision.message,
        decision.outcome
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_scenario_is_valid() {
        let sim = Simulator::default_scenario();
        assert!(sim.tool_pose().is_ok());
        assert!(sim.check_rcm().is_ok());
    }

    #[test]
    fn plans_to_target() {
        let sim = Simulator::default_scenario();
        let home = sim.tool_pose().unwrap();
        let axis = home.orientation.transform_vector(Vector3::Z);
        let target = home.position + axis * 0.05;
        let (_, set) = sim.plan_to(target).unwrap();
        assert!(!set.quantities.is_empty());
    }

    #[test]
    fn scenario_safe_insertion_releases() {
        let sim = Simulator::default_scenario();
        let res = sim.run_scenario("safe-insertion").unwrap();
        assert!(res.released);
        assert_eq!(res.outcome, ProofOutcome::Proved);
    }

    #[test]
    fn scenario_tissue_breach_blocks() {
        let sim = Simulator::default_scenario();
        let res = sim.run_scenario("tissue-breach").unwrap();
        assert!(!res.released);
        assert_eq!(res.outcome, ProofOutcome::Disproved);
        assert!(!res.violations.is_empty());
    }

    #[test]
    fn scenario_force_overload_blocks() {
        let sim = Simulator::default_scenario();
        let res = sim.run_scenario("force-overload").unwrap();
        assert!(!res.released);
        assert_eq!(res.outcome, ProofOutcome::Disproved);
    }

    #[test]
    fn scenario_rcm_deviation_blocks() {
        let sim = Simulator::default_scenario();
        let res = sim.run_scenario("rcm-deviation").unwrap();
        assert!(!res.released);
        assert_eq!(res.outcome, ProofOutcome::Disproved);
    }

    #[test]
    fn scenario_tainted_sensor_blocks() {
        let sim = Simulator::default_scenario();
        let res = sim.run_scenario("tainted-sensor").unwrap();
        assert!(!res.released);
        assert_eq!(res.outcome, ProofOutcome::Disproved);
    }

    #[test]
    fn scenario_patient_motion_blocks() {
        let sim = Simulator::default_scenario();
        let res = sim.run_scenario("patient-motion").unwrap();
        assert!(!res.released);
        assert_eq!(res.outcome, ProofOutcome::Disproved);
    }

    #[test]
    fn latency_benchmark_executes_and_passes() {
        let sim = Simulator::default_scenario();
        let bench = sim.run_latency_benchmark(50).unwrap();
        assert_eq!(bench.iterations, 50);
        assert!(bench.mean_us > 0.0);
        assert!(bench.hard_realtime_compliant);
    }
}