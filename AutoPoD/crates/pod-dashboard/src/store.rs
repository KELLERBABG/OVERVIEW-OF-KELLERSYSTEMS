use std::collections::HashMap;

use chrono::Utc;
use incident_audit::{generate_report, IncidentExplorer};
use physics_logic::conditions::{evaluate_combination, evaluate_policy};
use pod_core::event::TelemetryEvent;
use pod_core::policy::DamagePolicy;
use pod_core::sensor::SensorReading;
use serde::{Deserialize, Serialize};
use sensor_integrity::taint::audit_signed_readings;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimRecord {
    pub id: String,
    pub event_id: String,
    pub policy_name: String,
    pub status: ClaimStatus,
    pub verdict: Option<String>,
    pub classification: Option<String>,
    pub confidence: Option<f64>,
    pub trust_score: Option<f64>,
    pub taint_verdict: Option<String>,
    pub report_json: Option<String>,
    pub event_json: String,
    pub policy_json: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ClaimStatus {
    Pending,
    Verified,
    Classified,
    Reported,
    Tainted,
}

#[derive(Debug, Clone)]
pub struct Store {
    claims: HashMap<String, ClaimRecord>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            claims: HashMap::new(),
        }
    }

    pub fn all(&self) -> Vec<ClaimRecord> {
        let mut v: Vec<ClaimRecord> = self.claims.values().cloned().collect();
        v.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        v
    }

    pub fn get(&self, id: &str) -> Option<&ClaimRecord> {
        self.claims.get(id)
    }

    pub fn add(&mut self, event: TelemetryEvent, policy: DamagePolicy) -> ClaimRecord {
        let id = uuid::Uuid::new_v4().to_string();
        let policy_name = policy.name.clone();
        let event_id = event.event_id.to_string();

        let record = ClaimRecord {
            id,
            event_id,
            policy_name,
            status: ClaimStatus::Pending,
            verdict: None,
            classification: None,
            confidence: None,
            trust_score: None,
            taint_verdict: None,
            report_json: None,
            event_json: serde_json::to_string(&event).unwrap_or_default(),
            policy_json: serde_json::to_string(&policy).unwrap_or_default(),
            created_at: Utc::now().timestamp_millis(),
        };

        let id = record.id.clone();
        self.claims.insert(id.clone(), record);
        self.claims.get(&id).cloned().unwrap()
    }

    pub fn verify(&mut self, id: &str) -> Result<(), ()> {
        let record = self.claims.get_mut(id).ok_or(())?;
        let event: TelemetryEvent =
            serde_json::from_str(&record.event_json).map_err(|_| ())?;
        let policy: DamagePolicy =
            serde_json::from_str(&record.policy_json).map_err(|_| ())?;

        let readings: Vec<SensorReading> =
            event.readings.into_iter().map(|sr| sr.reading).collect();
        let conditions = evaluate_policy(&policy, &readings);
        let approved = evaluate_combination(&conditions, &policy.combination);

        record.verdict = Some(if approved {
            "APPROVED".into()
        } else {
            "REJECTED".into()
        });
        record.status = ClaimStatus::Verified;
        Ok(())
    }

    pub fn classify(&mut self, id: &str) -> Result<(), ()> {
        self.verify(id)?;

        let needs_classify = self.claims.get(id).map(|r| r.classification.is_none()).unwrap_or(false);
        if !needs_classify {
            return Ok(());
        }

        let (event_json, policy_json) = {
            let r = self.claims.get(id).ok_or(())?;
            (r.event_json.clone(), r.policy_json.clone())
        };

        let event: TelemetryEvent = serde_json::from_str(&event_json).map_err(|_| ())?;
        let policy: DamagePolicy = serde_json::from_str(&policy_json).map_err(|_| ())?;

        let readings: Vec<SensorReading> =
            event.readings.into_iter().map(|sr| sr.reading).collect();
        let trace = incident_audit::build_trace(&policy, &readings);
        let class = IncidentExplorer::classify(&trace);
        let confidence = IncidentExplorer::confidence(&trace);

        let record = self.claims.get_mut(id).ok_or(())?;
        record.classification = Some(class.label().to_string());
        record.confidence = Some(confidence);
        record.status = ClaimStatus::Classified;
        Ok(())
    }

    pub fn generate_report_data(&mut self, id: &str) -> Result<(), ()> {
        let (event_json, policy_json) = {
            let r = self.claims.get(id).ok_or(())?;
            (r.event_json.clone(), r.policy_json.clone())
        };
        let event: TelemetryEvent = serde_json::from_str(&event_json).map_err(|_| ())?;
        let policy: DamagePolicy = serde_json::from_str(&policy_json).map_err(|_| ())?;
        let report = generate_report(&event, &policy);
        let json = report.to_json_pretty();
        let record = self.claims.get_mut(id).ok_or(())?;
        record.report_json = Some(json);
        record.status = ClaimStatus::Reported;
        Ok(())
    }

    pub fn taint(&mut self, id: &str) -> Result<(), ()> {
        let record = self.claims.get_mut(id).ok_or(())?;
        let event: TelemetryEvent =
            serde_json::from_str(&record.event_json).map_err(|_| ())?;

        let report = audit_signed_readings(&event.readings);
        record.trust_score = Some(report.trust_score);
        record.taint_verdict = Some(format!("{:?}", report.verdict));
        record.status = ClaimStatus::Tainted;
        Ok(())
    }
}
