//! # op-gate — Trustless execution gate
//!
//! An automated control system that **blocks** the physical command to the
//! servo/hydraulics unless a valid mathematical proof for the absolute safety
//! of the action exists. The gate holds no trust in the command source: it
//! re-verifies the action path against lethal invariants, audits sensor taint,
//! checks energy limits, and only releases when the proof succeeds.
//!
//! > **Research tooling only — not a deployable medical device.**

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use op_constraints::{ConstraintSet, PathConstraintGenerator};
use op_energy::{EnergyAnalyzer, EnergyReport};
use op_taint::{ActuatorCommand as TaintActuatorCommand, TaintTracker};
use op_trajectory::MotionPath;
use op_verifier::{ProofOutcome, VerificationResult, Verifier, Violation};
use serde::{Deserialize, Serialize};

/// A physical actuation command that must pass the gate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActuationCommand {
    /// Target joint configuration (radians / metres).
    pub command: Vec<f64>,
    /// Human-readable description of the action.
    pub description: String,
    /// Optional taint-tracking provenance of the command values.
    pub taint_command: Option<TaintActuatorCommand>,
}

impl ActuationCommand {
    /// Construct a basic actuation command.
    pub fn new(command: Vec<f64>, description: impl Into<String>) -> Self {
        Self {
            command,
            description: description.into(),
            taint_command: None,
        }
    }

    /// Construct an actuation command with taint metadata.
    pub fn with_taint(
        command: Vec<f64>,
        description: impl Into<String>,
        taint: TaintActuatorCommand,
    ) -> Self {
        Self {
            command,
            description: description.into(),
            taint_command: Some(taint),
        }
    }
}

/// Decision of the execution gate for a command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GateDecision {
    /// True when the command may be released to the physical actuator.
    pub release: bool,
    /// Verification outcome that produced this decision.
    pub outcome: ProofOutcome,
    /// Explanation of the decision.
    pub message: String,
    /// Specific invariant violations if the command was blocked.
    pub violations: Vec<Violation>,
}

/// An immutable audit record of a gate decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GateAuditEntry {
    /// Monotonic sequence number of the gate evaluation.
    pub step: u64,
    /// Description of the command.
    pub description: String,
    /// Gate decision release flag.
    pub release: bool,
    /// Outcome of the proof.
    pub outcome: ProofOutcome,
    /// Explanatory message.
    pub message: String,
    /// Recorded violations.
    pub violations: Vec<Violation>,
}

/// Error raised by the execution gate.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum GateError {
    /// The gate cannot evaluate an empty command.
    #[error("gate cannot evaluate an empty command")]
    EmptyCommand,
    /// Error during kinetic path constraint generation.
    #[error("path constraint generation failed: {0}")]
    ConstraintGenError(String),
    /// Sensor taint audit failed (untrusted un-sanitized sensor values detected).
    #[error("sensor taint audit failed: {0}")]
    TaintError(String),
}

/// Trustless execution gate: blocks actuation unless the proof succeeds.
#[derive(Debug, Clone)]
pub struct ExecutionGate {
    /// The verifier used to prove action safety against lethal invariants.
    pub verifier: Verifier,
    /// Optional energy analyzer for tool wrench and power verification.
    pub energy_analyzer: EnergyAnalyzer,
    /// Number of commands released so far.
    pub released_count: u64,
    /// Number of commands blocked so far.
    pub blocked_count: u64,
    /// Complete chronological audit trail of all evaluations.
    pub audit_log: Vec<GateAuditEntry>,
}

impl ExecutionGate {
    /// Construct a gate backed by a verifier.
    pub fn new(verifier: Verifier) -> Self {
        Self {
            verifier,
            energy_analyzer: EnergyAnalyzer,
            released_count: 0,
            blocked_count: 0,
            audit_log: Vec::new(),
        }
    }

    /// Evaluate a pre-generated constraint set through the verifier and gate.
    pub fn evaluate_constraint_set(
        &mut self,
        set: &ConstraintSet,
        description: &str,
    ) -> GateDecision {
        let result: VerificationResult = self.verifier.verify(set);
        let release = result.outcome == ProofOutcome::Proved;
        let step = self.released_count + self.blocked_count + 1;

        if release {
            self.released_count += 1;
        } else {
            self.blocked_count += 1;
        }

        let entry = GateAuditEntry {
            step,
            description: description.to_string(),
            release,
            outcome: result.outcome,
            message: result.message.clone(),
            violations: result.violations.clone(),
        };
        self.audit_log.push(entry);

        GateDecision {
            release,
            outcome: result.outcome,
            message: result.message,
            violations: result.violations,
        }
    }

    /// Plan, constrain, verify, and gate a candidate motion path.
    pub fn evaluate_path(
        &mut self,
        path: &MotionPath,
        generator: &PathConstraintGenerator,
        samples: usize,
        description: &str,
    ) -> Result<GateDecision, GateError> {
        let set = generator
            .generate(path, samples)
            .map_err(|e| GateError::ConstraintGenError(e.to_string()))?;
        Ok(self.evaluate_constraint_set(&set, description))
    }

    /// Evaluate a physical command with optional constraint set, taint tracker, and energy report.
    pub fn evaluate_full(
        &mut self,
        command: &ActuationCommand,
        constraint_set: Option<&ConstraintSet>,
        taint_tracker: Option<&TaintTracker>,
        energy_report: Option<&EnergyReport>,
    ) -> Result<GateDecision, GateError> {
        if command.command.is_empty() {
            return Err(GateError::EmptyCommand);
        }

        // 1. Taint Audit: If command contains sensor taint, verify it against taint tracker.
        if let (Some(tracker), Some(taint_cmd)) = (taint_tracker, &command.taint_command) {
            let sanitize_res = tracker
                .audit(taint_cmd)
                .map_err(|e| GateError::TaintError(e.to_string()))?;
            if !sanitize_res.approved {
                self.blocked_count += 1;
                let step = self.released_count + self.blocked_count;
                let msg = sanitize_res
                    .reason
                    .unwrap_or_else(|| "untrusted sensor taint detected".into());
                let entry = GateAuditEntry {
                    step,
                    description: command.description.clone(),
                    release: false,
                    outcome: ProofOutcome::Disproved,
                    message: format!("BLOCKED by taint audit: {}", msg),
                    violations: Vec::new(),
                };
                self.audit_log.push(entry);
                return Ok(GateDecision {
                    release: false,
                    outcome: ProofOutcome::Disproved,
                    message: format!("BLOCKED by taint audit: {}", msg),
                    violations: Vec::new(),
                });
            }
        }

        // 2. Build or use constraint set for SMT/Invariant verification
        let mut set = match constraint_set {
            Some(s) => s.clone(),
            None => self.command_to_constraints(command),
        };

        // 3. Inject energy & power evaluations if available
        if let Some(er) = energy_report {
            set.evaluations.push(op_constraints::Evaluation {
                name: "energy".into(),
                value: er.total_energy,
            });
            set.evaluations.push(op_constraints::Evaluation {
                name: "power".into(),
                value: er.peak_power,
            });
        }

        Ok(self.evaluate_constraint_set(&set, &command.description))
    }

    /// Legacy evaluate helper for simple actuation commands.
    pub fn evaluate(&mut self, command: &ActuationCommand) -> Result<GateDecision, GateError> {
        self.evaluate_full(command, None, None, None)
    }

    /// Construct a basic constraint set for a raw joint command.
    fn command_to_constraints(&self, command: &ActuationCommand) -> ConstraintSet {
        let depth_val = command.command.first().copied().unwrap_or(0.0);
        let evaluations = vec![op_constraints::Evaluation {
            name: "depth".into(),
            value: depth_val,
        }];

        ConstraintSet {
            formula: op_constraints::ConstraintExpr::Compare {
                lhs: "depth".into(),
                op: op_constraints::Comparison::Le,
                rhs: "depth_limit".into(),
            },
            quantities: vec!["depth".into()],
            evaluations,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_math::{Isometry3, Quaternion, Vector3};
    use op_trajectory::{TissueBoundary, Volume, Waypoint, Workspace};
    use op_verifier::{InvariantKind, LethalInvariant, LethalInvariantSet};

    fn test_gate() -> ExecutionGate {
        let invariants = LethalInvariantSet::new(vec![
            LethalInvariant {
                name: "depth".into(),
                kind: InvariantKind::DepthLimit { max_depth: 0.1 },
            },
            LethalInvariant {
                name: "tissue".into(),
                kind: InvariantKind::TissueBoundary { clearance: 0.01 },
            },
        ]);
        ExecutionGate::new(Verifier::new(invariants))
    }

    #[test]
    fn releases_valid_command() {
        let mut g = test_gate();
        let cmd = ActuationCommand::new(vec![0.05], "insert tool 5cm");
        let d = g.evaluate(&cmd).unwrap();
        assert!(d.release);
        assert_eq!(d.outcome, ProofOutcome::Proved);
        assert_eq!(g.released_count, 1);
        assert_eq!(g.audit_log.len(), 1);
    }

    #[test]
    fn rejects_depth_exceeding_command() {
        let mut g = test_gate();
        let cmd = ActuationCommand::new(vec![0.15], "insert tool 15cm (unsafe)");
        let d = g.evaluate(&cmd).unwrap();
        assert!(!d.release);
        assert_eq!(d.outcome, ProofOutcome::Disproved);
        assert_eq!(g.blocked_count, 1);
    }

    #[test]
    fn rejects_empty_command() {
        let mut g = test_gate();
        let cmd = ActuationCommand::new(vec![], "empty");
        assert!(matches!(g.evaluate(&cmd), Err(GateError::EmptyCommand)));
    }

    #[test]
    fn evaluates_full_path_through_gate() {
        let mut g = test_gate();
        let vol = Volume::new(
            Vector3::new(-0.2, -0.2, -0.05),
            Vector3::new(0.2, 0.2, 0.2),
        )
        .unwrap();
        let boundary = TissueBoundary::new(Vector3::new(0.0, 0.0, 0.12), -Vector3::Z);
        let ws = Workspace::new(vol, vec![boundary]);
        let start = Waypoint {
            pose: Isometry3::from_rotation_quaternion(
                Quaternion::IDENTITY,
                Vector3::new(0.0, 0.0, 0.02),
            ),
            force: Some(1.0),
        };
        let end_safe = Waypoint {
            pose: Isometry3::from_rotation_quaternion(
                Quaternion::IDENTITY,
                Vector3::new(0.0, 0.0, 0.05),
            ),
            force: Some(1.0),
        };
        let end_breach = Waypoint {
            pose: Isometry3::from_rotation_quaternion(
                Quaternion::IDENTITY,
                Vector3::new(0.0, 0.0, 0.15), // breaches boundary at 0.12!
            ),
            force: Some(1.0),
        };

        let gen = PathConstraintGenerator::new(ws, None, 5.0, 0.1);

        let safe_path = MotionPath::new(start, end_safe);
        let d_safe = g.evaluate_path(&safe_path, &gen, 5, "safe incision").unwrap();
        assert!(d_safe.release);

        let breach_path = MotionPath::new(start, end_breach);
        let d_breach = g.evaluate_path(&breach_path, &gen, 5, "tissue breach").unwrap();
        assert!(!d_breach.release);
        assert!(!d_breach.violations.is_empty());
    }

    #[test]
    fn blocks_untrusted_tainted_sensor_command() {
        let mut g = test_gate();
        let mut sources = std::collections::HashMap::new();
        sources.insert("untrusted_joint_encoder".into(), op_taint::TrustLevel::Untrusted);
        let tracker = TaintTracker::new(sources);

        let taint_cmd = op_taint::ActuatorCommand {
            name: "joint_0_target".into(),
            value: 0.05,
            taint: vec![op_taint::TaintedValue {
                source: "untrusted_joint_encoder".into(),
                trust: op_taint::TrustLevel::Untrusted,
                value: 0.05,
            }],
        };

        let cmd = ActuationCommand::with_taint(vec![0.05], "actuate from encoder", taint_cmd);
        let decision = g.evaluate_full(&cmd, None, Some(&tracker), None).unwrap();
        assert!(!decision.release);
        assert!(decision.message.contains("BLOCKED by taint audit"));
    }
}