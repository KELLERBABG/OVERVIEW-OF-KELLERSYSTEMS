//! `zkg` — Zero-Knowledge Genome demo CLI.
//!
//! Subcommands:
//!    `zkg pipeline <vcf>`            - parse -> normalize -> compress -> index
//!    `zkg demo`                       - synthetic SNP demo without a VCF
//!    `zkg prove-rsids <vcf> <rsids>`  - stub-prove matches for named rsIDs
//!    `zkg stats <vcf>`               - aggregate stats with histogram
//!
//! The CLI never talks to the API; it's a self-contained showcase of the
//! library. Outputs are deterministic so you can re-run and diff them.

#![allow(missing_docs)]
#![allow(clippy::pedantic)]
#![allow(clippy::nursery)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

use sha3::{Digest, Sha3_512};
use zk_genome_entropy_auditor::hash_variant_block;
use zk_genome_genome_annotate::{AnnotationDb, VariantAnnotation};
use zk_genome_genome_compress::compress_variants;
use zk_genome_genome_core::{Chromosome, GenomeSummary, Position, ReferenceGenome, VariantType};
use zk_genome_genome_normalizer::normalize_variant;
use zk_genome_genome_parser::VcfParser;
use zk_genome_query_engine::{GenomicQuery, VariantIndex};
use zk_genome_zk_core::prover::Prover;
use zk_genome_zk_core::ZkQuery;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("pipeline") => match args.get(2) {
            Some(p) => run_pipeline(&PathBuf::from(p)),
            None => {
                eprintln!("usage: zkg pipeline <vcf>");
                ExitCode::from(2)
            }
        },
        Some("demo") => run_demo(),
        Some("prove-rsids") => {
            let path = match args.get(2) {
                Some(p) => PathBuf::from(p),
                None => {
                    eprintln!("usage: zkg prove-rsids <vcf> <comma-separated-rsids>");
                    return ExitCode::from(2);
                }
            };
            let rsids: Vec<String> = args
                .get(3)
                .map(|s| s.split(',').map(|r| r.trim().to_string()).collect())
                .unwrap_or_default();
            run_prove_rsids(&path, &rsids)
        }
        Some("stats") => match args.get(2) {
            Some(p) => run_stats(&PathBuf::from(p)),
            None => {
                eprintln!("usage: zkg stats <vcf>");
                ExitCode::from(2)
            }
        },
        Some("--help") | Some("help") | None => {
            print_help();
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("Unknown subcommand: {other}\n");
            print_help();
            ExitCode::from(2)
        }
    }
}

fn print_help() {
    println!("zkg - Zero-Knowledge Genome demo CLI");
    println!();
    println!("SUBCOMMANDS");
    println!("    pipeline <vcf>              Full pipeline on a real VCF");
    println!("    demo                        Synthetic rs53576/rs4680/rs1800497 demo");
    println!("    prove-rsids <vcf> <list>    Stub-prove matches for named rsIDs");
    println!("    stats <vcf>                 Per-chromosome + per-type histogram");
    println!();
    println!("Run: zkg pipeline data/samples/example.vcf");
}

fn run_pipeline(path: &PathBuf) -> ExitCode {
    println!("Parsing VCF {}", path.display());
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("failed to read {path:?}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let mut parser = VcfParser::new(&bytes);
    let meta = match parser.parse_metadata() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("metadata parse failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    let variants: Vec<_> = match parser.records().collect::<Result<Vec<_>, _>>() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("record parse failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    println!(
        "  fileformat: {}",
        meta.version.as_deref().unwrap_or("(unknown)")
    );
    println!("  variants  : {}", variants.len());
    if variants.is_empty() {
        println!("(no variants to process)");
        return ExitCode::SUCCESS;
    }

    // Normalize.
    let normalized: Vec<_> = variants
        .iter()
        .map(|v| (v.clone(), normalize_variant(v)))
        .collect();
    let mutated = normalized
        .iter()
        .filter(|(_, n)| n.steps_applied > 0)
        .count();
    println!(
        "  normalized: {} ({} required trimming)",
        variants.len(),
        mutated
    );

    // Sort + compress.
    let mut sorted = variants.clone();
    sorted.sort_by_key(|v| (v.chrom.as_u8(), v.pos.get()));
    let compressed = compress_variants(&sorted);
    println!(
        "  compressed: {} SNPs out of {} records (delta-encoded)",
        compressed.iter().filter(|c| c.is_snp).count(),
        compressed.len()
    );

    // SHA-3-512 hash chain over variants.
    let mut prev: [u8; 64] = [0u8; 64];
    for v in &sorted {
        let mut buf = Vec::with_capacity(96);
        buf.extend_from_slice(&prev);
        buf.extend_from_slice(format!("{}:{}", v.chrom, v.pos).as_bytes());
        buf.extend_from_slice(v.ref_allele.as_bytes());
        prev = hash_variant_block(&buf);
    }
    println!("  entropy   : {}", short_hex(&prev));

    // Build the index.
    let mut index = VariantIndex::new();
    for v in &sorted {
        index.insert(v.clone());
    }
    let mut rng = xorshift64(0xc0ffee);
    let sample = sample_n(&sorted, 3.min(sorted.len()), &mut rng);
    println!("  index     : {} entries", index.len());
    println!();
    println!("Sample positional queries:");
    for v in &sample {
        let q = GenomicQuery::Positional { chrom: v.chrom, pos: v.pos };
        let hits = index.query(&q);
        let mark = if hits.is_empty() { " " } else { "+" };
        println!(
            "  {mark} chr{}:{}  -> {} hit(s)  (rsID: {})",
            v.chrom,
            v.pos,
            hits.len(),
            v.id.as_deref().unwrap_or("(no rsID)"),
        );
    }

    // Full summary.
    let mut summary = GenomeSummary {
        total_variants: sorted.len() as u64,
        snp_count: 0,
        insertion_count: 0,
        deletion_count: 0,
        cnv_count: 0,
        structural_count: 0,
        reference_genome: ReferenceGenome::Hg38,
        entropy_root: Some(prev),
    };
    for v in &sorted {
        match v.variant_type {
            VariantType::Snp => summary.snp_count += 1,
            VariantType::Insertion => summary.insertion_count += 1,
            VariantType::Deletion => summary.deletion_count += 1,
            VariantType::Cnv => summary.cnv_count += 1,
            VariantType::Structural => summary.structural_count += 1,
            VariantType::ComplexIndel => {}
        }
    }
    println!();
    println!("Summary");
    println!("  total variants : {}", summary.total_variants);
    println!("  SNPs           : {}", summary.snp_count);
    println!("  insertions     : {}", summary.insertion_count);
    println!("  deletions      : {}", summary.deletion_count);
    println!("  CNVs           : {}", summary.cnv_count);
    println!("  structural     : {}", summary.structural_count);
    println!(
        "  entropy root   : {}...",
        short_hex(&summary.entropy_root.unwrap_or([0u8; 64]))
    );
    ExitCode::SUCCESS
}

fn run_demo() -> ExitCode {
    println!("Synthetic SNP demo (no VCF required)");
    let mut db = AnnotationDb::new();
    db.insert(
        Chromosome::Chr3,
        Position::new(8_809_314),
        VariantAnnotation {
            rsid: Some("rs53576".into()),
            gene_symbol: Some("OXTR".into()),
            clinical_significance: Some("Benign".into()),
            consequence: Some("intronic variant".into()),
            allele_frequency: Some(0.617),
        },
    );
    db.insert(
        Chromosome::Chr22,
        Position::new(19_964_257),
        VariantAnnotation {
            rsid: Some("rs4680".into()),
            gene_symbol: Some("COMT".into()),
            clinical_significance: Some("Pharmacological relevance".into()),
            consequence: Some("missense".into()),
            allele_frequency: Some(0.482),
        },
    );

    let prover = Prover::new();
    struct Demo { rsid: &'static str, chrom: u8, pos: u32, refb: u8, altb: u8 }
    let entries = [
        Demo { rsid: "rs53576",    chrom:  3, pos:  8_809_314, refb: b'A', altb: b'G' },
        Demo { rsid: "rs4680",     chrom: 22, pos: 19_964_257, refb: b'G', altb: b'A' },
        Demo { rsid: "rs1800497",  chrom: 11, pos: 113_270_737, refb: b'C', altb: b'T' },
    ];
    println!();
    println!("Building stub proofs:");
    for e in entries {
        let query = ZkQuery::HasVariant {
            chrom: e.chrom,
            pos: e.pos,
            ref_hash: [e.refb; 32],
            alt_hash: [e.altb; 32],
        };
        match prover.prove(&query, b"compressed_variants", b"proving_key") {
            Ok(proof) => {
                let hash = sha3_512_of(&proof.proof_data);
                println!(
                    "  {:9} chr{}:{}  ref{} alt{} -> proof len={} (hash {}...)",
                    e.rsid, e.chrom, e.pos, e.refb as char, e.altb as char,
                    proof.proof_data.len(), short_hex(&hash),
                );
            }
            Err(err) => println!("  {}: prover failed: {err:?}", e.rsid),
        }
    }

    println!();
    println!("Loaded annotation hints:");
    for rsid in ["rs53576", "rs4680"] {
        match db.find_by_rsid(rsid) {
            Some(a) => println!(
                "  {rsid:9} -> {} (gene: {:?}, AF: {:?})",
                a.clinical_significance.clone().unwrap_or_else(|| "n/a".into()),
                a.gene_symbol,
                a.allele_frequency,
            ),
            None => println!("  {rsid:9} -> (not annotated)"),
        }
    }
    ExitCode::SUCCESS
}

fn run_prove_rsids(path: &PathBuf, rsids: &[String]) -> ExitCode {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("failed to read {path:?}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let mut parser = VcfParser::new(&bytes);
    let _ = match parser.parse_metadata() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("metadata parse failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    let variants: Vec<_> = match parser.records().collect::<Result<Vec<_>, _>>() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("parse failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mut index = VariantIndex::new();
    for v in &variants {
        index.insert(v.clone());
    }

    let prover = Prover::new();
    for rsid in rsids {
        match index.find_by_rsid(rsid) {
            Some(v) => {
                let refb = v.ref_allele.as_bytes()[0];
                let altb = v.alt_alleles.first().map(|a| a.as_bytes()[0]).unwrap_or(0);
                let query = ZkQuery::HasVariant {
                    chrom: v.chrom.as_u8(),
                    pos: v.pos.get(),
                    ref_hash: [refb; 32],
                    alt_hash: [altb; 32],
                };
                match prover.prove(&query, b"compressed_variants", b"proving_key") {
                    Ok(proof) => {
                        let hash = sha3_512_of(&proof.proof_data);
                        println!(
                            "  {rsid}: matched at chr{}:{} ref{} alt{}; proof len={} (hash {}...)",
                            v.chrom,
                            v.pos,
                            refb as char,
                            altb as char,
                            proof.proof_data.len(),
                            short_hex(&hash),
                        );
                    }
                    Err(e) => println!("  {rsid}: prover failed: {e:?}"),
                }
            }
            None => println!("  {rsid}: NOT FOUND"),
        }
    }
    ExitCode::SUCCESS
}

fn run_stats(path: &PathBuf) -> ExitCode {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => return ExitCode::FAILURE,
    };
    let mut parser = VcfParser::new(&bytes);
    if parser.parse_metadata().is_err() {
        return ExitCode::FAILURE;
    }
    let variants: Vec<_> = match parser.records().collect::<Result<Vec<_>, _>>() {
        Ok(v) => v,
        Err(_) => return ExitCode::FAILURE,
    };

    let mut per_chrom: BTreeMap<u8, u64> = BTreeMap::new();
    let mut per_type: BTreeMap<&'static str, u64> = BTreeMap::new();
    for v in &variants {
        *per_chrom.entry(v.chrom.as_u8()).or_insert(0) += 1;
        let key: &'static str = match v.variant_type {
            VariantType::Snp => "snp",
            VariantType::Insertion => "insertion",
            VariantType::Deletion => "deletion",
            VariantType::ComplexIndel => "complex_indel",
            VariantType::Cnv => "cnv",
            VariantType::Structural => "structural",
        };
        *per_type.entry(key).or_insert(0) += 1;
    }

    println!("VARIANT COUNTS - total {}", variants.len());
    println!("  per chromosome:");
    for (chrom, count) in &per_chrom {
        let label = match Chromosome::from_u8(*chrom) {
            Some(c) => c.to_string(),
            None => format!("chr?({chrom})"),
        };
        println!("    {label:>3} : {count}");
    }
    println!("  per type:");
    let max = per_type.values().copied().max().unwrap_or(0);
    for (key, count) in &per_type {
        let bar_width =
            30usize.saturating_mul(*count as usize) / std::cmp::max(max as usize, 1);
        println!("    {key:<15} {count:>5}  {}", "#".repeat(bar_width));
    }
    ExitCode::SUCCESS
}

fn xorshift64(mut state: u64) -> impl FnMut() -> u64 {
    move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    }
}

fn sample_n<T: Clone>(items: &[T], n: usize, rng: &mut impl FnMut() -> u64) -> Vec<T> {
    if items.len() <= n {
        return items.to_vec();
    }
    let step = (items.len() / n).max(1);
    (0..n)
        .map(|i| items[(i * step + (rng() as usize) % step).min(items.len() - 1)].clone())
        .collect()
}

fn short_hex(hash: &[u8; 64]) -> String {
    hash[..8].iter().map(|b| format!("{b:02x}")).collect()
}

fn sha3_512_of(input: &[u8]) -> [u8; 64] {
    let mut hasher = Sha3_512::new();
    hasher.update(input);
    let mut out = [0u8; 64];
    out.copy_from_slice(&hasher.finalize());
    out
}
