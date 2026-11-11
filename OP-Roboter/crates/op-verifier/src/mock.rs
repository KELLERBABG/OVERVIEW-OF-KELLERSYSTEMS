//! Built-in mock solver that numerically evaluates the constraint formula.

use op_constraints::{ConstraintExpr, Evaluation};
use serde::{Deserialize, Serialize};

/// Solver verdict on a constraint formula.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verdict {
    /// Formula evaluates to true for the provided environment.
    Satisfied,
    /// Formula evaluates to false for the provided environment.
    Unsatisfied,
    /// Solver could not decide (missing quantities, etc.).
    Unknown,
}

/// Error raised when the mock solver cannot evaluate a formula.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EvaluationError {
    /// A quantity referenced by the formula is missing from the environment.
    #[error("missing quantity in evaluation environment: {0}")]
    MissingQuantity(String),
}

/// Mock solver: evaluates the boolean formula against a concrete numeric
/// environment. Serves as a stand-in for Z3 when the `z3-solver` feature is
/// disabled.
#[derive(Debug, Clone, Default)]
pub struct MockSolver;

impl MockSolver {
    /// Evaluate the formula and return a verdict.
    pub fn check_formula(&self, formula: &ConstraintExpr, env: &[Evaluation]) -> Verdict {
        match eval_formula(formula, env) {
            Ok(true) => Verdict::Satisfied,
            Ok(false) => Verdict::Unsatisfied,
            Err(_) => Verdict::Unknown,
        }
    }
}

/// Recursive numerical evaluation of a constraint formula.
fn eval_formula(expr: &ConstraintExpr, env: &[Evaluation]) -> Result<bool, EvaluationError> {
    let value = |name: &String| -> Result<op_math::Scalar, EvaluationError> {
        env.iter()
            .find(|e| &e.name == name)
            .map(|e| e.value)
            .ok_or_else(|| EvaluationError::MissingQuantity(name.clone()))
    };
    match expr {
        ConstraintExpr::True => Ok(true),
        ConstraintExpr::False => Ok(false),
        ConstraintExpr::Compare { lhs, op, rhs } => {
            let l = value(lhs)?;
            let r = value(rhs)?;
            Ok(match op {
                op_constraints::Comparison::Ge => l >= r,
                op_constraints::Comparison::Le => l <= r,
                op_constraints::Comparison::Eq => l == r,
                op_constraints::Comparison::Gt => l > r,
                op_constraints::Comparison::Lt => l < r,
            })
        }
        ConstraintExpr::And(children) => {
            for c in children {
                if !eval_formula(c, env)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        ConstraintExpr::Or(children) => {
            for c in children {
                if eval_formula(c, env)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        ConstraintExpr::Not(child) => Ok(!eval_formula(child, env)?),
    }
}