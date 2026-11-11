//! Constraint conversion and management utilities.

use sca_types::{ConstraintExpr, ConstraintVariable, EdgeConstraint};

/// Build a constraint that the temperature never exceeds a threshold.
pub fn temperature_constraint(max_temp: f64) -> ConstraintExpr {
    ConstraintExpr::Comparison {
        variable: ConstraintVariable::Temperature,
        op: sca_types::ComparisonOp::Le,
        value: max_temp,
    }
}

/// Build a constraint that capacity is within bounds.
pub fn capacity_constraint(min_capacity: f64) -> ConstraintExpr {
    ConstraintExpr::capacity(sca_types::ComparisonOp::Ge, min_capacity)
}

/// Build a constraint that border crossing time is below a limit.
pub fn border_time_constraint(max_minutes: f64) -> ConstraintExpr {
    ConstraintExpr::Comparison {
        variable: ConstraintVariable::BorderTime,
        op: sca_types::ComparisonOp::Le,
        value: max_minutes,
    }
}

/// Build a constraint that fuel efficiency is above a minimum.
pub fn fuel_efficiency_constraint(min_efficiency: f64) -> ConstraintExpr {
    ConstraintExpr::Comparison {
        variable: ConstraintVariable::FuelEfficiency,
        op: sca_types::ComparisonOp::Ge,
        value: min_efficiency,
    }
}

/// Merge multiple edge constraints into a single AND expression.
pub fn merge_constraints(constraints: &[EdgeConstraint]) -> ConstraintExpr {
    let mut iter = constraints.iter().map(|c| c.expression.clone());
    let first = match iter.next() {
        Some(e) => e,
        None => return ConstraintExpr::always_true(),
    };

    iter.fold(first, |acc, expr| ConstraintExpr::And(Box::new(acc), Box::new(expr)))
}

/// Extract variable names from an expression for display/analysis.
pub fn variable_names(expr: &ConstraintExpr) -> Vec<ConstraintVariable> {
    let mut vars = Vec::new();
    collect_vars(expr, &mut vars);
    vars
}

fn collect_vars(expr: &ConstraintExpr, vars: &mut Vec<ConstraintVariable>) {
    match expr {
        ConstraintExpr::Literal(_) => {}
        ConstraintExpr::Comparison { variable, .. } => {
            vars.push(variable.clone());
        }
        ConstraintExpr::And(a, b) | ConstraintExpr::Or(a, b) => {
            collect_vars(a, vars);
            collect_vars(b, vars);
        }
        ConstraintExpr::Not(a) => collect_vars(a, vars),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temperature_constraint() {
        let expr = temperature_constraint(8.0);
        // Should be: temperature <= 8.0
        let vars = variable_names(&expr);
        assert!(vars.contains(&ConstraintVariable::Temperature));
    }

    #[test]
    fn test_merge_constraints_empty() {
        let merged = merge_constraints(&[]);
        assert_eq!(merged, ConstraintExpr::always_true());
    }

    #[test]
    fn test_merge_constraints_two() {
        let c1 = EdgeConstraint {
            edge_id: uuid::Uuid::new_v4(),
            name: "temp".into(),
            expression: temperature_constraint(8.0),
        };
        let c2 = EdgeConstraint {
            edge_id: uuid::Uuid::new_v4(),
            name: "capacity".into(),
            expression: capacity_constraint(100.0),
        };
        let merged = merge_constraints(&[c1, c2]);
        // Should be an AND of the two
        let vars = variable_names(&merged);
        assert!(vars.contains(&ConstraintVariable::Temperature));
        assert!(vars.contains(&ConstraintVariable::Capacity));
    }
}
