use crate::mrm::MrmCommand;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateVerdict {
    Allow,
    Block,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateDecision {
    pub verdict: GateVerdict,
    pub reason: Option<String>,
    pub timestamp: SystemTime,
    pub check_id: u64,
    pub mrm_fallback: Option<MrmCommand>,
}

impl GateDecision {
    pub fn allow(check_id: u64) -> Self {
        Self {
            verdict: GateVerdict::Allow,
            reason: None,
            timestamp: SystemTime::now(),
            check_id,
            mrm_fallback: None,
        }
    }

    pub fn block(check_id: u64, reason: &str) -> Self {
        Self {
            verdict: GateVerdict::Block,
            reason: Some(reason.to_string()),
            timestamp: SystemTime::now(),
            check_id,
            mrm_fallback: None,
        }
    }

    pub fn block_with_mrm(check_id: u64, reason: &str, mrm: MrmCommand) -> Self {
        Self {
            verdict: GateVerdict::Block,
            reason: Some(reason.to_string()),
            timestamp: SystemTime::now(),
            check_id,
            mrm_fallback: Some(mrm),
        }
    }
}
