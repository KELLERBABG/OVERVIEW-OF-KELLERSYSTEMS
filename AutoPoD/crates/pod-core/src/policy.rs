use serde::{Deserialize, Serialize};

use crate::sensor::SensorType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageThreshold {
    pub name: String,
    pub sensor_type: SensorType,
    pub parameter: ThresholdParameter,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThresholdParameter {
    GreaterThan(f64),
    LessThan(f64),
    Between(f64, f64),
    RateOfChange {
        max_delta: f64,
        over_duration_ms: i64,
    },
    StructuralShift {
        min_frequency_shift_hz: f64,
        min_amplitude_change: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamagePolicy {
    pub name: String,
    pub thresholds: Vec<DamageThreshold>,
    pub combination: CombinationLogic,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CombinationLogic {
    All,
    Any,
    AtLeast(usize),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensor::SensorType;

    #[test]
    fn test_policy_serialization() {
        let p = DamagePolicy {
            name: "test".into(),
            thresholds: vec![DamageThreshold {
                name: "g_force".into(),
                sensor_type: SensorType::Accelerometer,
                parameter: ThresholdParameter::GreaterThan(5.0),
                duration_ms: None,
            }],
            combination: CombinationLogic::Any,
        };
        let json = serde_json::to_string_pretty(&p).unwrap();
        let back: DamagePolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(p.name, back.name);
        assert_eq!(p.combination, back.combination);
    }
}
