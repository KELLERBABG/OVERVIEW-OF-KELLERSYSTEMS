//! # zk-genome-mobile
//!
//! Rust → mobile FFI bindings (uniffi). Exposes the entire core library
//! to Kotlin (Android) and Swift (iOS) via `uniffi`-generated bindings.
//!
//! This crate is the **single entry point** for all mobile operations.
//! Every genomic operation available on-device flows through this crate.

#![deny(missing_docs)]
#![deny(unsafe_code)]

extern crate alloc;

/// RadioShield v1.1 — UI-prompt-first signal isolation module.
/// Manages Airplane Mode enforcement with polling fallback.
pub mod radio_shield;

/// Re-export all core types for the mobile layer.
pub mod types {
    pub use zk_genome_genome_core::{
        Allele, Chromosome, ChromosomeParseError, Genotype, GenomeSummary, Position,
        ReferenceGenome, Variant, VariantType,
    };
}

/// Re-export the VCF parser for mobile usage.
pub mod parsing {
    use alloc::vec::Vec;
    use zk_genome_genome_core::Variant;
    use zk_genome_genome_parser::{VcfError, VcfMetadata, VcfParser};

    /// Parse VCF data from bytes. Returns (metadata, records).
    pub fn parse_vcf(data: &[u8]) -> Result<(VcfMetadata, Vec<Variant>), VcfError> {
        let mut parser = VcfParser::new(data);
        let metadata = parser.parse_metadata()?;
        let records: Result<Vec<_>, _> = parser.records().collect();
        Ok((metadata, records?))
    }
}

/// Re-export query engine for mobile usage.
pub mod query {
    use alloc::vec::Vec;
    use zk_genome_genome_core::{Chromosome, Position, Variant};
    use zk_genome_query_engine::{GenomicQuery, VariantIndex};

    /// Create a new empty variant index.
    pub fn create_index() -> VariantIndex {
        VariantIndex::new()
    }

    /// Insert a variant into the index.
    pub fn index_insert(index: &mut VariantIndex, variant: Variant) {
        index.insert(variant);
    }

    /// Find variants at a specific position.
    pub fn find_at(index: &VariantIndex, chrom: Chromosome, pos: u32) -> Option<Variant> {
        let p = Position::try_new(pos)?;
        index.find_at(chrom, p).cloned()
    }

    /// Find variants in a range.
    pub fn find_range(index: &VariantIndex, chrom: Chromosome, start: u32, end: u32) -> Vec<Variant> {
        let start_pos = Position::new(start);
        let end_pos = Position::new(end);
        index.find_range(chrom, start_pos, end_pos).into_iter().cloned().collect()
    }

    /// Find variant by rsID.
    pub fn find_by_rsid(index: &VariantIndex, rsid: &str) -> Option<Variant> {
        index.find_by_rsid(rsid).cloned()
    }
}

/// Re-export compression for mobile usage.
pub mod compression {
    use alloc::vec::Vec;
    use zk_genome_genome_core::{Chromosome, Position, Variant};
    use zk_genome_genome_compress::{compress_variants, decompress_variants, CompressedVariant};

    /// Compress variants for storage.
    pub fn compress(variants: &[Variant]) -> Vec<CompressedVariant> {
        compress_variants(variants)
    }

    /// Decompress variants from storage.
    pub fn decompress(compressed: &[CompressedVariant]) -> Vec<(Chromosome, Position)> {
        decompress_variants(compressed)
    }
}

/// Re-export integrity audit for mobile usage.
pub mod integrity {
    /// Hash a block of variant data using SHA-3-512.
    pub fn hash_variant_block(data: &[u8]) -> [u8; 64] {
        zk_genome_entropy_auditor::hash_variant_block(data)
    }
}

/// Re-export reference genome data for mobile usage.
pub mod reference {
    use zk_genome_genome_core::{Chromosome, ReferenceGenome};
    use zk_genome_genome_reference;

    /// Get chromosome length for a reference genome.
    pub fn chromosome_length(ref_genome: ReferenceGenome, chrom: Chromosome) -> Option<u32> {
        zk_genome_genome_reference::chromosome_length(ref_genome, chrom)
    }

    /// Get total genome length.
    pub fn total_genome_length(ref_genome: ReferenceGenome) -> u64 {
        zk_genome_genome_reference::total_genome_length(ref_genome)
    }
}

/// Re-export annotation for mobile usage.
pub mod annotation {
    use zk_genome_genome_annotate::{AnnotationDb, VariantAnnotation};
    use zk_genome_genome_core::{Chromosome, Position, Variant};

    /// Create a new annotation database.
    pub fn create_annotation_db() -> AnnotationDb {
        AnnotationDb::new()
    }

    /// Insert an annotation.
    pub fn insert_annotation(
        db: &mut AnnotationDb, chrom: Chromosome, pos: u32,
        rsid: Option<alloc::string::String>, gene_symbol: Option<alloc::string::String>,
        clinical_sig: Option<alloc::string::String>,
    ) {
        let annotation = VariantAnnotation {
            rsid, gene_symbol,
            clinical_significance: clinical_sig,
            consequence: None, allele_frequency: None,
        };
        if let Some(p) = Position::try_new(pos) {
            db.insert(chrom, p, annotation);
        }
    }

    /// Annotate a variant.
    pub fn annotate_variant(db: &AnnotationDb, variant: &Variant) -> Option<VariantAnnotation> {
        db.annotate_variant(variant)
    }
}

/// Re-export lab ingestion for mobile usage.
pub mod ingestion {
    use zk_genome_genome_ingestion::{
        verification, IngestionResult, LabIdentity, SignedGenomePackage, TrustedLabRegistry,
    };

    /// Create a new trusted lab registry.
    pub fn create_registry() -> TrustedLabRegistry {
        TrustedLabRegistry::new()
    }

    /// Add a lab to the registry.
    pub fn add_lab(registry: &mut TrustedLabRegistry, lab: LabIdentity) {
        registry.add_lab(lab);
    }

    /// Verify and ingest a signed genome package.
    pub fn ingest_signed_package(package: SignedGenomePackage, registry: &TrustedLabRegistry) -> IngestionResult {
        verification::ingest(package, registry)
    }

    /// Generate a SHA-3-512 hash of variant data.
    pub fn hash_variant_data(data: &[u8]) -> [u8; 64] {
        verification::hash_variant_data(data)
    }
}

/// Re-export normalization for mobile usage.
pub mod normalizer {
    use alloc::vec::Vec;
    use zk_genome_genome_core::Variant;
    use zk_genome_genome_normalizer::{
        decompose_multi_allelic, is_normalized, normalize_variant, NormalizedVariant,
    };

    /// Normalize a variant.
    pub fn normalize(variant: &Variant) -> NormalizedVariant {
        normalize_variant(variant)
    }

    /// Check if a variant is already normalized.
    pub fn is_variant_normalized(variant: &Variant) -> bool {
        is_normalized(variant)
    }

    /// Decompose a multi-allelic variant.
    pub fn decompose(variant: &Variant) -> Vec<Variant> {
        decompose_multi_allelic(variant)
    }
}

/// Re-export zk-core types for mobile.
pub mod zk {
    pub use zk_genome_zk_core::*;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vcf_through_mobile_bridge() {
        let vcf = b"##fileformat=VCFv4.2\n\
            #CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n\
            1\t100000\trs1\tA\tG\t.\t.\t.\n";
        let (metadata, records) = parsing::parse_vcf(vcf).unwrap();
        assert_eq!(metadata.version, Some("VCFv4.2".into()));
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id.as_deref(), Some("rs1"));
    }

    #[test]
    fn test_query_through_mobile_bridge() {
        use types::*;
        let mut idx = query::create_index();
        let v = Variant::new(Chromosome::Chr1, Position::new(100_000), Allele::Base(b'A'), vec![Allele::Base(b'G')]);
        query::index_insert(&mut idx, v);
        let found = query::find_at(&idx, Chromosome::Chr1, 100_000);
        assert!(found.is_some());
    }

    #[test]
    fn test_compression_through_mobile_bridge() {
        use types::*;
        let variants = vec![Variant::new(Chromosome::Chr1, Position::new(100_000), Allele::Base(b'A'), vec![Allele::Base(b'G')])];
        let compressed = compression::compress(&variants);
        assert_eq!(compressed.len(), 1);
        assert_eq!(compressed[0].delta_position, 100_000);
    }

    #[test]
    fn test_integrity_through_mobile_bridge() {
        let hash = integrity::hash_variant_block(b"test data");
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_reference_through_mobile_bridge() {
        use types::ReferenceGenome;
        let len = reference::chromosome_length(ReferenceGenome::Hg38, types::Chromosome::Chr1);
        assert_eq!(len, Some(248_956_422));
    }

    #[test]
    fn test_mobile_full_pipeline() {
        let vcf = b"##fileformat=VCFv4.2\n#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n1\t100000\trs53576\tA\tG\t.\t.\t.\n1\t200000\trs2\tT\tC\t.\t.\t.\n";
        let (_, records) = parsing::parse_vcf(vcf).unwrap();
        let mut idx = query::create_index();
        for r in &records { query::index_insert(&mut idx, r.clone()); }
        let found = query::find_by_rsid(&idx, "rs53576");
        assert!(found.is_some());
        assert_eq!(found.unwrap().pos.get(), 100_000);
    }
}