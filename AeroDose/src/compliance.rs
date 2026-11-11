//! Compliance shift report generator.

use crate::audit::AuditEntry;
use crate::dose::ShiftDoseTracker;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShiftComplianceReport {
    pub report_id: Uuid,
    pub worker_id: Uuid,
    pub shift_duration_hours: f64,
    pub twa_pm25_ug_m3: f64,
    pub peak_pm25_ug_m3: f64,
    pub total_dose_ug_h_m3: f64,
    pub max_alert_tier_reached: String,
    pub osha_compliant: bool,
    pub total_logged_samples: usize,
}

pub struct ComplianceEngine;

impl ComplianceEngine {
    pub fn generate_report(worker_id: Uuid, tracker: &ShiftDoseTracker, entries: &[AuditEntry]) -> ShiftComplianceReport {
        let twa = tracker.twa_pm25_ug_m3();
        let compliant = twa <= tracker.limits.respirator_pm25_ug_m3 && tracker.current_tier != crate::dose::AlertTier::StopWork;

        ShiftComplianceReport {
            report_id: Uuid::new_v4(),
            worker_id,
            shift_duration_hours: tracker.total_seconds / 3600.0,
            twa_pm25_ug_m3: twa,
            peak_pm25_ug_m3: tracker.peak_pm25_ug_m3,
            total_dose_ug_h_m3: tracker.cumulative_dose_ug_h_m3,
            max_alert_tier_reached: format!("{:?}", tracker.current_tier),
            osha_compliant: compliant,
            total_logged_samples: entries.len(),
        }
    }
}
