//! # zk-genome-genome-core
//!
//! Core data types for genomic data processing in the Zero-Knowledge Genome system.
//!
//! This crate defines the fundamental types used throughout the system:
//! - Chromosome representation
//! - Genomic positions and coordinates
//! - Allele and variant types (SNP, indel, CNV)
//! - Genotype representation
//! - Reference genome identification
//!
//! These types are designed for:
//! - **Post-quantum integrity**: All hashed types use SHA-3-512 via the `sha3` crate
//! - **Zero-copy parsing**: Types implement `TryFrom<&[u8]>` for efficient VCF parsing
//! - **Encrypted query matching**: Types are `Serialize`/`Deserialize` for encrypted storage
//! - **Memory efficiency**: Compact representation for mobile devices (1M+ variants)

#![no_std]
#![deny(missing_docs)]
#![deny(unsafe_code)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::fmt;
use core::str::FromStr;

/// The canonical set of human chromosomes (1–22, X, Y, MT).
///
/// Uses a compact `u8` representation internally (1–25) for efficient storage.
/// Chromosome 23 = X, 24 = Y, 25 = MT.
///
/// # Post-quantum note
/// When serialized for hashing, uses the numeric representation to avoid
/// encoding ambiguities (e.g., "chr1" vs "1").
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Chromosome {
    /// Chromosome 1
    Chr1 = 1,
    /// Chromosome 2
    Chr2 = 2,
    /// Chromosome 3
    Chr3 = 3,
    /// Chromosome 4
    Chr4 = 4,
    /// Chromosome 5
    Chr5 = 5,
    /// Chromosome 6
    Chr6 = 6,
    /// Chromosome 7
    Chr7 = 7,
    /// Chromosome 8
    Chr8 = 8,
    /// Chromosome 9
    Chr9 = 9,
    /// Chromosome 10
    Chr10 = 10,
    /// Chromosome 11
    Chr11 = 11,
    /// Chromosome 12
    Chr12 = 12,
    /// Chromosome 13
    Chr13 = 13,
    /// Chromosome 14
    Chr14 = 14,
    /// Chromosome 15
    Chr15 = 15,
    /// Chromosome 16
    Chr16 = 16,
    /// Chromosome 17
    Chr17 = 17,
    /// Chromosome 18
    Chr18 = 18,
    /// Chromosome 19
    Chr19 = 19,
    /// Chromosome 20
    Chr20 = 20,
    /// Chromosome 21
    Chr21 = 21,
    /// Chromosome 22
    Chr22 = 22,
    /// Chromosome X
    ChrX = 23,
    /// Chromosome Y
    ChrY = 24,
    /// Mitochondrial DNA
    ChrMT = 25,
}

impl Chromosome {
    /// All 25 chromosomes in canonical order.
    pub const ALL: [Chromosome; 25] = [
        Chromosome::Chr1,
        Chromosome::Chr2,
        Chromosome::Chr3,
        Chromosome::Chr4,
        Chromosome::Chr5,
        Chromosome::Chr6,
        Chromosome::Chr7,
        Chromosome::Chr8,
        Chromosome::Chr9,
        Chromosome::Chr10,
        Chromosome::Chr11,
        Chromosome::Chr12,
        Chromosome::Chr13,
        Chromosome::Chr14,
        Chromosome::Chr15,
        Chromosome::Chr16,
        Chromosome::Chr17,
        Chromosome::Chr18,
        Chromosome::Chr19,
        Chromosome::Chr20,
        Chromosome::Chr21,
        Chromosome::Chr22,
        Chromosome::ChrX,
        Chromosome::ChrY,
        Chromosome::ChrMT,
    ];

    /// Returns `true` if this is an autosome (1–22).
    #[inline]
    pub const fn is_autosome(&self) -> bool {
        (*self as u8) <= 22
    }

    /// Returns `true` if this is a sex chromosome (X or Y).
    #[inline]
    pub const fn is_sex_chromosome(&self) -> bool {
        matches!(self, Chromosome::ChrX | Chromosome::ChrY)
    }

    /// Returns `true` if this is mitochondrial DNA.
    #[inline]
    pub const fn is_mitochondrial(&self) -> bool {
        matches!(self, Chromosome::ChrMT)
    }

    /// Returns the numeric identifier (1–25).
    #[inline]
    pub const fn as_u8(&self) -> u8 {
        *self as u8
    }

    /// Create from a `u8` (1–25). Returns `None` for invalid values.
    #[inline]
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Chromosome::Chr1),
            2 => Some(Chromosome::Chr2),
            3 => Some(Chromosome::Chr3),
            4 => Some(Chromosome::Chr4),
            5 => Some(Chromosome::Chr5),
            6 => Some(Chromosome::Chr6),
            7 => Some(Chromosome::Chr7),
            8 => Some(Chromosome::Chr8),
            9 => Some(Chromosome::Chr9),
            10 => Some(Chromosome::Chr10),
            11 => Some(Chromosome::Chr11),
            12 => Some(Chromosome::Chr12),
            13 => Some(Chromosome::Chr13),
            14 => Some(Chromosome::Chr14),
            15 => Some(Chromosome::Chr15),
            16 => Some(Chromosome::Chr16),
            17 => Some(Chromosome::Chr17),
            18 => Some(Chromosome::Chr18),
            19 => Some(Chromosome::Chr19),
            20 => Some(Chromosome::Chr20),
            21 => Some(Chromosome::Chr21),
            22 => Some(Chromosome::Chr22),
            23 => Some(Chromosome::ChrX),
            24 => Some(Chromosome::ChrY),
            25 => Some(Chromosome::ChrMT),
            _ => None,
        }
    }
}

impl fmt::Display for Chromosome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Chromosome::ChrX => write!(f, "X"),
            Chromosome::ChrY => write!(f, "Y"),
            Chromosome::ChrMT => write!(f, "MT"),
            _ => write!(f, "{}", *self as u8),
        }
    }
}

/// Error type for chromosome parsing failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromosomeParseError(pub String);

impl fmt::Display for ChromosomeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid chromosome: {}", self.0)
    }
}

impl FromStr for Chromosome {
    type Err = ChromosomeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().to_uppercase();
        let s = s.strip_prefix("CHR").unwrap_or(&s);

        match s {
            "X" => Ok(Chromosome::ChrX),
            "Y" => Ok(Chromosome::ChrY),
            "MT" | "M" => Ok(Chromosome::ChrMT),
            _ => {
                let num: u8 = s.parse().map_err(|_| {
                    ChromosomeParseError(s.to_string())
                })?;
                Chromosome::from_u8(num).ok_or_else(|| {
                    ChromosomeParseError(format!("{} (out of range 1-25)", num))
                })
            }
        }
    }
}

/// A 1-based genomic position on a chromosome.
///
/// Positions in VCF are 1-based, and we maintain this convention
/// throughout the system. The maximum position for hg38 is ~248,956,422
/// (Chromosome 1), so we use `u32` which supports up to 4,294,967,295.
///
/// # Post-quantum note
/// Positions are always hashed alongside their chromosome in a single
/// SHA-3-512 operation to prevent position-reassignment attacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Position {
    /// The 1-based position on the chromosome.
    pos: u32,
}

impl Position {
    /// Create a new 1-based genomic position.
    ///
    /// # Panics
    /// In debug builds, panics if `pos` is 0 (positions are 1-based).
    #[inline]
    pub const fn new(pos: u32) -> Self {
        assert!(pos > 0, "Genomic positions are 1-based");
        Position { pos }
    }

    /// Create a new 1-based genomic position, returning `None` if `pos` is 0.
    #[inline]
    pub const fn try_new(pos: u32) -> Option<Self> {
        if pos == 0 {
            None
        } else {
            Some(Position { pos })
        }
    }

    /// Returns the 1-based position value.
    #[inline]
    pub const fn get(&self) -> u32 {
        self.pos
    }

    /// Returns the 0-based position (for internal indexing).
    #[inline]
    pub const fn zero_based(&self) -> u32 {
        self.pos - 1
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.pos)
    }
}

/// The reference or alternate allele base(s).
///
/// Stores up to 256 bases inline (no heap allocation for common SNPs).
/// For longer alleles (indels, structural variants), falls back to heap.
///
/// # Compact representation
/// - SNPs (single base): stored inline as `u8`, no allocation
/// - Short indels (<256 bases): stored inline as `SmallVec`
/// - Long variants: heap-allocated string
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Allele {
    /// A single nucleotide base.
    Base(u8),
    /// A short sequence of bases (inline, up to 255 bytes).
    Inline(heapless::String<255>),
    /// A long sequence (heap-allocated).
    Long(Box<str>),
}

impl Allele {
    /// Returns the sequence as a byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Allele::Base(b) => core::slice::from_ref(b),
            Allele::Inline(s) => s.as_bytes(),
            Allele::Long(s) => s.as_bytes(),
        }
    }

    /// Returns the length of the allele sequence.
    #[inline]
    pub fn len(&self) -> usize {
        match self {
            Allele::Base(_) => 1,
            Allele::Inline(s) => s.len(),
            Allele::Long(s) => s.len(),
        }
    }

    /// Returns `true` if the allele is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `true` if this is a single base (SNP).
    #[inline]
    pub const fn is_snp(&self) -> bool {
        matches!(self, Allele::Base(_))
    }
}

/// The type of genomic variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VariantType {
    /// Single Nucleotide Polymorphism — single base change
    Snp,
    /// Insertion — one or more bases inserted
    Insertion,
    /// Deletion — one or more bases deleted
    Deletion,
    /// Complex indel — combination of insertion and deletion
    ComplexIndel,
    /// Copy Number Variation
    Cnv,
    /// Structural Variant (large-scale)
    Structural,
}

impl VariantType {
    /// Returns `true` if the variant is a simple SNP.
    #[inline]
    pub const fn is_simple(&self) -> bool {
        matches!(self, VariantType::Snp)
    }
}

/// Core variant representation.
///
/// Represents a single genomic variant at a specific position.
/// This is the fundamental unit of genomic data in the system.
///
/// # Serialization guarantee
/// All fields implement `Serialize`/`Deserialize` for encrypted storage.
///
/// # Memory
/// A typical human genome has ~4–5M variants. At ~40 bytes each (compressed),
/// this is ~160–200MB for a full genome. Using delta-encoding and bit-packing
/// (see `genome-compress`), this is reduced to ~10–15MB.
#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    /// The chromosome this variant is on.
    pub chrom: Chromosome,
    /// The 1-based genomic position.
    pub pos: Position,
    /// Unique variant identifier (e.g., rsID), if known.
    pub id: Option<String>,
    /// The reference allele.
    pub ref_allele: Allele,
    /// The alternate allele(s). Most variants have a single alternate.
    pub alt_alleles: Vec<Allele>,
    /// Variant quality score (Phred-scaled).
    pub quality: Option<f64>,
    /// Filter status (PASS, VQSRTrancheSNP, etc.).
    pub filter: Option<String>,
    /// Variant type, inferred from ref/alt alleles.
    pub variant_type: VariantType,
}

impl Variant {
    /// Create a new variant with the minimum required fields.
    pub fn new(
        chrom: Chromosome,
        pos: Position,
        ref_allele: Allele,
        alt_alleles: Vec<Allele>,
    ) -> Self {
        let variant_type = classify_variant(&ref_allele, &alt_alleles);
        Variant {
            chrom,
            pos,
            id: None,
            ref_allele,
            alt_alleles,
            quality: None,
            filter: None,
            variant_type,
        }
    }
}

/// Classify the variant type based on reference and alternate alleles.
pub fn classify_variant(ref_allele: &Allele, alt_alleles: &[Allele]) -> VariantType {
    if alt_alleles.is_empty() {
        return VariantType::Deletion;
    }

    let alt = &alt_alleles[0];
    let ref_len = ref_allele.len();
    let alt_len = alt.len();

    if ref_len == 1 && alt_len == 1 {
        VariantType::Snp
    } else if ref_len < alt_len && alt_len <= 50 {
        VariantType::Insertion
    } else if ref_len > alt_len && ref_len <= 50 {
        VariantType::Deletion
    } else if ref_len > 1 && alt_len > 1 {
        VariantType::ComplexIndel
    } else if ref_len > 50 || alt_len > 50 {
        VariantType::Structural
    } else {
        VariantType::Snp
    }
}

/// A genotype at a specific variant position.
///
/// Uses `u8` for compact storage (0 = ref/ref, 1 = ref/alt, 2 = alt/alt, etc.).
/// For multi-allelic sites, stores both alleles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Genotype {
    /// First allele index (0 = reference).
    pub allele1: u8,
    /// Second allele index (0 = reference).
    pub allele2: u8,
    /// Whether the genotype is phased (known parental origin).
    pub phased: bool,
}

impl Genotype {
    /// Create a new homozygous reference genotype (0/0).
    #[inline]
    pub const fn hom_ref() -> Self {
        Genotype {
            allele1: 0,
            allele2: 0,
            phased: false,
        }
    }

    /// Create a new heterozygous genotype (0/1).
    #[inline]
    pub const fn het() -> Self {
        Genotype {
            allele1: 0,
            allele2: 1,
            phased: false,
        }
    }

    /// Create a new homozygous alternate genotype (1/1).
    #[inline]
    pub const fn hom_alt() -> Self {
        Genotype {
            allele1: 1,
            allele2: 1,
            phased: false,
        }
    }

    /// Returns `true` if the genotype is homozygous (both alleles the same).
    #[inline]
    pub const fn is_homozygous(&self) -> bool {
        self.allele1 == self.allele2
    }

    /// Returns `true` if the genotype is heterozygous.
    #[inline]
    pub const fn is_heterozygous(&self) -> bool {
        self.allele1 != self.allele2
    }

    /// Returns `true` if this genotype contains the alternate allele.
    #[inline]
    pub const fn has_alternate(&self) -> bool {
        self.allele1 > 0 || self.allele2 > 0
    }
}

/// Reference genome assembly identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReferenceGenome {
    /// GRCh37 / hg19 (2009)
    Hg19,
    /// GRCh38 / hg38 (2013)
    Hg38,
    /// GRCh38 with patches (current)
    Hg38Patch13,
    /// T2T-CHM13 (2022, complete telomere-to-telomere)
    T2tChm13,
}

impl ReferenceGenome {
    /// Returns the number of base pairs for each chromosome in this assembly.
    ///
    /// Returns `None` for the mitochondrial genome or if the chromosome is unknown.
    pub fn chromosome_length(&self, chrom: Chromosome) -> Option<u32> {
        match self {
            ReferenceGenome::Hg38 => Some(match chrom {
                Chromosome::Chr1 => 248_956_422,
                Chromosome::Chr2 => 242_193_529,
                Chromosome::Chr3 => 198_295_559,
                Chromosome::Chr4 => 190_214_555,
                Chromosome::Chr5 => 181_538_259,
                Chromosome::Chr6 => 170_805_979,
                Chromosome::Chr7 => 159_345_973,
                Chromosome::Chr8 => 145_138_636,
                Chromosome::Chr9 => 138_394_717,
                Chromosome::Chr10 => 133_797_422,
                Chromosome::Chr11 => 135_086_622,
                Chromosome::Chr12 => 133_275_309,
                Chromosome::Chr13 => 114_364_328,
                Chromosome::Chr14 => 107_043_718,
                Chromosome::Chr15 => 101_991_189,
                Chromosome::Chr16 => 90_338_348,
                Chromosome::Chr17 => 83_257_441,
                Chromosome::Chr18 => 80_373_285,
                Chromosome::Chr19 => 58_617_616,
                Chromosome::Chr20 => 64_441_745,
                Chromosome::Chr21 => 46_709_983,
                Chromosome::Chr22 => 50_818_468,
                Chromosome::ChrX => 156_040_895,
                Chromosome::ChrY => 57_227_415,
                Chromosome::ChrMT => 16_569,
            }),
            ReferenceGenome::Hg19 => Some(match chrom {
                Chromosome::Chr1 => 249_250_621,
                Chromosome::Chr2 => 243_199_373,
                Chromosome::Chr3 => 198_022_430,
                Chromosome::Chr4 => 191_154_276,
                Chromosome::Chr5 => 180_915_260,
                Chromosome::Chr6 => 171_115_067,
                Chromosome::Chr7 => 159_138_663,
                Chromosome::Chr8 => 146_364_022,
                Chromosome::Chr9 => 141_213_431,
                Chromosome::Chr10 => 135_534_747,
                Chromosome::Chr11 => 135_006_516,
                Chromosome::Chr12 => 133_851_895,
                Chromosome::Chr13 => 115_169_878,
                Chromosome::Chr14 => 107_349_540,
                Chromosome::Chr15 => 102_531_392,
                Chromosome::Chr16 => 90_354_753,
                Chromosome::Chr17 => 81_195_210,
                Chromosome::Chr18 => 78_077_248,
                Chromosome::Chr19 => 59_128_983,
                Chromosome::Chr20 => 63_025_520,
                Chromosome::Chr21 => 48_129_895,
                Chromosome::Chr22 => 51_304_566,
                Chromosome::ChrX => 155_270_560,
                Chromosome::ChrY => 59_373_566,
                Chromosome::ChrMT => 16_569,
            }),
            _ => None, // Extended assemblies pending implementation
        }
    }

    /// Returns the standard prefix used in VCF files for this assembly.
    pub fn vcf_prefix(&self) -> &str {
        match self {
            ReferenceGenome::Hg19 => "chr",
            ReferenceGenome::Hg38 => "chr",
            ReferenceGenome::Hg38Patch13 => "chr",
            ReferenceGenome::T2tChm13 => "chr",
        }
    }
}

impl fmt::Display for ReferenceGenome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReferenceGenome::Hg19 => write!(f, "hg19"),
            ReferenceGenome::Hg38 => write!(f, "hg38"),
            ReferenceGenome::Hg38Patch13 => write!(f, "hg38-patch13"),
            ReferenceGenome::T2tChm13 => write!(f, "T2T-CHM13"),
        }
    }
}

impl FromStr for ReferenceGenome {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "hg19" | "grch37" | "37" => Ok(ReferenceGenome::Hg19),
            "hg38" | "grch38" | "38" => Ok(ReferenceGenome::Hg38),
            "t2t" | "t2t-chm13" | "chm13" => Ok(ReferenceGenome::T2tChm13),
            _ => Err(()),
        }
    }
}

/// A record associating a variant with a known identifier (rsID, ClinVar, etc.).
#[derive(Debug, Clone, PartialEq)]
pub struct Annotation {
    /// The dbSNP rsID (e.g., "rs53576").
    pub rsid: Option<String>,
    /// The gene symbol (e.g., "OXTR").
    pub gene_symbol: Option<String>,
    /// Clinical significance from ClinVar.
    pub clinvar_significance: Option<String>,
    /// The allele frequency in 1000 Genomes (if known).
    pub allele_frequency: Option<f64>,
}

/// Summary statistics for a genome.
#[derive(Debug, Clone, PartialEq)]
pub struct GenomeSummary {
    /// Total number of variants stored.
    pub total_variants: u64,
    /// Number of SNPs.
    pub snp_count: u64,
    /// Number of insertions.
    pub insertion_count: u64,
    /// Number of deletions.
    pub deletion_count: u64,
    /// Number of CNVs.
    pub cnv_count: u64,
    /// Number of structural variants.
    pub structural_count: u64,
    /// The reference genome assembly used.
    pub reference_genome: ReferenceGenome,
    /// The SHA-3-512 root hash of the entropy chain.
    pub entropy_root: Option<[u8; 64]>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_chromosome_from_u8() {
        assert_eq!(Chromosome::from_u8(1), Some(Chromosome::Chr1));
        assert_eq!(Chromosome::from_u8(22), Some(Chromosome::Chr22));
        assert_eq!(Chromosome::from_u8(23), Some(Chromosome::ChrX));
        assert_eq!(Chromosome::from_u8(24), Some(Chromosome::ChrY));
        assert_eq!(Chromosome::from_u8(25), Some(Chromosome::ChrMT));
        assert_eq!(Chromosome::from_u8(0), None);
        assert_eq!(Chromosome::from_u8(26), None);
    }

    #[test]
    fn test_chromosome_parse() {
        assert_eq!("1".parse::<Chromosome>(), Ok(Chromosome::Chr1));
        assert_eq!("X".parse::<Chromosome>(), Ok(Chromosome::ChrX));
        assert_eq!("chrX".parse::<Chromosome>(), Ok(Chromosome::ChrX));
        assert_eq!("MT".parse::<Chromosome>(), Ok(Chromosome::ChrMT));
        assert_eq!("chrMT".parse::<Chromosome>(), Ok(Chromosome::ChrMT));
        assert!("0".parse::<Chromosome>().is_err());
        assert!("26".parse::<Chromosome>().is_err());
    }

    #[test]
    fn test_chromosome_display() {
        assert_eq!(Chromosome::Chr1.to_string(), "1");
        assert_eq!(Chromosome::ChrX.to_string(), "X");
        assert_eq!(Chromosome::ChrMT.to_string(), "MT");
    }

    #[test]
    fn test_chromosome_is_autosome() {
        assert!(Chromosome::Chr1.is_autosome());
        assert!(Chromosome::Chr22.is_autosome());
        assert!(!Chromosome::ChrX.is_autosome());
        assert!(!Chromosome::ChrMT.is_autosome());
    }

    #[test]
    fn test_position() {
        let pos = Position::new(100_000);
        assert_eq!(pos.get(), 100_000);
        assert_eq!(pos.zero_based(), 99_999);
        assert!(Position::try_new(0).is_none());
    }

    #[test]
    fn test_allele_snp() {
        let a = Allele::Base(b'A');
        assert!(a.is_snp());
        assert_eq!(a.len(), 1);
        assert_eq!(a.as_bytes(), b"A");
    }

    #[test]
    fn test_variant_classification_snp() {
        let ref_a = Allele::Base(b'A');
        let alt_t = Allele::Base(b'T');
        let var = Variant::new(Chromosome::Chr1, Position::new(100_000), ref_a, vec![alt_t]);
        assert_eq!(var.variant_type, VariantType::Snp);
    }

    #[test]
    fn test_variant_classification_insertion() {
        let ref_a = Allele::Base(b'A');
        let alt_at = Allele::Inline(heapless::String::try_from("AT").unwrap());
        let var = Variant::new(Chromosome::Chr1, Position::new(100_000), ref_a, vec![alt_at]);
        assert_eq!(var.variant_type, VariantType::Insertion);
    }

    #[test]
    fn test_variant_classification_deletion() {
        let ref_at = Allele::Inline(heapless::String::try_from("AT").unwrap());
        let alt_a = Allele::Base(b'A');
        let var = Variant::new(Chromosome::Chr1, Position::new(100_000), ref_at, vec![alt_a]);
        assert_eq!(var.variant_type, VariantType::Deletion);
    }

    #[test]
    fn test_genotypes() {
        let hom_ref = Genotype::hom_ref();
        assert!(hom_ref.is_homozygous());
        assert!(!hom_ref.has_alternate());

        let het = Genotype::het();
        assert!(het.is_heterozygous());
        assert!(het.has_alternate());

        let hom_alt = Genotype::hom_alt();
        assert!(hom_alt.is_homozygous());
        assert!(hom_alt.has_alternate());
    }

    #[test]
    fn test_reference_genome_hg38_length() {
        let hg38 = ReferenceGenome::Hg38;
        assert_eq!(
            hg38.chromosome_length(Chromosome::Chr1),
            Some(248_956_422)
        );
        assert_eq!(
            hg38.chromosome_length(Chromosome::ChrMT),
            Some(16_569)
        );
    }

    #[test]
    fn test_reference_genome_parse() {
        assert_eq!("hg38".parse::<ReferenceGenome>(), Ok(ReferenceGenome::Hg38));
        assert_eq!("GRCh38".parse::<ReferenceGenome>(), Ok(ReferenceGenome::Hg38));
        assert_eq!("t2t".parse::<ReferenceGenome>(), Ok(ReferenceGenome::T2tChm13));
        assert!("invalid".parse::<ReferenceGenome>().is_err());
    }

    #[test]
    fn test_variant_classify_empty_alt_is_deletion() {
        let ref_a = Allele::Base(b'A');
        let result = classify_variant(&ref_a, &[]);
        assert_eq!(result, VariantType::Deletion);
    }
}