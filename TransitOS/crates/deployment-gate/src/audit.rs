use crate::verdict::GateDecision;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: u64,
    pub timestamp: SystemTime,
    pub decision: GateDecision,
    pub latency_us: f64,
    pub checks_performed: Vec<String>,
}

pub struct AuditLog {
    entries: Vec<AuditEntry>,
    pub next_id: u64,
}

impl AuditLog {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            next_id: 1,
        }
    }

    pub fn record(&mut self, decision: GateDecision, latency_us: f64, checks: Vec<String>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.entries.push(AuditEntry {
            id,
            timestamp: SystemTime::now(),
            decision,
            latency_us,
            checks_performed: checks,
        });
        id
    }

    pub fn entries(&self) -> &[AuditEntry] {
        &self.entries
    }

    pub fn blocks(&self) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| matches!(e.decision.verdict, crate::verdict::GateVerdict::Block))
            .collect()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}
