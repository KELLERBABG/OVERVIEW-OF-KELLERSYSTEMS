use pod_core::event::TelemetryEvent;
use pod_core::policy::DamagePolicy;
use pod_core::sensor::SensorReading;
use serde::{Deserialize, Serialize};

use crate::trace::{build_trace, ConstraintTrace};
use crate::{IncidentClass, IncidentExplorer};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageReport {
    pub report_id: String,
    pub generated_at: i64,
    pub incident: IncidentSummary,
    pub policy: PolicySummary,
    pub trace: ConstraintTrace,
    pub classification: ClassificationSummary,
    pub readings: ReadingsSummary,
    pub conclusion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentSummary {
    pub event_id: String,
    pub vehicle_id: Option<String>,
    pub source: String,
    pub incident_window_start: i64,
    pub incident_window_end: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicySummary {
    pub name: String,
    pub threshold_count: usize,
    pub combination: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationSummary {
    pub class: IncidentClass,
    pub label: String,
    pub confidence: f64,
    pub verdict: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadingsSummary {
    pub total: usize,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub duration_ms: i64,
    pub sensor_types: Vec<String>,
}

impl DamageReport {
    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into())
    }

    pub fn to_json_compact(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".into())
    }

    pub fn summary_line(&self) -> String {
        format!(
            "[{}] {} | Verdict: {} | Class: {} | Confidence: {:.0}% | Readings: {}",
            self.report_id.get(..8).unwrap_or(&self.report_id),
            self.policy.name,
            self.classification.verdict,
            self.classification.label,
            self.classification.confidence * 100.0,
            self.readings.total,
        )
    }
}

pub fn generate_report(
    event: &TelemetryEvent,
    policy: &DamagePolicy,
) -> DamageReport {
    let readings: Vec<SensorReading> = event
        .readings
        .iter()
        .map(|sr| sr.reading.clone())
        .collect();

    let trace = build_trace(policy, &readings);
    let class = IncidentExplorer::classify(&trace);
    let confidence = IncidentExplorer::confidence(&trace);

    let verdict = if trace.overall_verdict {
        "APPROVED"
    } else {
        "REJECTED"
    };

    let values: Vec<f64> = readings.iter().map(|r| r.value).collect();
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let sum: f64 = values.iter().sum();
    let mean = sum / values.len() as f64;

    let sensor_types: std::collections::BTreeSet<String> = readings
        .iter()
        .map(|r| format!("{:?}", r.sensor_type))
        .collect();

    DamageReport {
        report_id: uuid::Uuid::new_v4().to_string(),
        generated_at: chrono::Utc::now().timestamp_millis(),
        incident: IncidentSummary {
            event_id: event.event_id.to_string(),
            vehicle_id: event.metadata.vehicle_id.clone(),
            source: event.metadata.source.clone(),
            incident_window_start: event.incident_window.start_timestamp,
            incident_window_end: event.incident_window.end_timestamp,
        },
        policy: PolicySummary {
            name: policy.name.clone(),
            threshold_count: policy.thresholds.len(),
            combination: format!("{:?}", policy.combination),
        },
        trace,
        classification: ClassificationSummary {
            class,
            label: class.label().to_string(),
            confidence,
            verdict: verdict.to_string(),
        },
        readings: ReadingsSummary {
            total: readings.len(),
            min,
            max,
            mean,
            duration_ms: event.incident_window.end_timestamp - event.incident_window.start_timestamp,
            sensor_types: sensor_types.into_iter().collect(),
        },
        conclusion: format!(
            "Incident classified as '{}' with {:.0}% confidence. Verdict: {}.",
            class.label(),
            confidence * 100.0,
            verdict,
        ),
    }
}

pub fn generate_report_from_data(
    readings: &[SensorReading],
    policy: &DamagePolicy,
    metadata: Option<&pod_core::event::EventMetadata>,
) -> DamageReport {
    let now = chrono::Utc::now().timestamp_millis();
    let event = TelemetryEvent {
        event_id: uuid::Uuid::new_v4(),
        incident_window: pod_core::event::IncidentWindow {
            start_timestamp: readings.first().map(|r| r.timestamp).unwrap_or(now),
            end_timestamp: readings.last().map(|r| r.timestamp).unwrap_or(now),
        },
        readings: readings
            .iter()
            .cloned()
            .map(|r| pod_core::sensor::SignedReading {
                reading: r,
                signature: vec![],
                signer_public_key: vec![],
            })
            .collect(),
        metadata: metadata.cloned().unwrap_or(pod_core::event::EventMetadata {
            source: "direct".into(),
            vehicle_id: None,
            location: None,
            description: None,
        }),
    };
    generate_report(&event, policy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pod_core::policy::{CombinationLogic, DamageThreshold, ThresholdParameter};
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

    fn sample_policy() -> DamagePolicy {
        DamagePolicy {
            name: "test-collision".into(),
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
    fn test_generate_report() {
        let policy = sample_policy();
        let readings = vec![reading(12.0, 1000), reading(0.5, 2000)];
        let metadata = pod_core::event::EventMetadata {
            source: "test".into(),
            vehicle_id: Some("VH-001".into()),
            location: None,
            description: None,
        };
        let report = generate_report_from_data(&readings, &policy, Some(&metadata));
        assert!(report.report_id.len() > 0);
        assert_eq!(report.incident.vehicle_id.unwrap(), "VH-001");
        assert_eq!(report.classification.verdict, "APPROVED");
        assert_eq!(report.readings.total, 2);
    }

    #[test]
    fn test_report_json_output() {
        let policy = sample_policy();
        let readings = vec![reading(12.0, 1000)];
        let report = generate_report_from_data(&readings, &policy, None);
        let json = report.to_json_pretty();
        assert!(json.contains("test-collision"));
        assert!(json.contains("APPROVED"));
    }

    #[test]
    fn test_report_summary_line() {
        let policy = sample_policy();
        let readings = vec![reading(12.0, 1000)];
        let report = generate_report_from_data(&readings, &policy, None);
        let summary = report.summary_line();
        assert!(summary.contains("test-collision"));
        assert!(summary.contains("APPROVED"));
    }

    #[test]
    fn test_report_without_metadata() {
        let policy = sample_policy();
        let readings = vec![
            reading(0.5, 0),
            reading(0.6, 100),
            reading(0.5, 200),
        ];
        let report = generate_report_from_data(&readings, &policy, None);
        assert_eq!(report.classification.class, IncidentClass::NoEvent);
        assert_eq!(report.classification.verdict, "REJECTED");
    }
}
