//! # zk-genome-query-engine
//!
//! Encrypted query matching engine — on-device variant search.
//!
//! Matches pharmaceutical queries against locally stored variants
//! without revealing non-matching variants or genome metadata.
//!
//! Supports:
//! - **Positional lookup:** find variant at specific chromosome+position
//! - **Range query:** find all variants in a genomic region
//! - **rsID lookup:** find variant by dbSNP identifier
//! - **Variant type filter:** filter by SNP/indel/CNV

#![no_std]
#![deny(missing_docs)]
#![deny(unsafe_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use zk_genome_genome_core::{Chromosome, Position, Variant, VariantType};

/// A query for matching variants.
#[derive(Debug, Clone, PartialEq)]
pub enum GenomicQuery {
    /// Find a variant at an exact (chrom, pos) coordinate.
    Positional {
        /// Chromosome to search.
        chrom: Chromosome,
        /// 1-based position.
        pos: Position,
    },
    /// Find variants in a genomic range.
    Range {
        /// Chromosome to search.
        chrom: Chromosome,
        /// Start position (1-based, inclusive).
        start: Position,
        /// End position (1-based, inclusive).
        end: Position,
    },
    /// Find a variant by dbSNP rsID.
    RsId(String),
    /// Find variants of a specific type.
    ByType(VariantType),
}

/// The result of matching a query against the genome.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryMatch {
    /// The matched variant.
    pub variant: Variant,
    /// The query that produced this match.
    pub matched_by: GenomicQuery,
}

/// An index of variants for efficient query matching.
///
/// Maintains a B-tree index keyed by (chromosome, position) for
/// O(log n) positional lookups and O(k + log n) range queries.
#[derive(Debug, Clone)]
pub struct VariantIndex {
    // (chromosome as u8, position) -> variant
    positional: BTreeMap<(u8, u32), Variant>,
    // rsID -> variant (for id lookups)
    by_rsid: BTreeMap<String, Variant>,
}

impl VariantIndex {
    /// Create a new empty variant index.
    pub fn new() -> Self {
        VariantIndex {
            positional: BTreeMap::new(),
            by_rsid: BTreeMap::new(),
        }
    }

    /// Insert a variant into the index.
    pub fn insert(&mut self, variant: Variant) {
        let key = (variant.chrom.as_u8(), variant.pos.get());
        if let Some(ref id) = variant.id {
            self.by_rsid.insert(id.clone(), variant.clone());
        }
        self.positional.insert(key, variant);
    }

    /// Insert multiple variants into the index.
    pub fn insert_all(&mut self, variants: impl IntoIterator<Item = Variant>) {
        for v in variants {
            self.insert(v);
        }
    }

    /// Number of variants in the index.
    pub fn len(&self) -> usize {
        self.positional.len()
    }

    /// Returns true if the index is empty.
    pub fn is_empty(&self) -> bool {
        self.positional.is_empty()
    }

    /// Find a variant at an exact (chrom, pos) coordinate.
    pub fn find_at(&self, chrom: Chromosome, pos: Position) -> Option<&Variant> {
        self.positional.get(&(chrom.as_u8(), pos.get()))
    }

    /// Find variants in a genomic range.
    pub fn find_range(
        &self,
        chrom: Chromosome,
        start: Position,
        end: Position,
    ) -> Vec<&Variant> {
        let chrom_u8 = chrom.as_u8();
        let start_key = (chrom_u8, start.get());
        let end_key = (chrom_u8, end.get());

        self.positional
            .range(start_key..=end_key)
            .map(|(_, v)| v)
            .collect()
    }

    /// Find a variant by dbSNP rsID.
    pub fn find_by_rsid(&self, rsid: &str) -> Option<&Variant> {
        self.by_rsid.get(rsid)
    }

    /// Find all variants of a specific type.
    pub fn find_by_type(&self, variant_type: VariantType) -> Vec<&Variant> {
        self.positional
            .values()
            .filter(|v| v.variant_type == variant_type)
            .collect()
    }

    /// Execute a query against the index.
    pub fn query(&self, query: &GenomicQuery) -> Vec<QueryMatch> {
        match query {
            GenomicQuery::Positional { chrom, pos } => {
                self.find_at(*chrom, *pos)
                    .map(|v| {
                        vec![QueryMatch {
                            variant: v.clone(),
                            matched_by: query.clone(),
                        }]
                    })
                    .unwrap_or_default()
            }
            GenomicQuery::Range { chrom, start, end } => {
                self.find_range(*chrom, *start, *end)
                    .into_iter()
                    .map(|v| QueryMatch {
                        variant: v.clone(),
                        matched_by: query.clone(),
                    })
                    .collect()
            }
            GenomicQuery::RsId(rsid) => {
                self.find_by_rsid(rsid)
                    .map(|v| {
                        vec![QueryMatch {
                            variant: v.clone(),
                            matched_by: query.clone(),
                        }]
                    })
                    .unwrap_or_default()
            }
            GenomicQuery::ByType(vt) => {
                self.find_by_type(*vt)
                    .into_iter()
                    .map(|v| QueryMatch {
                        variant: v.clone(),
                        matched_by: query.clone(),
                    })
                    .collect()
            }
        }
    }
}

impl Default for VariantIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a position is within a given range.
pub fn position_in_range(pos: Position, start: Position, end: Position) -> bool {
    pos.get() >= start.get() && pos.get() <= end.get()
}

/// Count the number of variants matching a query.
pub fn count_matches(index: &VariantIndex, query: &GenomicQuery) -> usize {
    index.query(query).len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use zk_genome_genome_core::Allele;

    fn create_variant(chrom: Chromosome, pos: u32, rsid: Option<&str>) -> Variant {
        let mut v = Variant::new(
            chrom,
            Position::new(pos),
            Allele::Base(b'A'),
            vec![Allele::Base(b'G')],
        );
        v.id = rsid.map(String::from);
        v
    }

    #[test]
    fn test_empty_index() {
        let idx = VariantIndex::new();
        assert!(idx.is_empty());
        assert_eq!(idx.len(), 0);
    }

    #[test]
    fn test_insert_and_find() {
        let mut idx = VariantIndex::new();
        idx.insert(create_variant(Chromosome::Chr1, 100_000, Some("rs1")));
        idx.insert(create_variant(Chromosome::Chr1, 200_000, None));
        idx.insert(create_variant(Chromosome::Chr22, 50_000_000, Some("rs2")));

        assert_eq!(idx.len(), 3);

        let found = idx.find_at(Chromosome::Chr1, Position::new(100_000));
        assert!(found.is_some());
        assert_eq!(found.unwrap().id.as_deref(), Some("rs1"));
    }

    #[test]
    fn test_find_by_rsid() {
        let mut idx = VariantIndex::new();
        idx.insert(create_variant(Chromosome::Chr1, 100_000, Some("rs53576")));

        let found = idx.find_by_rsid("rs53576");
        assert!(found.is_some());
        assert_eq!(found.unwrap().pos.get(), 100_000);
    }

    #[test]
    fn test_range_query() {
        let mut idx = VariantIndex::new();
        idx.insert(create_variant(Chromosome::Chr1, 100_000, None));
        idx.insert(create_variant(Chromosome::Chr1, 150_000, None));
        idx.insert(create_variant(Chromosome::Chr1, 200_000, None));
        idx.insert(create_variant(Chromosome::Chr2, 100_000, None));

        let results = idx.find_range(
            Chromosome::Chr1,
            Position::new(100_000),
            Position::new(150_000),
        );
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_query_positional() {
        let mut idx = VariantIndex::new();
        idx.insert(create_variant(Chromosome::ChrX, 100_000, None));

        let query = GenomicQuery::Positional {
            chrom: Chromosome::ChrX,
            pos: Position::new(100_000),
        };
        let results = idx.query(&query);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_query_rsid() {
        let mut idx = VariantIndex::new();
        idx.insert(create_variant(Chromosome::Chr1, 100_000, Some("rs53576")));

        let results = idx.query(&GenomicQuery::RsId("rs53576".into()));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].variant.pos.get(), 100_000);
    }

    #[test]
    fn test_position_in_range() {
        assert!(position_in_range(
            Position::new(100_000),
            Position::new(50_000),
            Position::new(150_000),
        ));
        assert!(!position_in_range(
            Position::new(200_000),
            Position::new(50_000),
            Position::new(150_000),
        ));
    }

    #[test]
    fn test_find_by_type() {
        let mut idx = VariantIndex::new();
        let snp = create_variant(Chromosome::Chr1, 100_000, None); // A->G = SNP
        let mut del = Variant::new(
            Chromosome::Chr1,
            Position::new(200_000),
            Allele::Inline(heapless::String::try_from("AT").unwrap()),
            vec![Allele::Base(b'A')],
        );
        del.variant_type = VariantType::Deletion;
        idx.insert(snp);
        idx.insert(del);

        let snps = idx.find_by_type(VariantType::Snp);
        assert_eq!(snps.len(), 1);
    }

    #[test]
    fn test_count_matches() {
        let mut idx = VariantIndex::new();
        for i in 0..10 {
            idx.insert(create_variant(Chromosome::Chr1, 100_000 + i * 1000, None));
        }

        let count = count_matches(
            &idx,
            &GenomicQuery::Range {
                chrom: Chromosome::Chr1,
                start: Position::new(100_000),
                end: Position::new(105_000),
            },
        );
        assert_eq!(count, 6);
    }
}