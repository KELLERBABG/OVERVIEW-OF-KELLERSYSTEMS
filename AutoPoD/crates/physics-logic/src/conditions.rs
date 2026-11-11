use pod_core::policy::{CombinationLogic, DamagePolicy, DamageThreshold, ThresholdParameter};
use pod_core::sensor::SensorReading;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageCondition {
    pub name: String,
    pub expression: String,
    pub satisfied: bool,
}

pub fn evaluate_policy(policy: &DamagePolicy, readings: &[SensorReading]) -> Vec<DamageCondition> {
    let mut conditions = Vec::new();

    for threshold in &policy.thresholds {
        let matching: Vec<&SensorReading> = readings
            .iter()
            .filter(|r| r.sensor_type == threshold.sensor_type)
            .collect();

        let satisfied = if matching.is_empty() {
            false
        } else {
            check_threshold(threshold, &matching)
        };

        conditions.push(DamageCondition {
            name: threshold.name.clone(),
            expression: format_threshold(threshold),
            satisfied,
        });
    }

    conditions
}

pub fn check_threshold(threshold: &DamageThreshold, readings: &[&SensorReading]) -> bool {
    match &threshold.parameter {
        ThresholdParameter::GreaterThan(min) => readings.iter().any(|r| r.value > *min),
        ThresholdParameter::LessThan(max) => readings.iter().any(|r| r.value < *max),
        ThresholdParameter::Between(low, high) => readings.iter().any(|r| *low <= r.value && r.value <= *high),
        ThresholdParameter::RateOfChange { max_delta, over_duration_ms } => {
            if readings.len() < 2 {
                return false;
            }
            let mut sorted: Vec<&SensorReading> = readings.to_vec();
            sorted.sort_by_key(|r| r.timestamp);
            for i in 0..sorted.len() {
                let window_end = sorted[i].timestamp;
                let window_start = window_end - over_duration_ms;
                let in_window: Vec<&SensorReading> = sorted
                    .iter()
                    .filter(|r| r.timestamp >= window_start && r.timestamp <= window_end)
                    .copied()
                    .collect();
                if in_window.len() < 2 {
                    continue;
                }
                let min_val = in_window.iter().map(|r| r.value).fold(f64::INFINITY, |a, b| a.min(b));
                let max_val = in_window.iter().map(|r| r.value).fold(f64::NEG_INFINITY, |a, b| a.max(b));
                if (max_val - min_val).abs() > *max_delta {
                    return true;
                }
            }
            false
        }
        ThresholdParameter::StructuralShift { .. } => {
            false
        }
    }
}

pub fn evaluate_combination(conditions: &[DamageCondition], logic: &CombinationLogic) -> bool {
    let satisfied_count = conditions.iter().filter(|c| c.satisfied).count();

    match logic {
        CombinationLogic::All => satisfied_count == conditions.len(),
        CombinationLogic::Any => satisfied_count > 0,
        CombinationLogic::AtLeast(n) => satisfied_count >= *n,
    }
}

fn format_threshold(threshold: &DamageThreshold) -> String {
    match &threshold.parameter {
        ThresholdParameter::GreaterThan(v) => format!("{} > {}", threshold.name, v),
        ThresholdParameter::LessThan(v) => format!("{} < {}", threshold.name, v),
        ThresholdParameter::Between(l, h) => format!("{} ∈ [{}, {}]", threshold.name, l, h),
        ThresholdParameter::RateOfChange { max_delta, .. } => {
            format!("Δ{} > {}", threshold.name, max_delta)
        }
        ThresholdParameter::StructuralShift { min_frequency_shift_hz, .. } => {
            format!("freq_shift({}) > {} Hz", threshold.name, min_frequency_shift_hz)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pod_core::policy::DamagePolicy;
    use pod_core::sensor::SensorType;
    use uuid::Uuid;

    fn make_reading(sensor_type: SensorType, value: f64, timestamp: i64) -> SensorReading {
        SensorReading {
            sensor_id: Uuid::nil(),
            sensor_type,
            timestamp,
            value,
            unit: "g".into(),
        }
    }

    #[test]
    fn test_greater_than_threshold_passes() {
        let threshold = DamageThreshold {
            name: "g_force".into(),
            sensor_type: SensorType::Accelerometer,
            parameter: ThresholdParameter::GreaterThan(5.0),
            duration_ms: None,
        };
        let readings = vec![make_reading(SensorType::Accelerometer, 8.0, 1000)];
        assert!(check_threshold(&threshold, &readings.iter().collect::<Vec<_>>()));
    }

    #[test]
    fn test_greater_than_threshold_fails() {
        let threshold = DamageThreshold {
            name: "g_force".into(),
            sensor_type: SensorType::Accelerometer,
            parameter: ThresholdParameter::GreaterThan(10.0),
            duration_ms: None,
        };
        let readings = vec![make_reading(SensorType::Accelerometer, 5.0, 1000)];
        assert!(!check_threshold(&threshold, &readings.iter().collect::<Vec<_>>()));
    }

    #[test]
    fn test_between_threshold() {
        let threshold = DamageThreshold {
            name: "range".into(),
            sensor_type: SensorType::Accelerometer,
            parameter: ThresholdParameter::Between(3.0, 7.0),
            duration_ms: None,
        };
        let readings = vec![make_reading(SensorType::Accelerometer, 5.0, 1000)];
        assert!(check_threshold(&threshold, &readings.iter().collect::<Vec<_>>()));
    }

    #[test]
    fn test_rate_of_change_detects_spike() {
        let threshold = DamageThreshold {
            name: "jerk".into(),
            sensor_type: SensorType::Accelerometer,
            parameter: ThresholdParameter::RateOfChange { max_delta: 5.0, over_duration_ms: 500 },
            duration_ms: None,
        };
        let readings = vec![
            make_reading(SensorType::Accelerometer, 1.0, 0),
            make_reading(SensorType::Accelerometer, 1.2, 100),
            make_reading(SensorType::Accelerometer, 8.0, 200),
            make_reading(SensorType::Accelerometer, 8.5, 300),
        ];
        assert!(check_threshold(&threshold, &readings.iter().collect::<Vec<_>>()));
    }

    #[test]
    fn test_evaluate_policy_approves_collision() {
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
        let readings = vec![make_reading(SensorType::Accelerometer, 12.0, 1000)];
        let conditions = evaluate_policy(&policy, &readings);
        assert!(conditions[0].satisfied);
        assert!(evaluate_combination(&conditions, &CombinationLogic::Any));
    }

    #[test]
    fn test_evaluate_policy_rejects_normal() {
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
        let readings = vec![make_reading(SensorType::Accelerometer, 0.5, 1000)];
        let conditions = evaluate_policy(&policy, &readings);
        assert!(!conditions[0].satisfied);
    }

    #[test]
    fn test_all_combination_requires_all() {
        let conditions = vec![
            DamageCondition { name: "a".into(), expression: "a>5".into(), satisfied: true },
            DamageCondition { name: "b".into(), expression: "b<10".into(), satisfied: true },
        ];
        assert!(evaluate_combination(&conditions, &CombinationLogic::All));

        let conditions2 = vec![
            DamageCondition { name: "a".into(), expression: "a>5".into(), satisfied: true },
            DamageCondition { name: "b".into(), expression: "b<10".into(), satisfied: false },
        ];
        assert!(!evaluate_combination(&conditions2, &CombinationLogic::All));
    }

    #[test]
    fn test_at_least_combination() {
        let conditions = vec![
            DamageCondition { name: "a".into(), expression: "".into(), satisfied: true },
            DamageCondition { name: "b".into(), expression: "".into(), satisfied: false },
            DamageCondition { name: "c".into(), expression: "".into(), satisfied: true },
        ];
        assert!(evaluate_combination(&conditions, &CombinationLogic::AtLeast(2)));
        assert!(!evaluate_combination(&conditions, &CombinationLogic::AtLeast(3)));
    }

    #[test]
    fn test_any_combination() {
        let all_false = vec![
            DamageCondition { name: "a".into(), expression: "".into(), satisfied: false },
            DamageCondition { name: "b".into(), expression: "".into(), satisfied: false },
        ];
        assert!(!evaluate_combination(&all_false, &CombinationLogic::Any));

        let one_true = vec![
            DamageCondition { name: "a".into(), expression: "".into(), satisfied: false },
            DamageCondition { name: "b".into(), expression: "".into(), satisfied: true },
        ];
        assert!(evaluate_combination(&one_true, &CombinationLogic::Any));
    }

    #[test]
    fn test_sensor_type_mismatch_ignored_by_policy() {
        let policy = DamagePolicy {
            name: "speed_check".into(),
            thresholds: vec![DamageThreshold {
                name: "speed".into(),
                sensor_type: SensorType::Speedometer,
                parameter: ThresholdParameter::GreaterThan(100.0),
                duration_ms: None,
            }],
            combination: CombinationLogic::Any,
        };
        let readings = vec![make_reading(SensorType::Accelerometer, 200.0, 1000)];
        let conditions = evaluate_policy(&policy, &readings);
        assert!(!conditions[0].satisfied);
    }

    #[test]
    fn test_format_threshold_strings() {
        let t = DamageThreshold {
            name: "g".into(),
            sensor_type: SensorType::Accelerometer,
            parameter: ThresholdParameter::GreaterThan(5.0),
            duration_ms: None,
        };
        assert_eq!(format_threshold(&t), "g > 5");
    }
}
