use crate::history::ViolationHistory;
use crate::integrity::SensorIntegrityReport;
use crate::metrics::SafetyMetrics;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSnapshot {
    pub metrics: SafetyMetrics,
    pub violations: Option<ViolationHistory>,
    pub sensor_integrity: Option<SensorIntegrityReport>,
    pub uptime_seconds: f64,
}

pub struct Dashboard {
    pub metrics: SafetyMetrics,
    pub violations: ViolationHistory,
    pub sensor_integrity: SensorIntegrityReport,
    pub start_time: std::time::Instant,
}

impl Dashboard {
    pub fn new() -> Self {
        Self {
            metrics: SafetyMetrics::new(),
            violations: ViolationHistory::new(),
            sensor_integrity: SensorIntegrityReport::new(),
            start_time: std::time::Instant::now(),
        }
    }

    pub fn record_check(
        &mut self,
        allowed: bool,
        latency_us: f64,
        violated_invariants: Vec<String>,
    ) {
        self.metrics.record_check(allowed, latency_us);
        if !allowed {
            self.violations
                .record_violation(violated_invariants, latency_us);
        }
    }

    pub fn snapshot(&self) -> DashboardSnapshot {
        DashboardSnapshot {
            metrics: self.metrics.clone(),
            violations: Some(self.violations.clone()),
            sensor_integrity: Some(self.sensor_integrity.clone()),
            uptime_seconds: self.start_time.elapsed().as_secs_f64(),
        }
    }
}

impl Default for Dashboard {
    fn default() -> Self {
        Self::new()
    }
}
