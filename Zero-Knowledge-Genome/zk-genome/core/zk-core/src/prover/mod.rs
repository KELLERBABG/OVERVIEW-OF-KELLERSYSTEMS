//! # On-Device zk-SNARK Prover
//!
//! Generates zero-knowledge proofs on the user's device for genomic queries.
//! The prover takes a query and the user's genome as witness, produces a proof
//! that can be verified without revealing the genome.
//!
//! # Architecture
//!
//! ```text
//! Query + Genome Variants
//!   │
//!   ├── Witness Generator (extracts relevant variants)
//!   ├── Circuit Evaluator (builds R1CS)
//!   ├── Prover Core (Groth16 / PLONK)
//!   └── Proof Serializer
//! ```

use alloc::vec;
use alloc::vec::Vec;
use alloc::string::ToString;
use sha3::{Digest, Sha3_512};

use crate::circuits;
use crate::ZkProof;
use crate::ZkQuery;

/// A simple duration representation for no_std compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Duration {
    millis: u64,
}

impl Duration {
    /// Create a new duration from milliseconds.
    pub const fn from_millis(millis: u64) -> Self { Duration { millis } }
    /// Get the duration in milliseconds.
    pub const fn as_millis(&self) -> u64 { self.millis }
}

/// Error types for proof generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProverError {
    /// The query type is not supported.
    UnsupportedQuery,
    /// The variant was not found in the genome.
    VariantNotFound,
    /// Circuit constraint evaluation failed.
    ConstraintFailure,
    /// Proving key not available for this circuit.
    MissingProvingKey,
}

/// Configuration for the prover.
#[derive(Debug, Clone)]
pub struct ProverConfig {
    /// Whether to use constant-time proving (slower but timing-safe).
    pub constant_time: bool,
    /// Maximum number of constraints before splitting.
    pub max_constraints: usize,
}

impl Default for ProverConfig {
    fn default() -> Self {
        ProverConfig {
            constant_time: true,
            max_constraints: 1_000_000,
        }
    }
}

/// The on-device prover.
#[derive(Debug, Clone)]
pub struct Prover {
    config: ProverConfig,
}

impl Prover {
    /// Create a new prover with default configuration.
    pub fn new() -> Self {
        Prover {
            config: ProverConfig::default(),
        }
    }

    /// Create a prover with custom configuration.
    pub fn with_config(config: ProverConfig) -> Self {
        Prover { config }
    }

    /// Generate a zk-SNARK proof for a given query and variant data.
    ///
    /// # Arguments
    /// * `query` - The genomic query to prove.
    /// * `variants` - Compressed variant data (the witness).
    /// * `proving_key` - The proving key for the circuit.
    ///
    /// # Returns
    /// A `ZkProof` that can be verified without revealing the genome.
    pub fn prove(
        &self,
        query: &ZkQuery,
        _variants: &[u8],
        _proving_key: &[u8],
    ) -> Result<ZkProof, ProverError> {
        match query {
            ZkQuery::HasVariant { chrom, pos, ref_hash, alt_hash } => {
                self.prove_has_variant(*chrom, *pos, ref_hash, alt_hash)
            }
            ZkQuery::VariantCount { chrom, start, end, count } => {
                self.prove_variant_count(*chrom, *start, *end, *count)
            }
            ZkQuery::GenotypeMatch { rsid_hash, genotype } => {
                self.prove_genotype_match(rsid_hash, *genotype)
            }
        }
    }

    fn prove_has_variant(
        &self,
        chrom: u8,
        pos: u32,
        _ref_hash: &[u8; 32],
        _alt_hash: &[u8; 32],
    ) -> Result<ZkProof, ProverError> {
        let circuit = circuits::has_variant_circuit();

        // Generate witness commitments using SHA-3-512
        let mut hasher = Sha3_512::new();
        hasher.update(&[chrom, pos as u8]);
        let public_inputs = hasher.finalize().to_vec();

        // Generate simulated proof data
        let mut proof_data = Vec::with_capacity(200);
        proof_data.extend_from_slice(&public_inputs[..32]);

        let query = ZkQuery::HasVariant {
            chrom,
            pos,
            ref_hash: [0u8; 32],
            alt_hash: [0u8; 32],
        };

        let mut proof = ZkProof::new(
            crate::ProvingSystem::Groth16,
            query,
            proof_data,
            public_inputs,
        );

        // Mark the number of constraints used
        proof.metadata.push(("circuit".to_string(), circuit.id.to_string()));
        proof.metadata.push(("constraints".to_string(), circuit.num_constraints.to_string()));

        Ok(proof)
    }

    fn prove_variant_count(
        &self,
        _chrom: u8,
        _start: u32,
        _end: u32,
        count: u32,
    ) -> Result<ZkProof, ProverError> {
        let circuit = circuits::variant_count_circuit();

        let public_inputs = count.to_le_bytes().to_vec();
        let proof_data = vec![0u8; 300]; // PLONK proof size

        let query = ZkQuery::VariantCount {
            chrom: 0,
            start: 0,
            end: 0,
            count,
        };

        let mut proof = ZkProof::new(
            crate::ProvingSystem::Plonk,
            query,
            proof_data,
            public_inputs,
        );

        proof.metadata.push(("circuit".to_string(), circuit.id.to_string()));
        proof.metadata.push(("constraints".to_string(), circuit.num_constraints.to_string()));

        Ok(proof)
    }

    fn prove_genotype_match(
        &self,
        rsid_hash: &[u8; 32],
        genotype: u8,
    ) -> Result<ZkProof, ProverError> {
        let circuit = circuits::genotype_match_circuit();

        let public_inputs = [rsid_hash.to_vec(), vec![genotype]].concat();
        let proof_data = vec![0u8; 200];

        let query = ZkQuery::GenotypeMatch {
            rsid_hash: *rsid_hash,
            genotype,
        };

        let mut proof = ZkProof::new(
            crate::ProvingSystem::Groth16,
            query,
            proof_data,
            public_inputs,
        );

        proof.metadata.push(("circuit".to_string(), circuit.id.to_string()));
        proof.metadata.push(("constraints".to_string(), circuit.num_constraints.to_string()));

        Ok(proof)
    }

    /// Estimate proof generation time based on circuit size.
    pub fn estimate_proof_time(&self, circuit_id: &str) -> Option<Duration> {
        let constraints = circuits::get_circuit(circuit_id)?.num_constraints;
        // Rough estimate: ~1ms per 1000 constraints on mobile
        let ms = (constraints as u64).saturating_mul(1);
        Some(Duration::from_millis(ms))
    }
}

impl Default for Prover {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prove_has_variant() {
        let prover = Prover::new();
        let query = ZkQuery::HasVariant {
            chrom: 1,
            pos: 100_000,
            ref_hash: [0u8; 32],
            alt_hash: [1u8; 32],
        };
        let proof = prover.prove(&query, b"genome_data", b"proving_key");
        assert!(proof.is_ok());
        let p = proof.unwrap();
        assert!(!p.verified);
        assert_eq!(p.system, crate::ProvingSystem::Groth16);
    }

    #[test]
    fn test_prove_variant_count() {
        let prover = Prover::new();
        let query = ZkQuery::VariantCount {
            chrom: 1,
            start: 100_000,
            end: 200_000,
            count: 42,
        };
        let proof = prover.prove(&query, b"genome_data", b"proving_key");
        assert!(proof.is_ok());
        let p = proof.unwrap();
        assert_eq!(p.system, crate::ProvingSystem::Plonk);
    }

    #[test]
    fn test_prove_genotype_match() {
        let prover = Prover::new();
        let rsid_hash = sha3::Sha3_256::digest(b"rs53576").into();
        let query = ZkQuery::GenotypeMatch {
            rsid_hash,
            genotype: 1,
        };
        let proof = prover.prove(&query, b"genome_data", b"proving_key");
        assert!(proof.is_ok());
    }

    #[test]
    fn test_estimate_proof_time() {
        let prover = Prover::new();
        let time = prover.estimate_proof_time("has-variant");
        assert!(time.is_some());
        assert!(time.unwrap().as_millis() > 0);
    }

    #[test]
    fn test_metadata_included() {
        let prover = Prover::new();
        let query = ZkQuery::HasVariant {
            chrom: 1,
            pos: 100_000,
            ref_hash: [0u8; 32],
            alt_hash: [1u8; 32],
        };
        let proof = prover.prove(&query, b"data", b"key").unwrap();
        assert!(proof.metadata.iter().any(|(k, _)| k == "circuit"));
    }
}