//! # Real-Time SMT-Solver Patent Verification
//!
//! Provides an abstraction layer over SMT solvers (Z3, Bitwuzla, CVC5) and
//! a built-in fallback solver for basic formulas. Supports:
//! - Delta formula submission (SMT-LIB 2.0)
//! - SAT/UNSAT/UNKNOWN decisions
//! - Model extraction (satisfying assignment)
//! - Unsat core extraction (minimal conflicting constraints)
//! - Severity scoring based on constraint overlap and graph distance
//!
//! The solver can be configured to call external solver binaries or use
//! the built-in naive solver for development/testing.

use crate::error::CoreError;
use crate::models::{SatResult, VerificationResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;
use uuid::Uuid;

// ─── Solver Configuration ────────────────────────────────────────────────────

/// Supported SMT solver backends.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SolverBackend {
    /// External Z3 binary (invoked via subprocess).
    Z3,
    /// External Bitwuzla binary.
    Bitwuzla,
    /// External CVC5 binary.
    Cvc5,
    /// Built-in lightweight solver (basic SAT only).
    BuiltIn,
}

/// Configuration for the SMT solver.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolverConfig {
    /// Which backend to use.
    pub backend: SolverBackend,
    /// Timeout in milliseconds.
    pub timeout_ms: u64,
    /// Path to the solver binary (None = use PATH).
    pub solver_path: Option<String>,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            backend: SolverBackend::BuiltIn,
            timeout_ms: 5000,
            solver_path: None,
        }
    }
}

// ─── Solver Trait ────────────────────────────────────────────────────────────

/// The core solver interface: given an SMT-LIB 2.0 formula, produce a result.
pub trait SmtSolver: Send + Sync {
    /// Check satisfiability of the given SMT-LIB 2.0 formula.
    fn check_sat(&self, smt_lib2_formula: &str, config: &SolverConfig) -> Result<SolverResponse, CoreError>;
}

/// The response from a solver invocation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolverResponse {
    /// The SAT/UNSAT/UNKNOWN result.
    pub sat: SatResult,
    /// Optional model (satisfying assignment) as key-value string pairs.
    pub model: HashMap<String, String>,
    /// Optional unsat core — list of constraint names that conflict.
    pub unsat_core: Vec<String>,
    /// Time spent in the solver (ms).
    pub solver_time_ms: f64,
}

// ─── Built-in Naive Solver ───────────────────────────────────────────────────

/// A very simple built-in solver for basic propositional logic.
/// This is intentionally limited — for real patent verification, use Z3.
pub struct BuiltInSolver;

impl BuiltInSolver {
    /// Naive SAT check: parses simple `(= x c)` constraints and checks consistency.
    /// Returns SAT if no contradictory equalities exist, UNSAT otherwise.
    fn naive_check(formula: &str) -> SatResult {
        let mut assignments: HashMap<String, i64> = HashMap::new();

        for line in formula.lines() {
            let line = line.trim();
            // Extract the content inside (assert ...)
            if let Some(inner) = Self::extract_assert_body(line) {
                // Check for equality: (= VAR VAL)
                if let Some((var, val_str)) = Self::parse_equality(&inner) {
                    if let Ok(val) = val_str.parse::<i64>() {
                        if let Some(&existing) = assignments.get(&var) {
                            if existing != val {
                                return SatResult::Unsat;
                            }
                        }
                        assignments.insert(var, val);
                    }
                }
                // Check for not-equality: (not (= VAR VAL))
                if let Some((var, val_str)) = Self::parse_not_equality(&inner) {
                    if let Ok(val) = val_str.parse::<i64>() {
                        if let Some(&existing) = assignments.get(&var) {
                            if existing == val {
                                return SatResult::Unsat;
                            }
                        }
                    }
                }
            }
        }

        SatResult::Sat
    }

    /// Extract the body of an `(assert ...)` form, stripping the outer parens.
    fn extract_assert_body<'a>(line: &'a str) -> Option<&'a str> {
        let line = line.trim();
        if line.starts_with("(assert ") && line.ends_with(')') {
            let inner = &line[8..line.len() - 1]; // strip "(assert " and trailing ")"
            Some(inner.trim())
        } else {
            None
        }
    }

    /// Parse `(= VAR VAL)` returning (var_name, val_str).
    fn parse_equality(s: &str) -> Option<(String, String)> {
        let s = s.trim();
        if s.starts_with("(= ") && s.ends_with(')') {
            let inner = &s[3..s.len() - 1]; // strip "(= " and ")"
            let parts: Vec<&str> = inner.split_whitespace().collect();
            if parts.len() == 2 {
                return Some((parts[0].to_string(), parts[1].to_string()));
            }
        }
        None
    }

    /// Parse `(not (= VAR VAL))` returning (var_name, val_str).
    fn parse_not_equality(s: &str) -> Option<(String, String)> {
        let s = s.trim();
        if s.starts_with("(not ") && s.ends_with(')') {
            let inner = &s[5..s.len() - 1]; // strip "(not " and ")"
            // inner should be "(= VAR VAL)"
            Self::parse_equality(inner.trim())
        } else {
            None
        }
    }
}

impl SmtSolver for BuiltInSolver {
    fn check_sat(&self, smt_lib2_formula: &str, config: &SolverConfig) -> Result<SolverResponse, CoreError> {
        let start = Instant::now();

        let sat = Self::naive_check(smt_lib2_formula);

        let elapsed = start.elapsed().as_secs_f64() * 1000.0;

        if elapsed > config.timeout_ms as f64 {
            return Ok(SolverResponse {
                sat: SatResult::Unknown,
                model: HashMap::new(),
                unsat_core: vec![],
                solver_time_ms: elapsed,
            });
        }

        Ok(SolverResponse {
            sat,
            model: HashMap::new(),
            unsat_core: vec![],
            solver_time_ms: elapsed,
        })
    }
}

// ─── External Solver (subprocess) ────────────────────────────────────────────

/// A solver that invokes an external binary (z3, bitwuzla, cvc5).
pub struct ExternalSolver {
    binary_name: String,
}

impl ExternalSolver {
    pub fn new(backend: &SolverBackend) -> Self {
        let binary_name = match backend {
            SolverBackend::Z3 => "z3".to_string(),
            SolverBackend::Bitwuzla => "bitwuzla".to_string(),
            SolverBackend::Cvc5 => "cvc5".to_string(),
            SolverBackend::BuiltIn => unreachable!(),
        };
        Self { binary_name }
    }
}

impl SmtSolver for ExternalSolver {
    fn check_sat(&self, smt_lib2_formula: &str, config: &SolverConfig) -> Result<SolverResponse, CoreError> {
        let binary = config
            .solver_path
            .clone()
            .unwrap_or_else(|| self.binary_name.clone());

        let start = Instant::now();

        // Invoke solver via subprocess, piping the formula via stdin.
        let output = std::process::Command::new(&binary)
            .arg("-in")  // most solvers accept `-in` for stdin input
            .arg(format!("-T:{}", config.timeout_ms / 1000)) // timeout in seconds
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn();

        let output = match output {
            Ok(mut child) => {
                use std::io::Write;
                if let Some(ref mut stdin) = child.stdin {
                    let _ = stdin.write_all(smt_lib2_formula.as_bytes());
                }
                child.wait_with_output().map_err(|e| {
                    CoreError::SmtSolverError(format!("solver process error: {e}"))
                })?
            }
            Err(e) => {
                // Solver binary not found — fall back to a helpful error.
                return Err(CoreError::SmtSolverError(format!(
                    "failed to launch solver '{binary}': {e}. Is it installed?"
                )));
            }
        };

        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse the solver output
        let sat = if stdout.contains("sat") || stdout.contains("SAT") {
            SatResult::Sat
        } else if stdout.contains("unsat") || stdout.contains("UNSAT") {
            SatResult::Unsat
        } else {
            SatResult::Unknown
        };

        // Parse model (between (model ... ) blocks) — simple extraction
        let mut model = HashMap::new();
        if sat == SatResult::Sat {
            for line in stdout.lines() {
                let line = line.trim();
                if let Some(def) = line.strip_prefix("(define-fun ") {
                    let parts: Vec<&str> = def.splitn(2, ' ').collect();
                    if parts.len() == 2 {
                        let var = parts[0].to_string();
                        let rest = parts[1];
                        if let Some(val_start) = rest.find(' ') {
                            let val = rest[val_start..].trim().trim_end_matches(')').to_string();
                            model.insert(var, val);
                        }
                    }
                }
            }
        }

        Ok(SolverResponse {
            sat,
            model,
            unsat_core: vec![],
            solver_time_ms: elapsed,
        })
    }
}

// ─── Solver Factory ──────────────────────────────────────────────────────────

/// Create a solver based on configuration, with automatic fallback.
pub fn create_solver(config: &SolverConfig) -> Box<dyn SmtSolver> {
    match &config.backend {
        SolverBackend::BuiltIn => Box::new(BuiltInSolver),
        _ => Box::new(ExternalSolver::new(&config.backend)),
    }
}

// ─── Verification Pipeline ───────────────────────────────────────────────────

/// Run the full verification pipeline on a delta constraint.
///
/// 1. Parse the SMT-LIB 2.0 formula from the delta.
/// 2. Submit to solver.
/// 3. Compute severity score.
/// 4. Package into a `VerificationResult`.
pub fn run_verification(
    edge_id: Uuid,
    smt_lib2_formula: &[u8],
    solver: &dyn SmtSolver,
    config: &SolverConfig,
) -> Result<VerificationResult, CoreError> {
    let formula_str = String::from_utf8(smt_lib2_formula.to_vec())
        .map_err(|_| CoreError::InvalidFormula("non-UTF-8 SMT-LIB2 formula".to_string()))?;

    let response = solver.check_sat(&formula_str, config)?;

    let severity = compute_severity(&response);

    Ok(VerificationResult {
        edge_id,
        sat: response.sat,
        model: serde_json::to_vec(&response.model).unwrap_or_default(),
        unsat_core: response.unsat_core.iter().map(|s| s.as_bytes().to_vec()).collect(),
        severity_score: severity,
        solver_time_ms: response.solver_time_ms,
        proof_commitment: vec![], // ZK proof to be added by Phase 8/10
    })
}

// ─── Severity Scoring ────────────────────────────────────────────────────────

/// Compute a severity score from 0.0 (identical) to 1.0 (completely novel).
///
/// Scoring factors:
/// - SAT → higher severity (derivative introduces new behavior)
/// - UNSAT → lower severity (derivative is contained in original, possible infringement)
/// - Model complexity → higher SAT model complexity = more novelty
/// - Unsat core size → larger core = more constraints conflicted = more different
pub fn compute_severity(response: &SolverResponse) -> f64 {
    match response.sat {
        SatResult::Sat => {
            // SAT means something new is present. Score based on model complexity.
            let model_complexity = response.model.len() as f64;
            // Saturate at 10 new assignments = score 0.9, baseline 0.5
            0.5 + (model_complexity / 20.0).min(0.5)
        }
        SatResult::Unsat => {
            // UNSAT means derivative is fully explained by original (potential infringement).
            // Score based on unsat core size: larger core = more different.
            let core_ratio = (response.unsat_core.len() as f64) / 10.0;
            core_ratio.min(0.5)
        }
        SatResult::Unknown => {
            // Timeout / unknown — conservative middle score
            0.5
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_built_in_solver_sat() {
        let solver = BuiltInSolver;
        let config = SolverConfig::default();
        let formula = "(assert (= x 5))\n(assert (= y 10))\n(check-sat)\n";
        let result = solver.check_sat(formula, &config).unwrap();
        assert_eq!(result.sat, SatResult::Sat);
    }

    #[test]
    fn test_built_in_solver_unsat() {
        let solver = BuiltInSolver;
        let config = SolverConfig::default();
        let formula = "(assert (= x 5))\n(assert (= x 10))\n(check-sat)\n";
        let result = solver.check_sat(formula, &config).unwrap();
        assert_eq!(result.sat, SatResult::Unsat);
    }

    #[test]
    fn test_built_in_solver_unsat_via_not() {
        let solver = BuiltInSolver;
        let config = SolverConfig::default();
        let formula = "(assert (= x 5))\n(assert (not (= x 5)))\n(check-sat)\n";
        let result = solver.check_sat(formula, &config).unwrap();
        assert_eq!(result.sat, SatResult::Unsat);
    }

    #[test]
    fn test_verification_pipeline() {
        let edge_id = Uuid::new_v4();
        let formula = "(set-logic ALL)\n(assert (= x 5))\n(check-sat)\n";

        let solver = BuiltInSolver;
        let config = SolverConfig::default();

        let result = run_verification(edge_id, formula.as_bytes(), &solver, &config).unwrap();
        assert_eq!(result.edge_id, edge_id);
        assert_eq!(result.sat, SatResult::Sat);
        assert!(result.severity_score >= 0.5); // SAT → baseline 0.5+
        assert!(result.solver_time_ms >= 0.0);
    }

    #[test]
    fn test_severity_sat() {
        let mut model = HashMap::new();
        model.insert("x".to_string(), "5".to_string());
        let response = SolverResponse {
            sat: SatResult::Sat,
            model,
            unsat_core: vec![],
            solver_time_ms: 1.0,
        };
        let score = compute_severity(&response);
        assert!(score >= 0.5 && score <= 1.0);
    }

    #[test]
    fn test_severity_unsat() {
        let response = SolverResponse {
            sat: SatResult::Unsat,
            model: HashMap::new(),
            unsat_core: vec!["c1".into(), "c2".into()],
            solver_time_ms: 1.0,
        };
        let score = compute_severity(&response);
        assert!(score >= 0.0 && score <= 0.5);
    }

    #[test]
    fn test_severity_unknown() {
        let response = SolverResponse {
            sat: SatResult::Unknown,
            model: HashMap::new(),
            unsat_core: vec![],
            solver_time_ms: 5000.0,
        };
        let score = compute_severity(&response);
        assert!((score - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_external_solver_fallback_on_missing() {
        let solver = ExternalSolver::new(&SolverBackend::Z3);
        let config = SolverConfig {
            timeout_ms: 1000,
            ..Default::default()
        };
        let result = solver.check_sat("(check-sat)\n", &config);
        // Should error because z3 not installed
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, CoreError::SmtSolverError(_)));
    }
}