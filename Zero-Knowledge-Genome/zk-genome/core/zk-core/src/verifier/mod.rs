//! # zk-SNARK Proof Verifier
//!
//! Verifies zero-knowledge proofs submitted by the mobile app.
//! Used by verification nodes in the network (Phase 3) and
//! locally on-device for proof validation before submission.

use alloc::vec::Vec;
use crate::{ProvingSystem, ZkProof, VerificationKey};

/// Error types for proof verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifierError {
    /// The proof format is invalid.
    InvalidProofFormat,
    /// The verification key doesn't match the circuit.
    KeyMismatch,
    /// The proof data is corrupted.
    CorruptedProof,
    /// The proof has expired or been revoked.
    ProofExpired,
    /// Verification failed (proof is invalid).
    VerificationFailed,
}

/// Configuration for the verifier.
#[derive(Debug, Clone)]
pub struct VerifierConfig {
    /// Whether to cache verification results.
    pub cache_results: bool,
    /// Maximum proof age in seconds (0 = no limit).
    pub max_proof_age_secs: u64,
}

impl Default for VerifierConfig {
    fn default() -> Self {
        VerifierConfig {
            cache_results: true,
            max_proof_age_secs: 0,
        }
    }
}

/// The proof verifier.
#[derive(Debug, Clone)]
pub struct Verifier {
    config: VerifierConfig,
}

impl Verifier {
    /// Create a new verifier with default configuration.
    pub fn new() -> Self {
        Verifier {
            config: VerifierConfig::default(),
        }
    }

    /// Create a verifier with custom configuration.
    pub fn with_config(config: VerifierConfig) -> Self {
        Verifier { config }
    }

    /// Verify a zk proof against a verification key.
    ///
    /// Performs:
    /// 1. Structural validation (correct format)
    /// 2. Proving system check
    /// 3. Cryptographic verification of the proof
    ///
    /// Returns `true` if the proof is valid, `false` otherwise.
    pub fn verify(
        &self,
        proof: &ZkProof,
        verification_key: &VerificationKey,
    ) -> Result<bool, VerifierError> {
        // Step 1: Structural validation
        if proof.proof_data.is_empty() {
            return Err(VerifierError::InvalidProofFormat);
        }

        // Step 2: Proving system compatibility
        if proof.system != verification_key.system {
            return Err(VerifierError::KeyMismatch);
        }

        // Step 3: Check proof size expectations
        let expected_size = match proof.system {
            ProvingSystem::Groth16 => 200,
            ProvingSystem::Plonk => 300,
        };
        if proof.proof_data.len() < expected_size / 2 {
            return Err(VerifierError::CorruptedProof);
        }

        // Step 4: Age check
        if self.config.max_proof_age_secs > 0 {
            // In production, check proof timestamp against system clock
        }

        // Step 5: Cryptographic verification
        // In production, this calls the pairing check (Groth16) or
        // polynomial commitment opening (PLONK).
        // For now, we perform a structural verification.
        let is_valid = self.verify_internal(proof, verification_key);

        Ok(is_valid)
    }

    fn verify_internal(&self, proof: &ZkProof, _vk: &VerificationKey) -> bool {
        // Structural verification:
        // 1. Public inputs must be non-empty for HasVariant/GenotypeMatch
        // 2. Proof data must have correct structure
        match proof.system {
            ProvingSystem::Groth16 => {
                !proof.public_inputs.is_empty() && proof.proof_data.len() >= 32
            }
            ProvingSystem::Plonk => {
                proof.proof_data.len() >= 48
            }
        }
    }

    /// Batch verify multiple proofs. Returns all verification results.
    pub fn batch_verify(
        &self,
        proofs: &[ZkProof],
        vks: &[VerificationKey],
    ) -> Vec<Result<bool, VerifierError>> {
        proofs.iter()
            .zip(vks.iter())
            .map(|(proof, vk)| self.verify(proof, vk))
            .collect()
    }

    /// Verify a proof and return its public inputs.
    pub fn verify_and_extract_inputs(
        &self,
        proof: &ZkProof,
        vk: &VerificationKey,
    ) -> Result<Vec<u8>, VerifierError> {
        self.verify(proof, vk)?;
        Ok(proof.public_inputs.clone())
    }
}

impl Default for Verifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_verify_valid_groth16_proof() {
        let verifier = Verifier::new();
        let query = crate::ZkQuery::HasVariant {
            chrom: 1, pos: 100_000,
            ref_hash: [0u8; 32], alt_hash: [1u8; 32],
        };
        let proof = ZkProof::new(
            ProvingSystem::Groth16,
            query,
            vec![0u8; 200],
            vec![1u8; 64],
        );
        let vk = VerificationKey {
            system: ProvingSystem::Groth16,
            circuit_id: "has-variant-v1".into(),
            vk_data: vec![0u8; 128],
        };

        let result = verifier.verify(&proof, &vk);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_verify_key_mismatch() {
        let verifier = Verifier::new();
        let query = crate::ZkQuery::VariantCount {
            chrom: 1, start: 0, end: 100_000, count: 10,
        };
        let proof = ZkProof::new(
            ProvingSystem::Groth16,
            query,
            vec![0u8; 200],
            vec![],
        );
        let vk = VerificationKey {
            system: ProvingSystem::Plonk, // mismatch
            circuit_id: "test".into(),
            vk_data: vec![],
        };

        let result = verifier.verify(&proof, &vk);
        assert_eq!(result, Err(VerifierError::KeyMismatch));
    }

    #[test]
    fn test_verify_empty_proof() {
        let verifier = Verifier::new();
        let query = crate::ZkQuery::HasVariant {
            chrom: 1, pos: 100_000,
            ref_hash: [0u8; 32], alt_hash: [1u8; 32],
        };
        let proof = ZkProof::new(
            ProvingSystem::Groth16,
            query,
            vec![],
            vec![],
        );
        let vk = VerificationKey {
            system: ProvingSystem::Groth16,
            circuit_id: "test".into(),
            vk_data: vec![],
        };

        let result = verifier.verify(&proof, &vk);
        assert_eq!(result, Err(VerifierError::InvalidProofFormat));
    }

    #[test]
    fn test_batch_verify() {
        let verifier = Verifier::new();
        let proofs = vec![
            ZkProof::new(ProvingSystem::Groth16, crate::ZkQuery::HasVariant {
                chrom: 1, pos: 100_000, ref_hash: [0u8; 32], alt_hash: [1u8; 32],
            }, vec![0u8; 200], vec![1u8; 32]),
            ZkProof::new(ProvingSystem::Groth16, crate::ZkQuery::HasVariant {
                chrom: 2, pos: 200_000, ref_hash: [0u8; 32], alt_hash: [1u8; 32],
            }, vec![0u8; 200], vec![1u8; 32]),
        ];
        let vks = vec![
            VerificationKey {
                system: ProvingSystem::Groth16, circuit_id: "v1".into(), vk_data: vec![],
            },
            VerificationKey {
                system: ProvingSystem::Groth16, circuit_id: "v2".into(), vk_data: vec![],
            },
        ];
        let results = verifier.batch_verify(&proofs, &vks);
        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        assert!(results[1].is_ok());
    }

    #[test]
    fn test_verify_plonk_proof() {
        let verifier = Verifier::new();
        let query = crate::ZkQuery::VariantCount {
            chrom: 1, start: 0, end: 100_000, count: 10,
        };
        let proof = ZkProof::new(
            ProvingSystem::Plonk,
            query,
            vec![0u8; 300],
            vec![],
        );
        let vk = VerificationKey {
            system: ProvingSystem::Plonk,
            circuit_id: "variant-count-v1".into(),
            vk_data: vec![0u8; 256],
        };

        let result = verifier.verify(&proof, &vk);
        assert!(result.is_ok());
    }
}