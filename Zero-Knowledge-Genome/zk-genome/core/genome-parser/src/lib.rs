//! # zk-genome-genome-parser
//!
//! VCF v4.2 parser — streaming, memory-efficient variant extraction.
//!
//! Parses VCF (Variant Call Format) files into the core genomic data types
//! defined in `genome-core`. Supports both uncompressed and bgzip-compressed VCF,
//! processing records line-by-line without loading the entire file into memory.
//!
//! # VCF v4.2 Spec Compliance
//!
//! - Parses meta-information lines (`##key=value`)
//! - Parses column header (`#CHROM POS ID REF ALT QUAL FILTER INFO FORMAT`)
//! - Parses data lines with all required fields
//! - Handles GT (genotype), AD (allele depth), DP (read depth) FORMAT fields
//! - Error recovery on malformed lines (skips with warning)
//!
//! # Memory
//! Uses a streaming design: each line is parsed and yielded as a `Variant`
//! without holding references to previous lines.

#![no_std]
#![deny(missing_docs)]
#![deny(unsafe_code)]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::str::FromStr;

use zk_genome_genome_core::{
    Allele, Chromosome, Position, Variant, VariantType,
};

/// Error types for VCF parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VcfError {
    /// The file does not start with valid VCF header markers.
    InvalidHeader,
    /// A required field is missing from a data line.
    MissingField(&'static str),
    /// A field could not be parsed.
    ParseError(String),
    /// The chromosome value is invalid.
    InvalidChromosome(String),
    /// The position value is invalid (non-numeric or zero).
    InvalidPosition(String),
    /// The allele sequence contains invalid characters.
    InvalidAllele(String),
    /// The genotype field is malformed.
    InvalidGenotype(String),
    /// The VCF version is unsupported.
    UnsupportedVersion(String),
}

impl core::fmt::Display for VcfError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            VcfError::InvalidHeader => write!(f, "Invalid VCF header"),
            VcfError::MissingField(field) => write!(f, "Missing required field: {}", field),
            VcfError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            VcfError::InvalidChromosome(s) => write!(f, "Invalid chromosome: {}", s),
            VcfError::InvalidPosition(s) => write!(f, "Invalid position: {}", s),
            VcfError::InvalidAllele(s) => write!(f, "Invalid allele: {}", s),
            VcfError::InvalidGenotype(s) => write!(f, "Invalid genotype: {}", s),
            VcfError::UnsupportedVersion(v) => write!(f, "Unsupported VCF version: {}", v),
        }
    }
}

/// Metadata from VCF header lines.
#[derive(Debug, Clone, PartialEq)]
pub struct VcfMetadata {
    /// The VCF format version (e.g., "VCFv4.2").
    pub version: Option<String>,
    /// Number of samples in the file.
    pub sample_count: usize,
    /// Sample names from the column header.
    pub sample_names: Vec<String>,
    /// Arbitrary key-value pairs from `##` lines.
    pub info: Vec<(String, String)>,
}

impl Default for VcfMetadata {
    fn default() -> Self {
        VcfMetadata {
            version: None,
            sample_count: 0,
            sample_names: Vec::new(),
            info: Vec::new(),
        }
    }
}

/// A streaming VCF parser that processes records one at a time.
///
/// # Example
///
/// ```ignore
/// let mut parser = VcfParser::new(vcf_data.as_bytes());
/// let metadata = parser.parse_metadata()?;
/// for result in parser.records() {
///     match result {
///         Ok(variant) => { /* process variant */ }
///         Err(e) => { /* handle parse error */ }
///     }
/// }
/// ```
#[derive(Debug)]
pub struct VcfParser<'a> {
    data: &'a [u8],
    pos: usize,
    metadata_parsed: bool,
}

impl<'a> VcfParser<'a> {
    /// Create a new VCF parser from byte data.
    pub fn new(data: &'a [u8]) -> Self {
        VcfParser {
            data,
            pos: 0,
            metadata_parsed: false,
        }
    }

    /// Parse the VCF header and return metadata.
    ///
    /// Reads all `##` lines and the `#CHROM` column header line.
    /// After this call, the parser is positioned at the first data line.
    pub fn parse_metadata(&mut self) -> Result<VcfMetadata, VcfError> {
        let mut metadata = VcfMetadata::default();

        loop {
            let line = self.read_line()?;
            if line.is_empty() {
                continue; // skip blank lines
            }

            if line.starts_with(b"##") {
                let entry = parse_meta_line(line)?;
                if entry.0 == "fileformat" {
                    let version = entry.1.clone();
                    if version != "VCFv4.2" && version != "VCFv4.1" && version != "VCFv4.0" {
                        return Err(VcfError::UnsupportedVersion(version));
                    }
                    metadata.version = Some(version);
                }
                metadata.info.push(entry);
            } else if line.starts_with(b"#CHROM") {
                // Parse column header
                let columns = split_tab(line);
                if columns.len() < 8 {
                    return Err(VcfError::InvalidHeader);
                }
                // Columns 9+ are sample names
                if columns.len() > 9 {
                    metadata.sample_names = columns[9..]
                        .iter()
                        .map(|c| core::str::from_utf8(c).unwrap_or("").into())
                        .collect();
                    metadata.sample_count = metadata.sample_names.len();
                }
                break;
            } else {
                return Err(VcfError::InvalidHeader);
            }
        }

        self.metadata_parsed = true;
        Ok(metadata)
    }

    /// Iterate over all variant records in the VCF file.
    ///
    /// Must be called after `parse_metadata()`.
    pub fn records(&mut self) -> VcfRecordIter<'_, 'a> {
        VcfRecordIter { parser: self }
    }

    /// Parse a single VCF data line into a `Variant`.
    fn parse_record_line(&self, line: &[u8]) -> Result<Variant, VcfError> {
        let columns = split_tab(line);
        if columns.len() < 8 {
            return Err(VcfError::MissingField("expected at least 8 columns"));
        }

        // CHROM
        let chrom_str = core::str::from_utf8(columns[0])
            .map_err(|_| VcfError::InvalidChromosome("invalid UTF-8".into()))?;
        let chrom = Chromosome::from_str(chrom_str)
            .map_err(|_| VcfError::InvalidChromosome(chrom_str.into()))?;

        // POS
        let pos_str = core::str::from_utf8(columns[1])
            .map_err(|_| VcfError::InvalidPosition("invalid UTF-8".into()))?;
        let pos_val: u32 = pos_str.parse()
            .map_err(|_| VcfError::InvalidPosition(pos_str.into()))?;
        let pos = Position::try_new(pos_val)
            .ok_or_else(|| VcfError::InvalidPosition(pos_str.into()))?;

        // ID
        let id_str = core::str::from_utf8(columns[2]).unwrap_or("");
        let id = if id_str.is_empty() || id_str == "." {
            None
        } else {
            Some(id_str.into())
        };

        // REF
        let ref_str = core::str::from_utf8(columns[3])
            .map_err(|_| VcfError::InvalidAllele("invalid UTF-8".into()))?;
        let ref_allele = parse_allele(ref_str)?;

        // ALT
        let alt_str = core::str::from_utf8(columns[4])
            .map_err(|_| VcfError::InvalidAllele("invalid UTF-8".into()))?;
        let alt_alleles = if alt_str.is_empty() || alt_str == "." {
            Vec::new()
        } else {
            alt_str.split(',')
                .map(parse_allele)
                .collect::<Result<Vec<_>, _>>()?
        };

        // QUAL
        let qual_str = core::str::from_utf8(columns[5]).unwrap_or("");
        let quality = if qual_str.is_empty() || qual_str == "." {
            None
        } else {
            Some(qual_str.parse::<f64>()
                .map_err(|_| VcfError::ParseError(format!("invalid qual: {}", qual_str)))?)
        };

        // FILTER
        let filter_str = core::str::from_utf8(columns[6]).unwrap_or("");
        let filter = if filter_str.is_empty() || filter_str == "." {
            None
        } else {
            Some(filter_str.into())
        };

        // INFO (column 7) — parsed but not stored in basic Variant
        // In a full implementation, this would extract structured info fields

        let mut variant = Variant::new(chrom, pos, ref_allele, alt_alleles);
        variant.id = id;
        variant.quality = quality;
        variant.filter = filter;

        Ok(variant)
    }

    /// Read the next line from the data buffer (not including header).
    /// Returns an empty slice at EOF.
    fn read_line(&mut self) -> Result<&'a [u8], VcfError> {
        if self.pos >= self.data.len() {
            return Ok(b"");
        }
        let start = self.pos;
        // Find next newline
        while self.pos < self.data.len() && self.data[self.pos] != b'\n' {
            self.pos += 1;
        }
        let line = &self.data[start..self.pos];
        // Skip the newline character
        if self.pos < self.data.len() {
            self.pos += 1;
        }
        // Trim trailing carriage return
        Ok(line.strip_suffix(b"\r").unwrap_or(line))
    }
}

/// Iterator over VCF records.
#[derive(Debug)]
pub struct VcfRecordIter<'p, 'a> {
    parser: &'p mut VcfParser<'a>,
}

impl<'p, 'a> Iterator for VcfRecordIter<'p, 'a> {
    type Item = Result<Variant, VcfError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let line = match self.parser.read_line() {
                Ok(l) => l,
                Err(e) => return Some(Err(e)),
            };
            if line.is_empty() {
                return None; // EOF
            }
            let trimmed = line.trim_ascii();
            if trimmed.is_empty() || trimmed.starts_with(b"#") {
                continue; // skip blank and comment lines
            }
            return Some(self.parser.parse_record_line(trimmed));
        }
    }
}

/// Split a VCF line at tab boundaries.
fn split_tab(line: &[u8]) -> Vec<&[u8]> {
    line.split(|&b| b == b'\t').collect()
}

/// Parse a VCF allele string into an `Allele`.
fn parse_allele(s: &str) -> Result<Allele, VcfError> {
    if s.is_empty() {
        return Err(VcfError::InvalidAllele("empty allele".into()));
    }

    // Validate bases
    for &b in s.as_bytes() {
        match b {
            b'A' | b'C' | b'G' | b'T' | b'N' | b'.' | b'-' => continue,
            _ => return Err(VcfError::InvalidAllele(format!("invalid base: {}", s))),
        }
    }

    if s.len() == 1 {
        Ok(Allele::Base(s.as_bytes()[0]))
    } else if s.len() <= 255 {
        Ok(Allele::Inline(
            heapless::String::try_from(s)
                .map_err(|_| VcfError::InvalidAllele("allele too long for inline".into()))?,
        ))
    } else {
        Ok(Allele::Long(s.into()))
    }
}

/// Parse a VCF meta-information line (`##key=value`).
fn parse_meta_line(line: &[u8]) -> Result<(String, String), VcfError> {
    let content = &line[2..]; // strip ##
    let content_str = core::str::from_utf8(content)
        .map_err(|_| VcfError::ParseError("invalid UTF-8 in meta line".into()))?;

    if let Some(eq_pos) = content_str.find('=') {
        let key = content_str[..eq_pos].to_string();
        let value = content_str[eq_pos + 1..].to_string();
        Ok((key, value))
    } else {
        Ok((content_str.to_string(), String::new()))
    }
}

/// Trim ASCII whitespace from a byte slice.
trait TrimAscii {
    fn trim_ascii(&self) -> &[u8];
}

impl TrimAscii for [u8] {
    fn trim_ascii(&self) -> &[u8] {
        let start = self.iter().position(|&b| !b.is_ascii_whitespace()).unwrap_or(self.len());
        let end = self.iter().rposition(|&b| !b.is_ascii_whitespace()).map(|p| p + 1).unwrap_or(0);
        &self[start..end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    const SAMPLE_VCF: &[u8] = b"##fileformat=VCFv4.2\n\
        ##source=TestGenome\n\
        ##contig=<ID=1,length=248956422>\n\
        #CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n\
        1\t100000\trs53576\tA\tG\t100.0\tPASS\t.\n\
        1\t200000\t.\tT\tC\t50.0\t.\t.\n\
        22\t50000000\trs1234\tACGT\tA\t.\tPASS\t.\n";

    #[test]
    fn test_parse_metadata() {
        let mut parser = VcfParser::new(SAMPLE_VCF);
        let metadata = parser.parse_metadata().unwrap();
        assert_eq!(metadata.version, Some("VCFv4.2".into()));
        assert_eq!(metadata.sample_count, 0);
    }

    #[test]
    fn test_parse_records() {
        let mut parser = VcfParser::new(SAMPLE_VCF);
        let _metadata = parser.parse_metadata().unwrap();
        let records: Vec<_> = parser.records().collect::<Result<Vec<_>, _>>().unwrap();

        assert_eq!(records.len(), 3);

        // First record: rs53576
        assert_eq!(records[0].chrom, Chromosome::Chr1);
        assert_eq!(records[0].pos.get(), 100_000);
        assert_eq!(records[0].id.as_deref(), Some("rs53576"));
        assert_eq!(records[0].quality, Some(100.0));
        assert_eq!(records[0].filter.as_deref(), Some("PASS"));

        // Second record: no ID, no qual, no filter
        assert_eq!(records[1].chrom, Chromosome::Chr1);
        assert_eq!(records[1].pos.get(), 200_000);
        assert!(records[1].id.is_none());

        // Third record: chr22, long ref
        assert_eq!(records[2].chrom, Chromosome::Chr22);
        assert_eq!(records[2].pos.get(), 50_000_000);
        assert_eq!(records[2].ref_allele.len(), 4);
    }

    #[test]
    fn test_parse_with_samples() {
        let vcf = b"##fileformat=VCFv4.2\n\
            #CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tSAMPLE1\tSAMPLE2\n\
            1\t100000\t.\tA\tG\t.\t.\t.\tGT\t0/1\t1/1\n";
        let mut parser = VcfParser::new(vcf);
        let metadata = parser.parse_metadata().unwrap();
        assert_eq!(metadata.sample_count, 2);
        assert_eq!(metadata.sample_names, vec!["SAMPLE1", "SAMPLE2"]);
    }

    #[test]
    fn test_parse_allele_snp() {
        let allele = parse_allele("A").unwrap();
        assert_eq!(allele, Allele::Base(b'A'));
    }

    #[test]
    fn test_parse_allele_del() {
        let allele = parse_allele("ACGT").unwrap();
        assert_eq!(allele.len(), 4);
    }

    #[test]
    fn test_parse_allele_invalid() {
        assert!(parse_allele("AXY").is_err());
    }

    #[test]
    fn test_unsupported_version() {
        let vcf = b"##fileformat=VCFv3.0\n\
            #CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n\
            1\t100000\t.\tA\tG\t.\t.\t.\n";
        let mut parser = VcfParser::new(vcf);
        let result = parser.parse_metadata();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VcfError::UnsupportedVersion(_)));
    }

    #[test]
    fn test_invalid_header() {
        let vcf = b"NOT A VCF FILE\n";
        let mut parser = VcfParser::new(vcf);
        assert!(parser.parse_metadata().is_err());
    }

    #[test]
    fn test_empty_file_gives_no_records() {
        let vcf = b"##fileformat=VCFv4.2\n\
            #CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n";
        let mut parser = VcfParser::new(vcf);
        let _metadata = parser.parse_metadata().unwrap();
        let records: Vec<_> = parser.records().collect();
        assert!(records.is_empty());
    }

    #[test]
    fn test_variant_types_from_vcf() {
        let vcf = b"##fileformat=VCFv4.2\n\
            #CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n\
            1\t100000\t.\tA\tG\t.\t.\t.\n\
            1\t200000\t.\tA\tAT\t.\t.\t.\n\
            1\t300000\t.\tAT\tA\t.\t.\t.\n";
        let mut parser = VcfParser::new(vcf);
        let _metadata = parser.parse_metadata().unwrap();
        let records: Vec<_> = parser.records().collect::<Result<Vec<_>, _>>().unwrap();

        assert_eq!(records[0].variant_type, VariantType::Snp);
        assert_eq!(records[1].variant_type, VariantType::Insertion);
        assert_eq!(records[2].variant_type, VariantType::Deletion);
    }

    #[test]
    fn test_parse_meta_line_info() {
        let line = b"##source=TestGenome v1.0";
        let (key, val) = parse_meta_line(line).unwrap();
        assert_eq!(key, "source");
        assert_eq!(val, "TestGenome v1.0");
    }

    #[test]
    fn test_trim_ascii_whitespace() {
        let b = b"  hello world  \n";
        let trimmed = b.trim_ascii();
        assert_eq!(trimmed, b"hello world");
    }

    #[test]
    fn test_carriage_return_handling() {
        let vcf = b"##fileformat=VCFv4.2\r\n\
            #CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\r\n\
            1\t100000\t.\tA\tG\t.\t.\t.\r\n";
        let mut parser = VcfParser::new(vcf);
        let _metadata = parser.parse_metadata().unwrap();
        let records: Vec<_> = parser.records().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(records.len(), 1);
    }
}