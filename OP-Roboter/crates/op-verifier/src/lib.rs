//! # op-verifier — SMT verification engine + lethal invariant enforcement
//!
//! Checks whether a planned surgical action path violates any **lethal
//! invariant** before the command is released.
//!
//! - **Default build** uses a built-in mock solver (no Z3 required).
//! - **`z3-solver` feature** plugs in the real Z3 SMT solver.
//!
//! > **Research tooling only — not a deployable medical device.**

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod invariant;
mod mock;

#[cfg(feature = "z3-solver")]
mod z3;

pub use op_constraints::{Comparison, ConstraintExpr, ConstraintSet};
pub use invariant::{InvariantKind, LethalInvariant, LethalInvariantSet, Violation};
pub use mock::{EvaluationError, MockSolver, Verdict};

/// What the verification proved about a constraint formula.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ProofOutcome {
    /// The formula is valid: no fault state is reachable. Command may release.
    Proved,
    /// A fault state is reachable. Command must be blocked.
    Disproved,
    /// The solver could not decide within the resource limits.
    Unknown,
}

/// Result of verifying a single command path.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VerificationResult {
    /// Solver verdict on the safety formula.
    pub outcome: ProofOutcome,
    /// Human-readable explanation.
    pub message: String,
    /// Invariants that would be violated (empty when `Proved`).
    pub violations: Vec<Violation>,
}

/// Engine that verifies constraint formulas against lethal invariants.
#[derive(Debug, Clone)]
pub struct Verifier {
    /// Lethal invariants that must never be violated.
    pub invariants: LethalInvariantSet,
    /// Backend solver.
    pub solver: MockSolver,
}

impl Verifier {
    /// Construct a verifier with the given invariants.
    pub fn new(invariants: LethalInvariantSet) -> Self {
        Self { invariants, solver: MockSolver }
    }

    /// Verify a constraint formula produced for an action path.
    pub fn verify(&self, set: &ConstraintSet) -> VerificationResult {
        let env = self.build_env(set);
        let verdict = self.solver.check_formula(&set.formula, &env);
        if verdict == Verdict::Satisfied {
            let violations = self.find_violations(set, &env);
            if violations.is_empty() {
                VerificationResult {
                    outcome: ProofOutcome::Proved,
                    message: "all lethal invariants hold — command may release".into(),
                    violations: Vec::new(),
                }
            } else {
                VerificationResult {
                    outcome: ProofOutcome::Disproved,
                    message: format!("{} violation(s) detected", violations.len()),
                    violations,
                }
            }
        } else if verdict == Verdict::Unsatisfied {
            VerificationResult {
                outcome: ProofOutcome::Disproved,
                message: "a lethal invariant is violated by the planned path".into(),
                violations: self.find_violations(set, &env),
            }
        } else {
            VerificationResult {
                outcome: ProofOutcome::Unknown,
                message: "solver could not decide the safety formula".into(),
                violations: Vec::new(),
            }
        }
    }

    /// Populate a quantity binding.
    fn env_push(env: &mut Vec<op_constraints::Evaluation>, name: &str, value: f64) {
        if let Some(existing) = env.iter_mut().find(|e| e.name == name) {
            existing.value = value;
        } else {
            env.push(op_constraints::Evaluation {
                name: name.to_string(),
                value,
            });
        }
    }

    fn build_env(&self, set: &ConstraintSet) -> Vec<op_constraints::Evaluation> {
        let mut env = set.evaluations.clone();
        for inv in &self.invariants.invariants {
            match inv.kind {
                InvariantKind::WorkspaceEnvelope { min, max } => {
                    Self::env_push(&mut env, "ws_min_x", min.x);
                    Self::env_push(&mut env, "ws_max_x", max.x);
                    Self::env_push(&mut env, "ws_min_y", min.y);
                    Self::env_push(&mut env, "ws_max_y", max.y);
                    Self::env_push(&mut env, "ws_min_z", min.z);
                    Self::env_push(&mut env, "ws_max_z", max.z);
                }
                InvariantKind::TissueBoundary { clearance } => {
                    Self::env_push(&mut env, "margin", clearance);
                    for q in &set.quantities {
                        if (q.starts_with("clear_") || q.starts_with("seg_clear_"))
                            && !env.iter().any(|e| &e.name == q)
                        {
                            Self::env_push(&mut env, q, clearance);
                        }
                    }
                }
                InvariantKind::ForceLimit { max_force } => {
                    Self::env_push(&mut env, "force_limit", max_force);
                    if !env.iter().any(|e| e.name == "force") {
                        Self::env_push(&mut env, "force", max_force);
                    }
                }
                InvariantKind::DepthLimit { max_depth } => {
                    Self::env_push(&mut env, "depth_limit", max_depth);
                    if !env.iter().any(|e| e.name == "depth") {
                        Self::env_push(&mut env, "depth", max_depth);
                    }
                }
                InvariantKind::RcmStay { tolerance } => {
                    Self::env_push(&mut env, "rcm_tol", tolerance);
                    for q in &set.quantities {
                        if q.starts_with("rcm_off_") && !env.iter().any(|e| &e.name == q) {
                            Self::env_push(&mut env, q, tolerance);
                        }
                    }
                }
                InvariantKind::EnergyBudgetLimit { max_joules } => {
                    Self::env_push(&mut env, "energy_limit", max_joules);
                    if !env.iter().any(|e| e.name == "energy") {
                        Self::env_push(&mut env, "energy", max_joules);
                    }
                }
                InvariantKind::PowerLimit { max_watts } => {
                    Self::env_push(&mut env, "power_limit", max_watts);
                    if !env.iter().any(|e| e.name == "power") {
                        Self::env_push(&mut env, "power", max_watts);
                    }
                }
            }
        }

        // Provide unconstrained defaults for bounds not specified by invariants.
        if !env.iter().any(|e| e.name == "ws_min_x") {
            Self::env_push(&mut env, "ws_min_x", -f64::INFINITY);
            Self::env_push(&mut env, "ws_max_x", f64::INFINITY);
            Self::env_push(&mut env, "ws_min_y", -f64::INFINITY);
            Self::env_push(&mut env, "ws_max_y", f64::INFINITY);
            Self::env_push(&mut env, "ws_min_z", -f64::INFINITY);
            Self::env_push(&mut env, "ws_max_z", f64::INFINITY);
        }
        if !env.iter().any(|e| e.name == "margin") {
            Self::env_push(&mut env, "margin", 0.0);
        }
        if !env.iter().any(|e| e.name == "rcm_tol") {
            Self::env_push(&mut env, "rcm_tol", f64::INFINITY);
        }
        if !env.iter().any(|e| e.name == "force_limit") {
            Self::env_push(&mut env, "force_limit", f64::INFINITY);
        }
        if !env.iter().any(|e| e.name == "depth_limit") {
            Self::env_push(&mut env, "depth_limit", f64::INFINITY);
        }
        if !env.iter().any(|e| e.name == "energy_limit") {
            Self::env_push(&mut env, "energy_limit", f64::INFINITY);
        }
        if !env.iter().any(|e| e.name == "power_limit") {
            Self::env_push(&mut env, "power_limit", f64::INFINITY);
        }

        env
    }

    fn find_violations(
        &self,
        set: &ConstraintSet,
        env: &[op_constraints::Evaluation],
    ) -> Vec<Violation> {
        let mut out = Vec::new();
        let get_val = |name: &str| -> Option<f64> {
            env.iter().find(|e| e.name == name).map(|e| e.value)
        };

        for inv in &self.invariants.invariants {
            let mut is_violated = false;
            let mut detail = String::new();

            match inv.kind {
                InvariantKind::WorkspaceEnvelope { min, max } => {
                    for e in env {
                        if e.name.starts_with("pos_") && e.name.ends_with("_x") && (e.value < min.x || e.value > max.x) {
                            is_violated = true;
                            detail = format!("X position {:.4} m outside [{:.4}, {:.4}]", e.value, min.x, max.x);
                            break;
                        } else if e.name.starts_with("pos_") && e.name.ends_with("_y") && (e.value < min.y || e.value > max.y) {
                            is_violated = true;
                            detail = format!("Y position {:.4} m outside [{:.4}, {:.4}]", e.value, min.y, max.y);
                            break;
                        } else if e.name.starts_with("pos_") && e.name.ends_with("_z") && (e.value < min.z || e.value > max.z) {
                            is_violated = true;
                            detail = format!("Z position {:.4} m outside [{:.4}, {:.4}]", e.value, min.z, max.z);
                            break;
                        }
                    }
                }
                InvariantKind::TissueBoundary { clearance } => {
                    for e in env {
                        if (e.name.starts_with("clear_") || e.name.starts_with("seg_clear_"))
                            && e.value < clearance - 1e-9
                        {
                            is_violated = true;
                            detail = format!("tissue clearance {:.4} m below required {:.4} m", e.value, clearance);
                            break;
                        }
                    }
                }
                InvariantKind::ForceLimit { max_force } => {
                    if let Some(f) = get_val("force") {
                        if f > max_force + 1e-9 {
                            is_violated = true;
                            detail = format!("tool force {:.2} N exceeds limit {:.2} N", f, max_force);
                        }
                    }
                }
                InvariantKind::DepthLimit { max_depth } => {
                    if let Some(d) = get_val("depth") {
                        if d > max_depth + 1e-9 {
                            is_violated = true;
                            detail = format!("depth {:.4} m exceeds limit {:.4} m", d, max_depth);
                        }
                    }
                }
                InvariantKind::RcmStay { tolerance } => {
                    for e in env {
                        if e.name.starts_with("rcm_off_") && e.value > tolerance + 1e-9 {
                            is_violated = true;
                            detail = format!("RCM off-axis deviation {:.4} m exceeds tolerance {:.4} m", e.value, tolerance);
                            break;
                        }
                    }
                }
                InvariantKind::EnergyBudgetLimit { max_joules } => {
                    if let Some(en) = get_val("energy") {
                        if en > max_joules + 1e-9 {
                            is_violated = true;
                            detail = format!("action energy {:.4} J exceeds budget {:.4} J", en, max_joules);
                        }
                    }
                }
                InvariantKind::PowerLimit { max_watts } => {
                    if let Some(p) = get_val("power") {
                        if p > max_watts + 1e-9 {
                            is_violated = true;
                            detail = format!("peak power {:.4} W exceeds limit {:.4} W", p, max_watts);
                        }
                    }
                }
            }

            // If direct diagnostic didn't catch it but formula failed as a whole
            if !is_violated && self.solver.check_formula(&set.formula, env) != Verdict::Satisfied {
                is_violated = true;
                detail = "planned path violates this lethal invariant".into();
            }

            if is_violated {
                out.push(Violation {
                    invariant: inv.name.clone(),
                    kind: inv.kind,
                    detail: if detail.is_empty() { "invariant violated".into() } else { detail },
                });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_constraints::{Comparison, ConstraintExpr, ConstraintSet};

    fn depth_set(op: Comparison) -> ConstraintSet {
        ConstraintSet::new(
            ConstraintExpr::Compare {
                lhs: "depth".into(),
                op,
                rhs: "depth_limit".into(),
            },
            vec!["depth".into()],
        )
    }

    #[test]
    fn safe_path_is_proved() {
        let invariants = LethalInvariantSet::new(vec![LethalInvariant {
            name: "depth".into(),
            kind: InvariantKind::DepthLimit { max_depth: 0.1 },
        }]);
        let v = Verifier::new(invariants);
        let r = v.verify(&depth_set(Comparison::Le));
        assert_eq!(r.outcome, ProofOutcome::Proved, "{}", r.message);
    }

    #[test]
    fn unsafe_path_is_disproved() {
        let invariants = LethalInvariantSet::new(vec![LethalInvariant {
            name: "depth".into(),
            kind: InvariantKind::DepthLimit { max_depth: 0.05 },
        }]);
        let v = Verifier::new(invariants);
        let r = v.verify(&depth_set(Comparison::Gt));
        assert_eq!(r.outcome, ProofOutcome::Disproved, "{}", r.message);
    }

    #[test]
    fn tissue_clearance_violation_is_detected() {
        let invariants = LethalInvariantSet::new(vec![LethalInvariant {
            name: "tissue_boundary".into(),
            kind: InvariantKind::TissueBoundary { clearance: 0.01 },
        }]);
        let v = Verifier::new(invariants);
        let set = ConstraintSet::with_evaluations(
            ConstraintExpr::Compare {
                lhs: "clear_0_b0".into(),
                op: Comparison::Ge,
                rhs: "margin".into(),
            },
            vec!["clear_0_b0".into()],
            vec![op_constraints::Evaluation {
                name: "clear_0_b0".into(),
                value: -0.005, // Inside tissue!
            }],
        );
        let r = v.verify(&set);
        assert_eq!(r.outcome, ProofOutcome::Disproved);
        assert!(!r.violations.is_empty());
        assert_eq!(r.violations[0].invariant, "tissue_boundary");
    }

    #[test]
    fn energy_budget_violation_is_detected() {
        let invariants = LethalInvariantSet::new(vec![LethalInvariant {
            name: "energy_budget".into(),
            kind: InvariantKind::EnergyBudgetLimit { max_joules: 2.0 },
        }]);
        let v = Verifier::new(invariants);
        let set = ConstraintSet::with_evaluations(
            ConstraintExpr::Compare {
                lhs: "energy".into(),
                op: Comparison::Le,
                rhs: "energy_limit".into(),
            },
            vec!["energy".into()],
            vec![op_constraints::Evaluation {
                name: "energy".into(),
                value: 4.5, // Exceeds 2.0 J
            }],
        );
        let r = v.verify(&set);
        assert_eq!(r.outcome, ProofOutcome::Disproved);
        assert_eq!(r.violations[0].invariant, "energy_budget");
    }
}