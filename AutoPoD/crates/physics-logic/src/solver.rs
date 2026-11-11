use pod_core::event::Verdict;
use pod_core::policy::{CombinationLogic, DamagePolicy, ThresholdParameter};
use pod_core::sensor::SensorReading;
use serde::{Deserialize, Serialize};

use crate::conditions::{evaluate_combination, evaluate_policy, DamageCondition};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub policy_name: String,
    pub verdict: Verdict,
    pub conditions: Vec<DamageCondition>,
    pub smt_satisfiable: bool,
}

#[derive(Debug, Clone)]
pub enum SmtExpr {
    Bool(bool),
    /// A boolean variable (for SAT)
    Var(String),
    Not(Box<SmtExpr>),
    And(Vec<SmtExpr>),
    Or(Vec<SmtExpr>),
    Implies(Box<SmtExpr>, Box<SmtExpr>),
    /// Theory atom: numeric variable > threshold
    GreaterThan(String, f64),
    LessThan(String, f64),
    Between(String, f64, f64),
}

#[derive(Debug, Clone)]
pub struct SmtSolver {
    assertions: Vec<SmtExpr>,
    /// Boolean variable assignments (for SAT reasoning)
    bool_assignments: Vec<(String, bool)>,
    /// Numeric variable values (for theory reasoning)
    numeric_values: Vec<(String, f64)>,
}

impl SmtSolver {
    pub fn new() -> Self {
        SmtSolver {
            assertions: Vec::new(),
            bool_assignments: Vec::new(),
            numeric_values: Vec::new(),
        }
    }

    pub fn assert(&mut self, expr: SmtExpr) {
        self.assertions.push(expr);
    }

    /// Assign a boolean variable
    pub fn assign_bool(&mut self, var: &str, val: bool) {
        self.bool_assignments.push((var.to_string(), val));
    }

    /// Assign a numeric value (for theory variables)
    pub fn assign_numeric(&mut self, var: &str, val: f64) {
        self.numeric_values.push((var.to_string(), val));
    }

    pub fn check(&self) -> bool {
        let formula = SmtExpr::And(self.assertions.clone());
        let reduced = self.reduce(&formula);
        self.evaluate(&reduced)
    }

    fn reduce(&self, expr: &SmtExpr) -> SmtExpr {
        match expr {
            SmtExpr::Var(name) => {
                let val = self
                    .bool_assignments
                    .iter()
                    .find(|(n, _)| n == name)
                    .map(|(_, v)| *v)
                    .unwrap_or(false);
                SmtExpr::Bool(val)
            }
            SmtExpr::Not(inner) => {
                let reduced = self.reduce(inner);
                match &reduced {
                    SmtExpr::Bool(b) => SmtExpr::Bool(!b),
                    _ => SmtExpr::Not(Box::new(reduced)),
                }
            }
            SmtExpr::And(children) => {
                let reduced: Vec<SmtExpr> = children.iter().map(|c| self.reduce(c)).collect();
                let has_false = reduced.iter().any(|c| matches!(c, SmtExpr::Bool(false)));
                if has_false {
                    return SmtExpr::Bool(false);
                }
                let all_bool = reduced.iter().all(|c| matches!(c, SmtExpr::Bool(_)));
                if all_bool {
                    return SmtExpr::Bool(true);
                }
                SmtExpr::And(reduced)
            }
            SmtExpr::Or(children) => {
                let reduced: Vec<SmtExpr> = children.iter().map(|c| self.reduce(c)).collect();
                let has_true = reduced.iter().any(|c| matches!(c, SmtExpr::Bool(true)));
                if has_true {
                    return SmtExpr::Bool(true);
                }
                let all_bool = reduced.iter().all(|c| matches!(c, SmtExpr::Bool(_)));
                if all_bool {
                    return SmtExpr::Bool(false);
                }
                SmtExpr::Or(reduced)
            }
            SmtExpr::Implies(a, b) => {
                let ra = self.reduce(a);
                let rb = self.reduce(b);
                match (&ra, &rb) {
                    (SmtExpr::Bool(false), _) => SmtExpr::Bool(true),
                    (_, SmtExpr::Bool(true)) => SmtExpr::Bool(true),
                    (SmtExpr::Bool(true), SmtExpr::Bool(b)) => SmtExpr::Bool(*b),
                    _ => SmtExpr::Implies(Box::new(ra), Box::new(rb)),
                }
            }
            SmtExpr::GreaterThan(var, threshold) => {
                let val = self.resolve_numeric(var);
                SmtExpr::Bool(val > *threshold)
            }
            SmtExpr::LessThan(var, threshold) => {
                let val = self.resolve_numeric(var);
                SmtExpr::Bool(val < *threshold)
            }
            SmtExpr::Between(var, lo, hi) => {
                let val = self.resolve_numeric(var);
                SmtExpr::Bool(*lo <= val && val <= *hi)
            }
            SmtExpr::Bool(_) => expr.clone(),
        }
    }

    fn resolve_numeric(&self, name: &str) -> f64 {
        self.numeric_values
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| *v)
            .unwrap_or(0.0)
    }

    fn evaluate(&self, expr: &SmtExpr) -> bool {
        match expr {
            SmtExpr::Bool(b) => *b,
            SmtExpr::Not(inner) => !self.evaluate(inner),
            SmtExpr::And(children) => children.iter().all(|c| self.evaluate(c)),
            SmtExpr::Or(children) => children.iter().any(|c| self.evaluate(c)),
            SmtExpr::Implies(a, b) => !self.evaluate(a) || self.evaluate(b),
            SmtExpr::Var(name) => self
                .bool_assignments
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| *v)
                .unwrap_or(false),
            SmtExpr::GreaterThan(var, threshold) => self.resolve_numeric(var) > *threshold,
            SmtExpr::LessThan(var, threshold) => self.resolve_numeric(var) < *threshold,
            SmtExpr::Between(var, lo, hi) => {
                let v = self.resolve_numeric(var);
                *lo <= v && v <= *hi
            }
        }
    }
}

impl Default for SmtSolver {
    fn default() -> Self {
        Self::new()
    }
}

pub fn verify_damage(policy: &DamagePolicy, readings: &[SensorReading]) -> VerificationResult {
    let conditions = evaluate_policy(policy, readings);
    let logic_result = evaluate_combination(&conditions, &policy.combination);
    let smt_result = smt_check(policy, readings);
    let sat = logic_result || smt_result;

    VerificationResult {
        policy_name: policy.name.clone(),
        verdict: if sat { Verdict::Approved } else { Verdict::Rejected },
        conditions,
        smt_satisfiable: smt_result,
    }
}

pub fn smt_check(policy: &DamagePolicy, readings: &[SensorReading]) -> bool {
    let mut solver = SmtSolver::new();
    let mut threshold_formulas: Vec<SmtExpr> = Vec::new();

    for threshold in &policy.thresholds {
        let matching: Vec<&SensorReading> = readings
            .iter()
            .filter(|r| r.sensor_type == threshold.sensor_type)
            .collect();

        if matching.is_empty() {
            continue;
        }

        let mut reading_formulas = Vec::new();

        for reading in &matching {
            let var_name = format!("val_{}", reading.timestamp);
            solver.assign_numeric(&var_name, reading.value);

            let expr = match &threshold.parameter {
                ThresholdParameter::GreaterThan(min) => {
                    SmtExpr::GreaterThan(var_name, *min)
                }
                ThresholdParameter::LessThan(max) => SmtExpr::LessThan(var_name, *max),
                ThresholdParameter::Between(low, high) => {
                    SmtExpr::Between(var_name, *low, *high)
                }
                ThresholdParameter::RateOfChange { .. } => continue,
                ThresholdParameter::StructuralShift { .. } => continue,
            };
            reading_formulas.push(expr);
        }

        if reading_formulas.is_empty() {
            continue;
        }

        let or_expr = SmtExpr::Or(reading_formulas);
        threshold_formulas.push(or_expr);
    }

    if threshold_formulas.is_empty() {
        return false;
    }

    let combination_expr = match &policy.combination {
        CombinationLogic::All => SmtExpr::And(threshold_formulas),
        CombinationLogic::Any => SmtExpr::Or(threshold_formulas),
        CombinationLogic::AtLeast(n) => {
            if *n >= threshold_formulas.len() {
                SmtExpr::And(threshold_formulas)
            } else {
                SmtExpr::Or(threshold_formulas)
            }
        }
    };

    solver.assert(combination_expr);
    solver.check()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pod_core::policy::{DamagePolicy, DamageThreshold};
    use pod_core::sensor::SensorType;
    use uuid::Uuid;

    fn reading(val: f64, ts: i64) -> SensorReading {
        SensorReading {
            sensor_id: Uuid::nil(),
            sensor_type: SensorType::Accelerometer,
            timestamp: ts,
            value: val,
            unit: "g".into(),
        }
    }

    #[test]
    fn test_smt_solver_simple_and() {
        let mut solver = SmtSolver::new();
        solver.assign_bool("a", true);
        solver.assign_bool("b", true);
        solver.assert(SmtExpr::And(vec![SmtExpr::Var("a".into()), SmtExpr::Var("b".into())]));
        assert!(solver.check());
    }

    #[test]
    fn test_smt_solver_and_fails() {
        let mut solver = SmtSolver::new();
        solver.assign_bool("a", true);
        solver.assign_bool("b", false);
        solver.assert(SmtExpr::And(vec![SmtExpr::Var("a".into()), SmtExpr::Var("b".into())]));
        assert!(!solver.check());
    }

    #[test]
    fn test_smt_solver_or() {
        let mut solver = SmtSolver::new();
        solver.assign_bool("a", false);
        solver.assign_bool("b", true);
        solver.assert(SmtExpr::Or(vec![SmtExpr::Var("a".into()), SmtExpr::Var("b".into())]));
        assert!(solver.check());
    }

    #[test]
    fn test_smt_solver_not() {
        let mut solver = SmtSolver::new();
        solver.assign_bool("a", true);
        solver.assert(SmtExpr::Not(Box::new(SmtExpr::Var("a".into()))));
        assert!(!solver.check());
    }

    #[test]
    fn test_smt_solver_implies() {
        let mut solver = SmtSolver::new();
        solver.assign_bool("a", true);
        solver.assign_bool("b", true);
        solver.assert(SmtExpr::Implies(
            Box::new(SmtExpr::Var("a".into())),
            Box::new(SmtExpr::Var("b".into())),
        ));
        assert!(solver.check());
    }

    #[test]
    fn test_smt_theory_greater_than() {
        let mut solver = SmtSolver::new();
        solver.assign_numeric("x", 12.0);
        let expr = SmtExpr::GreaterThan("x".into(), 8.0);
        solver.assert(expr);
        assert!(solver.check());
    }

    #[test]
    fn test_smt_theory_less_than() {
        let mut solver = SmtSolver::new();
        solver.assign_numeric("x", 3.0);
        solver.assert(SmtExpr::LessThan("x".into(), 5.0));
        assert!(solver.check());
    }

    #[test]
    fn test_smt_theory_between() {
        let mut solver = SmtSolver::new();
        solver.assign_numeric("x", 5.0);
        solver.assert(SmtExpr::Between("x".into(), 3.0, 7.0));
        assert!(solver.check());
    }

    #[test]
    fn test_smt_damage_check_frontal() {
        let policy = DamagePolicy {
            name: "collision".into(),
            thresholds: vec![DamageThreshold {
                name: "impact_g".into(),
                sensor_type: SensorType::Accelerometer,
                parameter: ThresholdParameter::GreaterThan(8.0),
                duration_ms: None,
            }],
            combination: CombinationLogic::Any,
        };
        let readings = vec![reading(12.0, 1000)];
        let result = smt_check(&policy, &readings);
        assert!(result);
    }

    #[test]
    fn test_smt_damage_check_rejects_normal() {
        let policy = DamagePolicy {
            name: "collision".into(),
            thresholds: vec![DamageThreshold {
                name: "impact_g".into(),
                sensor_type: SensorType::Accelerometer,
                parameter: ThresholdParameter::GreaterThan(8.0),
                duration_ms: None,
            }],
            combination: CombinationLogic::Any,
        };
        let readings = vec![reading(0.5, 1000)];
        let result = smt_check(&policy, &readings);
        assert!(!result);
    }

    #[test]
    fn test_between_threshold_smt() {
        let policy = DamagePolicy {
            name: "range".into(),
            thresholds: vec![DamageThreshold {
                name: "g_range".into(),
                sensor_type: SensorType::Accelerometer,
                parameter: ThresholdParameter::Between(3.0, 7.0),
                duration_ms: None,
            }],
            combination: CombinationLogic::Any,
        };
        let readings = vec![reading(5.0, 1000)];
        assert!(smt_check(&policy, &readings));
    }

    #[test]
    fn test_verify_damage_integration() {
        let policy = DamagePolicy {
            name: "full_test".into(),
            thresholds: vec![
                DamageThreshold {
                    name: "g_force".into(),
                    sensor_type: SensorType::Accelerometer,
                    parameter: ThresholdParameter::GreaterThan(8.0),
                    duration_ms: None,
                },
            ],
            combination: CombinationLogic::Any,
        };
        let readings = vec![reading(12.0, 1000)];
        let result = verify_damage(&policy, &readings);
        assert_eq!(result.verdict, Verdict::Approved);
        assert!(result.smt_satisfiable);
    }
}
