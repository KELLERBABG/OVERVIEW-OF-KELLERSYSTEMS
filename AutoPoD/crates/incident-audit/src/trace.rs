use pod_core::policy::{CombinationLogic, DamagePolicy, ThresholdParameter};
use pod_core::sensor::{SensorReading, SensorType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintEvent {
    pub timestamp: i64,
    pub threshold_name: String,
    pub sensor_type: SensorType,
    pub reading_value: f64,
    pub satisfied: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintTrace {
    pub incident_id: String,
    pub policy_name: String,
    pub events: Vec<ConstraintEvent>,
    pub overall_combination: CombinationLogic,
    pub overall_verdict: bool,
    pub boolean_expression: String,
}

impl ConstraintTrace {
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn unique_thresholds(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .events
            .iter()
            .map(|e| e.threshold_name.clone())
            .collect();
        names.sort();
        names.dedup();
        names
    }

    pub fn satisfied_thresholds(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .events
            .iter()
            .filter(|e| e.satisfied)
            .map(|e| e.threshold_name.clone())
            .collect();
        names.sort();
        names.dedup();
        names
    }

    pub fn window_of_interest(&self) -> Option<(i64, i64)> {
        let satisfied: Vec<&ConstraintEvent> = self.events.iter().filter(|e| e.satisfied).collect();
        if satisfied.is_empty() {
            return None;
        }
        let start = satisfied.iter().map(|e| e.timestamp).min().unwrap();
        let end = satisfied.iter().map(|e| e.timestamp).max().unwrap();
        Some((start, end))
    }
}

pub fn build_trace(
    policy: &DamagePolicy,
    readings: &[SensorReading],
) -> ConstraintTrace {
    let mut events = Vec::new();

    for reading in readings {
        for threshold in &policy.thresholds {
            if reading.sensor_type != threshold.sensor_type {
                continue;
            }

            let (satisfied, detail) = check_reading_against_threshold(threshold, reading);
            events.push(ConstraintEvent {
                timestamp: reading.timestamp,
                threshold_name: threshold.name.clone(),
                sensor_type: threshold.sensor_type.clone(),
                reading_value: reading.value,
                satisfied,
                detail,
            });
        }
    }

    let final_conditions = physics_logic::conditions::evaluate_policy(policy, readings);
    let overall_verdict =
        physics_logic::conditions::evaluate_combination(&final_conditions, &policy.combination);

    let expr = build_boolean_expression(&events, &policy.combination);

    ConstraintTrace {
        incident_id: uuid::Uuid::new_v4().to_string(),
        policy_name: policy.name.clone(),
        events,
        overall_combination: policy.combination.clone(),
        overall_verdict,
        boolean_expression: expr,
    }
}

fn check_reading_against_threshold(
    threshold: &pod_core::policy::DamageThreshold,
    reading: &SensorReading,
) -> (bool, String) {
    match &threshold.parameter {
        ThresholdParameter::GreaterThan(min) => {
            let pass = reading.value > *min;
            (pass, format!("{} > {} → {}", reading.value, min, pass))
        }
        ThresholdParameter::LessThan(max) => {
            let pass = reading.value < *max;
            (pass, format!("{} < {} → {}", reading.value, max, pass))
        }
        ThresholdParameter::Between(low, high) => {
            let pass = *low <= reading.value && reading.value <= *high;
            (pass, format!("{} ∈ [{}, {}] → {}", reading.value, low, high, pass))
        }
        ThresholdParameter::RateOfChange { .. } | ThresholdParameter::StructuralShift { .. } => {
            let pass = physics_logic::conditions::check_threshold(threshold, &[reading]);
            (pass, format!("special: {}", pass))
        }
    }
}

fn build_boolean_expression(events: &[ConstraintEvent], logic: &CombinationLogic) -> String {
    let satisfied_names: Vec<&str> = events
        .iter()
        .filter(|e| e.satisfied)
        .map(|e| e.threshold_name.as_str())
        .collect();

    if satisfied_names.is_empty() {
        return "false".into();
    }

    let mut unique: Vec<&str> = satisfied_names.clone();
    unique.sort();
    unique.dedup();

    match logic {
        CombinationLogic::All => format!("({})", unique.join(" ∧ ")),
        CombinationLogic::Any => format!("({})", unique.join(" ∨ ")),
        CombinationLogic::AtLeast(n) => format!("at_least({}, {})", n, unique.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pod_core::policy::{DamagePolicy, DamageThreshold, CombinationLogic, ThresholdParameter};
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

    fn collision_policy() -> DamagePolicy {
        DamagePolicy {
            name: "collision".into(),
            thresholds: vec![
                DamageThreshold {
                    name: "impact_g".into(),
                    sensor_type: SensorType::Accelerometer,
                    parameter: ThresholdParameter::GreaterThan(8.0),
                    duration_ms: None,
                },
            ],
            combination: CombinationLogic::Any,
        }
    }

    #[test]
    fn test_trace_builds_events() {
        let policy = collision_policy();
        let readings = vec![reading(0.5, 0), reading(12.0, 1000), reading(0.3, 2000)];
        let trace = build_trace(&policy, &readings);
        assert_eq!(trace.events.len(), 3);
    }

    #[test]
    fn test_trace_records_satisfied() {
        let policy = collision_policy();
        let readings = vec![reading(0.5, 0), reading(12.0, 1000), reading(0.3, 2000)];
        let trace = build_trace(&policy, &readings);
        let satisfied: Vec<_> = trace.events.iter().filter(|e| e.satisfied).collect();
        assert_eq!(satisfied.len(), 1);
        assert_eq!(satisfied[0].timestamp, 1000);
    }

    #[test]
    fn test_boolean_expression() {
        let policy = collision_policy();
        let readings = vec![reading(12.0, 1000)];
        let trace = build_trace(&policy, &readings);
        assert!(trace.boolean_expression.contains("impact_g"));
        assert!(trace.overall_verdict);
    }

    #[test]
    fn test_false_boolean_expression() {
        let policy = collision_policy();
        let readings = vec![reading(0.5, 1000)];
        let trace = build_trace(&policy, &readings);
        assert_eq!(trace.boolean_expression, "false");
        assert!(!trace.overall_verdict);
    }

    #[test]
    fn test_satisfied_thresholds() {
        let policy = collision_policy();
        let readings = vec![reading(12.0, 1000)];
        let trace = build_trace(&policy, &readings);
        let sat = trace.satisfied_thresholds();
        assert_eq!(sat, vec!["impact_g"]);
    }

    #[test]
    fn test_window_of_interest() {
        let policy = collision_policy();
        let readings = vec![reading(0.5, 0), reading(12.0, 1000), reading(15.0, 2000), reading(0.3, 3000)];
        let trace = build_trace(&policy, &readings);
        let (start, end) = trace.window_of_interest().unwrap();
        assert_eq!(start, 1000);
        assert_eq!(end, 2000);
    }

    #[test]
    fn test_multiple_thresholds_all_satisfied() {
        let policy = DamagePolicy {
            name: "multi".into(),
            thresholds: vec![
                DamageThreshold {
                    name: "g_force".into(),
                    sensor_type: SensorType::Accelerometer,
                    parameter: ThresholdParameter::GreaterThan(5.0),
                    duration_ms: None,
                },
                DamageThreshold {
                    name: "speed".into(),
                    sensor_type: SensorType::Speedometer,
                    parameter: ThresholdParameter::GreaterThan(50.0),
                    duration_ms: None,
                },
            ],
            combination: CombinationLogic::All,
        };
        let readings = vec![
            reading(8.0, 1000),
            SensorReading {
                sensor_id: Uuid::nil(),
                sensor_type: SensorType::Speedometer,
                timestamp: 1000,
                value: 80.0,
                unit: "km/h".into(),
            },
        ];
        let trace = build_trace(&policy, &readings);
        assert!(trace.overall_verdict);
        assert_eq!(trace.unique_thresholds().len(), 2);
    }
}
