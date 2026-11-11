//! # ExecutionGate
//!
//! Automated control system that atomically releases protected blueprints
//! to manufacturing systems only when BOTH conditions are met:
//! 1. **CLEAN** — Cryptographic verification confirms no infringement
//! 2. **PAID** — Smart-contract micro-payments are successfully completed

use dfip_core::models::SatResult;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The status of the execution gate for a blueprint.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum GateStatus {
    /// Blueprint locked (cannot be accessed).
    Locked,
    /// Verification passed, awaiting payment.
    VerifiedNotPaid,
    /// Payment received, awaiting verification.
    PaidNotVerified,
    /// Both conditions met — blueprint released.
    Released,
    /// Verification failed (infringement detected).
    Blocked,
}

/// A blueprint release request.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlueprintRelease {
    /// The patent node being requested for manufacturing.
    pub patent_node_id: Uuid,
    /// The manufacturing entity requesting access.
    pub requester: String,
    /// The IPFS/Arweave hash of the encrypted blueprint.
    pub blueprint_hash: String,
    /// Current gate status.
    pub status: GateStatus,
    /// Timestamps.
    pub created_at: i64,
    pub released_at: Option<i64>,
}

/// The execution gate contract.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionGate {
    /// Release requests indexed by (patent_id, requester).
    requests: Vec<BlueprintRelease>,
}

impl ExecutionGate {
    /// Create a new execution gate.
    pub fn new() -> Self {
        Self {
            requests: Vec::new(),
        }
    }

    /// Request blueprint release for manufacturing.
    pub fn request_release(
        &mut self,
        patent_node_id: Uuid,
        requester: &str,
        blueprint_hash: &str,
    ) -> BlueprintRelease {
        let release = BlueprintRelease {
            patent_node_id,
            requester: requester.to_string(),
            blueprint_hash: blueprint_hash.to_string(),
            status: GateStatus::Locked,
            created_at: chrono::Utc::now().timestamp(),
            released_at: None,
        };
        self.requests.push(release.clone());
        release
    }

    /// Submit a verification result. If CLEAN, advances status.
    pub fn submit_verification(
        &mut self,
        patent_node_id: &Uuid,
        requester: &str,
        sat: SatResult,
    ) -> Result<GateStatus, String> {
        let request = self
            .requests
            .iter_mut()
            .find(|r| r.patent_node_id == *patent_node_id && r.requester == requester)
            .ok_or_else(|| "no matching release request".to_string())?;

        match request.status {
            GateStatus::Locked => {
                if sat == SatResult::Unsat {
                    request.status = GateStatus::VerifiedNotPaid;
                } else if sat == SatResult::Sat {
                    request.status = GateStatus::Blocked;
                }
            }
            GateStatus::PaidNotVerified => {
                if sat == SatResult::Unsat {
                    request.status = GateStatus::Released;
                    request.released_at = Some(chrono::Utc::now().timestamp());
                } else if sat == SatResult::Sat {
                    request.status = GateStatus::Blocked;
                }
            }
            GateStatus::Released => {
                return Err("already released".to_string());
            }
            GateStatus::Blocked => {
                return Err("permanently blocked".to_string());
            }
            GateStatus::VerifiedNotPaid => {
                return Err("payment still pending".to_string());
            }
        }

        Ok(request.status.clone())
    }

    /// Confirm payment for a blueprint release.
    pub fn confirm_payment(
        &mut self,
        patent_node_id: &Uuid,
        requester: &str,
        amount: u64,
    ) -> Result<GateStatus, String> {
        let request = self
            .requests
            .iter_mut()
            .find(|r| r.patent_node_id == *patent_node_id && r.requester == requester)
            .ok_or_else(|| "no matching release request".to_string())?;

        match request.status {
            GateStatus::Locked => {
                request.status = GateStatus::PaidNotVerified;
            }
            GateStatus::VerifiedNotPaid => {
                request.status = GateStatus::Released;
                request.released_at = Some(chrono::Utc::now().timestamp());
            }
            GateStatus::Released => {
                return Err("already released".to_string());
            }
            GateStatus::Blocked => {
                return Err("permanently blocked".to_string());
            }
            GateStatus::PaidNotVerified => {
                return Err("verification already pending".to_string());
            }
        }

        Ok(request.status.clone())
    }

    /// Check if a blueprint is released.
    pub fn is_released(&self, patent_node_id: &Uuid, requester: &str) -> bool {
        self.requests.iter().any(|r| {
            r.patent_node_id == *patent_node_id
                && r.requester == requester
                && r.status == GateStatus::Released
        })
    }

    /// Get all requests.
    pub fn all_requests(&self) -> &[BlueprintRelease] {
        &self.requests
    }
}

impl Default for ExecutionGate {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dfip_core::models::SatResult;

    #[test]
    fn test_full_release_pipeline() {
        let mut gate = ExecutionGate::new();

        let patent_id = Uuid::new_v4();
        gate.request_release(patent_id, "0xFactory", "QmXyz...");
        assert!(!gate.is_released(&patent_id, "0xFactory"));

        // Step 1: Verification passes (CLEAN)
        gate.submit_verification(&patent_id, "0xFactory", SatResult::Unsat)
            .unwrap();
        assert!(!gate.is_released(&patent_id, "0xFactory")); // not paid yet

        // Step 2: Payment confirmed
        gate.confirm_payment(&patent_id, "0xFactory", 1000)
            .unwrap();

        // Now released!
        assert!(gate.is_released(&patent_id, "0xFactory"));
    }

    #[test]
    fn test_infringement_blocks_release() {
        let mut gate = ExecutionGate::new();

        let patent_id = Uuid::new_v4();
        gate.request_release(patent_id, "0xFactory", "QmXyz...");

        // Verification fails (infringement)
        let status = gate
            .submit_verification(&patent_id, "0xFactory", SatResult::Sat)
            .unwrap();
        assert_eq!(status, GateStatus::Blocked);
        assert!(!gate.is_released(&patent_id, "0xFactory"));

        // Payment should fail even if attempted
        let err = gate.confirm_payment(&patent_id, "0xFactory", 1000);
        assert!(err.is_err());
    }

    #[test]
    fn test_payment_before_verification() {
        let mut gate = ExecutionGate::new();

        let patent_id = Uuid::new_v4();
        gate.request_release(patent_id, "0xFactory", "QmXyz...");

        // Pay first, then verify
        gate.confirm_payment(&patent_id, "0xFactory", 500)
            .unwrap();
        gate.submit_verification(&patent_id, "0xFactory", SatResult::Unsat)
            .unwrap();

        assert!(gate.is_released(&patent_id, "0xFactory"));
    }
}