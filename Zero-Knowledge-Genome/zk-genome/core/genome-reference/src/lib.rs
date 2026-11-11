//! # zk-genome-genome-reference
//!
//! Reference genome support — coordinate mapping for hg19, hg38, and T2T-CHM13.
//!
//! Provides chromosome lengths, coordinate conversions, reference allele
//! lookups, and metadata for supported reference genome assemblies.

#![no_std]
#![deny(missing_docs)]
#![deny(unsafe_code)]

extern crate alloc;

use alloc::string::ToString;
use zk_genome_genome_core::{Chromosome, ReferenceGenome};

/// Get the length of a chromosome in a given reference assembly.
pub fn chromosome_length(ref_genome: ReferenceGenome, chrom: Chromosome) -> Option<u32> {
    ref_genome.chromosome_length(chrom)
}

/// Get the total span covered by a reference genome assembly.
pub fn total_genome_length(ref_genome: ReferenceGenome) -> u64 {
    Chromosome::ALL
        .iter()
        .filter_map(|c| ref_genome.chromosome_length(*c))
        .map(u64::from)
        .sum()
}

/// Check if a position is valid for the given reference genome.
pub fn is_valid_position(ref_genome: ReferenceGenome, chrom: Chromosome, pos: u32) -> bool {
    ref_genome
        .chromosome_length(chrom)
        .map(|len| pos > 0 && pos <= len)
        .unwrap_or(false)
}

/// Convert a contig name between different reference genome conventions.
///
/// e.g., "chr1" -> "1", "1" -> "chr1", "chrX" -> "X"
pub fn normalize_contig_name(name: &str, target_prefix: &str) -> alloc::string::String {
    let stripped = name.strip_prefix("chr").or_else(|| name.strip_prefix("CHR")).unwrap_or(name);
    if target_prefix == "chr" {
        alloc::format!("chr{}", stripped)
    } else {
        stripped.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zk_genome_genome_core::ReferenceGenome;

    #[test]
    fn test_chromosome_length_hg38() {
        let len = chromosome_length(ReferenceGenome::Hg38, Chromosome::Chr1);
        assert_eq!(len, Some(248_956_422));
    }

    #[test]
    fn test_chromosome_length_hg19() {
        let len = chromosome_length(ReferenceGenome::Hg19, Chromosome::ChrMT);
        assert_eq!(len, Some(16_569));
    }

    #[test]
    fn test_total_genome_length() {
        let total = total_genome_length(ReferenceGenome::Hg38);
        assert!(total > 3_000_000_000); // ~3.1 billion bp
    }

    #[test]
    fn test_is_valid_position() {
        assert!(is_valid_position(ReferenceGenome::Hg38, Chromosome::Chr1, 1));
        assert!(is_valid_position(ReferenceGenome::Hg38, Chromosome::Chr1, 248_956_422));
        assert!(!is_valid_position(ReferenceGenome::Hg38, Chromosome::Chr1, 0));
        assert!(!is_valid_position(ReferenceGenome::Hg38, Chromosome::Chr1, 248_956_423));
    }

    #[test]
    fn test_normalize_contig_add_prefix() {
        let result = normalize_contig_name("1", "chr");
        assert_eq!(result, "chr1");
    }

    #[test]
    fn test_normalize_contig_remove_prefix() {
        let result = normalize_contig_name("chrX", "");
        assert_eq!(result, "X");
    }

    #[test]
    fn test_is_valid_position_wrong_chromosome() {
        assert_eq!(is_valid_position(ReferenceGenome::Hg38, Chromosome::ChrMT, 100_000), false);
    }
}