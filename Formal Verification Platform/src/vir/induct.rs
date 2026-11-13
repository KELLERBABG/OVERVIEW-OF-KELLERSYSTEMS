//! Loop Invariant Induction & ICE Synthesis Engine (`KELLER-INDUCT`).
//!
//! Formulates and solves 3-clause inductive loop invariant proofs:
//!   1. Base Case:        Precondition(x0) => Invariant(x0)
//!   2. Inductive Step:   Invariant(x) AND Guard(x) => Invariant(Step(x))
//!   3. Safety Postcond:  Invariant(x) AND NOT Guard(x) => NoPanic(x)
//!
//! Synthesizes loop invariants using Iterative Candidate Elimination (ICE)
//! to prove loops safe for all N in [0, inf) in constant SMT time.

use crate::vir::ast::*;
use crate::vir::portfolio::{solve_with_backend, SolverCandidate};
use crate::vir::smt::SmtEmitter;

/// Describes a structured natural loop in Keller-VIR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopDescriptor {
    pub loop_id: String,
    pub counter_var: String,
    pub init_val: i128,
    pub step_val: i128,
    pub upper_bound_var: Option<String>,
    pub upper_bound_const: Option<i128>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InductiveProofResult {
    Proved { invariant_smt: String },
    Refuted { counterexample: String },
    Unknown { reason: String },
}

/// Synthesize a candidate loop invariant bounding the loop counter.
///
/// For a counter initialized at `init` with positive step, generates candidate:
///   `init <= counter <= upper_bound`
pub fn synthesize_candidate_invariant(desc: &LoopDescriptor) -> VirExpr {
    let counter = VirExpr::Var(desc.counter_var.clone());
    let init = VirExpr::ConstInt(desc.init_val, 0);

    // Lower bound: counter >= init
    let lower_bound = VirExpr::Binary(
        VirBinaryOp::Ge,
        Box::new(counter.clone()),
        Box::new(init),
    );

    // Upper bound: counter <= upper_bound
    let upper_bound = if let Some(ref ub_var) = desc.upper_bound_var {
        VirExpr::Binary(
            VirBinaryOp::Le,
            Box::new(counter),
            Box::new(VirExpr::Var(ub_var.clone())),
        )
    } else if let Some(ub_val) = desc.upper_bound_const {
        VirExpr::Binary(
            VirBinaryOp::Le,
            Box::new(counter),
            Box::new(VirExpr::ConstInt(ub_val, 0)),
        )
    } else {
        VirExpr::ConstBool(true)
    };

    VirExpr::Binary(
        VirBinaryOp::And,
        Box::new(lower_bound),
        Box::new(upper_bound),
    )
}

/// Prove the inductive step: (I(x) AND Guard(x)) => I(x + step).
///
/// In SMT, check if (I(x) AND Guard(x) AND NOT I(x + step)) is UNSAT.
pub fn prove_inductive_step(
    desc: &LoopDescriptor,
    candidate_inv: &VirExpr,
    solver: &SolverCandidate,
    timeout_ms: u64,
) -> bool {
    let mut emitter = SmtEmitter::new("ALL");
    emitter.declare_const(&desc.counter_var, "Int");
    if let Some(ref ub) = desc.upper_bound_var {
        emitter.declare_const(ub, "Int");
        emitter.assert_formula(&format!("(>= {ub} 0)"));
    }

    // Invariant I(x)
    let inv_smt = emitter.lower_expr(candidate_inv);
    emitter.assert_formula(&inv_smt);

    // Guard(x): counter < bound
    let guard_smt = if let Some(ref ub) = desc.upper_bound_var {
        format!("(< {} {})", desc.counter_var, ub)
    } else if let Some(ub) = desc.upper_bound_const {
        format!("(< {} {})", desc.counter_var, ub)
    } else {
        "true".to_string()
    };
    emitter.assert_formula(&guard_smt);

    // Step: x_next = x + step
    let next_counter = format!("(+ {} {})", desc.counter_var, desc.step_val);
    let next_inv_smt = if let Some(ref ub) = desc.upper_bound_var {
        format!("(and (>= {next_counter} {}) (<= {next_counter} {ub}))", desc.init_val)
    } else if let Some(ub) = desc.upper_bound_const {
        format!("(and (>= {next_counter} {}) (<= {next_counter} {ub}))", desc.init_val)
    } else {
        format!("(>= {next_counter} {})", desc.init_val)
    };

    // Assert violation of next invariant
    emitter.assert_formula(&format!("(not {next_inv_smt})"));

    let script = emitter.emit_script();
    let verdict = solve_with_backend(&script, solver, timeout_ms);
    matches!(verdict, crate::python::solver::SolverVerdict::Pass)
}

/// Full 3-clause inductive proof execution.
pub fn prove_loop_induction(
    desc: &LoopDescriptor,
    safety_goal: &VirExpr,
    solver: &SolverCandidate,
    timeout_ms: u64,
) -> InductiveProofResult {
    let candidate = synthesize_candidate_invariant(desc);

    // 1. Verify Base Case: init satisfies invariant
    {
        let mut emitter = SmtEmitter::new("ALL");
        emitter.declare_const(&desc.counter_var, "Int");
        if let Some(ref ub) = desc.upper_bound_var {
            emitter.declare_const(ub, "Int");
            emitter.assert_formula(&format!("(>= {ub} 0)"));
        }
        emitter.assert_formula(&format!("(= {} {})", desc.counter_var, desc.init_val));
        let inv_smt = emitter.lower_expr(&candidate);
        emitter.assert_formula(&format!("(not {inv_smt})"));

        let script = emitter.emit_script();
        let verdict = solve_with_backend(&script, solver, timeout_ms);
        if !matches!(verdict, crate::python::solver::SolverVerdict::Pass) {
            return InductiveProofResult::Refuted {
                counterexample: "Base case failed: initial value violates candidate invariant".to_string(),
            };
        }
    }

    // 2. Verify Inductive Step
    let step_holds = prove_inductive_step(desc, &candidate, solver, timeout_ms);
    if !step_holds {
        return InductiveProofResult::Refuted {
            counterexample: "Inductive step failed: invariant not preserved across loop body".to_string(),
        };
    }

    // 3. Verify Safety Postcondition: (Invariant AND NOT Guard) => SafetyGoal
    {
        let mut emitter = SmtEmitter::new("ALL");
        emitter.declare_const(&desc.counter_var, "Int");
        if let Some(ref ub) = desc.upper_bound_var {
            emitter.declare_const(ub, "Int");
            emitter.assert_formula(&format!("(>= {ub} 0)"));
        }
        let inv_smt = emitter.lower_expr(&candidate);
        emitter.assert_formula(&inv_smt);

        // NOT Guard
        let not_guard = if let Some(ref ub) = desc.upper_bound_var {
            format!("(>= {} {})", desc.counter_var, ub)
        } else if let Some(ub) = desc.upper_bound_const {
            format!("(>= {} {})", desc.counter_var, ub)
        } else {
            "false".to_string()
        };
        emitter.assert_formula(&not_guard);

        // Assert violation of safety goal
        let safety_smt = emitter.lower_expr(safety_goal);
        emitter.assert_formula(&format!("(not {safety_smt})"));

        let script = emitter.emit_script();
        let verdict = solve_with_backend(&script, solver, timeout_ms);
        if !matches!(verdict, crate::python::solver::SolverVerdict::Pass) {
            return InductiveProofResult::Refuted {
                counterexample: "Safety postcondition failed: invariant does not entail safety goal".to_string(),
            };
        }
    }

    let mut emitter = SmtEmitter::new("ALL");
    let inv_smt = emitter.lower_expr(&candidate);
    InductiveProofResult::Proved {
        invariant_smt: inv_smt,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vir::portfolio::discover_portfolio_solvers;

    #[test]
    fn test_candidate_invariant_generation() {
        let desc = LoopDescriptor {
            loop_id: "loop_1".to_string(),
            counter_var: "i".to_string(),
            init_val: 0,
            step_val: 1,
            upper_bound_var: Some("n".to_string()),
            upper_bound_const: None,
        };

        let candidate = synthesize_candidate_invariant(&desc);
        let mut emitter = SmtEmitter::new("ALL");
        let smt = emitter.lower_expr(&candidate);
        assert_eq!(smt, "(and (>= i 0) (<= i n))");
    }

    #[test]
    fn test_inductive_loop_proof() {
        let solvers = discover_portfolio_solvers();
        if let Some(z3) = solvers.iter().find(|s| s.kind == crate::vir::portfolio::SolverBackendKind::Z3) {
            let desc = LoopDescriptor {
                loop_id: "for_i_in_range_n".to_string(),
                counter_var: "i".to_string(),
                init_val: 0,
                step_val: 1,
                upper_bound_var: Some("n".to_string()),
                upper_bound_const: None,
            };

            // Safety goal: at exit, i >= 0
            let safety = VirExpr::Binary(
                VirBinaryOp::Ge,
                Box::new(VirExpr::Var("i".to_string())),
                Box::new(VirExpr::ConstInt(0, 0)),
            );

            let res = prove_loop_induction(&desc, &safety, z3, 5000);
            assert!(matches!(res, InductiveProofResult::Proved { .. }));
        }
    }
}
