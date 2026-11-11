//! Path constraint routing types.
//!
//! Defines the constraint system for route verification:
//! boolean expressions over real-time variables (capacity,
//! fuel, border times) that are fed to the SMT solver (Z3)
//! for path feasibility checking.

use serde::{Deserialize, Serialize};

/// A variable that can appear in routing constraints.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ConstraintVariable {
    /// Available capacity (e.g., `capacity >= 100.0`).
    Capacity,
    /// Fuel efficiency or cost per km.
    FuelEfficiency,
    /// Estimated border crossing time in minutes.
    BorderTime,
    /// Temperature in Celsius (for perishability).
    Temperature,
    /// Custom named variable.
    Custom(String),
}

/// A boolean expression for path constraints.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConstraintExpr {
    /// A literal boolean value.
    Literal(bool),
    /// A comparison: variable `op` value.
    /// Example: `capacity >= 100.0`
    Comparison {
        variable: ConstraintVariable,
        op: ComparisonOp,
        value: f64,
    },
    /// Logical AND of two sub-expressions.
    And(Box<ConstraintExpr>, Box<ConstraintExpr>),
    /// Logical OR of two sub-expressions.
    Or(Box<ConstraintExpr>, Box<ConstraintExpr>),
    /// Logical NOT of a sub-expression.
    Not(Box<ConstraintExpr>),
}

/// Comparison operators for constraints.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComparisonOp {
    Gt,
    Ge,
    Lt,
    Le,
    Eq,
}

/// A named constraint associated with a graph edge.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EdgeConstraint {
    /// The edge ID this constraint applies to.
    pub edge_id: uuid::Uuid,
    /// Human-readable name.
    pub name: String,
    /// The boolean expression to satisfy.
    pub expression: ConstraintExpr,
}

impl ConstraintExpr {
    /// Create a constant true constraint.
    pub fn always_true() -> Self {
        Self::Literal(true)
    }

    /// Create a constant false constraint.
    pub fn always_false() -> Self {
        Self::Literal(false)
    }

    /// Create a capacity constraint.
    pub fn capacity(op: ComparisonOp, value: f64) -> Self {
        Self::Comparison {
            variable: ConstraintVariable::Capacity,
            op,
            value,
        }
    }

    /// Evaluate the expression against a set of variable values.
    /// Returns `None` if a required variable is missing.
    pub fn evaluate(&self, vars: &std::collections::HashMap<ConstraintVariable, f64>) -> Option<bool> {
        match self {
            Self::Literal(b) => Some(*b),
            Self::Comparison { variable, op, value } => {
                let actual = vars.get(variable)?;
                Some(match op {
                    ComparisonOp::Gt => *actual > *value,
                    ComparisonOp::Ge => *actual >= *value,
                    ComparisonOp::Lt => *actual < *value,
                    ComparisonOp::Le => *actual <= *value,
                    ComparisonOp::Eq => (*actual - *value).abs() < f64::EPSILON,
                })
            }
            Self::And(a, b) => Some(a.evaluate(vars)? && b.evaluate(vars)?),
            Self::Or(a, b) => Some(a.evaluate(vars)? || b.evaluate(vars)?),
            Self::Not(a) => Some(!a.evaluate(vars)?),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_literal_constraint() {
        assert_eq!(ConstraintExpr::always_true().evaluate(&HashMap::new()), Some(true));
        assert_eq!(ConstraintExpr::always_false().evaluate(&HashMap::new()), Some(false));
    }

    #[test]
    fn test_capacity_constraint() {
        let expr = ConstraintExpr::capacity(ComparisonOp::Ge, 100.0);
        let mut vars = HashMap::new();
        vars.insert(ConstraintVariable::Capacity, 150.0);
        assert_eq!(expr.evaluate(&vars), Some(true));

        vars.insert(ConstraintVariable::Capacity, 50.0);
        assert_eq!(expr.evaluate(&vars), Some(false));
    }

    #[test]
    fn test_missing_variable_returns_none() {
        let expr = ConstraintExpr::capacity(ComparisonOp::Ge, 100.0);
        assert_eq!(expr.evaluate(&HashMap::new()), None);
    }

    #[test]
    fn test_composite_expression() {
        let expr = ConstraintExpr::And(
            Box::new(ConstraintExpr::capacity(ComparisonOp::Ge, 100.0)),
            Box::new(ConstraintExpr::Comparison {
                variable: ConstraintVariable::Temperature,
                op: ComparisonOp::Le,
                value: 8.0,
            }),
        );
        let mut vars = HashMap::new();
        vars.insert(ConstraintVariable::Capacity, 150.0);
        vars.insert(ConstraintVariable::Temperature, 5.0);
        assert_eq!(expr.evaluate(&vars), Some(true));

        vars.insert(ConstraintVariable::Temperature, 12.0);
        assert_eq!(expr.evaluate(&vars), Some(false));
    }

    #[test]
    fn test_not_expression() {
        let expr = ConstraintExpr::Not(Box::new(ConstraintExpr::always_true()));
        assert_eq!(expr.evaluate(&HashMap::new()), Some(false));
    }
}