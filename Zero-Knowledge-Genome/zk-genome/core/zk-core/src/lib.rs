//! # zk-genome-zk-core
//!
//! Zero-knowledge proof foundations — circuit types, proof generation,
//! and verification infrastructure for genomic queries.

#![no_std]
#![deny(missing_docs)]
#![deny(unsafe_code)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

pub mod circuits;
pub mod prover;
pub mod verifier;
pub mod query_crypto;
pub mod differential_privacy;
pub mod proof_checkpoint;
pub mod phenotype;

/// Supported proving systems for zero-knowledge proofs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvingSystem {
    /// Groth16 — fastest verification, requires per-circuit trusted setup
    Groth16,
    /// PLONK — universal trusted setup, larger proofs
    Plonk,
}

/// A genomic query that can be converted to a zk-SNARK circuit.
#[derive(Debug, Clone, PartialEq)]
pub enum ZkQuery {
    /// Prove that a variant exists at (chrom, pos) with ref/alt alleles.
    HasVariant {
        /// Chromosome as u8 (1-25).
        chrom: u8,
        /// 1-based position.
        pos: u32,
        /// Reference allele hash.
        ref_hash: [u8; 32],
        /// Alternate allele hash.
        alt_hash: [u8; 32],
    },
    /// Prove the number of variants in a genomic range.
    VariantCount {
        /// Chromosome as u8.
        chrom: u8,
        /// Start of range.
        start: u32,
        /// End of range.
        end: u32,
        /// The claimed count.
        count: u32,
    },
    /// Prove a specific genotype at a known rsID.
    GenotypeMatch {
        /// rsID hash.
        rsid_hash: [u8; 32],
        /// Genotype: 0=hom-ref, 1=het, 2=hom-alt.
        genotype: u8,
    },
}

/// A zk-SNARK proof generated on-device.
#[derive(Debug, Clone, PartialEq)]
pub struct ZkProof {
    /// The proving system used.
    pub system: ProvingSystem,
    /// The query this proof was generated for.
    pub query: ZkQuery,
    /// The proof bytes.
    pub proof_data: Vec<u8>,
    /// Public inputs revealed by the proof.
    pub public_inputs: Vec<u8>,
    /// Whether this proof has been verified.
    pub verified: bool,
    /// Metadata key-value pairs.
    pub metadata: Vec<(String, String)>,
}

impl ZkProof {
    /// Create a new unverified proof.
    pub fn new(system: ProvingSystem, query: ZkQuery, proof_data: Vec<u8>, public_inputs: Vec<u8>) -> Self {
        ZkProof { system, query, proof_data, public_inputs, verified: false, metadata: Vec::new() }
    }

    /// Mark this proof as verified.
    pub fn verify(&mut self) { self.verified = true; }

    /// Get metadata value by key.
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.metadata.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }
}

/// A verification key for checking zk proofs.
#[derive(Debug, Clone, PartialEq)]
pub struct VerificationKey {
    /// The proving system.
    pub system: ProvingSystem,
    /// The circuit identifier.
    pub circuit_id: String,
    /// The verification key bytes.
    pub vk_data: Vec<u8>,
}

/// A proving key for generating zk proofs on-device.
#[derive(Debug, Clone, PartialEq)]
pub struct ProvingKey {
    /// The proving system.
    pub system: ProvingSystem,
    /// The circuit identifier.
    pub circuit_id: String,
    /// The proving key bytes.
    pub pk_data: Vec<u8>,
}

/// Groth16 proof size.
pub const fn groth16_proof_size() -> usize { 200 }
/// PLONK proof size.
pub const fn plonk_proof_size() -> usize { 300 }

/// Estimate the constraint count for a HasVariant circuit.
pub fn estimate_has_variant_constraints(chrom_bits: u8, pos_bits: u8) -> u32 {
    1000 + (chrom_bits as u32) * 100 + (pos_bits as u32) * 50
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_create_has_variant_proof() {
        let query = ZkQuery::HasVariant { chrom: 1, pos: 100_000, ref_hash: [0u8; 32], alt_hash: [1u8; 32] };
        let proof = ZkProof::new(ProvingSystem::Groth16, query, vec![0u8; groth16_proof_size()], vec![1u8; 64]);
        assert!(!proof.verified);
        assert_eq!(proof.system, ProvingSystem::Groth16);
    }

    #[test]
    fn test_metadata() {
        let mut proof = ZkProof::new(ProvingSystem::Groth16, ZkQuery::HasVariant { chrom: 1, pos: 100_000, ref_hash: [0u8; 32], alt_hash: [1u8; 32] }, vec![0u8; 200], vec![1u8; 64]);
        proof.metadata.push(("circuit".into(), "has-variant".into()));
        assert_eq!(proof.get_metadata("circuit"), Some("has-variant"));
    }
}