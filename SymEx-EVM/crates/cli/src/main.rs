//! Symbolic Execution Engine — CLI
//!
//! Command-line interface for analyzing smart contracts. Supports multiple
//! input formats (raw bytecode hex, Solidity source files via compilation),
//! output formats (JSON, human-readable), and various analysis options.

use clap::{Parser, Subcommand, Args};
use std::path::PathBuf;

/// Symbolic execution engine for smart contract formal verification.
///
/// Analyzes EVM bytecode for vulnerabilities, gas issues, and verifies
/// formal invariants using symbolic execution and SMT solving.
#[derive(Parser)]
#[command(name = "symex")]
#[command(author, version = symbolic_execution_core::VERSION, about)]
struct Cli {
    /// Subcommand to execute
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Output format (human, json)
    #[arg(short, long, global = true, default_value = "human")]
    format: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze a smart contract from bytecode hex
    Analyze(AnalyzeArgs),

    /// Scan a contract for known vulnerability patterns
    Scan(ScanArgs),

    /// Analyze gas consumption of a contract
    Gas(GasArgs),

    /// List supported vulnerability detectors
    Detectors,

    /// Show version information
    Version,
}

#[derive(Args)]
struct AnalyzeArgs {
    /// Path to bytecode hex file (.hex or .bin)
    #[arg(short, long)]
    input: PathBuf,

    /// Function selector(s) to analyze (e.g., "a9059cbb")
    #[arg(short, long)]
    function: Vec<String>,

    /// Maximum path depth for exploration
    #[arg(long, default_value = "1024")]
    max_depth: usize,

    /// Exploration timeout in seconds
    #[arg(long, default_value = "300")]
    timeout: u64,
}

#[derive(Args)]
struct ScanArgs {
    /// Path to bytecode hex file
    #[arg(short, long)]
    input: PathBuf,

    /// Specific detectors to run (runs all if not specified)
    #[arg(short, long)]
    detector: Vec<String>,
}

#[derive(Args)]
struct GasArgs {
    /// Path to bytecode hex file
    #[arg(short, long)]
    input: PathBuf,

    /// Block gas limit to compare against
    #[arg(long, default_value = "30000000")]
    block_gas_limit: u64,
}

fn main() {
    let cli = Cli::parse();

    // Initialize tracing
    if cli.verbose {
        tracing_subscriber::fmt::init();
    }

    match cli.command {
        Commands::Analyze(args) => cmd_analyze(args, &cli.format),
        Commands::Scan(args) => cmd_scan(args, &cli.format),
        Commands::Gas(args) => cmd_gas(args, &cli.format),
        Commands::Detectors => cmd_detectors(),
        Commands::Version => cmd_version(),
    }
}

fn cmd_analyze(args: AnalyzeArgs, _format: &str) {
    eprintln!("[symex] Loading bytecode from: {:?}", args.input);
    let bytecode = match load_bytecode(&args.input) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[symex] Error loading bytecode: {}", e);
            std::process::exit(1);
        }
    };

    let engine = symbolic_execution_core::AnalysisEngine::new();
    let report = engine.analyze_bytecode(&bytecode);

    println!("=== Symbolic Execution Analysis Report ===");
    println!("Bytecode size: {} bytes", bytecode.len());
    println!("Function selectors found: {}", report.ir.function_selectors.len());
    println!("Vulnerabilities detected: {}", report.vulnerabilities.vulnerabilities.len());
    println!("Gas analysis results: {}", report.gas_results.len());
    println!("Spec candidates: {}", report.spec_candidates.len());

    if !report.vulnerabilities.vulnerabilities.is_empty() {
        println!("\nVulnerabilities:");
        for v in &report.vulnerabilities.vulnerabilities {
            println!("  [{}] {} — {}", v.severity, v.name, v.description);
        }
    }

    if !report.spec_candidates.is_empty() {
        println!("\nGenerated Specification Candidates:");
        for spec in &report.spec_candidates {
            println!("  - {} (confidence: {:.2})", spec.name, spec.confidence);
        }
    }
}

fn cmd_scan(args: ScanArgs, _format: &str) {
    eprintln!("[symex] Scanning bytecode from: {:?}", args.input);
    let bytecode = match load_bytecode(&args.input) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[symex] Error loading bytecode: {}", e);
            std::process::exit(1);
        }
    };

    let engine = symbolic_execution_core::AnalysisEngine::new();
    let report = engine.analyze_bytecode(&bytecode);

    println!("=== Vulnerability Scan Report ===");
    println!("Detectors run: {}", report.vulnerabilities.detectors_run);
    println!("Issues found: {}", report.vulnerabilities.vulnerabilities.len());

    if report.vulnerabilities.vulnerabilities.is_empty() {
        println!("\nNo vulnerabilities detected.");
    } else {
        for v in &report.vulnerabilities.vulnerabilities {
            println!("\n  [{}] {}", v.severity, v.name);
            println!("    Location: {}", v.location);
            println!("    Description: {}", v.description);
            if let Some(swc) = v.swc_id {
                println!("    SWC ID: SWC-{:03}", swc);
            }
        }
    }
}

fn cmd_gas(args: GasArgs, _format: &str) {
    eprintln!("[symex] Analyzing gas from: {:?}", args.input);
    let bytecode = match load_bytecode(&args.input) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[symex] Error loading bytecode: {}", e);
            std::process::exit(1);
        }
    };

    let engine = symbolic_execution_core::AnalysisEngine::new();
    let report = engine.analyze_bytecode(&bytecode);

    println!("=== Gas Analysis Report ===");
    println!("Block gas limit: {}", args.block_gas_limit);
    println!("Functions analyzed: {}", report.gas_results.len());

    for gas_result in &report.gas_results {
        println!("\n  Function: {}", gas_result.function_name);
        println!("    Min gas: {}", gas_result.min_gas);
        println!("    Max gas: {}", gas_result.max_gas);
        if gas_result.has_unbounded_loop {
            println!("    ⚠️  Potentially unbounded loop detected!");
        }
        if gas_result.max_gas > args.block_gas_limit {
            println!("    ❌ Exceeds block gas limit!");
        }
    }
}

fn cmd_detectors() {
    let scanner = symbolic_execution_core::VulnerabilityScanner::new();
    println!("=== Available Vulnerability Detectors ===");
    println!("Total detectors: {}", scanner.detectors.len());
    for detector in &scanner.detectors {
        println!("  - {}", detector.name());
    }
}

fn cmd_version() {
    println!("Symbolic Execution Engine v{}", symbolic_execution_core::VERSION);
}

/// Load bytecode from a hex file or raw binary file.
fn load_bytecode(path: &PathBuf) -> Result<Vec<u8>, String> {
    let data = std::fs::read(path).map_err(|e| format!("failed to read {}: {}", path.display(), e))?;

    // Try to decode as hex if the file looks like hex text
    if data.first().map_or(false, |b| b.is_ascii_hexdigit()) {
        let s = String::from_utf8(data).map_err(|_| "invalid UTF-8 in hex file".to_string())?;
        hex::decode(s.trim()).map_err(|e| format!("hex decode error: {}", e))
    } else {
        Ok(data)
    }
}