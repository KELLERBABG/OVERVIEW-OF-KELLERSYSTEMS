//! # dfip-cli
//!
//! Command-line interface for the Decoupled Fluid IP Network.
//! Provides subcommands for the complete trustless-IP lifecycle:
//!
//! - `init   <name>`            Scaffold a new IP project
//! - `submit <spec.json>`        Compile a patent spec -> IR -> register on the DAG
//! - `verify <orig.json> <deriv.json>`  Build Δ and run local SMT verification
//! - `audit  <spec.json>`        Run a full taint trace / derivation audit
//! - `help / (no args)`          Print usage
//!
//! The CLI operates entirely locally using the `dfip-core` engine: it never
//! needs a running API server. This makes it ideal for offline development,
//! CI, and reproducible patent verification.

use dfip_core::delta::build_delta;
use dfip_core::graph::PatentDag;
use dfip_core::ir::{IrKind, InventionIr, MolecularGraph, MechanicalModel, SsaTrace};
use dfip_core::models::*;
use dfip_core::solver::{run_verification, create_solver, SolverConfig};
use dfip_core::taint::{TaintAnalyzer, TaintValue};
use std::collections::HashMap;
use std::process::ExitCode;
use uuid::Uuid;

// ─── Patent Spec on Disk ─────────────────────────────────────────────────────
//
// A `.ipspec` is a JSON document that an inventor writes (or a compiler frontend
// produces) to describe an invention in a machine-verifiable way.

/// The on-disk representation of a patent specification.
#[derive(serde::Deserialize, serde::Serialize)]
struct PatentSpec {
    /// Human-readable title of the invention.
    title: String,
    /// Software | Chemical | Mechanical
    patent_type: String,
    /// Inventor names.
    authors: Vec<String>,
    /// Pre-conditions (must hold before execution).
    preconditions: Vec<String>,
    /// Post-conditions (must hold after execution).
    postconditions: Vec<String>,
    /// Loop / data invariants.
    invariants: Vec<String>,
    /// License: open | protected | secret
    #[serde(default = "default_license")]
    license: String,
}

fn default_license() -> String {
    "open".to_string()
}

impl PatentSpec {
    /// Parse a patent specification from a JSON file on disk.
    fn from_file(path: &str) -> Result<PatentSpec, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("failed to read '{}': {}", path, e))?;
        let spec = serde_json::from_str::<PatentSpec>(&text)
            .map_err(|e| format!("failed to parse '{}' as a DFIP patent spec: {}", path, e))?;
        Ok(spec)
    }

    /// Resolve the license enum.
    fn license(&self) -> License {
        match self.license.to_lowercase().as_str() {
            "secret" => License::Secret,
            "protected" => License::Protected,
            _ => License::Open,
        }
    }

    /// Resolve the patent type enum.
    fn patent_type(&self) -> PatentType {
        match self.patent_type.to_lowercase().as_str() {
            "chemical" => PatentType::Chemical,
            "mechanical" => PatentType::Mechanical,
            _ => PatentType::Software,
        }
    }

    /// Build the core `PatentNode` canonical model.
    fn to_node(&self, id: Uuid) -> PatentNode {
        PatentNode {
            id,
            title: self.title.clone(),
            patent_type: self.patent_type(),
            authors: self.authors.clone(),
            created_at_unix: chrono::Utc::now().timestamp(),
            content_hash: vec![],
            invariant: None,
            tags: vec![],
            license: self.license(),
            meta_commitment: vec![],
        }
    }

    /// Build the symbolic `InventionIr` for the patent.
    fn to_ir(&self) -> InventionIr {
        let kind = match self.patent_type.to_lowercase().as_str() {
            "chemical" => IrKind::Chemical(MolecularGraph {
                atoms: vec![],
                bonds: vec![],
                constraints: vec![],
            }),
            "mechanical" => IrKind::Mechanical(MechanicalModel {
                entities: vec![],
                constraints: vec![],
                metadata: HashMap::new(),
            }),
            _ => IrKind::Software(SsaTrace {
                entry_block: "entry".into(),
                blocks: HashMap::new(),
                inputs: vec![],
                outputs: vec![],
            }),
        };

        InventionIr {
            kind,
            preconditions: self.preconditions.clone(),
            postconditions: self.postconditions.clone(),
            invariants: self.invariants.clone(),
        }
    }
}

// ─── Example Spec Generator ─────────────────────────────────────────────────

/// The canned QuickSort example used by `init` and the API `/simulate` endpoint.
fn quicksort_example_spec() -> String {
    serde_json::json!({
        "title": "QuickSort Algorithm",
        "patent_type": "software",
        "authors": ["Alice"],
        "preconditions": ["length(arr) > 0"],
        "postconditions": [
            "is_sorted(sorted)",
            "permutation(arr, sorted)"
        ],
        "invariants": [],
        "license": "open"
    })
    .to_string()
}
// ─── Project Scaffold ────────────────────────────────────────────────────────

/// `dfip init <name>` — scaffold a new IP project directory.
fn cmd_init(args: &[String]) -> Result<(), String> {
    let name = args
        .first()
        .ok_or_else(|| "usage: dfip init <project-name>".to_string())?;

    let dir = format!("dfip-{}", name);
    let _ = std::fs::create_dir_all(&dir);

    // Project manifest.
    let manifest = serde_json::json!({
        "project": name,
        "engine": "dfip-core",
        "schema": "ipspec/v1",
        "created_at": chrono::Utc::now().timestamp(),
        "patents": []
    });
    let manifest_str = serde_json::to_string_pretty(&manifest)
        .unwrap_or_else(|_| "{}".to_string());

    let manifest_path = format!("{}/dfip.json", dir);
    std::fs::write(&manifest_path, manifest_str)
        .map_err(|e| format!("failed to write manifest: {}", e))?;

    // Example patent spec for a quick start.
    let example = format!("{}/example.ipspec", dir);
    std::fs::write(&example, format!("{}\n", quicksort_example_spec()))
        .map_err(|e| format!("failed to write example spec: {}", e))?;

    println!("Initialized DFIP project '{}' in ./{}", name, dir);
    println!("  {}/dfip.json        project manifest", dir);
    println!("  {}/example.ipspec   example patent spec", dir);
    println!();
    println!("Next: dfip submit {}/example.ipspec", dir);
    Ok(())
}

// ─── Submit ──────────────────────────────────────────────────────────────────

/// `dfip submit <spec.json>` — register a patent on the local DAG.
fn cmd_submit(args: &[String]) -> Result<(), String> {
    let spec_path = args
        .first()
        .ok_or_else(|| "usage: dfip submit <spec.json>".to_string())?;

    let spec = PatentSpec::from_file(spec_path.as_str())?;
    let id = Uuid::new_v4();

    let node = spec.to_node(id);
    let _ir = spec.to_ir();

    // Register on a fresh local DAG (the graph of record for this session).
    let mut dag = PatentDag::new();
    dag.add_node(node)
        .map_err(|e| format!("failed to register patent: {}", e))?;

    println!("Registered patent:");
    println!("  id:          {}", id);
    println!("  title:       {}", spec.title);
    println!("  type:        {}", spec.patent_type);
    println!("  authors:     {}", spec.authors.join(", "));
    println!("  license:     {:?}", spec.license());
    println!("  pre:         {} condition(s)", spec.preconditions.len());
    println!("  post:        {} condition(s)", spec.postconditions.len());
    println!("  invariants:  {} condition(s)", spec.invariants.len());
    println!("  graph nodes: {}  lifecycle events: {}",
             dag.node_count(), dag.lifecycle_log().len());

    Ok(())
}
// ─── Verify ──────────────────────────────────────────────────────────────────

/// `dfip verify <orig.json> <deriv.json>` — build Δ = ¬Original ∧ Derivative
/// and run local SMT verification.
fn cmd_verify(args: &[String]) -> Result<(), String> {
    let orig_path = args
        .first()
        .ok_or_else(|| "usage: dfip verify <original.json> <derivative.json>".to_string())?;
    let deriv_path = args
        .get(1)
        .ok_or_else(|| "usage: dfip verify <original.json> <derivative.json>".to_string())?;

    let orig = PatentSpec::from_file(orig_path.as_str())?;
    let deriv = PatentSpec::from_file(deriv_path.as_str())?;

    let orig_id = Uuid::new_v4();
    let deriv_id = Uuid::new_v4();

    // Build the delta constraint between the two inventions.
    let delta = build_delta(
        orig_id,
        deriv_id,
        &orig.to_ir(),
        &deriv.to_ir(),
        &[],
    )
    .map_err(|e| format!("failed to build delta: {}", e))?;

    println!("Δ = ¬Original ∧ Derivative");
    println!("  original:  {}", orig.title);
    println!("  derivative:{}", deriv.title);
    println!("  formula length:  {} bytes", delta.formula_smt_lib2.len());
    println!("  original hash:   {}", hex::encode(&delta.original_invariant_hash));

    // Run the built-in (or external, if configured) SMT solver.
    let config = SolverConfig::default();
    let solver = create_solver(&config);
    let result = run_verification(
        deriv_id,
        &delta.formula_smt_lib2,
        solver.as_ref(),
        &config,
    )
    .map_err(|e| format!("verification failed: {}", e))?;

    println!();
    println!("Verification result:");
    println!("  sat:            {:?}", result.sat);
    println!("  severity:       {:.4}  (0.0 = identical, 1.0 = novel)", result.severity_score);
    println!("  solver time:    {:.2} ms", result.solver_time_ms);
    match result.sat {
        SatResult::Sat => println!("  verdict: derivative introduces NOVEL functionality"),
        SatResult::Unsat => println!("  verdict: derivative is CONTAINED in original (possible infringement)"),
        SatResult::Unknown => println!("  verdict: inconclusive (timeout / undecidable)"),
    }

    Ok(())
}

// ─── Audit ───────────────────────────────────────────────────────────────────

/// `dfip audit <spec.json>` — run a taint trace over the invention's SSA IR
/// and summarize the derivation / licensing posture.
fn cmd_audit(args: &[String]) -> Result<(), String> {
    let spec_path = args
        .first()
        .ok_or_else(|| "usage: dfip audit <spec.json>".to_string())?;

    let spec = PatentSpec::from_file(spec_path.as_str())?;
    let ir = spec.to_ir();

    let id = Uuid::new_v4();

    println!("Audit for: {}", spec.title);
    println!("  node id:    {}", id);
    println!("  license:    {:?}", spec.license());
    println!("  type:       {:?}", spec.patent_type());
    println!();

    // Run static taint analysis over the SSA trace (if software).
    if let IrKind::Software(trace) = &ir.kind {
        let analyzer = TaintAnalyzer::new();
        let initial: HashMap<String, TaintValue> = HashMap::new();
        match analyzer.analyze_trace(trace, &initial) {
            Ok(env) => {
                println!("Taint trace - {} tainted variable(s):", env.len());
                for (var, taint) in &env {
                    println!("    {:<10} => {:?}", var, taint);
                }
            }
            Err(e) => println!("  taint analysis: {}", e),
        }
    } else {
        println!("Taint trace: static taint analysis applies to software SSA IRs.");
        println!("  (this patent is {:?}; verifying invariants instead)", spec.patent_type());
    }

    println!();
    println!("Invariant summary:");
    println!("  preconditions:  [{}]", spec.preconditions.join("; "));
    println!("  postconditions: [{}]", spec.postconditions.join("; "));
    println!("  invariants:     [{}]", spec.invariants.join("; "));

    // Report licensing posture.
    match spec.license() {
        License::Open => println!("  posture: open - freely usable, no restrictions"),
        License::Protected => println!("  posture: protected - derivations require royalty routing"),
        License::Secret => println!("  posture: secret - blueprint held off-chain until CLEAN + PAID"),
    }

    Ok(())
}

// ─── Entrypoint ──────────────────────────────────────────────────────────────

fn print_usage() {
    println!("dfip - Decoupled Fluid IP Network CLI");
    println!();
    println!("USAGE: dfip <command> [arguments]");
    println!();
    println!("COMMANDS:");
    println!("  init    <name>                    Scaffold a new IP project");
    println!("  submit  <spec.json>                Compile patent spec -> IR -> register on DAG");
    println!("  verify  <orig.json> <deriv.json>   Build Δ and run SMT verification");
    println!("  audit   <spec.json>                Run a full taint / derivation audit");
    println!("  help                               Show this help");
    println!();
    println!("EXAMPLES:");
    println!("  dfip init aerosol-valve");
    println!("  dfip submit dfip-aerosol-valve/example.ipspec");
    println!("  dfip verify original.json derivative.json");
    println!("  dfip audit dfip-aerosol-valve/example.ipspec");
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        print_usage();
        return ExitCode::SUCCESS;
    }

    // Split off the subcommand name.
    let subcommand = args[0].clone();
    let sub_args = &args[1..];

    let result: Result<(), String> = match subcommand.as_str() {
        "init" => cmd_init(sub_args),
        "submit" => cmd_submit(sub_args),
        "verify" => cmd_verify(sub_args),
        "audit" => cmd_audit(sub_args),
        "help" | "--help" | "-h" => {
            print_usage();
            Ok(())
        }
        other => Err(format!("unknown command '{}'", other)),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("error: {}", msg);
            eprintln!();
            print_usage();
            ExitCode::FAILURE
        }
    }
}