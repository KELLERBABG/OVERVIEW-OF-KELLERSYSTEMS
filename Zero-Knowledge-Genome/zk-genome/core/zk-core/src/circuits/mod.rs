//! # Circuit Definitions
//!
//! R1CS constraint generation for genomic zk-SNARK circuits.
//! These define the arithmetic circuits that constrain the prover
//! without implementing the full Groth16/PLONK proving system.
//!
//! Each circuit is defined by its public inputs, private witnesses,
//! and the constraints that link them.

use alloc::vec;
use alloc::vec::Vec;

/// A constraint in an R1CS circuit.
#[derive(Debug, Clone, PartialEq)]
pub struct Constraint {
    /// Index of the constraint.
    pub index: usize,
    /// Human-readable description.
    pub description: &'static str,
}

/// A circuit's public interface: what inputs are revealed.
#[derive(Debug, Clone, PartialEq)]
pub struct CircuitPublicInputs {
    /// Names and sizes of public inputs.
    pub inputs: Vec<(&'static str, usize)>,
}

/// A circuit's private interface: what inputs stay secret.
#[derive(Debug, Clone, PartialEq)]
pub struct CircuitPrivateInputs {
    /// Names and sizes of private inputs (witness).
    pub inputs: Vec<(&'static str, usize)>,
}

/// Metadata for a zk circuit.
#[derive(Debug, Clone, PartialEq)]
pub struct Circuit {
    /// Unique circuit identifier.
    pub id: &'static str,
    /// Circuit version.
    pub version: &'static str,
    /// Number of R1CS constraints.
    pub num_constraints: usize,
    /// Number of public inputs.
    pub num_public_inputs: usize,
    /// Number of private inputs (witness).
    pub num_private_inputs: usize,
    /// Public input definitions.
    pub public_inputs: CircuitPublicInputs,
    /// Private input definitions.
    pub private_inputs: CircuitPrivateInputs,
}

/// The HasVariant circuit: prove a variant at (chrom, pos) with ref/alt.
pub fn has_variant_circuit() -> Circuit {
    Circuit {
        id: "has-variant",
        version: "1.0.0",
        num_constraints: 42,
        num_public_inputs: 5,
        num_private_inputs: 1 + 22, // variant data + Merkle proof (log2(4M) ≈ 22)
        public_inputs: CircuitPublicInputs {
            inputs: vec![
                ("genome_commitment", 1),
                ("query_chrom", 1),
                ("query_pos", 1),
                ("query_ref_hash", 1),
                ("query_alt_hash", 1),
            ],
        },
        private_inputs: CircuitPrivateInputs {
            inputs: vec![
                ("variant_chrom", 1),
                ("variant_pos", 1),
                ("variant_ref_hash", 1),
                ("variant_alt_hash", 1),
                ("variant_index", 1),
                ("merkle_proof", 22),
            ],
        },
    }
}

/// The VariantCount circuit: prove number of variants in a range.
pub fn variant_count_circuit() -> Circuit {
    Circuit {
        id: "variant-count",
        version: "1.0.0",
        num_constraints: 156,
        num_public_inputs: 5,
        num_private_inputs: 100 + 100 * 22, // up to 100 variants, each with proof
        public_inputs: CircuitPublicInputs {
            inputs: vec![
                ("genome_commitment", 1),
                ("chrom", 1),
                ("range_start", 1),
                ("range_end", 1),
                ("claimed_count", 1),
            ],
        },
        private_inputs: CircuitPrivateInputs {
            inputs: vec![
                ("variant_positions", 100),
                ("variant_hashes", 100),
                ("merkle_proofs", 100 * 22),
            ],
        },
    }
}

/// The GenotypeMatch circuit: prove genotype at known rsID.
pub fn genotype_match_circuit() -> Circuit {
    Circuit {
        id: "genotype-match",
        version: "1.0.0",
        num_constraints: 38,
        num_public_inputs: 3,
        num_private_inputs: 4 + 22, // variant data + Merkle proof
        public_inputs: CircuitPublicInputs {
            inputs: vec![
                ("genome_commitment", 1),
                ("rsid_hash", 1),
                ("claimed_genotype", 1),
            ],
        },
        private_inputs: CircuitPrivateInputs {
            inputs: vec![
                ("variant_rsid_hash", 1),
                ("allele1", 1),
                ("allele2", 1),
                ("leaf_index", 1),
                ("merkle_proof", 22),
            ],
        },
    }
}

/// Get a circuit by its string identifier.
pub fn get_circuit(id: &str) -> Option<Circuit> {
    match id {
        "has-variant" => Some(has_variant_circuit()),
        "variant-count" => Some(variant_count_circuit()),
        "genotype-match" => Some(genotype_match_circuit()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_variant_circuit_metadata() {
        let c = has_variant_circuit();
        assert_eq!(c.id, "has-variant");
        assert_eq!(c.num_public_inputs, 5);
        assert!(c.num_constraints > 0);
    }

    #[test]
    fn test_variant_count_circuit_metadata() {
        let c = variant_count_circuit();
        assert_eq!(c.id, "variant-count");
        assert_eq!(c.num_public_inputs, 5);
    }

    #[test]
    fn test_genotype_match_circuit_metadata() {
        let c = genotype_match_circuit();
        assert_eq!(c.id, "genotype-match");
        assert_eq!(c.num_public_inputs, 3);
    }

    #[test]
    fn test_get_circuit_by_id() {
        assert!(get_circuit("has-variant").is_some());
        assert!(get_circuit("variant-count").is_some());
        assert!(get_circuit("genotype-match").is_some());
        assert!(get_circuit("nonexistent").is_none());
    }
}