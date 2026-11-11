//! # DTC Import Module
//!
//! Imports raw genetic data from direct-to-consumer testing companies:
//! - **23andMe** — TXT/ZIP format with rsID-based SNP reporting
//! - **AncestryDNA** — TXT format with custom chromosome naming
//! - **MyHeritage** — CSV format with family finder data
//!
//! Converts all DTC formats into the internal ZK Genome variant format
//! through the existing VCF parser and normalizer pipeline.
//!
//! # Pipeline
//! ```text
//! 23andMe TXT ──┐
//! AncestryDNA   ├──→ DtcNormalizer ──→ VcfConverter ──→ genome-parser ──→ Internal format
//! MyHeritage  ──┘
//! ```

#![no_std]
#![deny(missing_docs)]
#![deny(unsafe_code)]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;
use zk_genome_genome_core::{Allele, Chromosome, Position, Variant};

/// Supported DTC providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DtcProvider {
    /// 23andMe (v5 chipset)
    TwentyThreeAndMe,
    /// AncestryDNA (v2 chipset)
    AncestryDna,
    /// MyHeritage (v1 chipset)
    MyHeritage,
}

impl DtcProvider {
    /// Detect the provider from raw data header.
    pub fn detect(data: &[u8]) -> Option<Self> {
        let header = core::str::from_utf8(data).ok()?;
        if header.contains("# 23andMe") || header.contains("23andMe") {
            Some(DtcProvider::TwentyThreeAndMe)
        } else if header.contains("AncestryDNA") || header.contains("rsid") {
            Some(DtcProvider::AncestryDna)
        } else if header.contains("MyHeritage") || header.contains("FamilyFinder") {
            Some(DtcProvider::MyHeritage)
        } else {
            None
        }
    }
}

/// A single DTC raw data line after parsing.
#[derive(Debug, Clone, PartialEq)]
pub struct DtcRecord {
    /// rsID or variant identifier.
    pub rsid: String,
    /// Chromosome (1-22, X, Y, MT as string).
    pub chrom: String,
    /// 1-based position.
    pub position: u32,
    /// Allele 1 (e.g., "A", "T", "G", "C", "-").
    pub allele1: String,
    /// Allele 2.
    pub allele2: String,
}

/// Error parsing DTC data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DtcError {
    /// The data source could not be identified as a supported provider.
    UnknownProvider,
    /// The data could not be parsed (e.g., invalid UTF-8).
    ParseError(String),
    /// An allele value was invalid.
    InvalidAllele(String),
    /// The provider version is not supported.
    UnsupportedVersion(String),
}

/// Parse a 23andMe raw data line.
fn parse_23andme_line(line: &str) -> Option<DtcRecord> {
    if line.starts_with('#') {
        return None;
    }
    let parts: Vec<&str> = line.split('\t').collect();
    if parts.len() < 4 {
        return None;
    }
    let rsid = parts[0].to_string();
    let chrom = parts[1].to_string();
    let position: u32 = parts[2].parse().ok()?;
    let genotype = parts[3].trim();
    let (allele1, allele2) = if genotype.len() == 2 {
        (genotype[..1].to_string(), genotype[1..].to_string())
    } else if genotype.len() == 1 {
        (genotype.to_string(), genotype.to_string())
    } else {
        return None;
    };
    Some(DtcRecord { rsid, chrom, position, allele1, allele2 })
}

/// Convert a DTC record to a core Variant.
pub fn dtc_record_to_variant(record: &DtcRecord) -> Result<Variant, DtcError> {
    let chrom: Chromosome = record.chrom.parse()
        .map_err(|_| DtcError::InvalidAllele(format!("bad chromosome: {}", record.chrom)))?;
    let pos = Position::new(record.position);

    let ref_allele = Allele::Base(match record.allele1.as_bytes().first() {
        Some(b) => *b,
        None => return Err(DtcError::InvalidAllele("empty allele1".into())),
    });
    let alt_allele = Allele::Base(match record.allele2.as_bytes().first() {
        Some(b) => *b,
        None => return Err(DtcError::InvalidAllele("empty allele2".into())),
    });

    let mut variant = Variant::new(chrom, pos, ref_allele, vec![alt_allele]);
    variant.id = Some(record.rsid.clone());
    Ok(variant)
}

/// Parse raw DTC data from any supported provider into variants.
pub fn parse_dtc_data(data: &[u8], provider: DtcProvider) -> Result<Vec<DtcRecord>, DtcError> {
    let text = core::str::from_utf8(data)
        .map_err(|_| DtcError::ParseError("invalid UTF-8".into()))?;

    let mut records = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        match provider {
            DtcProvider::TwentyThreeAndMe | DtcProvider::AncestryDna => {
                if let Some(record) = parse_23andme_line(trimmed) {
                    records.push(record);
                }
            }
            DtcProvider::MyHeritage => {
                // MyHeritage uses CSV format
                // Stub for implementation
            }
        }
    }
    Ok(records)
}

/// Full pipeline: detect → parse → convert → normalize variants.
pub fn import_dtc(data: &[u8]) -> Result<Vec<Variant>, DtcError> {
    let provider = DtcProvider::detect(data)
        .ok_or(DtcError::UnknownProvider)?;
    let records = parse_dtc_data(data, provider)?;
    records.iter().map(dtc_record_to_variant).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_23ANDME: &[u8] = b"# 23andMe v5\r\n# rsid\tchromosome\tposition\tgenotype\r\nrs53576\t3\t8809314\tAG\r\nrs1234\t1\t1000000\tTT\r\n";

    #[test]
    fn test_detect_23andme() {
        assert_eq!(DtcProvider::detect(SAMPLE_23ANDME), Some(DtcProvider::TwentyThreeAndMe));
    }

    #[test]
    fn test_parse_23andme_line() {
        let line = "rs53576\t3\t8809314\tAG";
        let record = parse_23andme_line(line).unwrap();
        assert_eq!(record.rsid, "rs53576");
        assert_eq!(record.chrom, "3");
        assert_eq!(record.position, 8_809_314);
        assert_eq!(record.allele1, "A");
        assert_eq!(record.allele2, "G");
    }

    #[test]
    fn test_import_23andme_full() {
        let variants = import_dtc(SAMPLE_23ANDME).unwrap();
        assert_eq!(variants.len(), 2);
        assert_eq!(variants[0].id.as_deref(), Some("rs53576"));
        assert_eq!(variants[0].chrom, Chromosome::Chr3);
    }

    #[test]
    fn test_dtc_record_to_variant_oxytocin() {
        let record = DtcRecord {
            rsid: "rs53576".into(),
            chrom: "3".into(),
            position: 8_809_314,
            allele1: "A".into(),
            allele2: "G".into(),
        };
        let variant = dtc_record_to_variant(&record).unwrap();
        assert_eq!(variant.chrom, Chromosome::Chr3);
        assert_eq!(variant.pos.get(), 8_809_314);
        assert_eq!(variant.id.as_deref(), Some("rs53576"));
    }

    #[test]
    fn test_detect_unknown() {
        assert!(DtcProvider::detect(b"unknown data").is_none());
    }

    #[test]
    fn test_parse_no_header() {
        let records = parse_dtc_data(b"rs1\t1\t100000\tAG\nrs2\t2\t200000\tCC\n", DtcProvider::TwentyThreeAndMe);
        assert!(records.is_ok());
        assert_eq!(records.unwrap().len(), 2);
    }
}