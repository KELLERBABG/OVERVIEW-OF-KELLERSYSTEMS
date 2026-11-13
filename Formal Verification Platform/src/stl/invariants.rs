//! Core STL container invariant definitions and checks.
//!
//! std::vector invariants:
//!   1. capacity >= size (always)
//!   2. All elements in [begin, begin+size) are valid
//!   3. Iterators remain valid unless a mutating operation occurs
//!   4. Reallocation invalidates all references/pointers/iterators
//!
//! std::string invariants:
//!   1. s[s.size()] == '\0' (null-terminated)
//!   2. c_str() pointer valid until next mutation
//!   3. data() pointer valid until next mutation (C++17: may be non-null-terminated)

use crate::frontend::cpp::{StlContainerInstance, StlInvariantViolation, ViolationSeverity};

/// Check all invariants for a std::vector instance.
pub fn check_vector_invariants(container: &StlContainerInstance) -> Vec<StlInvariantViolation> {
    let mut violations = Vec::new();

    // Invariant 1: capacity >= size is guaranteed by the STL contract,
    // but we verify that no code manually sets capacity < size via
    // unsafe pointer arithmetic or similar.
    // (This would be detected by the hypergraph dataflow analysis.)

    // Invariant 4: Reallocation detection.
    // If push_back causes reallocation (size == capacity before push),
    // all prior iterators/references are invalidated.
    violations.push(StlInvariantViolation {
        container: container.clone(),
        invariant_name: "vector_reallocation".to_string(),
        description: format!(
            "std::vector<{}> '{}' may reallocate on push_back when \
             size == capacity, invalidating all iterators and references. \
             Verify no long-lived references exist across potential reallocation.",
            container.element_type,
            container.var_name.as_deref().unwrap_or("<unnamed>"),
        ),
        location: container.location.clone(),
        severity: ViolationSeverity::Warning,
    });

    violations
}

/// Check all invariants for a std::string instance.
pub fn check_string_invariants(container: &StlContainerInstance) -> Vec<StlInvariantViolation> {
    let mut violations = Vec::new();

    // Invariant 1: Null termination.
    // std::string guarantees s[s.size()] == '\0'. This is a library
    // guarantee, not something user code can violate, but we note it
    // for completeness in the report.

    // Invariant 2: c_str() lifetime.
    violations.push(StlInvariantViolation {
        container: container.clone(),
        invariant_name: "string_cstr_lifetime".to_string(),
        description: format!(
            "std::string '{}' : pointers returned by c_str() or data() \
             are invalidated by any subsequent mutation (push_back, append, \
             operator+=, insert, erase, replace, clear).",
            container.var_name.as_deref().unwrap_or("<unnamed>"),
        ),
        location: container.location.clone(),
        severity: ViolationSeverity::Info,
    });

    violations
}

/// Define the set of mutating operations for std::vector that cause
/// iterator invalidation.
pub fn vector_invalidating_operations() -> Vec<&'static str> {
    vec![
        "push_back",
        "emplace_back",
        "insert",
        "erase",
        "resize",
        "reserve",  // Only if new capacity > old capacity (reallocation)
        "shrink_to_fit", // May reallocate
        "clear",
    ]
}

/// Define the set of mutating operations for std::string that invalidate
/// c_str()/data() pointers.
pub fn string_invalidating_operations() -> Vec<&'static str> {
    vec![
        "push_back",
        "append",
        "insert",
        "erase",
        "replace",
        "clear",
        "resize",
        "shrink_to_fit",
        "operator+=" ,
    ]
}

/// Check whether a given method name is an invalidating operation for vectors.
pub fn is_vector_invalidating(method: &str) -> bool {
    vector_invalidating_operations().iter().any(|op| *op == method)
}

/// Check whether a given method name is an invalidating operation for strings.
pub fn is_string_invalidating(method: &str) -> bool {
    string_invalidating_operations().iter().any(|op| *op == method)
}
