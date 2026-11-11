use crate::trace::ConstraintTrace;
use crate::IncidentClass;

pub struct IncidentExplorer;

impl IncidentExplorer {
    pub fn classify(trace: &ConstraintTrace) -> IncidentClass {
        let satisfied_events: Vec<&crate::trace::ConstraintEvent> =
            trace.events.iter().filter(|e| e.satisfied).collect();

        if satisfied_events.is_empty() {
            return IncidentClass::NoEvent;
        }

        let timestamps: Vec<i64> = satisfied_events.iter().map(|e| e.timestamp).collect();
        let min_ts = *timestamps.iter().min().unwrap();
        let max_ts = *timestamps.iter().max().unwrap();
        let span = max_ts - min_ts;

        let unique_thresholds_hit: std::collections::HashSet<&str> = satisfied_events
            .iter()
            .map(|e| e.threshold_name.as_str())
            .collect();

        let threshold_count = unique_thresholds_hit.len();

        let peak_readings: Vec<f64> = satisfied_events.iter().map(|e| e.reading_value).collect();
        let max_val = peak_readings.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        if max_val > 20.0 && satisfied_events.len() <= 3 {
            return IncidentClass::Anomaly;
        }

        if span < 2000 && max_val > 5.0 && threshold_count >= 1 {
            let is_sharp_spike = peak_readings.windows(2).any(|w| (w[1] - w[0]).abs() > 3.0);
            if is_sharp_spike {
                return IncidentClass::AcuteImpact;
            }
            if satisfied_events.len() >= 3 {
                return IncidentClass::SustainedEvent;
            }
            return IncidentClass::AcuteImpact;
        }

        if span > 10_000 && max_val < 5.0 && satisfied_events.len() >= 5 {
            return IncidentClass::GradualDegradation;
        }

        if satisfied_events.len() >= threshold_count * 10 {
            return IncidentClass::SustainedEvent;
        }

        IncidentClass::Undetermined
    }

    pub fn confidence(trace: &ConstraintTrace) -> f64 {
        let satisfied_events: Vec<&crate::trace::ConstraintEvent> =
            trace.events.iter().filter(|e| e.satisfied).collect();

        if satisfied_events.is_empty() {
            return 0.0;
        }

        let total = trace.events.len() as f64;
        let satisfied = satisfied_events.len() as f64;
        let ratio = satisfied / total;

        let class = Self::classify(trace);
        let class_weight = match class {
            IncidentClass::AcuteImpact => 0.9,
            IncidentClass::SustainedEvent => 0.7,
            IncidentClass::GradualDegradation => 0.5,
            IncidentClass::Anomaly => 0.4,
            IncidentClass::NoEvent => 0.0,
            IncidentClass::Undetermined => 0.3,
        };

        ratio * 0.4 + class_weight * 0.6
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::build_trace;
    use pod_core::policy::{CombinationLogic, DamagePolicy, DamageThreshold, ThresholdParameter};
    use pod_core::sensor::{SensorReading, SensorType};
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
            thresholds: vec![DamageThreshold {
                name: "impact_g".into(),
                sensor_type: SensorType::Accelerometer,
                parameter: ThresholdParameter::GreaterThan(8.0),
                duration_ms: None,
            }],
            combination: CombinationLogic::Any,
        }
    }

    #[test]
    fn test_classify_acute_impact() {
        let policy = collision_policy();
        let readings = vec![
            reading(0.5, 0), reading(0.6, 100), reading(0.5, 200),
            reading(12.0, 300), reading(14.0, 400), reading(0.4, 500),
        ];
        let trace = build_trace(&policy, &readings);
        assert_eq!(IncidentExplorer::classify(&trace), IncidentClass::AcuteImpact);
    }

    #[test]
    fn test_classify_no_event() {
        let policy = collision_policy();
        let readings = vec![reading(0.5, 0), reading(0.6, 100)];
        let trace = build_trace(&policy, &readings);
        assert_eq!(IncidentExplorer::classify(&trace), IncidentClass::NoEvent);
    }

    #[test]
    fn test_classify_anomaly() {
        let policy = collision_policy();
        let readings = vec![reading(99.9, 500)];
        let trace = build_trace(&policy, &readings);
        assert_eq!(IncidentExplorer::classify(&trace), IncidentClass::Anomaly);
    }

    #[test]
    fn test_confidence_score() {
        let policy = collision_policy();
        let readings = vec![reading(12.0, 500)];
        let trace = build_trace(&policy, &readings);
        let conf = IncidentExplorer::confidence(&trace);
        assert!(conf > 0.0);
        assert!(conf <= 1.0);
    }

    #[test]
    fn test_confidence_zero_for_no_event() {
        let policy = collision_policy();
        let readings = vec![reading(0.5, 500)];
        let trace = build_trace(&policy, &readings);
        assert!((IncidentExplorer::confidence(&trace) - 0.0).abs() < 1e-6);
    }
}
