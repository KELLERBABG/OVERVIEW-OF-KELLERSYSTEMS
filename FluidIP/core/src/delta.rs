//! # Path Constraint Modification Audit
//!
//! Builds the delta formula `Δ = ¬Original ∧ Derivative` and produces an
//! auditable trail of constraint changes between patent nodes in the DAG.
//!
//! The delta formula is expressed in SMT-LIB 2.0 format and can be fed
//! directly into the SMT solver (Phase 4) for SAT/UNSAT checking.

use crate::error::CoreError;
use crate::ir::{extract_invariants, InventionIr};
use crate::models::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

// ─── Delta Builder ───────────────────────────────────────────────────────────

/// Builds a `DeltaConstraint` between an original and a derivative IR.
///
/// The delta formula is: `Δ = ¬(Original_Invariants) ∧ (Derivative_Constraints)`.
/// If the solver finds this SAT, the derivative introduces something new
/// beyond the original. If UNSAT, the derivative is fully contained in
/// the original (possible infringement).
pub fn build_delta(
    original_id: Uuid,
    derivative_id: Uuid,
    original_ir: &InventionIr,
    derivative_ir: &InventionIr,
    assumptions: &[String],
) -> Result<DeltaConstraint, CoreError> {
    let original_invariants = extract_invariants(original_ir);
    let derivative_invariants = extract_invariants(derivative_ir);

    // Compute hash of original invariants for audit trail.
    let mut hasher = Sha256::new();
    for inv in &original_invariants {
        hasher.update(inv.as_bytes());
    }
    let original_invariant_hash = hasher.finalize().to_vec();

    // Build SMT-LIB 2.0 formula: Δ = ¬(∧ Original) ∧ (∧ Derivative)
    let formula = build_smt_lib2_formula(&original_invariants, &derivative_invariants);

    Ok(DeltaConstraint {
        original_node_id: original_id,
        derivative_node_id: derivative_id,
        original_invariant_hash,
        formula_smt_lib2: formula.into_bytes(),
        assumptions: assumptions.to_vec(),
    })
}

/// Construct the SMT-LIB 2.0 text for Δ = ¬(∧ Original) ∧ (∧ Derivative).
fn build_smt_lib2_formula(original: &[String], derivative: &[String]) -> String {
    let negated_original = if original.is_empty() {
        "true".to_string()
    } else {
        let conj = original
            .iter()
            .map(|s| format!("(assert {s})"))
            .collect::<Vec<_>>()
            .join("\n  ");
        format!("(not (and\n  {conj}\n))")
    };

    let derivative_conj = if derivative.is_empty() {
        "true".to_string()
    } else {
        derivative
            .iter()
            .map(|s| format!("(assert {s})"))
            .collect::<Vec<_>>()
            .join("\n  ")
    };

    format!(
        "(set-logic ALL)\n\
         ; Δ = ¬Original ∧ Derivative\n\
         (declare-fun delta-query () Bool)\n\
         (assert\n\
           (and\n\
             {negated_original}\n\
             {derivative_conj}\n\
           )\n\
         )\n\
         (check-sat)\n\
         (get-model)\n"
    )
}

// ─── Audit Trail ─────────────────────────────────────────────────────────────

/// An entry in the constraint modification audit trail.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AuditEntry {
    /// Unique audit entry ID.
    pub id: Uuid,
    /// The edge / delta this audit entry corresponds to.
    pub delta_constraint: DeltaConstraint,
    /// Timestamp of the audit.
    pub timestamp_unix: i64,
    /// Hash of the previous audit entry (Merkle chain).
    pub previous_hash: Vec<u8>,
    /// Merkle root of this entry.
    pub merkle_root: Vec<u8>,
}

/// An audit trail — a Merkle chain of delta constraint entries.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditTrail {
    entries: Vec<AuditEntry>,
    last_hash: Vec<u8>,
}

impl AuditTrail {
    /// Create a new empty audit trail.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            last_hash: Sha256::digest(b"audit-genesis").to_vec(),
        }
    }

    /// Append a new delta constraint to the audit trail.
    pub fn push(&mut self, delta: DeltaConstraint) -> Uuid {
        let id = Uuid::new_v4();
        let mut hasher = Sha256::new();
        hasher.update(&self.last_hash);
        hasher.update(delta.formula_smt_lib2.as_slice());
        hasher.update(id.as_bytes());
        let merkle_root = hasher.finalize().to_vec();

        let entry = AuditEntry {
            id,
            delta_constraint: delta,
            timestamp_unix: chrono::Utc::now().timestamp(),
            previous_hash: self.last_hash.clone(),
            merkle_root: merkle_root.clone(),
        };

        self.last_hash = merkle_root;
        self.entries.push(entry);
        id
    }

    /// Iterate over all entries in the audit trail.
    pub fn iter(&self) -> impl Iterator<Item = &AuditEntry> {
        self.entries.iter()
    }

    /// Get the number of entries in the trail.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the audit trail is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get the Merkle root (chain head) of the audit trail.
    pub fn merkle_root(&self) -> &[u8] {
        &self.last_hash
    }

    /// Verify the integrity of the entire audit trail from genesis to head.
    pub fn verify(&self) -> bool {
        let mut current_hash = Sha256::digest(b"audit-genesis").to_vec();
        for entry in &self.entries {
            let mut hasher = Sha256::new();
            hasher.update(&current_hash);
            hasher.update(entry.delta_constraint.formula_smt_lib2.as_slice());
            hasher.update(entry.id.as_bytes());
            let expected = hasher.finalize().to_vec();
            if expected != entry.merkle_root {
                return false;
            }
            current_hash = entry.merkle_root.clone();
        }
        current_hash == self.last_hash
    }
}

impl Default for AuditTrail {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::*;
    use std::collections::HashMap;

    fn make_software_ir(pre: &[&str], post: &[&str]) -> InventionIr {
        InventionIr {
            kind: IrKind::Software(SsaTrace {
                entry_block: "entry".into(),
                blocks: HashMap::new(),
                inputs: vec![],
                outputs: vec![],
            }),
            preconditions: pre.iter().map(|s| s.to_string()).collect(),
            postconditions: post.iter().map(|s| s.to_string()).collect(),
            invariants: vec![],
        }
    }

    #[test]
    fn test_build_delta_smt_lib2() {
        let orig = make_software_ir(&["(= x 0)"], &["(= y 1)"]);
        let deriv = make_software_ir(&["(= x 0)"], &["(= y 2)"]);

        let delta = build_delta(
            Uuid::new_v4(),
            Uuid::new_v4(),
            &orig,
            &deriv,
            &[],
        ).unwrap();

        let formula = String::from_utf8(delta.formula_smt_lib2).unwrap();
        // Formula should contain the delta structure
        assert!(formula.contains("¬Original ∧ Derivative") || formula.contains("delta-query"));
        assert!(formula.contains("check-sat"));
        assert!(!delta.original_invariant_hash.is_empty());
    }

    #[test]
    fn test_audit_trail_integrity() {
        let mut trail = AuditTrail::new();
        assert!(trail.is_empty());

        let orig = make_software_ir(&["(= x 0)"], &["(= y 1)"]);
        let deriv = make_software_ir(&["(= x 0)"], &["(= y 2)"]);

        let delta = build_delta(
            Uuid::new_v4(),
            Uuid::new_v4(),
            &orig,
            &deriv,
            &[],
        ).unwrap();

        let id = trail.push(delta);
        assert_eq!(trail.len(), 1);
        assert!(!trail.is_empty());
        assert!(!trail.merkle_root().is_empty());

        // Verify integrity
        assert!(trail.verify());

        // Second entry
        let delta2 = build_delta(
            Uuid::new_v4(),
            Uuid::new_v4(),
            &orig,
            &deriv,
            &[],
        ).unwrap();
        trail.push(delta2);
        assert_eq!(trail.len(), 2);
        assert!(trail.verify());
    }

    #[test]
    fn test_audit_trail_tamper_detection() {
        let mut trail = AuditTrail::new();

        let orig = make_software_ir(&["(= x 0)"], &["(= y 1)"]);
        let deriv = make_software_ir(&["(= x 0)"], &["(= y 2)"]);

        let delta = build_delta(
            Uuid::new_v4(),
            Uuid::new_v4(),
            &orig,
            &deriv,
            &[],
        ).unwrap();

        trail.push(delta);

        // Tamper with the entry's hash
        trail.entries[0].merkle_root[0] ^= 0x01;

        assert!(!trail.verify());
    }

    #[test]
    fn test_delta_empty_invariants() {
        let empty = make_software_ir(&[], &[]);
        let has_something = make_software_ir(&["(= x 0)"], &[]);

        let delta = build_delta(
            Uuid::new_v4(),
            Uuid::new_v4(),
            &empty,
            &has_something,
            &[],
        ).unwrap();

        let formula = String::from_utf8(delta.formula_smt_lib2).unwrap();
        // When original is empty, Δ = true ∧ derivative = derivative
        assert!(formula.contains("true"));
    }
}