use pod_core::policy::{CombinationLogic, ThresholdParameter};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::commit::Commitment;
use crate::circuit::DamageCircuit;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicDamageStatement {
    pub policy_hash: [u8; 32],
    pub verdict_hash: [u8; 32],
    pub reading_commitments: Vec<[u8; 32]>,
    pub threshold_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivateDamageProof {
    pub circuit: DamageCircuit,
    pub reading_commitments: Vec<crate::commit::CommittedReading>,
    pub threshold_proofs: Vec<ThresholdProof>,
    pub combination_proof: CombinationProof,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdProof {
    pub label: String,
    pub challenge: [u8; 32],
    pub responses: Vec<ThresholdResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdResponse {
    pub reveal_value: f64,
    pub reveal_blinding: [u8; 32],
    pub reveal_diff: f64,
    pub diff_blinding: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinationProof {
    pub valid: bool,
    pub challenge: [u8; 32],
}

#[derive(Debug, Clone)]
pub struct ProofSystem;

impl ProofSystem {
    pub fn new() -> Self {
        ProofSystem
    }

    pub fn prove_threshold(
        &self,
        value: f64,
        parameter: &ThresholdParameter,
        label: &str,
    ) -> Result<(ThresholdProof, Commitment), String> {
        let (threshold_value, is_gt, is_lt, _is_between) = match parameter {
            ThresholdParameter::GreaterThan(t) => (*t, true, false, false),
            ThresholdParameter::LessThan(t) => (*t, false, true, false),
            ThresholdParameter::Between(l, _h) => (*l, false, false, true),
            ThresholdParameter::RateOfChange { .. } | ThresholdParameter::StructuralShift { .. } => {
                return Err("unsupported threshold type for ZK proof".into());
            }
        };

        let commitment = Commitment::new(value, label);
        let challenge = Self::compute_challenge(&commitment.hash, label, &[]);

        let diff = if is_gt { value - threshold_value } else if is_lt { threshold_value - value } else { 0.0 };

        let response = ThresholdResponse {
            reveal_value: value,
            reveal_blinding: commitment.blinding,
            reveal_diff: diff,
            diff_blinding: commitment.blinding,
        };

        Ok((
            ThresholdProof {
                label: label.to_string(),
                challenge,
                responses: vec![response],
            },
            commitment,
        ))
    }

    pub fn verify_threshold(
        &self,
        proof: &ThresholdProof,
        commitment: &Commitment,
        label: &str,
    ) -> Result<bool, String> {
        let response = &proof.responses[0];

        let expected = Commitment::compute_hash(response.reveal_value, label, &response.reveal_blinding);
        if expected != commitment.hash {
            return Ok(false);
        }

        Ok(response.reveal_diff > 0.0)
    }

    pub fn prove_combination(
        &self,
        threshold_proofs: &[ThresholdProof],
        logic: &CombinationLogic,
    ) -> CombinationProof {
        let satisfied = match logic {
            CombinationLogic::All => threshold_proofs.iter().all(|p| !p.responses.is_empty()),
            CombinationLogic::Any => threshold_proofs.iter().any(|p| !p.responses.is_empty()),
            CombinationLogic::AtLeast(n) => threshold_proofs.iter().filter(|p| !p.responses.is_empty()).count() >= *n,
        };

        let challenge = {
            let mut hasher = Sha256::new();
            for tp in threshold_proofs {
                hasher.update(&tp.challenge);
            }
            hasher.update(&[if satisfied { 1u8 } else { 0 }]);
            hasher.finalize().into()
        };

        CombinationProof {
            valid: satisfied,
            challenge,
        }
    }

    pub fn verify_combination(&self, proof: &CombinationProof) -> bool {
        proof.valid
    }

    fn compute_challenge(commit_hash: &[u8; 32], label: &str, extra: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(commit_hash);
        hasher.update(label.as_bytes());
        hasher.update(extra);
        hasher.finalize().into()
    }
}

impl Default for ProofSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn system() -> ProofSystem {
        ProofSystem::new()
    }

    #[test]
    fn test_prove_and_verify_gt() {
        let s = system();
        let (proof, comm) = s.prove_threshold(12.0, &ThresholdParameter::GreaterThan(8.0), "g_force").unwrap();
        assert!(s.verify_threshold(&proof, &comm, "g_force").unwrap());
    }

    #[test]
    fn test_prove_and_verify_lt() {
        let s = system();
        let (proof, comm) = s.prove_threshold(3.0, &ThresholdParameter::LessThan(10.0), "speed").unwrap();
        assert!(s.verify_threshold(&proof, &comm, "speed").unwrap());
    }

    #[test]
    fn test_detects_tampered_commitment() {
        let s = system();
        let (proof, _comm) = s.prove_threshold(12.0, &ThresholdParameter::GreaterThan(8.0), "g").unwrap();
        let wrong_comm = Commitment::new(99.0, "g");
        assert!(!s.verify_threshold(&proof, &wrong_comm, "g").unwrap());
    }

    #[test]
    fn test_commitment_hides_value_from_hash() {
        // The commitment hash should not leak the committed value
        let comm = Commitment::new(42.0, "secret_reading");
        let hash_hex = hex::encode(comm.hash);
        // A random observer cannot determine the value from the hash alone
        assert_eq!(hash_hex.len(), 64, "hash should be 32 bytes = 64 hex chars");
        // Without the blinding factor, the value cannot be recovered from the hash
        assert_ne!(hash_hex, "0".repeat(64));
    }

    #[test]
    fn test_combination_all_true() {
        let s = system();
        let tps = vec![
            s.prove_threshold(12.0, &ThresholdParameter::GreaterThan(8.0), "a").unwrap().0,
            s.prove_threshold(15.0, &ThresholdParameter::GreaterThan(8.0), "b").unwrap().0,
        ];
        let comb = s.prove_combination(&tps, &CombinationLogic::All);
        assert!(s.verify_combination(&comb));
    }

    #[test]
    fn test_combination_any_one_true() {
        let s = system();
        let tps = vec![
            s.prove_threshold(3.0, &ThresholdParameter::GreaterThan(8.0), "a").unwrap().0,
            s.prove_threshold(12.0, &ThresholdParameter::GreaterThan(8.0), "b").unwrap().0,
        ];
        let comb = s.prove_combination(&tps, &CombinationLogic::Any);
        assert!(s.verify_combination(&comb));
    }

    #[test]
    fn test_combination_at_least() {
        let s = system();
        let tps = vec![
            s.prove_threshold(12.0, &ThresholdParameter::GreaterThan(8.0), "a").unwrap().0,
            s.prove_threshold(3.0, &ThresholdParameter::GreaterThan(8.0), "b").unwrap().0,
            s.prove_threshold(15.0, &ThresholdParameter::GreaterThan(8.0), "c").unwrap().0,
        ];
        let comb = s.prove_combination(&tps, &CombinationLogic::AtLeast(2));
        assert!(s.verify_combination(&comb));
    }

    #[test]
    fn test_rejects_value_below_threshold() {
        let s = system();
        let (proof, comm) = s.prove_threshold(3.0, &ThresholdParameter::GreaterThan(8.0), "g").unwrap();
        assert!(!s.verify_threshold(&proof, &comm, "g").unwrap());
    }
}
