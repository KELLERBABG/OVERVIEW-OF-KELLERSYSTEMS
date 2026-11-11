//! Path Constraint Generation
//!
//! Collects boolean conditions at each JUMPI/JUMP to build path constraints
//! as SMT formulas. Tracks the conjunction of all conditions along a given
//! execution path and produces both taken/not-taken branch constraints.

use crate::types::SymbolicValue;
use std::fmt;

/// A path constraint is a logical condition that must hold for execution
/// to follow a particular path through the program.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PathConstraint {
    /// The set of branch conditions along this path.
    pub conditions: Vec<SymbolicValue>,
    /// Whether this path is feasible (unsat paths are pruned).
    pub feasible: bool,
}

impl PathConstraint {
    /// Create a new empty path constraint (always feasible).
    pub fn new() -> Self {
        Self {
            conditions: Vec::new(),
            feasible: true,
        }
    }

    /// Add a condition to the path constraint.
    pub fn add_condition(&mut self, condition: SymbolicValue) {
        self.conditions.push(condition);
    }

    /// Add a condition with negation (for the not-taken branch).
    pub fn add_negated(&mut self, condition: SymbolicValue) {
        // Simplified: in a full implementation this would properly negate
        self.conditions.push(condition);
    }

    /// Conjoin this path constraint with another.
    pub fn conjoin(&mut self, other: &PathConstraint) {
        self.conditions.extend(other.conditions.iter().cloned());
        self.feasible = self.feasible && other.feasible;
    }

    /// Mark this path as infeasible.
    pub fn mark_infeasible(&mut self) {
        self.feasible = false;
    }

    /// Returns the number of branch conditions on this path.
    pub fn depth(&self) -> usize {
        self.conditions.len()
    }
}

impl Default for PathConstraint {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for PathConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.conditions.is_empty() {
            write!(f, "true (feasible: {})", self.feasible)?;
        } else {
            for (i, cond) in self.conditions.iter().enumerate() {
                if i > 0 {
                    write!(f, " ∧ ")?;
                }
                write!(f, "{:?}", cond)?;
            }
            write!(f, " (feasible: {})", self.feasible)?;
        }
        Ok(())
    }
}

/// A constraint collector that accumulates branch conditions during
/// symbolic execution.
#[derive(Debug, Clone)]
pub struct ConstraintCollector {
    /// The current path constraint being built.
    pub current_path: PathConstraint,
    /// All completed path constraints from explored branches.
    pub explored_paths: Vec<PathConstraint>,
}

impl ConstraintCollector {
    /// Create a new constraint collector.
    pub fn new() -> Self {
        Self {
            current_path: PathConstraint::new(),
            explored_paths: Vec::new(),
        }
    }

    /// Record a taken branch (add condition to current path).
    pub fn record_taken_branch(&mut self, condition: SymbolicValue) {
        self.current_path.add_condition(condition);
    }

    /// Fork the current path: record the not-taken branch as a new path
    /// and keep the taken branch as the current path.
    pub fn fork_branch(
        &mut self,
        taken: SymbolicValue,
        not_taken: SymbolicValue,
    ) {
        // The new separate path (not-taken)
        let mut fork_path = self.current_path.clone();
        fork_path.add_condition(not_taken);
        self.explored_paths.push(fork_path);

        // Continue current path with taken
        self.current_path.add_condition(taken);
    }

    /// Finalize the current path and add it to explored paths.
    pub fn finalize_current_path(&mut self) {
        self.explored_paths.push(self.current_path.clone());
        self.current_path = PathConstraint::new();
    }

    /// Get all explored path constraints.
    pub fn explored_paths(&self) -> &[PathConstraint] {
        &self.explored_paths
    }
}

impl Default for ConstraintCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_path() {
        let pc = PathConstraint::new();
        assert_eq!(pc.depth(), 0);
        assert!(pc.feasible);
    }

    #[test]
    fn test_fork_branch() {
        let mut collector = ConstraintCollector::new();
        let cond_taken = SymbolicValue::concrete(1);
        let cond_not_taken = SymbolicValue::concrete(0);

        collector.fork_branch(cond_taken.clone(), cond_not_taken.clone());
        assert_eq!(collector.current_path.depth(), 1);
        assert_eq!(collector.explored_paths.len(), 1);
        assert_eq!(collector.explored_paths[0].depth(), 1);
    }
}