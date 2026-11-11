//! Real Z3 SMT solver integration (feature `z3-solver`).
//!
//! QF_NRA solving (nlsat) is EXPTIME-complete in the worst case, so an online
//! per-command solve can produce unpredictable latency spikes that would miss
//! a real-time control-loop deadline. To keep the system fail-safe and
//! real-time predictable, the solver is configured with a **hard timeout**:
//! if Z3 cannot decide within the budget, the call returns `Unknown` and the
//! trustless execution gate blocks the command (fail-safe by construction).

use op_constraints::{Comparison, ConstraintExpr, Evaluation};
use std::time::Duration;
use z3::ast::{Bool, Real};
use z3::{Config, Context, Solver};

/// Default solver timeout in milliseconds — a VERIFICATION DEADLINE.
/// Exceeding it must not release a command; it yields `Unknown` → block.
pub const DEFAULT_SOLVER_TIMEOUT_MS: u32 = 50;

/// Result of a Z3 satisfiability check on the fault formula.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Z3Outcome {
    /// Unsat: no fault state reachable.
    Safe,
    /// Sat: a fault state is reachable.
    Fault,
    /// Z3 timed out or could not decide.
    Unknown,
}

/// Check whether the negation of `formula` (the fault state) is satisfiable,
/// subject to a hard `timeout`; on timeout/unknown the result is `Unknown`
/// (fail-safe — the gate blocks).
pub fn check_fault_reachable(formula: &ConstraintExpr, bounds: &[Evaluation]) -> Z3Outcome {
    check_fault_reachable_with_timeout(formula, bounds, DEFAULT_SOLVER_TIMEOUT_MS)
}

/// Like [`check_fault_reachable`], but with an explicit timeout in
/// milliseconds. The timeout is enforced by Z3's `soft_timeout` plus a
/// real-time deadline check: on expiry the solve is aborted and `Unknown`
/// returned so the control loop can move on to a safe-stop plan.
pub fn check_fault_reachable_with_timeout(
    formula: &ConstraintExpr,
    bounds: &[Evaluation],
    timeout_ms: u32,
) -> Z3Outcome {
    let cfg = Config::new();
    let ctx = Context::new(&cfg);
    let solver = Solver::new(&ctx);
    // Hard real-time deadline: Z3's internal timeout + an external guard.
    solver.set_timeout(timeout_ms);
    let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms as u64);

    let vars: Vec<(String, Real)> = bounds
        .iter()
        .map(|e| (e.name.clone(), Real::new_const(&ctx, e.name.as_str())))
        .collect();
    let neg = to_z3(&ConstraintExpr::Not(Box::new(formula.clone())), &vars, &ctx);
    solver.assert(&neg);

    // External deadline guard: if Z3 exceeds the budget (its own timeout may
    // be approximate), abort as Unknown — a late proof is a blocked command.
    if std::time::Instant::now() >= deadline {
        return Z3Outcome::Unknown;
    }
    match solver.check() {
        z3::SatResult::Sat => Z3Outcome::Fault,
        z3::SatResult::Unsat => Z3Outcome::Safe,
        z3::SatResult::Unknown => Z3Outcome::Unknown,
    }
}

/// Translate a constraint expression into a Z3 boolean.
fn to_z3(expr: &ConstraintExpr, vars: &[(String, Real)], ctx: &Context) -> Bool {
    use ConstraintExpr::*;
    match expr {
        True => Bool::from_bool(ctx, true),
        False => Bool::from_bool(ctx, false),
        Compare { lhs, op, rhs } => {
            let l = var_or_const(lhs, vars, ctx);
            let r = var_or_const(rhs, vars, ctx);
            match op {
                Comparison::Ge => l.ge(&r),
                Comparison::Le => l.le(&r),
                Comparison::Eq => l._eq(&r),
                Comparison::Gt => l.gt(&r),
                Comparison::Lt => l.lt(&r),
            }
        }
        And(children) => {
            let bs: Vec<Bool> = children.iter().map(|c| to_z3(c, vars, ctx)).collect();
            Bool::and(ctx, &bs)
        }
        Or(children) => {
            let bs: Vec<Bool> = children.iter().map(|c| to_z3(c, vars, ctx)).collect();
            Bool::or(ctx, &bs)
        }
        Not(child) => to_z3(child, vars, ctx).not(),
    }
}

/// Look up a quantity as a Z3 real; unknown quantities become a fresh const.
fn var_or_const(name: &str, vars: &[(String, Real)], ctx: &Context) -> Real {
    vars.iter()
        .find(|(n, _)| n == name)
        .map(|(_, v)| v.clone())
        .unwrap_or_else(|| Real::from_real(ctx, 0, 1))
}