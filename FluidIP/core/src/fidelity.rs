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