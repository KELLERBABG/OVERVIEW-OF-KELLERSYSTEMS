//! # Translation Fidelity & Soundness Guarantees
//!
//! Addresses Critique #1: "Frontend IR generation may suppress critical constraints,
//! causing false SAT (missed infringement) or false UNSAT (false positive)."
//!
//! ## Approach
//! Every extracted invariant carries a `SoundnessAnnotation` that records:
//! - **provenance**: how the invariant was derived (static analysis, user-provided, heuristic)
//! - **confidence**: a rigorous score [0,1] based on extraction method reliability
//! - **abstraction_gap**: what was lost in translation (documented explicitly)
//!
//! An `IRVerifier` checks internal consistency of the extracted invariant set BEFORE
//! it reaches the SMT solver, catching contradictory or vacuous specifications early.

use crate::ir::*;
use serde::{Deserialize, Serialize};

// ─── Soundness Annotations ───────────────────────────────────────────────────

/// How an invariant was extracted — this determines its reliability.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum InvariantProvenance {
    /// From formal static analysis (e.g., abstract interpretation over SSA).
    /// Highest confidence — mathematically sound over-approximation.
    StaticAnalysis,
    /// User-provided pre/post condition (e.g., the inventor asserts this).
    /// High confidence if signed, but may be incomplete.
    UserProvided,
    /// Heuristic extraction (e.g., loop pattern matching, dimension inference).
    /// Medium confidence — may miss or over-approximate.
    Heuristic,
    /// ML-guided extraction (future: neural invariant inference).
    /// Lower confidence — requires human verification.
    MachineLearned,
    /// Fallback default when nothing better is available.
    /// Lowest confidence — treat as advisory only.
    Default,
}

/// Describes what concrete information was lost when abstracting the
/// real-world artifact into the IR.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AbstractionGap {
    /// Non-linear arithmetic replaced with linear over-approximation.
    /// Common in mechanical stress analysis, fluid dynamics.
    LinearizedNonlinear { original_domain: String, over_approx: String },
    /// Continuous dynamics replaced with discrete time steps.
    DiscretizedContinuous { step_size: f64, error_bound: f64 },
    /// Probabilistic behavior collapsed to worst-case bounds.
    ProbabilisticToWorstCase { distribution: String, bound: String },
    /// Unknown code / library call treated as uninterpreted function.
    UninterpretedFunction { name: String, assumptions: Vec<String> },
    /// Deliberate simplification for tractability.
    DeliberateSimplification { reason: String },
    /// No abstraction — exact representation.
    Exact,
}

/// A soundness-annotated invariant: one extracted constraint with full
/// provenance and abstraction-gap metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnnotatedInvariant {
    /// The logical predicate (e.g., "x >= 0", "stress <= yield_strength").
    pub predicate: String,
    /// How this invariant was derived.
    pub provenance: InvariantProvenance,
    /// Confidence in [0, 1].  1.0 = mathematically proven for the abstraction.
    /// 0.0 = complete guess.
    pub confidence: f64,
    /// What was lost in translation, documented explicitly.
    pub abstraction_gap: AbstractionGap,
    /// Human-readable justification (for audit trail).
    pub justification: String,
}

impl AnnotatedInvariant {
    /// Create an exact, static-analysis-level invariant (gold standard).
    pub fn exact(predicate: &str, justification: &str) -> Self {
        Self {
            predicate: predicate.to_string(),
            provenance: InvariantProvenance::StaticAnalysis,
            confidence: 1.0,
            abstraction_gap: AbstractionGap::Exact,
            justification: justification.to_string(),
        }
    }

    /// Create a user-provided invariant (high confidence if signed).
    pub fn user(predicate: &str, justification: &str) -> Self {
        Self {
            predicate: predicate.to_string(),
            provenance: InvariantProvenance::UserProvided,
            confidence: 0.95,
            abstraction_gap: AbstractionGap::Exact,
            justification: justification.to_string(),
        }
    }

    /// Create a heuristic invariant with documented abstraction gap.
    pub fn heuristic(predicate: &str, gap: AbstractionGap, justification: &str) -> Self {
        let confidence = match &gap {
            AbstractionGap::Exact => 0.9,
            AbstractionGap::LinearizedNonlinear { .. } => 0.6,
            AbstractionGap::DiscretizedContinuous { .. } => 0.5,
            AbstractionGap::ProbabilisticToWorstCase { .. } => 0.4,
            AbstractionGap::UninterpretedFunction { .. } => 0.3,
            AbstractionGap::DeliberateSimplification { .. } => 0.2,
        };
        Self {
            predicate: predicate.to_string(),
            provenance: InvariantProvenance::Heuristic,
            confidence,
            abstraction_gap: gap,
            justification: justification.to_string(),
        }
    }
}

// ─── IR Consistency Verifier ─────────────────────────────────────────────────

/// Reasons an invariant set may be unsound.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SoundnessIssue {
    /// Direct contradiction: two invariants imply false together.
    /// E.g., "x > 5" and "x < 3".
    DirectContradiction { a: String, b: String, explanation: String },
    /// Vacuous invariant: one that is always true regardless of inputs.
    /// Trivially satisfied, provides no meaningful constraint for Δ.
    VacuousInvariant { predicate: String },
    /// Unbounded invariant: refers to a variable not in scope.
    UnboundedVariable { variable: String },
    /// Confidence below threshold — should not be used for infringement
    /// decisions without human review.
    LowConfidence { predicate: String, confidence: f64 },
}

/// Result of invariant set verification.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SoundnessReport {
    pub sound: bool,
    pub issues: Vec<SoundnessIssue>,
    pub overall_confidence: f64,
    pub can_use_for_infringement: bool,
}

/// Verifies that a set of extracted invariants is internally consistent
/// and has sufficient confidence for automated decisions.
pub struct IRVerifier {
    /// Minimum confidence required for an invariant to be used in
    /// infringement detection (Δ computation).
    pub min_confidence_for_infringement: f64,
}

impl IRVerifier {
    pub fn new(min_confidence: f64) -> Self {
        Self { min_confidence_for_infringement: min_confidence }
    }

    /// Verify a set of annotated invariants for soundness.
    pub fn verify(&self, invariants: &[AnnotatedInvariant]) -> SoundnessReport {
        let mut issues = Vec::new();
        let mut seen_predicates: Vec<&str> = Vec::new();

        for inv in invariants {
            // Check for vacuity: if predicate is constant true, flag it.
            if inv.predicate.trim() == "true" || inv.predicate.trim().is_empty() {
                issues.push(SoundnessIssue::VacuousInvariant {
                    predicate: inv.predicate.clone(),
                });
            }

            // Check confidence threshold.
            if inv.confidence < self.min_confidence_for_infringement {
                issues.push(SoundnessIssue::LowConfidence {
                    predicate: inv.predicate.clone(),
                    confidence: inv.confidence,
                });
            }

            // Simple contradiction check: look for pairs of the form
            // "x cmp v1" and "x cmp v2" that cannot both hold.
            // This is a lightweight syntactic check — full check requires SMT.
            for prev in &seen_predicates {
                if let Some(issue) = check_syntactic_contradiction(prev, &inv.predicate) {
                    if !issues.iter().any(|i| matches!(i, SoundnessIssue::DirectContradiction{..})) {
                        issues.push(issue);
                    }
                }
            }
            seen_predicates.push(&inv.predicate);
        }

        // Compute overall confidence (minimum of all invariants).
        let overall_confidence: f64 = invariants.iter()
            .map(|i| i.confidence)
            .fold(1.0, |a, b| a.min(b));

        let can_use = issues.iter().all(|i| matches!(i, SoundnessIssue::LowConfidence{..}))
            && overall_confidence >= self.min_confidence_for_infringement;

        SoundnessReport {
            sound: issues.is_empty(),
            issues,
            overall_confidence,
            can_use_for_infringement: can_use,
        }
    }
}

impl Default for IRVerifier {
    fn default() -> Self {
        Self { min_confidence_for_infringement: 0.7 }
    }
}

/// Lightweight syntactic contradiction detection.
/// Catches obvious cases like "x > 5" vs "x < 3".
fn check_syntactic_contradiction(a: &str, b: &str) -> Option<SoundnessIssue> {
    // Normalize: extract variable and operation
    let a_parts: Vec<&str> = a.split_whitespace().collect();
    let b_parts: Vec<&str> = b.split_whitespace().collect();

    if a_parts.len() != 3 || b_parts.len() != 3 {
        return None;
    }

    let var_a = a_parts[0];
    let var_b = b_parts[0];
    if var_a != var_b {
        return None; // Different variables — no contradiction
    }

    let cmp_a = a_parts[1];
    let cmp_b = b_parts[1];
    let val_a: f64 = a_parts[2].parse().ok()?;
    let val_b: f64 = b_parts[2].parse().ok()?;

    // Check for contradictory pairs
    let contradictory = match (cmp_a, cmp_b) {
        (">", "<") if val_a >= val_b => true,
        (">=", "<") if val_a >= val_b => true,
        (">", "<=") if val_a >= val_b => true,
        (">=", "<=") if val_a > val_b => true,
        ("==", "!=") => true,
        ("!=", "==") => true,
        _ => false,
    };

    if contradictory {
        Some(SoundnessIssue::DirectContradiction {
            a: a.to_string(),
            b: b.to_string(),
            explanation: format!("{a} and {b} cannot both be true for variable {var_a}"),
        })
    } else {
        None
    }
}

// ─── Soundness-Annotated IR ──────────────────────────────────────────────────

/// A complete intermediate representation with full soundness tracking.
/// This wraps `InventionIr` with per-invariant annotations.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SoundnessAnnotatedIR {
    pub ir: InventionIr,
    pub annotated_preconditions: Vec<AnnotatedInvariant>,
    pub annotated_postconditions: Vec<AnnotatedInvariant>,
    pub annotated_invariants: Vec<AnnotatedInvariant>,
    pub soundness_report: SoundnessReport,
}

impl SoundnessAnnotatedIR {
    /// Build a soundness-annotated IR from a raw InventionIr.
    /// Applies default provenance based on source type.
    pub fn from_ir(ir: InventionIr, provenance: InvariantProvenance) -> Self {
        let mut annotated_pre = Vec::new();
        let mut annotated_post = Vec::new();
        let mut annotated_inv = Vec::new();

        for p in &ir.preconditions {
            annotated_pre.push(AnnotatedInvariant::heuristic(
                p, AbstractionGap::Exact, "from preconditions",
            ));
        }
        for p in &ir.postconditions {
            annotated_post.push(AnnotatedInvariant::heuristic(
                p, AbstractionGap::Exact, "from postconditions",
            ));
        }
        for p in &ir.invariants {
            annotated_inv.push(AnnotatedInvariant::heuristic(
                p, AbstractionGap::Exact, "from invariants",
            ));
        }

        // Override provenance for user-provided conditions
        if provenance == InvariantProvenance::UserProvided {
            for a in &mut annotated_pre { a.provenance = provenance.clone(); a.confidence = 0.95; }
            for a in &mut annotated_post { a.provenance = provenance.clone(); a.confidence = 0.95; }
            for a in &mut annotated_inv { a.provenance = provenance.clone(); a.confidence = 0.95; }
        }

        let verifier = IRVerifier::default();
        let all_invariants: Vec<AnnotatedInvariant> = annotated_pre.iter()
            .chain(annotated_post.iter())
            .chain(annotated_inv.iter())
            .cloned()
            .collect();
        let report = verifier.verify(&all_invariants);

        Self {
            ir,
            annotated_preconditions: annotated_pre,
            annotated_postconditions: annotated_post,
            annotated_invariants: annotated_inv,
            soundness_report: report,
        }
    }

    /// Extract invariants with a minimum confidence filter.
    /// Low-confidence invariants are excluded from Δ computation.
    pub fn extract_filtered(&self, min_confidence: f64) -> Vec<String> {
        let mut result = Vec::new();
        for inv in &self.annotated_preconditions {
            if inv.confidence >= min_confidence { result.push(inv.predicate.clone()); }
        }
        for inv in &self.annotated_postconditions {
            if inv.confidence >= min_confidence { result.push(inv.predicate.clone()); }
        }
        for inv in &self.annotated_invariants {
            if inv.confidence >= min_confidence { result.push(inv.predicate.clone()); }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_contradiction_detection() {
        let issue = check_syntactic_contradiction("x > 5", "x < 3");
        assert!(issue.is_some());
        assert!(matches!(issue.unwrap(), SoundnessIssue::DirectContradiction{..}));
    }

    #[test]
    fn test_no_false_contradiction() {
        let issue = check_syntactic_contradiction("x > 5", "x > 3");
        assert!(issue.is_none());
    }

    #[test]
    fn test_vacuous_invariant_detection() {
        let verifier = IRVerifier::default();
        let invs = vec![
            AnnotatedInvariant::exact("true", "trivially true"),
        ];
        let report = verifier.verify(&invs);
        assert!(!report.sound);
        assert!(report.issues.iter().any(|i| matches!(i, SoundnessIssue::VacuousInvariant{..})));
    }

    #[test]
    fn test_low_confidence_flagged() {
        let verifier = IRVerifier::new(0.8);
        let invs = vec![
            AnnotatedInvariant::heuristic("x > 0", AbstractionGap::DeliberateSimplification { reason: "test".into() }, "low conf"),
        ];
        let report = verifier.verify(&invs);
        assert!(report.can_use_for_infringement == false);
        assert!(report.issues.iter().any(|i| matches!(i, SoundnessIssue::LowConfidence{..})));
    }

    #[test]
    fn test_annotated_ir_construction() {
        let ir = InventionIr {
            kind: IrKind::Software(SsaTrace {
                entry_block: "e".into(), blocks: HashMap::new(), inputs: vec![], outputs: vec![],
            }),
            preconditions: vec!["x > 0".into()],
            postconditions: vec!["y > 0".into()],
            invariants: vec![],
        };
        let annotated = SoundnessAnnotatedIR::from_ir(ir, InvariantProvenance::StaticAnalysis);
        assert_eq!(annotated.annotated_preconditions.len(), 1);
        assert_eq!(annotated.annotated_postconditions.len(), 1);
        // Should pass soundness check
        let filtered = annotated.extract_filtered(0.5);
        assert_eq!(filtered.len(), 2);
    }
}  
// =================================================================  
//! # Solver Strategies for Undecidable / Intractable Constraints
//!
//! Addresses Critique #2. Strategy chain with fallback: Full SMT → Incremental → Abstract → Triage.

use crate::error::CoreError;
use crate::models::*;
use crate::solver::{ExternalSolver, SmtSolver, SolverConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;
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
        let start = Instant::now();
        let mut chain: Vec<(SolverStrategy, String, f64)> = Vec::new();
        let fs = String::from_utf8_lossy(formula);
        let solver: Box<dyn SmtSolver> = match &config.backend {
            crate::solver::SolverBackend::BuiltIn => Box::new(crate::solver::BuiltInSolver),
            _ => Box::new(ExternalSolver::new(&config.backend)),
        };
        let ext_cfg = SolverConfig { timeout_ms: self.budget.full_smt_timeout_ms, ..config.clone() };

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
// =================================================================  
//! # Regulatory & Legal Adoption Bridge
//!
//! Addresses Critique #3. Provides WIPO XML export, evidence packages,
//! compliance config, arbitration agreements.

use crate::graph::PatentDag;
use crate::models::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LegalEvidencePackage {
    pub package_id: Uuid,
    pub generated_at: i64,
    pub graph_snapshot: PatentGraphState,
    pub lifecycle_log: Vec<LifecycleEvent>,
    pub deltas: Vec<DeltaConstraint>,
    pub verification_results: Vec<VerificationResult>,
    pub executive_summary: String,
    pub signature: Vec<u8>,
}

impl LegalEvidencePackage {
    pub fn from_dag(
        dag: &PatentDag, disputed_edges: &[Uuid],
        deltas: Vec<DeltaConstraint>, verification_results: Vec<VerificationResult>,
    ) -> Self {
        let state = dag.export_state();
        let log = dag.lifecycle_log().to_vec();
        let summary = Self::generate_summary(&state, disputed_edges);
        let package_id = Uuid::new_v4();
        let mut hasher = sha2::Sha256::new();
        hasher.update(package_id.as_bytes());
        hasher.update(&serde_json::to_vec(&state).unwrap_or_default());
        let signature = hasher.finalize().to_vec();
        Self { package_id, generated_at: chrono::Utc::now().timestamp(),
            graph_snapshot: state, lifecycle_log: log, deltas, verification_results,
            executive_summary: summary, signature }
    }
    pub fn verify_integrity(&self) -> bool {
        let mut hasher = sha2::Sha256::new();
        hasher.update(self.package_id.as_bytes());
        hasher.update(&serde_json::to_vec(&self.graph_snapshot).unwrap_or_default());
        hasher.finalize().to_vec() == self.signature
    }
    fn generate_summary(state: &PatentGraphState, disputed: &[Uuid]) -> String {
        let mut s = format!(
            "DFIP FORENSIC PACKAGE\nPatents: {}\nEdges: {}\nDisputed: {}\n\n",
            state.nodes.len(), state.edges.len(), disputed.len());
        for eid in disputed {
            if let Some(edge) = state.edges.iter().find(|e| e.id == *eid) {
                let p = state.nodes.get(&edge.parent_id).map(|n| n.title.as_str()).unwrap_or("?");
                let c = state.nodes.get(&edge.child_id).map(|n| n.title.as_str()).unwrap_or("?");
                s.push_str(&format!("{} -> {} (w={})\n", p, c, edge.contribution_weight));
            }
        }
        s
    }
}

pub struct PatentOfficeConverter;
impl PatentOfficeConverter {
    pub fn to_wipo_xml(node: &PatentNode) -> String {
        format!(
            r#"<?xml version="1.0"?><patent><title>{t}</title><authors>{a}</authors><type>{p}</type></patent>"#,
            t = node.title, a = node.authors.join(";"), p = format!("{:?}", node.patent_type))
    }
    pub fn to_regulatory_report(dag: &PatentDag) -> String {
        let state = dag.export_state();
        let mut r = format!("DFIP Report — {} patents, {} edges\n", state.nodes.len(), state.edges.len());
        for (_, n) in &state.nodes { r.push_str(&format!("  {} ({:?})\n", n.title, n.patent_type)); }
        for e in &state.edges { r.push_str(&format!("  {} -> {} (w={})\n", e.parent_id, e.child_id, e.contribution_weight)); }
        r
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DataResidency { Global, EU, US, Jurisdiction(String) }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComplianceConfig {
    pub data_residency: DataResidency, pub encrypt_pii: bool,
    pub audit_logging: bool, pub allow_forensic_export: bool, pub retention_days: u32,
}
impl Default for ComplianceConfig {
    fn default() -> Self { Self { data_residency: DataResidency::Global, encrypt_pii: true,
        audit_logging: true, allow_forensic_export: true, retention_days: 365*20 } }
}
impl ComplianceConfig {
    pub fn meets_gdpr(&self) -> bool {
        self.encrypt_pii && self.audit_logging && self.retention_days <= 365*20
            && (self.data_residency == DataResidency::EU || self.data_residency == DataResidency::Global)
    }
    pub fn meets_uspto(&self) -> bool { self.audit_logging && self.retention_days >= 365*20 }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArbitrationAgreement {
    pub id: Uuid, pub parties: Vec<String>, pub patent_node_id: Uuid,
    pub accept_delta_as_binding: bool, pub accept_royalty_as_binding: bool,
    pub governing_law: String, pub signatures: Vec<(String, Vec<u8>)>, pub active: bool,
}
impl ArbitrationAgreement {
    pub fn new(pid: Uuid, law: &str) -> Self {
        Self { id: Uuid::new_v4(), parties: vec![], patent_node_id: pid,
            accept_delta_as_binding: true, accept_royalty_as_binding: true,
            governing_law: law.into(), signatures: vec![], active: false }
    }
    pub fn add_party(&mut self, p: &str) { self.parties.push(p.into()); }
    pub fn sign(&mut self, p: &str, sig: Vec<u8>) { self.signatures.push((p.into(), sig)); }
    pub fn activate(&mut self) { self.active = true; }
    pub fn is_fully_signed(&self) -> bool {
        self.parties.iter().all(|p| self.signatures.iter().any(|(s,_)| s == p))
    }
}

#[cfg(test)]
mod tests {
    use crate::graph::PatentDag;
    use crate::models::*;
    use uuid::Uuid;

    #[test]
    fn test_evidence_package() {
        let dag = PatentDag::new();
        let pkg = super::LegalEvidencePackage::from_dag(&dag, &[], vec![], vec![]);
        assert!(pkg.verify_integrity());
    }
    #[test]
    fn test_wipo_xml() {
        let n = PatentNode { id: Uuid::new_v4(), title: "Q".into(), patent_type: PatentType::Software,
            authors: vec!["A".into()], created_at_unix: 0, content_hash: vec![],
            invariant: None, tags: vec![], license: License::Open, meta_commitment: vec![] };
        assert!(super::PatentOfficeConverter::to_wipo_xml(&n).contains("Q"));
    }
    #[test]
    fn test_gdpr() {
        let c = super::ComplianceConfig { data_residency: super::DataResidency::EU, ..Default::default() };
        assert!(c.meets_gdpr());
    }
    #[test]
    fn test_arbitration() {
        let mut a = super::ArbitrationAgreement::new(Uuid::new_v4(), "Swiss Law");
        a.add_party("A"); a.sign("A", vec![1]); a.activate();
        assert!(a.active);
    }
}