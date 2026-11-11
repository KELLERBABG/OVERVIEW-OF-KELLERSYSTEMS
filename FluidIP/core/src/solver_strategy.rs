//! # Solver Strategies for Undecidable / Intractable Constraints
//!
//! Addresses Critique #2. Strategy chain with fallback: Full SMT → Incremental → Abstract → Triage.

use crate::error::CoreError;
use crate::models::*;
use crate::solver::{ExternalSolver, SmtSolver, SolverConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SolverStrategy { FullSmt, IncrementalSmt, AbstractInterpretation, Triage, BuiltIn }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrategyResult {
    pub result: VerificationResult,
    pub strategy_used: SolverStrategy,
    pub solver_time_ms: f64,
    pub total_time_ms: f64,
    pub strategy_chain: Vec<(SolverStrategy, String, f64)>,
    pub is_conclusive: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolverBudget {
    pub full_smt_timeout_ms: u64,
    pub incremental_timeout_ms: u64,
    pub abstract_interpretation_timeout_ms: u64,
    pub max_sub_formulas: usize,
}
impl Default for SolverBudget {
    fn default() -> Self {
        Self { full_smt_timeout_ms: 30_000, incremental_timeout_ms: 60_000,
            abstract_interpretation_timeout_ms: 10_000, max_sub_formulas: 10 }
    }
}

// ─── Abstract Interpretation ────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Interval { pub lo: f64, pub hi: f64 }
impl Interval {
    pub fn new(lo: f64, hi: f64) -> Self { Self { lo, hi } }
    pub fn meet(&self, other: &Self) -> Option<Self> {
        let lo_val = f64::max(self.lo, other.lo);
        let hi_val = f64::min(self.hi, other.hi);
        if lo_val <= hi_val { Some(Self { lo: lo_val, hi: hi_val }) } else { None }
    }
}

pub struct AbstractInterpreter;
impl AbstractInterpreter {
    pub fn check_sat(constraints: &[String]) -> Option<bool> {
        let mut intervals: HashMap<String, Interval> = HashMap::new();
        for c in constraints {
            let parts: Vec<&str> = c.split_whitespace().collect();
            if parts.len() != 3 { continue; }
            let var = parts[0].to_string();
            let val: f64 = parts[2].parse().ok()?;
            let ni = match parts[1] {
                ">" | ">=" => Interval::new(val, f64::INFINITY),
                "<" | "<=" => Interval::new(f64::NEG_INFINITY, val),
                "==" => Interval::new(val, val),
                _ => continue,
            };
            match intervals.get(&var) {
                Some(ex) => { if let Some(m) = ex.meet(&ni) { intervals.insert(var, m); } else { return Some(false); } }
                None => { intervals.insert(var, ni); }
            }
        }
        Some(true)
    }
}

// ─── Incremental Solver ─────────────────────────────────────────────────────

pub struct IncrementalSolver;
impl IncrementalSolver {
    pub fn decompose(formula: &str, max: usize) -> Vec<String> {
        let mut parts = Vec::new();
        let mut depth = 0;
        let mut cur = String::new();
        for ch in formula.chars() {
            match ch {
                '(' => { depth += 1; cur.push(ch); }
                ')' => { depth -= 1; cur.push(ch);
                    if depth == 0 && cur.contains("assert") {
                        parts.push(::std::mem::take(&mut cur));
                        if parts.len() >= max { break; }
                    }
                }
                _ => { cur.push(ch); }
            }
        }
        if parts.is_empty() { parts.push(formula.to_string()); }
        parts
    }
    pub fn check_sat(formula: &str, solver: &dyn SmtSolver, config: &SolverConfig, budget: &SolverBudget)
        -> Result<(SatResult, f64), CoreError>
    {
        let start = Instant::now();
        for part in Self::decompose(formula, budget.max_sub_formulas) {
            if start.elapsed().as_millis() as u64 > budget.incremental_timeout_ms {
                return Ok((SatResult::Unknown, start.elapsed().as_secs_f64() * 1000.0));
            }
            let cfg = SolverConfig { timeout_ms: 5000, ..config.clone() };
            let r = solver.check_sat(&part, &cfg)?;
            if r.sat == SatResult::Unsat {
                return Ok((SatResult::Unsat, start.elapsed().as_secs_f64() * 1000.0));
            }
        }
        Ok((SatResult::Sat, start.elapsed().as_secs_f64() * 1000.0))
    }
}

// ─── Strategy Executor ───────────────────────────────────────────────────────

pub struct StrategyExecutor { budget: SolverBudget }
impl StrategyExecutor {
    pub fn new(budget: SolverBudget) -> Self { Self { budget } }
    pub fn execute(&self, edge_id: Uuid, formula: &[u8], config: &SolverConfig) -> StrategyResult {
        let timeout = Duration::from_millis(config.timeout_ms);
        let start = Instant::now();
        let mut chain: Vec<(SolverStrategy, String, f64)> = Vec::new();
        let fs = String::from_utf8_lossy(formula);
        let solver: Box<dyn SmtSolver> = match &config.backend {
            crate::solver::SolverBackend::BuiltIn => Box::new(crate::solver::BuiltInSolver),
            _ => Box::new(ExternalSolver::new(&config.backend)),
        };
        let ext_cfg = SolverConfig { timeout_ms: self.budget.full_smt_timeout_ms, ..config.clone() };

        // Strategy 1: Full SMT — uses config.timeout_ms directly.
        // If the solver exceeds the budget, the response will be Unknown,
        // triggering automatic fallthrough to the next strategy.
        match solver.check_sat(&fs, &ext_cfg) {
            Ok(r) if r.sat != SatResult::Unknown => {
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                chain.push((SolverStrategy::FullSmt, "ok".into(), elapsed));
                let sev = crate::solver::compute_severity(&r);
                return StrategyResult {
                    result: VerificationResult { edge_id, sat: r.sat,
                        model: serde_json::to_vec(&r.model).unwrap_or_default(),
                        unsat_core: vec![], severity_score: sev,
                        solver_time_ms: r.solver_time_ms, proof_commitment: vec![] },
                    strategy_used: SolverStrategy::FullSmt, solver_time_ms: r.solver_time_ms,
                    total_time_ms: elapsed, strategy_chain: chain, is_conclusive: true };
            }
            Ok(_) => chain.push((SolverStrategy::FullSmt, "timeout".into(), start.elapsed().as_secs_f64()*1000.0)),
            Err(e) => chain.push((SolverStrategy::FullSmt, format!("err: {e}"), start.elapsed().as_secs_f64()*1000.0)),
        }

        if let Ok((sat, t)) = IncrementalSolver::check_sat(&fs, &*solver, &ext_cfg, &self.budget) {
            chain.push((SolverStrategy::IncrementalSmt, format!("{:?}", sat), t));
            if sat == SatResult::Unsat {
                return StrategyResult {
                    result: VerificationResult { edge_id, sat, model: vec![],
                        unsat_core: vec![], severity_score: 0.2, solver_time_ms: t,
                        proof_commitment: vec![] },
                    strategy_used: SolverStrategy::IncrementalSmt, solver_time_ms: t,
                    total_time_ms: start.elapsed().as_secs_f64()*1000.0,
                    strategy_chain: chain, is_conclusive: true };
            }
        }

        let constraints: Vec<String> = fs.lines()
            .filter(|l| l.contains(">") || l.contains("<"))
            .map(|l| l.replace("(assert ", "").replace(")", "").trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if let Some(false) = AbstractInterpreter::check_sat(&constraints) {
            let t = start.elapsed().as_secs_f64()*1000.0;
            chain.push((SolverStrategy::AbstractInterpretation, "UNSAT".into(), t));
            return StrategyResult {
                result: VerificationResult { edge_id, sat: SatResult::Unsat,
                    model: vec![], unsat_core: vec![], severity_score: 0.2,
                    solver_time_ms: t, proof_commitment: vec![] },
                strategy_used: SolverStrategy::AbstractInterpretation, solver_time_ms: t,
                total_time_ms: t, strategy_chain: chain, is_conclusive: true };
        }

        let t = start.elapsed().as_secs_f64()*1000.0;
        chain.push((SolverStrategy::Triage, "exhausted".into(), t));
        StrategyResult {
            result: VerificationResult { edge_id, sat: SatResult::Unknown,
                model: vec![], unsat_core: vec![], severity_score: 0.5,
                solver_time_ms: t, proof_commitment: vec![] },
            strategy_used: SolverStrategy::Triage, solver_time_ms: t,
            total_time_ms: t, strategy_chain: chain, is_conclusive: false }
    }
}
impl Default for StrategyExecutor { fn default() -> Self { Self { budget: SolverBudget::default() } } }

// ─── Stuck-State Recovery ────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum StuckState {
    InProgress, Stuck(StrategyResult), PendingHumanReview { request_id: Uuid },
    HumanResolved { result: VerificationResult, reviewer: String }, Resolved(VerificationResult),
}

pub struct StuckStateManager { requests: HashMap<Uuid, StuckState> }
impl StuckStateManager {
    pub fn new() -> Self { Self { requests: HashMap::new() } }
    pub fn register(&mut self, id: Uuid) { self.requests.insert(id, StuckState::InProgress); }
    pub fn update(&mut self, id: Uuid, r: StrategyResult) -> StuckState {
        let s = if r.is_conclusive { StuckState::Resolved(r.result) } else { StuckState::Stuck(r) };
        self.requests.insert(id, s.clone()); s
    }
    pub fn request_human_review(&mut self, id: Uuid) -> Result<(), CoreError> {
        match self.requests.get(&id) {
            Some(StuckState::Stuck(_)) => {
                self.requests.insert(id, StuckState::PendingHumanReview { request_id: Uuid::new_v4() });
                Ok(())
            }
            _ => Err(CoreError::Internal("not stuck".into())),
        }
    }
    pub fn human_resolve(&mut self, id: Uuid, result: VerificationResult, reviewer: &str) -> Result<(), CoreError> {
        match self.requests.get(&id) {
            Some(StuckState::Stuck(_)) | Some(StuckState::PendingHumanReview{..}) => {
                self.requests.insert(id, StuckState::HumanResolved { result, reviewer: reviewer.into() });
                Ok(())
            }
            _ => Err(CoreError::Internal("bad state".into())),
        }
    }
    pub fn get(&self, id: &Uuid) -> Option<&StuckState> { self.requests.get(id) }
}
impl Default for StuckStateManager { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use crate::models::*;
    use crate::solver::SolverConfig;
    use uuid::Uuid;

    #[test] fn test_interval_unsat() {
        assert_eq!(super::AbstractInterpreter::check_sat(&vec!["x > 5".into(), "x < 3".into()]), Some(false));
    }
    #[test] fn test_interval_sat() {
        assert_eq!(super::AbstractInterpreter::check_sat(&vec!["x > 5".into(), "x < 10".into()]), Some(true));
    }
    #[test] fn test_decompose() {
        assert!(!super::IncrementalSolver::decompose("(assert (> x 5))\n(assert (< x 10))", 5).is_empty());
    }
    #[test] fn test_chain() {
        let r = super::StrategyExecutor::default().execute(Uuid::new_v4(), b"(check-sat)", &SolverConfig::default());
        assert!(!r.strategy_chain.is_empty());
    }
    #[test] fn test_stuck() {
        let mut m = super::StuckStateManager::new();
        let id = Uuid::new_v4();
        // Register and manually set to Stuck state.
        m.register(id);
        let sr = super::StrategyResult {
            result: VerificationResult { edge_id: id, sat: SatResult::Unknown, model: vec![],
                unsat_core: vec![], severity_score: 0.5, solver_time_ms: 0.0, proof_commitment: vec![] },
            strategy_used: super::SolverStrategy::Triage, solver_time_ms: 0.0, total_time_ms: 0.0,
            strategy_chain: vec![], is_conclusive: false,
        };
        m.update(id, sr);
        assert!(m.request_human_review(id).is_ok());
        let vr = VerificationResult { edge_id: id, sat: SatResult::Sat, model: vec![],
            unsat_core: vec![], severity_score: 0.5, solver_time_ms: 0.0, proof_commitment: vec![] };
        assert!(m.human_resolve(id, vr.clone(), "ex").is_ok());
        // Verify manager won't double-resolve
        assert!(m.human_resolve(id, vr, "ex2").is_err());
    }
}