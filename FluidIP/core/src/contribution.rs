//! # Contribution Weight Governance
//!
//! Addresses: "How are initial edge contribution weights (w) negotiated or bounded?"
//!
//! ## Approach
//! Rather than letting authors arbitrarily set weights (which enables royalty manipulation),
//! the system computes a **recommended maximum weight** based on the IR AST delta size
//! between parent and child inventions. The actual weight must be ≤ this maximum.
//!
//! The rationale: a derivative that changes only 10% of the original's logic should
//! not claim 90% of the value. The weight represents the *fraction of new value added*,
//! which is bounded by the proportion of novel constraints.

use crate::ir::*;
use std::collections::HashSet;
use uuid::Uuid;

/// The result of contribution weight governance.
#[derive(Clone, Debug)]
pub struct ContributionGovernance {
    /// The parent (original) node.
    pub parent_id: Uuid,
    /// The child (derivative) node.
    pub child_id: Uuid,
    /// The weight the author requested.
    pub requested_weight: f64,
    /// The maximum weight permitted by governance rules.
    pub max_allowed_weight: f64,
    /// The final weight (min of requested and max allowed).
    pub final_weight: f64,
    /// Justification for the bound.
    pub justification: String,
}

/// Analyzes the delta between two IRs to compute a fair maximum contribution weight.
pub struct ContributionGovernor;

impl ContributionGovernor {
    /// Compute the maximum allowable contribution weight for a derivative.
    ///
    /// The max weight is based on:
    /// 1. **AST Delta Ratio**: fraction of the child IR that is genuinely novel
    ///    (not present in the parent). Measured as structural difference in IR.
    /// 2. **Invariant Novelty**: fraction of child invariants that are new.
    /// 3. **Information Gain**: normalized Kolmogorov-style complexity ratio.
    ///
    /// The result is clamped to [0.0, 1.0].
    pub fn govern(parent: &InventionIr, child: &InventionIr, requested: f64) -> ContributionGovernance {
        let ast_ratio = Self::compute_ast_delta_ratio(parent, child);
        let inv_ratio = Self::compute_invariant_novelty(parent, child);
        let complexity_ratio = Self::compute_complexity_ratio(parent, child);

        // Combine ratios: take the maximum since any single one could indicate novelty.
        let raw_bound = ast_ratio.max(inv_ratio).max(complexity_ratio);
        // Apply a cap: even with 100% novelty, weight is bounded by 0.5 unless
        // the derivative is a truly independent invention (which would be a root node).
        let max_allowed = raw_bound.min(0.5);

        let final_weight = requested.min(max_allowed);

        let justification = format!(
            "AST delta ratio: {:.3}, invariant novelty: {:.3}, complexity ratio: {:.3} → max allowed: {:.3}",
            ast_ratio, inv_ratio, complexity_ratio, max_allowed
        );

        ContributionGovernance {
            parent_id: Uuid::new_v4(),
            child_id: Uuid::new_v4(),
            requested_weight: requested,
            max_allowed_weight: max_allowed,
            final_weight,
            justification,
        }
    }

    /// Compute the fraction of the child's IR structure that is new.
    fn compute_ast_delta_ratio(parent: &InventionIr, child: &InventionIr) -> f64 {
        match (&parent.kind, &child.kind) {
            (IrKind::Software(p), IrKind::Software(c)) => {
                let parent_blocks: HashSet<&str> = p.blocks.keys().map(|s| s.as_str()).collect();
                let child_blocks: HashSet<&str> = c.blocks.keys().map(|s| s.as_str()).collect();
                let new_blocks = child_blocks.difference(&parent_blocks).count();
                let total = child_blocks.len().max(1);
                new_blocks as f64 / total as f64
            }
            (IrKind::Chemical(p), IrKind::Chemical(c)) => {
                let new_atoms = c.atoms.len().saturating_sub(p.atoms.len());
                let total = c.atoms.len().max(1);
                new_atoms as f64 / total as f64
            }
            (IrKind::Mechanical(p), IrKind::Mechanical(c)) => {
                let new_ents = c.entities.len().saturating_sub(p.entities.len());
                let total = c.entities.len().max(1);
                new_ents as f64 / total as f64
            }
            _ => 0.5, // Different IR kinds → significant delta
        }
    }

    /// Compute the fraction of child invariants that are novel.
    fn compute_invariant_novelty(parent: &InventionIr, child: &InventionIr) -> f64 {
        let p_vec = crate::ir::extract_invariants(parent);
        let c_vec = crate::ir::extract_invariants(child);
        let p_invs: HashSet<&str> = p_vec.iter().map(|s| s.as_str()).collect();
        let c_invs: HashSet<&str> = c_vec.iter().map(|s| s.as_str()).collect();
        let new_invs = c_invs.difference(&p_invs).count();
        let total = c_invs.len().max(1);
        new_invs as f64 / total as f64
    }

    /// Compute a simple complexity ratio based on the total size of constraints.
    fn compute_complexity_ratio(parent: &InventionIr, child: &InventionIr) -> f64 {
        let parent_size = Self::ir_complexity(parent);
        let child_size = Self::ir_complexity(child);
        if child_size <= parent_size || parent_size == 0 {
            return 0.0; // no complexity gain
        }
        (child_size - parent_size) as f64 / child_size as f64
    }

    /// Compute a rough size metric for an IR.
    fn ir_complexity(ir: &InventionIr) -> usize {
        let mut size = 0;
        size += ir.preconditions.iter().map(|s| s.len()).sum::<usize>();
        size += ir.postconditions.iter().map(|s| s.len()).sum::<usize>();
        size += ir.invariants.iter().map(|s| s.len()).sum::<usize>();
        match &ir.kind {
            IrKind::Software(t) => { size += t.blocks.len() * 10; }
            IrKind::Chemical(m) => { size += m.atoms.len() * 5 + m.bonds.len() * 3; }
            IrKind::Mechanical(m) => { size += m.entities.len() * 7; }
        }
        size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ir(blocks: usize, pre: &[&str], post: &[&str]) -> InventionIr {
        let mut b = std::collections::HashMap::new();
        for i in 0..blocks {
            b.insert(format!("b{i}"), BasicBlock {
                label: format!("b{i}"), instructions: vec![],
                terminator: Terminator::Return(None),
            });
        }
        InventionIr {
            kind: IrKind::Software(SsaTrace {
                entry_block: "b0".into(), blocks: b, inputs: vec![], outputs: vec![],
            }),
            preconditions: pre.iter().map(|s| s.to_string()).collect(),
            postconditions: post.iter().map(|s| s.to_string()).collect(),
            invariants: vec![],
        }
    }

    #[test]
    fn test_governor_reduces_excessive_weight() {
        let parent = make_ir(2, &["x > 0"], &["y = x"]);
        let child = make_ir(2, &["x > 0"], &["y = x"]); // identical
        let result = ContributionGovernor::govern(&parent, &child, 0.9);
        assert!(result.final_weight < 0.9, "identical IRs should not allow 0.9 weight");
    }

    #[test]
    fn test_governor_allows_reasonable_weight() {
        let parent = make_ir(1, &["x > 0"], &["y = x"]);
        let child = make_ir(5, &["x > 0"], &["y = x", "z = y + 1", "w = z * 2"]); // substantially new
        let result = ContributionGovernor::govern(&parent, &child, 0.3);
        assert!(result.final_weight > 0.0);
        assert!(result.final_weight <= 0.5);
    }

    #[test]
    fn test_invariant_novelty() {
        let parent = make_ir(1, &["x > 0"], &["y = x"]);
        let child = make_ir(1, &["x > 0"], &["y = x", "z = y + 1"]);
        let novelty = ContributionGovernor::compute_invariant_novelty(&parent, &child);
        assert!((novelty - 0.333).abs() < 0.01); // 1 new postcondition out of 3 total invariants
    }

    #[test]
    fn test_ast_delta_ratio() {
        let parent = make_ir(2, &[], &[]);
        let child = make_ir(5, &[], &[]); // 3 new blocks out of 5
        let ratio = ContributionGovernor::compute_ast_delta_ratio(&parent, &child);
        assert!((ratio - 0.6).abs() < 0.01); // 3/5 = 0.6
    }
}