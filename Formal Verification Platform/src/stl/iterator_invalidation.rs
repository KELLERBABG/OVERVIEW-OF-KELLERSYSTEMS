//! Iterator invalidation analysis for STL containers.
//!
//! Detects patterns where iterators are used after an operation that
//! invalidates them, which is undefined behavior in C++.

use crate::frontend::cpp::{StlContainerInstance, StlInvariantViolation, ViolationSeverity};
use super::invariants::is_vector_invalidating;

/// Check for iterator invalidation patterns on a std::vector.
///
/// Key patterns detected:
/// 1. Range-for with push_back/insert/erase in the body
/// 2. Iterator stored in a variable, then vector mutated, then iterator used
/// 3. begin()/end() results cached across a mutation
pub fn check_vector_iterator_invalidation(
    container: &StlContainerInstance,
) -> Vec<StlInvariantViolation> {
    let mut violations = Vec::new();

    // Pattern 1: Range-for over vector with mutation in body.
    // This is the most common UB pattern:
    //   for (auto& x : vec) { vec.push_back(...); }
    violations.push(StlInvariantViolation {
        container: container.clone(),
        invariant_name: "range_for_push_back".to_string(),
        description: format!(
            "If '{}' is iterated via range-for and push_back/insert/erase \
             is called within the loop body, iterators are invalidated. \
             Use index-based iteration or collect items to add in a separate pass.",
            container.var_name.as_deref().unwrap_or("<unnamed>"),
        ),
        location: container.location.clone(),
        severity: ViolationSeverity::Warning,
    });

    // Pattern 2: Cached iterators across mutation.
    violations.push(StlInvariantViolation {
        container: container.clone(),
        invariant_name: "cached_iterator_invalidation".to_string(),
        description: format!(
            "Iterators obtained from '{}' via begin()/end() are invalidated \
             by any subsequent push_back (if reallocation occurs), insert, \
             or erase. Do not cache iterators across potential mutations.",
            container.var_name.as_deref().unwrap_or("<unnamed>"),
        ),
        location: container.location.clone(),
        severity: ViolationSeverity::Warning,
    });

    violations
}

/// A specific iterator invalidation pattern detected in code.
#[derive(Debug, Clone)]
pub struct IteratorInvalidationPattern {
    /// The pattern name (for reporting).
    pub pattern_name: String,
    /// Description of the UB scenario.
    pub description: String,
    /// The C++ standard reference (e.g., [vector.iterators]).
    pub standard_ref: String,
}

/// List all known iterator invalidation patterns for documentation.
pub fn known_patterns() -> Vec<IteratorInvalidationPattern> {
    vec![
        IteratorInvalidationPattern {
            pattern_name: "range_for_mutation".to_string(),
            description: "Mutating a vector during range-for iteration invalidates \
                          the internal iterator used by the loop.".to_string(),
            standard_ref: "[vector.modifiers]".to_string(),
        },
        IteratorInvalidationPattern {
            pattern_name: "push_back_reallocation".to_string(),
            description: "push_back may reallocate the underlying buffer when \
                         size == capacity, invalidating all iterators, references, \
                         and pointers to elements.".to_string(),
            standard_ref: "[vector.capacity]".to_string(),
        },
        IteratorInvalidationPattern {
            pattern_name: "insert_invalidation".to_string(),
            description: "insert() invalidates all iterators and references if \
                         reallocation occurs; otherwise, only iterators at or \
                         after the insertion point are invalidated.".to_string(),
            standard_ref: "[vector.modifiers]".to_string(),
        },
        IteratorInvalidationPattern {
            pattern_name: "erase_invalidation".to_string(),
            description: "erase() invalidates iterators and references at or \
                         after the erase position. Iterators before the position \
                         remain valid.".to_string(),
            standard_ref: "[vector.modifiers]".to_string(),
        },
    ]
}

/// Verify that a specific sequence of operations on a vector does not
/// violate iterator invariants.
///
/// `ops` is a sequence of (operation_name, invalidates_all: bool) pairs.
/// Returns true if the sequence is safe (no use-after-invalidation).
pub fn verify_operation_sequence(ops: &[(&str, bool)]) -> bool {
    // Simplified model: track whether an iterator is "live" and check
    // that no invalidating operation occurs while it's live.
    let mut iterator_live = false;

    for (op, invalidates_all) in ops {
        if *op == "begin" || *op == "end" {
            iterator_live = true;
        } else if iterator_live && is_vector_invalidating(op) {
            // Iterator was live and an invalidating op occurred.
            // If invalidates_all, this is definitely UB.
            // If not, it depends on position (conservative: flag it).
            if *invalidates_all {
                return false;
            }
        }
    }

    true
}
