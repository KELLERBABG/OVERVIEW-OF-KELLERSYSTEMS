use crate::domain::Interval;
use crate::interpreter::AbstractInterpreter;
use crate::symbolic::SymbolicExpr;
use std::collections::HashMap;

pub struct SafetyCondition {
    pub name: &'static str,
    pub expr: SymbolicExpr,
}

impl SafetyCondition {
    pub fn check(&self, env: &HashMap<String, Interval>) -> Option<bool> {
        AbstractInterpreter::eval_bool(&self.expr, env)
    }
}

pub fn verify_braking_safety(
    speed_interval: Interval,
    mu_interval: Interval,
    obstacle_dist: f64,
) -> bool {
    let mut env = HashMap::new();
    env.insert("velocity".into(), speed_interval);
    env.insert("friction_coeff".into(), mu_interval);

    let stop_dist_expr = crate::physics::stopping_distance_condition(obstacle_dist);
    let result = AbstractInterpreter::eval(&stop_dist_expr, &env);
    result.hi <= 0.0
}
