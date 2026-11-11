//! Boolean constraint expression AST and evaluation.

use op_math::Scalar;
use serde::{Deserialize, Serialize};

/// Atomic comparisons over scalar quantities.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Comparison {
    /// `lhs >= rhs`
    Ge,
    /// `lhs <= rhs`
    Le,
    /// `lhs == rhs`
    Eq,
    /// `lhs > rhs`
    Gt,
    /// `lhs < rhs`
    Lt,
}

/// A boolean constraint expression (Z3-friendly subset).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ConstraintExpr {
    /// Constant true.
    #[default]
    True,
    /// Constant false.
    False,
    /// Scalar comparison of two named quantities.
    Compare {
        /// Left-hand quantity name.
        lhs: String,
        /// Comparison operator.
        op: Comparison,
        /// Right-hand quantity name.
        rhs: String,
    },
    /// Conjunction.
    And(Vec<ConstraintExpr>),
    /// Disjunction.
    Or(Vec<ConstraintExpr>),
    /// Negation.
    Not(Box<ConstraintExpr>),
}

/// Binary boolean operator used in conjunction/disjunction contexts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOp {
    /// Logical AND.
    And,
    /// Logical OR.
    Or,
}

/// A named scalar value used during evaluation.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Evaluation {
    /// Quantity name.
    pub name: String,
    /// Scalar value.
    pub value: Scalar,
}

/// Evaluate a constraint expression against a set of named scalar values.
pub fn eval(expr: &ConstraintExpr, env: &[Evaluation]) -> bool {
    let value = |name: &String| -> Option<Scalar> {
        env.iter().find(|e| &e.name == name).map(|e| e.value)
    };
    match expr {
        ConstraintExpr::True => true,
        ConstraintExpr::False => false,
        ConstraintExpr::Compare { lhs, op, rhs } => {
            let (Some(l), Some(r)) = (value(lhs), value(rhs)) else {
                return false;
            };
            match op {
                Comparison::Ge => l >= r,
                Comparison::Le => l <= r,
                Comparison::Eq => l == r,
                Comparison::Gt => l > r,
                Comparison::Lt => l < r,
            }
        }
        ConstraintExpr::And(children) => children.iter().all(|c| eval(c, env)),
        ConstraintExpr::Or(children) => children.iter().any(|c| eval(c, env)),
        ConstraintExpr::Not(child) => !eval(child, env),
    }
}

/// A constraint formula plus its declared free quantities and concrete evaluations.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ConstraintSet {
    /// The boolean formula describing the action path.
    pub formula: ConstraintExpr,
    /// Names of quantities appearing in the formula (for SMT variable mapping).
    pub quantities: Vec<String>,
    /// Concrete evaluated values of path quantities generated from the actual motion.
    pub evaluations: Vec<Evaluation>,
}

impl ConstraintSet {
    /// Create a new constraint set.
    pub fn new(formula: ConstraintExpr, quantities: Vec<String>) -> Self {
        Self {
            formula,
            quantities,
            evaluations: Vec::new(),
        }
    }

    /// Create a new constraint set with concrete evaluations.
    pub fn with_evaluations(
        formula: ConstraintExpr,
        quantities: Vec<String>,
        evaluations: Vec<Evaluation>,
    ) -> Self {
        Self {
            formula,
            quantities,
            evaluations,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env() -> Vec<Evaluation> {
        vec![
            Evaluation { name: "depth".into(), value: 10.0 },
            Evaluation { name: "limit".into(), value: 15.0 },
            Evaluation { name: "force".into(), value: 5.0 },
        ]
    }

    #[test]
    fn evaluates_simple_compare() {
        let e = ConstraintExpr::Compare {
            lhs: "depth".into(),
            op: Comparison::Le,
            rhs: "limit".into(),
        };
        assert!(eval(&e, &env()));
    }

    #[test]
    fn evaluates_and_or_not() {
        let e = ConstraintExpr::And(vec![
            ConstraintExpr::Compare {
                lhs: "depth".into(),
                op: Comparison::Le,
                rhs: "limit".into(),
            },
            ConstraintExpr::Not(Box::new(ConstraintExpr::Compare {
                lhs: "force".into(),
                op: Comparison::Gt,
                rhs: "limit".into(),
            })),
        ]);
        assert!(eval(&e, &env()));
    }
}