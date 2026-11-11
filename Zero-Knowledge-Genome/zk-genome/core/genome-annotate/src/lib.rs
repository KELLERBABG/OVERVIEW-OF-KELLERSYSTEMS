//! # zk-genome-genome-annotate
//!
//! Variant annotation — rsID, gene symbol, and ClinVar significance lookups.
//!
//! Provides annotation data for variants using embedded databases
//! from dbSNP, RefSeq, and ClinVar.

#![no_std]
#![deny(missing_docs)]
#![deny(unsafe_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use zk_genome_genome_core::{Chromosome, Position, Variant};

/// An annotation for a specific variant position.
#[derive(Debug, Clone, PartialEq)]
pub struct VariantAnnotation {
    /// The dbSNP rsID (e.g., "rs53576").
    pub rsid: Option<String>,
    /// The gene symbol (e.g., "OXTR").
    pub gene_symbol: Option<String>,
    /// Clinical significance from ClinVar.
    pub clinical_significance: Option<String>,
    /// The functional consequence.
    pub consequence: Option<String>,
    /// Allele frequency in 1000 Genomes (0.0 - 1.0).
    pub allele_frequency: Option<f64>,
}

/// A database for looking up variant annotations.
#[derive(Debug, Clone)]
pub struct AnnotationDb {
    by_position: BTreeMap<(u8, u32), VariantAnnotation>,
    by_rsid: BTreeMap<String, VariantAnnotation>,
}

impl AnnotationDb {
    /// Create a new empty annotation database.
    pub fn new() -> Self {
        AnnotationDb {
            by_position: BTreeMap::new(),
            by_rsid: BTreeMap::new(),
        }
    }

    /// Insert an annotation for a variant.
    pub fn insert(&mut self, chrom: Chromosome, pos: Position, annotation: VariantAnnotation) {
        let key = (chrom.as_u8(), pos.get());
        if let Some(ref rsid) = annotation.rsid {
            self.by_rsid.insert(rsid.clone(), annotation.clone());
        }
        self.by_position.insert(key, annotation);
    }

    /// Look up annotation by genomic position.
    pub fn find_by_position(&self, chrom: Chromosome, pos: Position) -> Option<&VariantAnnotation> {
        self.by_position.get(&(chrom.as_u8(), pos.get()))
    }

    /// Look up annotation by rsID.
    pub fn find_by_rsid(&self, rsid: &str) -> Option<&VariantAnnotation> {
        self.by_rsid.get(rsid)
    }

    /// Return the number of annotations stored.
    pub fn len(&self) -> usize {
        self.by_position.len()
    }

    /// Returns true if the database is empty.
    pub fn is_empty(&self) -> bool {
        self.by_position.is_empty()
    }

    /// Annotate a variant with known data.
    pub fn annotate_variant(&self, variant: &Variant) -> Option<VariantAnnotation> {
        if let Some(ref id) = variant.id {
            if let Some(ann) = self.by_rsid.get(id) {
                return Some(ann.clone());
            }
        }
        self.by_position
            .get(&(variant.chrom.as_u8(), variant.pos.get()))
            .cloned()
    }
}

impl Default for AnnotationDb {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zk_genome_genome_core::{Allele, Position};

    fn make_variant(chrom: Chromosome, pos: u32, rsid: &str) -> Variant {
        let mut v = Variant::new(
            chrom,
            Position::new(pos),
            Allele::Base(b'A'),
            alloc::vec![Allele::Base(b'G')],
        );
        v.id = Some(rsid.into());
        v
    }

    #[test]
    fn test_empty_db() {
        let db = AnnotationDb::new();
        assert!(db.is_empty());
        assert_eq!(db.len(), 0);
    }

    #[test]
    fn test_insert_and_find_by_position() {
        let mut db = AnnotationDb::new();
        let ann = VariantAnnotation {
            rsid: Some("rs53576".into()),
            gene_symbol: Some("OXTR".into()),
            clinical_significance: Some("Benign".into()),
            consequence: Some("intronic".into()),
            allele_frequency: Some(0.5),
        };
        db.insert(Chromosome::Chr3, Position::new(8_809_314), ann);

        let found = db.find_by_position(Chromosome::Chr3, Position::new(8_809_314));
        assert!(found.is_some());
        assert_eq!(found.unwrap().gene_symbol.as_deref(), Some("OXTR"));
    }

    #[test]
    fn test_find_by_rsid() {
        let mut db = AnnotationDb::new();
        db.insert(
            Chromosome::Chr16,
            Position::new(1_000_000),
            VariantAnnotation {
                rsid: Some("rs1234".into()),
                gene_symbol: Some("GENE1".into()),
                clinical_significance: None,
                consequence: None,
                allele_frequency: None,
            },
        );

        let found = db.find_by_rsid("rs1234");
        assert!(found.is_some());
        assert_eq!(found.unwrap().gene_symbol.as_deref(), Some("GENE1"));
    }

    #[test]
    fn test_annotate_variant_by_rsid() {
        let mut db = AnnotationDb::new();
        db.insert(
            Chromosome::Chr1,
            Position::new(100_000),
            VariantAnnotation {
                rsid: Some("rs1".into()),
                gene_symbol: Some("BRCA1".into()),
                clinical_significance: None,
                consequence: None,
                allele_frequency: None,
            },
        );

        let variant = make_variant(Chromosome::Chr1, 100_000, "rs1");
        let ann = db.annotate_variant(&variant);
        assert!(ann.is_some());
        assert_eq!(ann.unwrap().gene_symbol.as_deref(), Some("BRCA1"));
    }

    #[test]
    fn test_not_found() {
        let db = AnnotationDb::new();
        assert!(db.find_by_position(Chromosome::Chr1, Position::new(999_999)).is_none());
        assert!(db.find_by_rsid("nonexistent").is_none());
    }
}