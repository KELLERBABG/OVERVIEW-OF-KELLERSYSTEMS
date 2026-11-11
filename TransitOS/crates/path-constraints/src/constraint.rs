use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Constraint {
    Eq(String, f64),
    Lt(String, f64),
    Le(String, f64),
    Gt(String, f64),
    Ge(String, f64),
    Within(String, f64, f64),
    And(Box<Constraint>, Box<Constraint>),
    Or(Box<Constraint>, Box<Constraint>),
    Implies(Box<Constraint>, Box<Constraint>),
    Not(Box<Constraint>),
    ForAll {
        var: String,
        range: (f64, f64),
        body: Box<Constraint>,
    },
    TemporalAlways {
        duration: f64,
        body: Box<Constraint>,
    },
}

#[derive(Debug, Clone)]
pub struct ConstraintSet {
    pub constraints: Vec<Constraint>,
}

impl ConstraintSet {
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
        }
    }

    pub fn add(&mut self, c: Constraint) {
        self.constraints.push(c);
    }

    pub fn is_empty(&self) -> bool {
        self.constraints.is_empty()
    }
}

impl Default for ConstraintSet {
    fn default() -> Self {
        Self::new()
    }
}

pub fn evaluate_basic(_name: &str, value: f64, bound: f64, op: &str) -> bool {
    match op {
        "lt" => value < bound,
        "le" => value <= bound,
        "gt" => value > bound,
        "ge" => value >= bound,
        "eq" => (value - bound).abs() < 1e-6,
        _ => false,
    }
}

pub fn evaluate_constraint(c: &Constraint, env: &HashMap<String, f64>) -> bool {
    match c {
        Constraint::Eq(var, val) => env.get(var).is_some_and(|v| (v - val).abs() < 1e-6),
        Constraint::Lt(var, val) => env.get(var).is_some_and(|v| *v < *val),
        Constraint::Le(var, val) => env.get(var).is_some_and(|v| *v <= *val),
        Constraint::Gt(var, val) => env.get(var).is_some_and(|v| *v > *val),
        Constraint::Ge(var, val) => env.get(var).is_some_and(|v| *v >= *val),
        Constraint::Within(var, lo, hi) => env.get(var).is_some_and(|v| *lo <= *v && *v <= *hi),
        Constraint::And(l, r) => evaluate_constraint(l, env) && evaluate_constraint(r, env),
        Constraint::Or(l, r) => evaluate_constraint(l, env) || evaluate_constraint(r, env),
        Constraint::Implies(l, r) => !evaluate_constraint(l, env) || evaluate_constraint(r, env),
        Constraint::Not(c) => !evaluate_constraint(c, env),
        Constraint::ForAll { var, range, body } => {
            let steps = 10;
            let step = (range.1 - range.0) / steps as f64;
            for i in 0..=steps {
                let val = range.0 + i as f64 * step;
                let mut new_env = env.clone();
                new_env.insert(var.clone(), val);
                if !evaluate_constraint(body, &new_env) {
                    return false;
                }
            }
            true
        }
        Constraint::TemporalAlways { duration: _, body } => evaluate_constraint(body, env),
    }
}
