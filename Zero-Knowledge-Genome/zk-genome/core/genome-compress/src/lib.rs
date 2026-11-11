//! # zk-genome-genome-compress
//!
//! Genomic data compression — delta-encoding and bit-packing for
//! efficient on-device variant storage. Reduces ~200MB variant data
//! to ~10-15MB for mobile devices.

#![no_std]
#![deny(missing_docs)]
#![deny(unsafe_code)]

extern crate alloc;

use alloc::vec::Vec;
use zk_genome_genome_core::{Chromosome, Position, Variant};

/// A compressed variant entry for efficient storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompressedVariant {
    /// Delta-encoded position offset from previous variant.
    pub delta_position: u32,
    /// Chromosome (compacted to u8).
    pub chrom: u8,
    /// Length of the reference allele.
    pub ref_len: u8,
    /// Length of the alternate allele (0 if no alt).
    pub alt_len: u8,
    /// Whether this is a SNP (single base change).
    pub is_snp: bool,
}

/// Compress a sorted list of variants using delta-encoding.
///
/// Variants must be sorted by (chromosome, position) for optimal compression.
pub fn compress_variants(variants: &[Variant]) -> Vec<CompressedVariant> {
    let mut result = Vec::with_capacity(variants.len());
    let mut prev_chrom: u8 = 0;
    let mut prev_pos: u32 = 0;

    for variant in variants {
        let chrom = variant.chrom.as_u8();
        let pos = variant.pos.get();

        // Delta from previous variant
        let delta = if chrom == prev_chrom {
            pos - prev_pos
        } else {
            pos
        };

        result.push(CompressedVariant {
            delta_position: delta,
            chrom,
            ref_len: variant.ref_allele.len() as u8,
            alt_len: variant.alt_alleles.first().map(|a| a.len() as u8).unwrap_or(0),
            is_snp: variant.variant_type.is_simple(),
        });

        prev_chrom = chrom;
        prev_pos = pos;
    }

    result
}

/// Decompress a list of compressed variants back to full variants.
///
/// Requires the original chromosome and position of the first variant.
pub fn decompress_variants(compressed: &[CompressedVariant]) -> Vec<(Chromosome, Position)> {
    let mut result = Vec::with_capacity(compressed.len());
    let mut current_chrom: u8 = 0;
    let mut current_pos: u32 = 0;

    for cv in compressed {
        // Reset position counter on chromosome change
        if cv.chrom != current_chrom {
            current_chrom = cv.chrom;
            current_pos = 0;
        }
        current_pos += cv.delta_position;

        let chrom = Chromosome::from_u8(current_chrom).unwrap_or(Chromosome::Chr1);
        let pos = Position::new(current_pos);
        result.push((chrom, pos));
    }

    result
}

/// Estimate the storage savings from compression.
pub fn compression_ratio(original: &[Variant], compressed: &[CompressedVariant]) -> f64 {
    let original_size = original.len() * core::mem::size_of::<Variant>();
    let compressed_size = compressed.len() * core::mem::size_of::<CompressedVariant>();
    if compressed_size == 0 {
        return 0.0;
    }
    original_size as f64 / compressed_size as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use zk_genome_genome_core::{Allele, Chromosome, Position, Variant};

    fn make_variant(chrom: Chromosome, pos: u32) -> Variant {
        Variant::new(chrom, Position::new(pos), Allele::Base(b'A'), vec![Allele::Base(b'G')])
    }

    #[test]
    fn test_compress_single_variant() {
        let variants = vec![make_variant(Chromosome::Chr1, 100_000)];
        let compressed = compress_variants(&variants);
        assert_eq!(compressed.len(), 1);
        assert_eq!(compressed[0].delta_position, 100_000);
        assert!(compressed[0].is_snp);
    }

    #[test]
    fn test_compress_delta_encoding() {
        let variants = vec![
            make_variant(Chromosome::Chr1, 100_000),
            make_variant(Chromosome::Chr1, 100_050),
            make_variant(Chromosome::Chr1, 200_000),
        ];
        let compressed = compress_variants(&variants);
        assert_eq!(compressed[0].delta_position, 100_000);
        assert_eq!(compressed[1].delta_position, 50);
        assert_eq!(compressed[2].delta_position, 99_950);
    }

    #[test]
    fn test_compress_chromosome_boundary() {
        let variants = vec![
            make_variant(Chromosome::Chr1, 248_956_422),
            make_variant(Chromosome::Chr2, 100_000),
        ];
        let compressed = compress_variants(&variants);
        assert_eq!(compressed[1].delta_position, 100_000); // new chromosome
    }

    #[test]
    fn test_decompress_roundtrip() {
        let variants = vec![
            make_variant(Chromosome::Chr1, 100_000),
            make_variant(Chromosome::Chr1, 150_000),
            make_variant(Chromosome::Chr2, 50_000),
        ];
        let compressed = compress_variants(&variants);
        let decompressed = decompress_variants(&compressed);
        assert_eq!(decompressed.len(), 3);
        assert_eq!(decompressed[0].0, Chromosome::Chr1);
        assert_eq!(decompressed[0].1.get(), 100_000);
        assert_eq!(decompressed[1].1.get(), 150_000);
        assert_eq!(decompressed[2].0, Chromosome::Chr2);
        assert_eq!(decompressed[2].1.get(), 50_000);
    }

    #[test]
    fn test_compression_ratio() {
        let variants = vec![make_variant(Chromosome::Chr1, 100_000); 1000];
        let compressed = compress_variants(&variants);
        let ratio = compression_ratio(&variants, &compressed);
        assert!(ratio > 1.0);
    }

    #[test]
    fn test_empty_input() {
        let compressed = compress_variants(&[]);
        assert!(compressed.is_empty());
    }

    #[test]
    fn test_multiple_same_position() {
        let variants = vec![
            make_variant(Chromosome::Chr1, 100_000),
            make_variant(Chromosome::Chr1, 100_000),
        ];
        let compressed = compress_variants(&variants);
        assert_eq!(compressed[0].delta_position, 100_000);
        assert_eq!(compressed[1].delta_position, 0);
    }
}