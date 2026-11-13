//! Keller Multi-Solver Portfolio Engine (`KELLER-SOLVE`).
//!
//! Orchestrates competitive racing across multiple SMT solvers:
//! - Z3 (Microsoft Research): Bitvector, Nonlinear real/integer arithmetic
//! - CVC5 (Stanford/Iowa): String theory, arrays, finite fields
//! - Bitwuzla: High-performance QF_BV bitvector solver
//!
//! Spawns competitive solver processes concurrently, returning the fastest conclusive verdict.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::python::solver::{parse_z3_model, SolverVerdict};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SolverBackendKind {
    Z3,
    Cvc5,
    Bitwuzla,
}

impl SolverBackendKind {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Z3 => "z3",
            Self::Cvc5 => "cvc5",
            Self::Bitwuzla => "bitwuzla",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolverCandidate {
    pub kind: SolverBackendKind,
    pub path: PathBuf,
}

/// Discover available SMT solvers installed on the host system or project-local directories.
pub fn discover_portfolio_solvers() -> Vec<SolverCandidate> {
    let mut candidates = Vec::new();

    // 1. Z3 solver discovery
    if let Ok(path) = std::env::var("Z3_PATH") {
        let p = PathBuf::from(path);
        if p.exists() {
            candidates.push(SolverCandidate {
                kind: SolverBackendKind::Z3,
                path: p,
            });
        }
    } else {
        let local_z3 = PathBuf::from(".z3/current/bin/z3.exe");
        if local_z3.exists() {
            candidates.push(SolverCandidate {
                kind: SolverBackendKind::Z3,
                path: local_z3,
            });
        } else if let Ok(which_z3) = which("z3") {
            candidates.push(SolverCandidate {
                kind: SolverBackendKind::Z3,
                path: which_z3,
            });
        }
    }

    // 2. CVC5 solver discovery
    if let Ok(which_cvc5) = which("cvc5") {
        candidates.push(SolverCandidate {
            kind: SolverBackendKind::Cvc5,
            path: which_cvc5,
        });
    }

    // 3. Bitwuzla solver discovery
    if let Ok(which_bitwuzla) = which("bitwuzla") {
        candidates.push(SolverCandidate {
            kind: SolverBackendKind::Bitwuzla,
            path: which_bitwuzla,
        });
    }

    candidates
}

fn which(binary_name: &str) -> Result<PathBuf, ()> {
    let ext = if cfg!(windows) { ".exe" } else { "" };
    let full_name = format!("{binary_name}{ext}");

    if let Ok(path_var) = std::env::var("PATH") {
        let sep = if cfg!(windows) { ';' } else { ':' };
        for part in path_var.split(sep) {
            let candidate = PathBuf::from(part).join(&full_name);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    Err(())
}

/// Dispatches an SMT query to a single solver backend.
pub fn solve_with_backend(
    smt_content: &str,
    candidate: &SolverCandidate,
    timeout_ms: u64,
) -> SolverVerdict {
    let mut cmd = Command::new(&candidate.path);
    let timeout_sec = (timeout_ms / 1000).max(1);

    match candidate.kind {
        SolverBackendKind::Z3 => {
            cmd.arg("-in").arg("-smt2").arg(format!("-T:{timeout_sec}"));
        }
        SolverBackendKind::Cvc5 => {
            cmd.arg("--lang=smt2.6")
                .arg(format!("--tlimit-per={timeout_ms}"))
                .arg("--produce-models");
        }
        SolverBackendKind::Bitwuzla => {
            cmd.arg("-m");
        }
    }

    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return SolverVerdict::Warn {
                reason: format!("Failed to spawn {}: {e}", candidate.kind.name()),
            };
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        let input = format!("{}\n(get-model)\n", smt_content);
        let _ = stdin.write_all(input.as_bytes());
    }

    let output = match child.wait_with_output() {
        Ok(out) => out,
        Err(e) => {
            return SolverVerdict::Warn {
                reason: format!("{} execution failed: {e}", candidate.kind.name()),
            };
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();

    if trimmed.starts_with("unsat") {
        SolverVerdict::Pass
    } else if trimmed.starts_with("sat") {
        let model = parse_z3_model(&stdout);
        SolverVerdict::Fail { model }
    } else {
        SolverVerdict::Warn {
            reason: if trimmed.is_empty() {
                format!("{} returned empty response (timeout)", candidate.kind.name())
            } else {
                trimmed.lines().next().unwrap_or("unknown").to_string()
            },
        }
    }
}

/// Executes a competitive multi-solver race across all available solver backends.
///
/// Returns the first definitive verdict (`Pass` or `Fail`). If all available solvers
/// return inconclusive results or warnings, returns the first warning.
pub fn solve_racing_portfolio(
    smt_content: &str,
    candidates: &[SolverCandidate],
    timeout_ms: u64,
) -> SolverVerdict {
    if candidates.is_empty() {
        return SolverVerdict::Warn {
            reason: "No SMT solvers available in portfolio".to_string(),
        };
    }

    if candidates.len() == 1 {
        return solve_with_backend(smt_content, &candidates[0], timeout_ms);
    }

    let (tx, rx) = mpsc::channel();

    for candidate in candidates {
        let tx = tx.clone();
        let candidate = candidate.clone();
        let content = smt_content.to_string();

        std::thread::spawn(move || {
            let verdict = solve_with_backend(&content, &candidate, timeout_ms);
            let is_decisive = matches!(verdict, SolverVerdict::Pass | SolverVerdict::Fail { .. });
            let _ = tx.send((candidate.kind, verdict, is_decisive));
        });
    }

    drop(tx); // Close original sender

    let mut first_warn = None;
    let mut completed = 0;
    let total = candidates.len();

    while completed < total {
        if let Ok((_kind, verdict, is_decisive)) = rx.recv_timeout(Duration::from_millis(timeout_ms + 500)) {
            completed += 1;
            if is_decisive {
                return verdict;
            } else if first_warn.is_none() {
                first_warn = Some(verdict);
            }
        } else {
            break;
        }
    }

    first_warn.unwrap_or(SolverVerdict::Warn {
        reason: "All portfolio solvers timed out or returned unknown".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solver_backend_names() {
        assert_eq!(SolverBackendKind::Z3.name(), "z3");
        assert_eq!(SolverBackendKind::Cvc5.name(), "cvc5");
        assert_eq!(SolverBackendKind::Bitwuzla.name(), "bitwuzla");
    }

    #[test]
    fn test_portfolio_discovery_finds_z3() {
        let solvers = discover_portfolio_solvers();
        // Z3 is installed in the project root (.z3/current/bin/z3.exe)
        assert!(!solvers.is_empty(), "Should discover at least one solver");
        assert!(
            solvers.iter().any(|s| s.kind == SolverBackendKind::Z3),
            "Z3 should be present in discovered solvers"
        );
    }

    #[test]
    fn test_portfolio_solve_unsat_formula() {
        let solvers = discover_portfolio_solvers();
        if let Some(z3) = solvers.iter().find(|s| s.kind == SolverBackendKind::Z3) {
            let smt = "(set-logic QF_BV)\n(declare-const x (_ BitVec 32))\n(assert (bvult x (_ bv0 32)))\n(check-sat)\n";
            let verdict = solve_with_backend(smt, z3, 5000);
            assert_eq!(verdict, SolverVerdict::Pass);
        }
    }

    #[test]
    fn test_portfolio_solve_sat_formula() {
        let solvers = discover_portfolio_solvers();
        if let Some(z3) = solvers.iter().find(|s| s.kind == SolverBackendKind::Z3) {
            let smt = "(set-logic QF_BV)\n(declare-const x (_ BitVec 32))\n(assert (= x (_ bv42 32)))\n(check-sat)\n(get-model)\n";
            let verdict = solve_with_backend(smt, z3, 5000);
            match verdict {
                SolverVerdict::Fail { model } => {
                    assert!(model.contains_key("x") || model.len() > 0);
                }
                other => panic!("Expected SAT/Fail, got {:?}", other),
            }
        }
    }
}
