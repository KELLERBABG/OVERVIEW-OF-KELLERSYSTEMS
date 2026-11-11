//! Kinetic Invariant Enforcement.
//!
//! Defines unbreakable logistical rules that must hold over the
//! entire supply chain path. These are encoded as SMT assertions
//! and proven or refuted by the solver.

use sca_types::{ComparisonOp, ConstraintExpr, ConstraintVariable};

/// A kinetic invariant — a rule that must remain true across the
/// entire supply chain path.
#[derive(Debug, Clone)]
pub struct KineticInvariant {
    /// Human-readable name.
    pub name: String,
    /// The constraint expression encoding this invariant.
    pub expression: ConstraintExpr,
    /// Severity: "hard" means the path is invalid; "soft" means advisory.
    pub severity: InvariantSeverity,
}

/// How strictly an invariant is enforced.
#[derive(Debug, Clone, PartialEq)]
pub enum InvariantSeverity {
    /// Path is invalid if violated.
    Hard,
    /// Path is flagged but not rejected.
    Soft,
}

/// Engine for constructing and verifying kinetic invariants.
#[derive(Debug, Clone)]
pub struct KineticInvariantEngine {
    invariants: Vec<KineticInvariant>,
}

impl KineticInvariantEngine {
    pub fn new() -> Self {
        Self { invariants: Vec::new() }
    }

    /// Add an invariant to the engine.
    pub fn add(&mut self, invariant: KineticInvariant) {
        self.invariants.push(invariant);
    }

    /// Get all registered invariants.
    pub fn invariants(&self) -> &[KineticInvariant] {
        &self.invariants
    }

    /// Collect all hard invariant expressions.
    pub fn hard_constraints(&self) -> Vec<ConstraintExpr> {
        self.invariants.iter().filter(|i| i.severity == InvariantSeverity::Hard).map(|i| i.expression.clone()).collect()
    }

    /// Collect all invariant expressions (hard + soft).
    pub fn all_constraints(&self) -> Vec<ConstraintExpr> {
        self.invariants.iter().map(|i| i.expression.clone()).collect()
    }

    // --- Factory methods for common supply chain invariants ---

    /// Perishability invariant: total transit time must not exceed `max_seconds`.
    /// This is the foundational kinetic invariant — it proves that a perishable
    /// good will not spoil before reaching its destination.
    pub fn perishability(max_seconds: i64) -> KineticInvariant {
        KineticInvariant {
            name: format!("Perishability (max {}s)", max_seconds),
            expression: ConstraintExpr::Comparison {
                variable: ConstraintVariable::Custom("transit_time_secs".into()),
                op: ComparisonOp::Le,
                value: max_seconds as f64,
            },
            severity: InvariantSeverity::Hard,
        }
    }

    /// Cold chain invariant: temperature must remain below `max_temp_celsius`.
    pub fn cold_chain(max_temp_celsius: f64) -> KineticInvariant {
        KineticInvariant {
            name: format!("Cold chain (≤{}°C)", max_temp_celsius),
            expression: ConstraintExpr::Comparison {
                variable: ConstraintVariable::Temperature,
                op: ComparisonOp::Le,
                value: max_temp_celsius,
            },
            severity: InvariantSeverity::Hard,
        }
    }

    /// Minimum capacity invariant: all vehicles on the path must have
    /// capacity >= `min_capacity`.
    pub fn minimum_capacity(min_capacity: f64) -> KineticInvariant {
        KineticInvariant {
            name: format!("Min capacity (≥{})", min_capacity),
            expression: ConstraintExpr::capacity(ComparisonOp::Ge, min_capacity),
            severity: InvariantSeverity::Hard,
        }
    }

    /// Maximum border delay invariant: border crossing must not exceed `max_minutes`.
    pub fn max_border_delay(max_minutes: f64) -> KineticInvariant {
        KineticInvariant {
            name: format!("Max border delay (≤{}min)", max_minutes),
            expression: ConstraintExpr::Comparison {
                variable: ConstraintVariable::BorderTime,
                op: ComparisonOp::Le,
                value: max_minutes,
            },
            severity: InvariantSeverity::Soft,
        }
    }

    /// Minimum fuel efficiency invariant.
    pub fn min_fuel_efficiency(min_val: f64) -> KineticInvariant {
        KineticInvariant {
            name: format!("Min fuel efficiency (≥{})", min_val),
            expression: ConstraintExpr::Comparison {
                variable: ConstraintVariable::FuelEfficiency,
                op: ComparisonOp::Ge,
                value: min_val,
            },
            severity: InvariantSeverity::Soft,
        }
    }
}

impl Default for KineticInvariantEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::{MockSolver, Solver};

    #[test]
    fn test_perishability_invariant() {
        let inv = KineticInvariantEngine::perishability(86400); // 24 hours
        assert_eq!(inv.severity, InvariantSeverity::Hard);

        let solver = MockSolver::new().with_var(ConstraintVariable::Custom("transit_time_secs".into()), 3600.0);
        let result = solver.check(&[inv.expression.clone()]);
        assert!(result.satisfiable);

        let solver = MockSolver::new().with_var(ConstraintVariable::Custom("transit_time_secs".into()), 172800.0);
        let result = solver.check(&[inv.expression]);
        assert!(!result.satisfiable);
    }

    #[test]
    fn test_cold_chain_invariant() {
        let inv = KineticInvariantEngine::cold_chain(8.0);
        let solver = MockSolver::new().with_var(ConstraintVariable::Temperature, 5.0);
        assert!(solver.check(&[inv.expression.clone()]).satisfiable);

        let solver = MockSolver::new().with_var(ConstraintVariable::Temperature, 12.0);
        assert!(!solver.check(&[inv.expression]).satisfiable);
    }

    #[test]
    fn test_multiple_invariants() {
        let mut engine = KineticInvariantEngine::new();
        engine.add(KineticInvariantEngine::perishability(86400));
        engine.add(KineticInvariantEngine::cold_chain(8.0));

        let constraints = engine.hard_constraints();
        assert_eq!(constraints.len(), 2);

        let solver = MockSolver::new()
            .with_var(ConstraintVariable::Custom("transit_time_secs".into()), 3600.0)
            .with_var(ConstraintVariable::Temperature, 5.0);

        let result = solver.check(&constraints);
        assert!(result.satisfiable);
    }
}
