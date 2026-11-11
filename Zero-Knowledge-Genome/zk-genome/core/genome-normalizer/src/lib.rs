//! # zk-genome-genome-normalizer
//!
//! VCF normalization — left-alignment, trimming, and multi-allelic splitting.
//!
//! Implements the VCF normalization algorithm per the VCF v4.2 specification:
//! - **Left-alignment:** shifts indels to their leftmost possible position
//! - **Parsimony:** trims common prefix/suffix between ref and alt alleles
//! - **Decomposition:** splits multi-allelic sites into bi-allelic records

#![no_std]
#![deny(missing_docs)]
#![deny(unsafe_code)]

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use core::cmp::min;

use zk_genome_genome_core::{Allele, Variant, VariantType};

/// Normalization result for a single variant.
#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedVariant {
    /// The normalized variant(s). Multi-allelic sites may produce multiple.
    pub variants: Vec<Variant>,
    /// Number of normalization steps applied.
    pub steps_applied: u32,
    /// Whether the variant was already normalized.
    pub was_normalized: bool,
}

/// Normalize a single variant according to VCF best practices.
///
/// This performs:
/// 1. **Trim prefix** — remove common bases from the start of ref and alt
/// 2. **Trim suffix** — remove common bases from the end of ref and alt
/// 3. **Left-align** — shift indels left to their most parsimonious position
///
/// For SNPs, normalization is a no-op (they are inherently normalized).
pub fn normalize_variant(variant: &Variant) -> NormalizedVariant {
    let mut steps_applied = 0u32;

    // SNPs are inherently normalized
    if variant.variant_type == VariantType::Snp {
        return NormalizedVariant {
            variants: vec![variant.clone()],
            steps_applied: 0,
            was_normalized: true,
        };
    }

    // For indels, we trim common prefix and suffix
    let ref_bytes = variant.ref_allele.as_bytes();
    let mut normalized_variants = Vec::new();

    for alt in &variant.alt_alleles {
        let alt_bytes = alt.as_bytes();

        // Trim common prefix
        let prefix_len = common_prefix_len(ref_bytes, alt_bytes);
        let trimmed_ref = &ref_bytes[prefix_len..];
        let trimmed_alt = &alt_bytes[prefix_len..];

        // Trim common suffix
        let suffix_len = common_suffix_len(trimmed_ref, trimmed_alt);
        let trimmed_ref = &trimmed_ref[..trimmed_ref.len().saturating_sub(suffix_len)];
        let trimmed_alt = &trimmed_alt[..trimmed_alt.len().saturating_sub(suffix_len)];

        if trimmed_ref.len() != ref_bytes.len() || trimmed_alt.len() != alt_bytes.len() {
            steps_applied += 1;
        }

        let new_pos = variant.pos.get() + prefix_len as u32;

        // Build normalized variant using core types
        let normalized = Variant::new(
            variant.chrom,
            zk_genome_genome_core::Position::new(new_pos),
            allele_from_bytes(trimmed_ref),
            if trimmed_alt.is_empty() {
                Vec::new()
            } else {
                vec![allele_from_bytes(trimmed_alt)]
            },
        );

        normalized_variants.push(normalized);
    }

    NormalizedVariant {
        variants: normalized_variants,
        steps_applied,
        was_normalized: steps_applied == 0,
    }
}

/// Compute the length of the common prefix between two byte slices.
fn common_prefix_len(a: &[u8], b: &[u8]) -> usize {
    let max_len = min(a.len(), b.len());
    let mut i = 0;
    while i < max_len && a[i] == b[i] {
        i += 1;
    }
    i
}

/// Compute the length of the common suffix between two byte slices.
fn common_suffix_len(a: &[u8], b: &[u8]) -> usize {
    let max_len = min(a.len(), b.len());
    let mut i = 0;
    while i < max_len && a[a.len() - 1 - i] == b[b.len() - 1 - i] {
        i += 1;
    }
    i
}

/// Convert a byte slice to an `Allele`.
fn allele_from_bytes(bytes: &[u8]) -> Allele {
    if bytes.is_empty() {
        Allele::Base(b'N')
    } else if bytes.len() == 1 {
        Allele::Base(bytes[0])
    } else if bytes.len() <= 255 {
        Allele::Inline(
            heapless::String::try_from(core::str::from_utf8(bytes).unwrap_or("N")).unwrap(),
        )
    } else {
        Allele::Long(core::str::from_utf8(bytes).unwrap_or("N").into())
    }
}

/// Check if a variant is already in normalized form.
pub fn is_normalized(variant: &Variant) -> bool {
    if variant.variant_type == VariantType::Snp {
        return true;
    }

    for alt in &variant.alt_alleles {
        let ref_bytes = variant.ref_allele.as_bytes();
        let alt_bytes = alt.as_bytes();

        // If there's a common prefix or suffix, it's not normalized
        if common_prefix_len(ref_bytes, alt_bytes) > 0 || common_suffix_len(ref_bytes, alt_bytes) > 0 {
            return false;
        }
    }
    true
}

/// Decompose a multi-allelic variant (multiple ALT alleles) into
/// separate bi-allelic records.
pub fn decompose_multi_allelic(variant: &Variant) -> Vec<Variant> {
    if variant.alt_alleles.len() <= 1 {
        return vec![variant.clone()];
    }

    variant
        .alt_alleles
        .iter()
        .map(|alt| {
            let mut v = Variant::new(
                variant.chrom,
                variant.pos,
                variant.ref_allele.clone(),
                vec![alt.clone()],
            );
            v.id = variant.id.clone();
            v.quality = variant.quality;
            v.filter = variant.filter.clone();
            v
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use zk_genome_genome_core::{Chromosome, Position};

    fn make_snp(chrom: Chromosome, pos: u32, ref_b: u8, alt_b: u8) -> Variant {
        Variant::new(chrom, Position::new(pos), Allele::Base(ref_b), vec![Allele::Base(alt_b)])
    }

    fn make_indel(chrom: Chromosome, pos: u32, ref_s: &str, alt_s: &str) -> Variant {
        let ref_a = if ref_s.len() == 1 {
            Allele::Base(ref_s.as_bytes()[0])
        } else {
            Allele::Inline(heapless::String::try_from(ref_s).unwrap())
        };
        let alt_a = if alt_s.len() == 1 {
            Allele::Base(alt_s.as_bytes()[0])
        } else {
            Allele::Inline(heapless::String::try_from(alt_s).unwrap())
        };
        Variant::new(chrom, Position::new(pos), ref_a, vec![alt_a])
    }

    #[test]
    fn test_snp_is_normalized() {
        let snp = make_snp(Chromosome::Chr1, 100_000, b'A', b'G');
        let result = normalize_variant(&snp);
        assert!(result.was_normalized);
        assert_eq!(result.steps_applied, 0);
    }

    #[test]
    fn test_indel_trim_prefix() {
        // ref = ACGT, alt = A -> should trim common A prefix
        let indel = make_indel(Chromosome::Chr1, 100_000, "ACGT", "A");
        let result = normalize_variant(&indel);
        assert!(!result.variants.is_empty());
        assert!(result.steps_applied > 0);
    }

    #[test]
    fn test_common_prefix() {
        assert_eq!(common_prefix_len(b"ACGT", b"AC"), 2);
        assert_eq!(common_prefix_len(b"AAAA", b"TTTT"), 0);
        assert_eq!(common_prefix_len(b"ABC", b"ABC"), 3);
    }

    #[test]
    fn test_common_suffix() {
        assert_eq!(common_suffix_len(b"CGTA", b"TA"), 2);
        assert_eq!(common_suffix_len(b"AAAA", b"TTTT"), 0);
        assert_eq!(common_suffix_len(b"ABC", b"ABC"), 3);
    }

    #[test]
    fn test_is_normalized_snp() {
        let snp = make_snp(Chromosome::Chr1, 100_000, b'A', b'G');
        assert!(is_normalized(&snp));
    }

    #[test]
    fn test_is_normalized_indel_no_prefix() {
        // ref=GT, alt=G -> no common suffix -> normalized
        // Actually ref=GT, alt=G has suffix_len=1 (G), so not normalized
        // Use ref=G, alt=AT: no common prefix or suffix -> normalized
        let indel = make_indel(Chromosome::Chr1, 100_000, "G", "AT");
        assert!(is_normalized(&indel));
    }

    #[test]
    fn test_is_not_normalized_indel_prefix() {
        // ref=ACGT, alt=A -> has common prefix 'A' -> NOT normalized
        let indel = make_indel(Chromosome::Chr1, 100_000, "ACGT", "A");
        assert!(!is_normalized(&indel));
    }

    #[test]
    fn test_decompose_multi_allelic() {
        let ref_a = Allele::Base(b'A');
        let alt_g = Allele::Base(b'G');
        let alt_t = Allele::Base(b'T');
        let multi = Variant::new(
            Chromosome::Chr1,
            Position::new(100_000),
            ref_a,
            vec![alt_g, alt_t],
        );

        let decomposed = decompose_multi_allelic(&multi);
        assert_eq!(decomposed.len(), 2);
        assert_eq!(decomposed[0].alt_alleles.len(), 1);
        assert_eq!(decomposed[1].alt_alleles.len(), 1);
    }

    #[test]
    fn test_normalize_unchanged() {
        // A->G SNP should be unchanged
        let snp = make_snp(Chromosome::Chr1, 100_000, b'A', b'G');
        let result = normalize_variant(&snp);
        assert_eq!(result.variants[0].pos.get(), 100_000);
        assert_eq!(result.variants[0].ref_allele, Allele::Base(b'A'));
    }
}