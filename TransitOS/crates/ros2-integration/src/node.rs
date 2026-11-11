use crate::messages::{SafetyVerdictMsg, VehicleStateMsg};
use transit_deployment_gate::verdict::{GateDecision, GateVerdict};

pub struct SafetyNode {
    pub node_name: String,
    pub check_count: u64,
}

impl SafetyNode {
    pub fn new(name: &str) -> Self {
        Self {
            node_name: name.to_string(),
            check_count: 0,
        }
    }

    pub fn process_state(&mut self, _state: &VehicleStateMsg) {
        self.check_count += 1;
    }

    pub fn publish_verdict(&self, decision: &GateDecision) -> SafetyVerdictMsg {
        SafetyVerdictMsg {
            allowed: matches!(decision.verdict, GateVerdict::Allow),
            reason: decision.reason.clone(),
            verification_id: decision.check_id,
            latency_us: 0.0,
        }
    }
}
