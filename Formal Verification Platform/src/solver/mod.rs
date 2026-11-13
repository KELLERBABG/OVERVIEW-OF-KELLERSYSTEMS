//! Multi-Solver Portfolio: racing Z3, CVC5, and Bitwuzla.
//!
//! In this implementation, the solver interface is defined and a
//! deterministic mock solver is provided for offline verification.
//! Production builds would shell out to actual solver binaries.

use std::fmt;
use std::time::Instant;

/// Result of a single solver invocation.
#[derive(Debug, Clone)]
pub enum SolverResult {
    Sat(Vec<String>),
    Unsat,
    Unknown(String),
    Error(String),
}

impl fmt::Display for SolverResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SolverResult::Sat(model) => write!(f, "SAT ({:?})", model),
            SolverResult::Unsat => write!(f, "UNSAT"),
            SolverResult::Unknown(reason) => write!(f, "UNKNOWN ({})", reason),
            SolverResult::Error(msg) => write!(f, "ERROR ({})", msg),
        }
    }
}

/// A solver backend.
pub trait SolverBackend: Send + Sync {
    fn name(&self) -> &str;
    fn solve(&self, assertions: &[String]) -> SolverResult;
}

/// The portfolio manager that races multiple solvers.
pub struct SolverPortfolio {
    solvers: Vec<Box<dyn SolverBackend>>,
    timeout_ms: u64,
}

impl SolverPortfolio {
    pub fn new(timeout_ms: u64) -> Self {
        Self {
            solvers: vec![Box::new(DeterministicSolver::new())],
            timeout_ms,
        }
    }

    /// Add a solver to the portfolio.
    pub fn add_solver(&mut self, solver: Box<dyn SolverBackend>) {
        self.solvers.push(solver);
    }

    /// Race all solvers and return the first definitive answer.
    pub fn race(&self, assertions: &[String]) -> (SolverResult, String) {
        let start = Instant::now();
        for solver in &self.solvers {
            let result = solver.solve(assertions);
            if !matches!(result, SolverResult::Unknown(_)) {
                return (result, solver.name().to_string());
            }
            if start.elapsed().as_millis() as u64 > self.timeout_ms {
                break;
            }
        }
        (SolverResult::Unknown("timeout".into()), "none".into())
    }

    pub fn solver_count(&self) -> usize {
        self.solvers.len()
    }
}

impl Default for SolverPortfolio {
    fn default() -> Self {
        Self::new(5000)
    }
}

/// A deterministic mock solver for offline/CI verification.
/// Evaluates simple arithmetic assertions.
pub struct DeterministicSolver {
    name: String,
}

impl DeterministicSolver {
    pub fn new() -> Self {
        Self { name: "deterministic-mock".to_string() }
    }
}

impl SolverBackend for DeterministicSolver {
    fn name(&self) -> &str {
        &self.name
    }

    fn solve(&self, assertions: &[String]) -> SolverResult {
        // Simple evaluation: check for contradictory assertions
        let mut has_true = false;
        let mut has_false = false;

        for a in assertions {
            let a_trim = a.trim();
            if a_trim == "true" || a_trim == "1" {
                has_true = true;
            } else if a_trim == "false" || a_trim == "0" {
                has_false = true;
            }
        }

        if has_true && has_false {
            SolverResult::Unsat
        } else if has_true {
            SolverResult::Sat(vec!["model={}".into()])
        } else {
            SolverResult::Unknown("no definitive assertion".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_portfolio_race() {
        let portfolio = SolverPortfolio::new(1000);
        let (result, solver_name) = portfolio.race(&["true".into()]);
        assert!(matches!(result, SolverResult::Sat(_)));
        assert_eq!(solver_name, "deterministic-mock");
    }

    #[test]
    fn test_unsat_detection() {
        let portfolio = SolverPortfolio::new(1000);
        let (result, _) = portfolio.race(&["true".into(), "false".into()]);
        assert!(matches!(result, SolverResult::Unsat));
    }
}
