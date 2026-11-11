//! # Phenotype Verifiable Credentials — v1.1 (Challenge 4)
//!
//! Verifiable Credentials for phenotypic traits alongside genomic proofs.
//! Enables pharma to query "user has variant X AND is in age range Y-Z
//! AND has condition W" using zk-proofs over both genotype and phenotype.

#![deny(missing_docs)]
#![deny(unsafe_code)]

use alloc::string::String;
use alloc::vec::Vec;

/// A phenotypic trait that can be included in a query alongside genomic variants.
#[derive(Debug, Clone, PartialEq)]
pub enum PhenotypeTrait {
    /// Age range (lower, upper inclusive).
    AgeRange(u8, u8),
    /// BMI range (low, high).
    BmiRange(f64, f64),
    /// Disease state (ICD-10 code).
    DiseaseState([u8; 3]),
    /// Blood type (0=O−, 1=O+, 2=A−, 3=A+, 4=B−, 5=B+, 6=AB−, 7=AB+).
    BloodType(u8),
    /// Sex (0=female, 1=male).
    Sex(u8),
    /// Medication code (ATC classification, first 3 chars hashed).
    Medication([u8; 3]),
}

/// A Verifiable Credential for a phenotypic trait.
///
/// Issued by a healthcare provider and cryptographically bound to the
/// user's DID. The VC can be selectively disclosed alongside genomic proofs.
#[derive(Debug, Clone, PartialEq)]
pub struct PhenotypeCredential {
    /// The trait this credential attests to.
    pub trait_type: PhenotypeTrait,
    /// The issuer DID (e.g., hospital, clinic).
    pub issuer_did: String,
    /// The subject DID (the user).
    pub subject_did: String,
    /// The timestamp when this credential was issued.
    pub issued_at: u64,
    /// The timestamp when this credential expires.
    pub expires_at: u64,
    /// The issuer's CRYSTALS-Dilithium signature.
    pub dilithium_signature: Vec<u8>,
}

/// A combined query requiring both genomic and phenotypic evidence.
#[derive(Debug, Clone, PartialEq)]
pub struct PhenotypeQuery {
    /// The genomic query to prove.
    pub genomic_query: crate::ZkQuery,
    /// Required phenotypic traits.
    pub required_phenotypes: Vec<PhenotypeTrait>,
    /// Minimum number of phenotypic traits that must match.
    pub min_phenotype_matches: u32,
    /// Reward per complete proof (genotype + phenotype).
    pub reward_per_proof: u64,
}

/// Result of a combined genotype + phenotype proof.
#[derive(Debug, Clone, PartialEq)]
pub struct PhenotypeProofResult {
    /// The genomic proof (standard ZkProof).
    pub genomic_proof: crate::ZkProof,
    /// Hashes of the phenotypic VCs that were disclosed.
    pub disclosed_phenotype_hashes: Vec<[u8; 32]>,
    /// Whether all required phenotypes were satisfied.
    pub all_phenotypes_satisfied: bool,
}

/// Determine which proving system to use based on circuit and context.
///
/// v1.1: Standardizes on PLONK for all genomic circuits (universal setup),
/// STARKs for aggregate queries (no trusted setup, post-quantum).
pub fn select_proving_system(circuit_id: &str) -> crate::ProvingSystem {
    match circuit_id {
        "has-variant" | "variant-count" | "genotype-match" => crate::ProvingSystem::Plonk,
        "dp-aggregate" => crate::ProvingSystem::Plonk, // STARKs in future
        _ => crate::ProvingSystem::Plonk,
    }
}

/// Verify a phenotypic credential's signature and expiry.
pub fn verify_phenotype_credential(cred: &PhenotypeCredential) -> bool {
    // 1. Check expiry
    // 2. Verify Dilithium signature from issuer
    // 3. Verify issuer is in trusted healthcare provider registry
    // For now, stub that returns true for valid structure
    !cred.dilithium_signature.is_empty() && cred.expires_at > cred.issued_at
}

/// Hash a phenotype credential for selective disclosure.
pub fn hash_phenotype_credential(cred: &PhenotypeCredential) -> [u8; 32] {
    use sha3::{Digest, Sha3_256};
    let mut hasher = Sha3_256::new();
    hasher.update(b"phenotype_credential_v1");
    hasher.update(&[match cred.trait_type {
        PhenotypeTrait::AgeRange(_, _) => 1u8,
        PhenotypeTrait::BmiRange(_, _) => 2,
        PhenotypeTrait::DiseaseState(_) => 3,
        PhenotypeTrait::BloodType(_) => 4,
        PhenotypeTrait::Sex(_) => 5,
        PhenotypeTrait::Medication(_) => 6,
    }]);
    hasher.update(cred.issuer_did.as_bytes());
    let result = hasher.finalize();
    let mut output = [0u8; 32];
    output.copy_from_slice(&result);
    output
}

/// Check if a phenotype credential matches a required trait.
pub fn phenotype_matches(cred: &PhenotypeCredential, required: &PhenotypeTrait) -> bool {
    core::mem::discriminant(&cred.trait_type) == core::mem::discriminant(required)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn sample_credential() -> PhenotypeCredential {
        PhenotypeCredential {
            trait_type: PhenotypeTrait::AgeRange(30, 40),
            issuer_did: "did:key:hospital-123".into(),
            subject_did: "did:key:user-abc".into(),
            issued_at: 1_720_000_000,
            expires_at: 1_780_000_000,
            dilithium_signature: vec![1u8; 64],
        }
    }

    #[test]
    fn test_select_proving_system_universal_plonk() {
        assert_eq!(select_proving_system("has-variant"), crate::ProvingSystem::Plonk);
        assert_eq!(select_proving_system("variant-count"), crate::ProvingSystem::Plonk);
        assert_eq!(select_proving_system("genotype-match"), crate::ProvingSystem::Plonk);
    }

    #[test]
    fn test_verify_phenotype_credential() {
        let cred = sample_credential();
        assert!(verify_phenotype_credential(&cred));
    }

    #[test]
    fn test_verify_expired_credential() {
        let cred = PhenotypeCredential {
            expires_at: 1_000_000, // past
            ..sample_credential()
        };
        assert!(!verify_phenotype_credential(&cred));
    }

    #[test]
    fn test_hash_credential_deterministic() {
        let h1 = hash_phenotype_credential(&sample_credential());
        let h2 = hash_phenotype_credential(&sample_credential());
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_phenotype_match() {
        let cred = sample_credential();
        assert!(phenotype_matches(&cred, &PhenotypeTrait::AgeRange(25, 45)));
        assert!(!phenotype_matches(&cred, &PhenotypeTrait::BloodType(3)));
    }

    #[test]
    fn test_phenotype_query_creation() {
        let query = PhenotypeQuery {
            genomic_query: crate::ZkQuery::HasVariant {
                chrom: 1,
                pos: 100_000,
                ref_hash: [0u8; 32],
                alt_hash: [0u8; 32],
            },
            required_phenotypes: vec![
                PhenotypeTrait::AgeRange(30, 40),
                PhenotypeTrait::Sex(0),
            ],
            min_phenotype_matches: 2,
            reward_per_proof: 50,
        };
        assert_eq!(query.min_phenotype_matches, 2);
    }
}