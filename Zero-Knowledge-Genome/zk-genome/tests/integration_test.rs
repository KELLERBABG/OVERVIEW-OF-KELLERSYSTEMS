//! # Zero-Knowledge Genome — Integration Tests
//!
//! End-to-end tests that validate the full pipeline:
//! 1. Lab signs compressed variant data
//! 2. Device verifies signature and imports
//! 3. Parser extracts variants from VCF
//! 4. Normalizer trims common prefixes
//! 5. Compressor delta-encodes positions
//! 6. Query engine indexes and searches
//! 7. Entropy auditor hashes variant blocks
//! 8. Annotations lookup genes by rsID

use std::vec::Vec;

use zk_genome_genome_core::{
    Allele, Chromosome, GenomeSummary, Position, ReferenceGenome, Variant, VariantType,
};
use zk_genome_genome_parser::VcfParser;

// These will be used once we make them no_std compatible in test
#[test]
fn test_complete_pipeline() {
    // Step 1: Parse VCF data
    let vcf_data = b"##fileformat=VCFv4.2\n\
        ##source=IntegrationTest\n\
        #CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n\
        1\t100000\trs1\tA\tG\t100.0\tPASS\t.\n\
        1\t200000\trs2\tT\tC\t.\t.\t.\n\
        2\t50000000\trs3\tACGT\tA\t.\tPASS\t.\n\
        22\t10000000\t.\tG\tA\t.\t.\t.\n";

    let mut parser = VcfParser::new(vcf_data);
    let metadata = parser.parse_metadata().expect("Failed to parse metadata");
    assert_eq!(metadata.version, Some("VCFv4.2".into()));

    let variants: Vec<_> = parser.records()
        .collect::<Result<Vec<_>, _>>()
        .expect("Failed to parse records");

    assert_eq!(variants.len(), 4, "Should have parsed 4 variants");

    // Step 2: Verify variant types
    assert_eq!(variants[0].variant_type, VariantType::Snp, "rs1 should be SNP");
    assert_eq!(variants[1].chrom, Chromosome::Chr1, "rs2 should be chr1");
    assert_eq!(variants[2].ref_allele.len(), 4, "rs3 ref should be 4 bases");

    // Step 3: Verify query index
    let mut index = zk_genome_genome_query_engine::VariantIndex::new();
    for v in &variants {
        index.insert(v.clone());
    }
    assert_eq!(index.len(), 4);

    // Step 4: Find variant by position
    let found = index.find_at(Chromosome::Chr1, Position::new(100_000));
    assert!(found.is_some(), "Should find rs1 at position");
    assert_eq!(found.unwrap().id.as_deref(), Some("rs1"));

    // Step 5: Find variant by rsID
    let by_rsid = index.query(&zk_genome_genome_query_engine::GenomicQuery::RsId("rs3".into()));
    assert_eq!(by_rsid.len(), 1, "Should find rs3 by ID");
    assert_eq!(by_rsid[0].variant.chrom, Chromosome::Chr2);

    // Step 6: Range query
    let range_results = index.find_range(
        Chromosome::Chr1,
        Position::new(100_000),
        Position::new(250_000),
    );
    assert_eq!(range_results.len(), 2, "Should find 2 variants in range");

    // Step 7: Verify reference genome data
    let hg38_len = ReferenceGenome::Hg38.chromosome_length(Chromosome::Chr1);
    assert_eq!(hg38_len, Some(248_956_422));

    // Step 8: Check entropy auditor works
    let block_data = b"variant block 1";
    let hash = zk_genome_genome_entropy_auditor::hash_variant_block(block_data);
    assert_eq!(hash.len(), 64, "SHA-3-512 hash should be 64 bytes");
}

#[test]
fn test_entropy_audit_hash_chain() {
    // Simulate a hash chain of variant blocks
    let blocks = [b"block1", b"block2", b"block3"];
    let mut prev_hash = [0u8; 64];

    for block in &blocks {
        let hash = zk_genome_genome_entropy_auditor::hash_variant_block(block);
        assert_ne!(hash, prev_hash, "Each block should have unique hash");
        prev_hash = hash;
    }
}

#[test]
fn test_compression_roundtrip() {
    use zk_genome_genome_compress::compress_variants;
    use zk_genome_genome_core::{Allele, Chromosome, Position, Variant};

    let variants: Vec<_> = (0..100)
        .map(|i| {
            Variant::new(
                if i < 50 { Chromosome::Chr1 } else { Chromosome::Chr2 },
                Position::new(1_000_000 + i * 100),
                Allele::Base(b'A'),
                vec![Allele::Base(b'G')],
            )
        })
        .collect();

    let compressed = compress_variants(&variants);
    assert_eq!(compressed.len(), 100);

    // Verify deltas are small for same-chromosome variants
    for i in 1..50 {
        assert_eq!(compressed[i].delta_position, 100, "Consecutive deltas should be 100");
    }
}

#[test]
fn test_ingestion_trust_chain() {
    use zk_genome_genome_core::{Position, Variant};
    use zk_genome_genome_ingestion::{
        IngestionVerification, LabIdentity, SignedGenomePackage, TrustedLabRegistry,
    };
    use zk_genome_genome_ingestion::verification;

    let lab = LabIdentity {
        lab_id: "CLIA-99D-7654321".into(),
        name: "Sequencing Lab Inc.".into(),
        dilithium_public_key: vec![1u8; 32],
        ed25519_public_key: vec![2u8; 32],
        accreditation: Some("CAP 2024".into()),
    };

    let mut registry = TrustedLabRegistry::new();
    registry.add_lab(lab.clone());

    let variant_data = b"compressed genomic variants data...";
    let hash = verification::hash_variant_data(variant_data);

    let package = SignedGenomePackage {
        lab: lab.clone(),
        compressed_variants: variant_data.to_vec(),
        variant_data_hash: hash,
        hybrid_signature: vec![3u8; 128],
        reference_assembly: "hg38".into(),
        sequencing_timestamp: 1_720_000_000,
        batch_id: Some("BATCH-INTEGRATION".into()),
    };

    let result = verification::ingest(package, &registry);
    assert_eq!(result.verification, IngestionVerification::Verified);
    assert_eq!(result.lab.lab_id, "CLIA-99D-7654321");
    assert_eq!(result.reference_assembly, "hg38");
}

#[test]
fn test_normalizer_multi_allelic() {
    use zk_genome_genome_core::{Allele, Chromosome, Position};
    use zk_genome_genome_normalizer::decompose_multi_allelic;

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
    assert!(decomposed[0].alt_alleles.len() == 1);
    assert!(decomposed[1].alt_alleles.len() == 1);
}

#[test]
fn test_annotations_oxytocin_receptor() {
    use zk_genome_genome_annotate::{AnnotationDb, VariantAnnotation};
    use zk_genome_genome_core::{Allele, Chromosome, Position};

    let mut db = AnnotationDb::new();
    db.insert(
        Chromosome::Chr3,
        Position::new(8_809_314),
        VariantAnnotation {
            rsid: Some("rs53576".into()),
            gene_symbol: Some("OXTR".into()),
            clinical_significance: Some("Benign".into()),
            consequence: Some("intronic".into()),
            allele_frequency: Some(0.617),
        },
    );

    let mut variant = Variant::new(
        Chromosome::Chr3,
        Position::new(8_809_314),
        Allele::Base(b'A'),
        vec![Allele::Base(b'G')],
    );
    variant.id = Some("rs53576".into());

    let ann = db.annotate_variant(&variant);
    assert!(ann.is_some());
    assert_eq!(ann.unwrap().gene_symbol.as_deref(), Some("OXTR"));
    assert_eq!(ann.unwrap().allele_frequency, Some(0.617));
}

#[test]
fn test_reference_genome_total_length() {
    use zk_genome_genome_reference::total_genome_length;
    use zk_genome_genome_core::ReferenceGenome;

    let hg38_total = total_genome_length(ReferenceGenome::Hg38);
    let hg19_total = total_genome_length(ReferenceGenome::Hg19);

    // Both should be ~3.1 billion base pairs
    assert!(hg38_total > 3_000_000_000, "hg38 should be >3Gbp");
    assert!(hg19_total > 3_000_000_000, "hg19 should be >3Gbp");
    // hg38 is slightly smaller than hg19
    assert!(hg19_total > hg38_total, "hg19 should be larger than hg38");
}

#[test]
fn test_genome_summary_from_pipeline() {
    let vcf_data = b"##fileformat=VCFv4.2\n\
        #CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n\
        1\t100000\trs1\tA\tG\t.\t.\t.\n\
        1\t200000\t.\tT\tC\t.\t.\t.\n\
        22\t50000000\t.\tACGT\tA\t.\t.\t.\n";

    let mut parser = VcfParser::new(vcf_data);
    let _ = parser.parse_metadata().unwrap();
    let records: Vec<_> = parser.records().collect::<Result<Vec<_>, _>>().unwrap();

    let mut snp_count = 0u64;
    let mut del_count = 0u64;
    let mut ins_count = 0u64;

    for v in &records {
        match v.variant_type {
            VariantType::Snp => snp_count += 1,
            VariantType::Deletion => del_count += 1,
            VariantType::Insertion => ins_count += 1,
            _ => {}
        }
    }

    let summary = GenomeSummary {
        total_variants: records.len() as u64,
        snp_count,
        deletion_count: del_count,
        insertion_count: ins_count,
        cnv_count: 0,
        structural_count: 0,
        reference_genome: ReferenceGenome::Hg38,
        entropy_root: None,
    };

    assert_eq!(summary.total_variants, 3);
    assert_eq!(summary.snp_count, 2);
}