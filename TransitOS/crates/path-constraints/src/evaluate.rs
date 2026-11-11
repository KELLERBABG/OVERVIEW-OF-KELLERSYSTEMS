use crate::constraint::Constraint;
use crate::constraint::ConstraintSet;
use std::collections::HashMap;

pub fn check_all<'a>(
    constraints: &'a ConstraintSet,
    env: &HashMap<String, f64>,
) -> Vec<(&'a Constraint, bool)> {
    constraints
        .constraints
        .iter()
        .map(|c| {
            let result = crate::constraint::evaluate_constraint(c, env);
            (c, result)
        })
        .collect()
}

pub fn all_pass(constraints: &ConstraintSet, env: &HashMap<String, f64>) -> bool {
    constraints
        .constraints
        .iter()
        .all(|c| crate::constraint::evaluate_constraint(c, env))
}
