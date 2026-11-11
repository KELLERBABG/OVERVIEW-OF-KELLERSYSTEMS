use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyMetrics {
    pub total_checks: u64,
    pub total_allowed: u64,
    pub total_blocked: u64,
    pub block_rate: f64,
    pub avg_latency_us: f64,
    pub max_latency_us: f64,
    pub p99_latency_us: f64,
    pub current_safety_status: SafetyStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SafetyStatus {
    Safe,
    Warning,
    Critical,
}

impl SafetyMetrics {
    pub fn new() -> Self {
        Self {
            total_checks: 0,
            total_allowed: 0,
            total_blocked: 0,
            block_rate: 0.0,
            avg_latency_us: 0.0,
            max_latency_us: 0.0,
            p99_latency_us: 0.0,
            current_safety_status: SafetyStatus::Safe,
        }
    }

    pub fn record_check(&mut self, allowed: bool, latency_us: f64) {
        self.total_checks += 1;
        if allowed {
            self.total_allowed += 1;
        } else {
            self.total_blocked += 1;
        }
        self.block_rate = self.total_blocked as f64 / self.total_checks as f64;
        self.avg_latency_us = ((self.avg_latency_us * (self.total_checks as f64 - 1.0))
            + latency_us)
            / self.total_checks as f64;
        self.max_latency_us = self.max_latency_us.max(latency_us);

        self.current_safety_status = if self.block_rate > 0.1 {
            SafetyStatus::Critical
        } else if self.block_rate > 0.02 {
            SafetyStatus::Warning
        } else {
            SafetyStatus::Safe
        };
    }
}

impl Default for SafetyMetrics {
    fn default() -> Self {
        Self::new()
    }
}
