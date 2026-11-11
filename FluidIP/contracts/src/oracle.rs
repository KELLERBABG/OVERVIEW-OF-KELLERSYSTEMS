//! # VerificationOracle
//!
//! Bridge between on-chain contracts and off-chain SMT solver results.
//! Accepts SMT results + ZK proofs of correct solver execution,
//! validates the proof on-chain, and emits verification events.

use dfip_core::models::{SatResult, VerificationResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// A verification request stored on-chain.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerificationRequest {
    /// The edge being verified.
    pub edge_id: Uuid,
    /// The original patent node.
    pub original_node_id: Uuid,
    /// The derivative patent node.
    pub derivative_node_id: Uuid,
    /// Whether the request has been resolved.
    pub resolved: bool,
    /// The result (if resolved).
    pub result: Option<VerificationResult>,
}

/// A verification event emitted by the oracle.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerificationEvent {
    pub edge_id: Uuid,
    pub sat: SatResult,
    pub severity_score: f64,
    pub solver_time_ms: f64,
    pub block_timestamp: i64,
}

/// The on-chain verification oracle.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerificationOracle {
    requests: Vec<VerificationRequest>,
    events: Vec<VerificationEvent>,
}

impl VerificationOracle {
    pub fn new() -> Self {
        Self {
            requests: Vec::new(),
            events: Vec::new(),
        }
    }

    /// Submit a verification request.
    pub fn submit_request(&mut self, edge_id: Uuid, original: Uuid, derivative: Uuid) {
        self.requests.push(VerificationRequest {
            edge_id,
            original_node_id: original,
            derivative_node_id: derivative,
            resolved: false,
            result: None,
        });
    }

    /// Submit a verification result (from off-chain solver).
    /// In production, this would require a ZK proof of correct execution.
    pub fn submit_result(
        &mut self,
        edge_id: &Uuid,
        result: VerificationResult,
        proof: &[u8],
    ) -> Result<VerificationEvent, String> {
        // Find the matching request
        let request = self
            .requests
            .iter_mut()
            .find(|r| r.edge_id == *edge_id && !r.resolved)
            .ok_or_else(|| "no pending verification request found".to_string())?;

        // Verify proof (simulated — in production, verify ZK proof)
        let proof_ok = Self::verify_proof(&result, proof);

        if !proof_ok {
            return Err("proof verification failed".to_string());
        }

        request.resolved = true;
        request.result = Some(result.clone());

        let event = VerificationEvent {
            edge_id: *edge_id,
            sat: result.sat,
            severity_score: result.severity_score,
            solver_time_ms: result.solver_time_ms,
            block_timestamp: chrono::Utc::now().timestamp(),
        };

        self.events.push(event.clone());
        Ok(event)
    }

    /// Verify a ZK proof of correct solver execution.
    fn verify_proof(result: &VerificationResult, proof: &[u8]) -> bool {
        if proof.is_empty() {
            return false; // proof required
        }
        // In production: verify Groth16/PLONK proof.
        // Here: just check proof is non-empty and has minimum length.
        proof.len() >= 32
    }

    /// Get all events.
    pub fn events(&self) -> &[VerificationEvent] {
        &self.events
    }

    /// Get all pending requests.
    pub fn pending_requests(&self) -> Vec<&VerificationRequest> {
        self.requests.iter().filter(|r| !r.resolved).collect()
    }
}

impl Default for VerificationOracle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dfip_core::models::{SatResult, VerificationResult};

    #[test]
    fn test_submit_and_resolve() {
        let mut oracle = VerificationOracle::new();

        let edge_id = Uuid::new_v4();
        let original = Uuid::new_v4();
        let derivative = Uuid::new_v4();

        oracle.submit_request(edge_id, original, derivative);
        assert_eq!(oracle.pending_requests().len(), 1);

        let result = VerificationResult {
            edge_id,
            sat: SatResult::Sat,
            model: vec![],
            unsat_core: vec![],
            severity_score: 0.8,
            solver_time_ms: 42.0,
            proof_commitment: vec![],
        };

        let proof = vec![0u8; 32];
        let event = oracle.submit_result(&edge_id, result, &proof).unwrap();
        assert_eq!(event.edge_id, edge_id);
        assert_eq!(event.sat, SatResult::Sat);
        assert_eq!(oracle.pending_requests().len(), 0);
    }

    #[test]
    fn test_empty_proof_rejected() {
        let mut oracle = VerificationOracle::new();
        let edge_id = Uuid::new_v4();
        oracle.submit_request(edge_id, Uuid::new_v4(), Uuid::new_v4());

        let result = VerificationResult {
            edge_id,
            sat: SatResult::Unsat,
            model: vec![],
            unsat_core: vec![],
            severity_score: 0.1,
            solver_time_ms: 10.0,
            proof_commitment: vec![],
        };

        let err = oracle.submit_result(&edge_id, result, &[]);
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("proof verification"));
    }
}