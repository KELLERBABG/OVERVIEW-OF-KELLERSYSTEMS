//! Basic analysis example — analyzes a simple contract and prints the report.
//!
//! Run with: `cargo run --example basic_analysis`

use symbolic_execution_core::*;

fn main() {
    // Minimal bytecode: PUSH1 0x42 PUSH1 0x00 MSTORE PUSH1 0x20 PUSH1 0x00 RETURN
    let bytecode = vec![
        0x60, 0x42, // PUSH1 0x42
        0x60, 0x00, // PUSH1 0x00
        0x52,       // MSTORE
        0x60, 0x20, // PUSH1 0x20
        0x60, 0x00, // PUSH1 0x00
        0xf3,       // RETURN
    ];

    println!("=== Symbolic Execution Engine Example ===");
    println!("Analyzing bytecode: {}", hex::encode(&bytecode));
    println!();

    // Create the analysis engine with default config
    let engine = AnalysisEngine::new();
    let report = engine.analyze_bytecode(&bytecode);

    // Print results
    println!("Analysis Results:");
    println!("  Bytecode size: {} bytes", report.ir.bytecode.len());
    println!("  Function selectors: {:?}", report.ir.function_selectors);
    println!("  CFG blocks: {}", report.ir.cfg.blocks.len());
    println!("  Paths explored: {}", report.exploration.paths_explored);
    println!("  States visited: {}", report.exploration.states_visited);
    println!("  Vulnerabilities found: {}", report.vulnerabilities.vulnerabilities.len());
    println!("  Gas analysis results: {}", report.gas_results.len());
    println!("  Spec candidates: {}", report.spec_candidates.len());

    // Print spec candidates
    println!("\nSpecification Candidates:");
    for spec in &report.spec_candidates {
        println!("  - [{}] {} (confidence: {:.2})",
            if spec.likely_intended { "✓" } else { "?" },
            spec.name,
            spec.confidence
        );
        println!("    Formula: {}", spec.formula);
    }

    println!("\nDone.");
}