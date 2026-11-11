use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViolationEvent {
    pub timestamp: SystemTime,
    pub violated_invariants: Vec<String>,
    pub latency_us: f64,
    pub event_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViolationHistory {
    events: Vec<ViolationEvent>,
    next_id: u64,
}

impl ViolationHistory {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            next_id: 1,
        }
    }

    pub fn record_violation(&mut self, invariants: Vec<String>, latency_us: f64) {
        self.events.push(ViolationEvent {
            timestamp: SystemTime::now(),
            violated_invariants: invariants,
            latency_us,
            event_id: self.next_id,
        });
        self.next_id += 1;
    }

    pub fn events(&self) -> &[ViolationEvent] {
        &self.events
    }

    pub fn count(&self) -> usize {
        self.events.len()
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }

    pub fn most_recent(&self) -> Option<&ViolationEvent> {
        self.events.last()
    }

    pub fn most_frequent_violation(&self) -> Option<(String, usize)> {
        let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for event in &self.events {
            for inv in &event.violated_invariants {
                *counts.entry(inv.clone()).or_default() += 1;
            }
        }
        counts.into_iter().max_by_key(|(_, count)| *count)
    }
}

impl Default for ViolationHistory {
    fn default() -> Self {
        Self::new()
    }
}
