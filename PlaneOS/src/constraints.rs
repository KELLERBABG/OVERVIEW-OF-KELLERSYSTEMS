//! Kinetic path constraint generation.
//!
//! Per the development plan, while the control engine computes the next
//! action, it collects the logical and physical conditions into a boolean
//! expression describing the entire flight path over a prediction horizon.
//!
//! In this prototype, [`PathConstraints`] expands each lethal hazard across
//! a horizon of `N` future cycles by substituting the canonical
//! `his.<name>_<k>` horizon variables for the current-state variables. The
//! simulation environment seeds those horizon values at each step, and the
//! execution gate can then prove that *no* horizon state violates the
//! envelope — critical state-space exploration in advance.

use crate::envelope::EnvelopeHazards;
use crate::formula::{Expr, Formula, VarNS};
use std::collections::BTreeMap;

/// Number of predicted cycles in the default horizon.
pub const DEFAULT_HORIZON: usize = 8;

/// Kinetic path constraint generation engine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathConstraints {
    pub horizon: usize,
}

impl Default for PathConstraints {
    fn default() -> Self {
        PathConstraints {
            horizon: DEFAULT_HORIZON,
        }
    }
}

impl PathConstraints {
    /// Substitute all `state.*` variables of a formula with the
    /// corresponding horizon variables `his.*_<k>`.
    pub fn to_horizon_step(formula: &Formula, k: usize) -> Formula {
        let mut env = BTreeMap::new();
        let mut vars = Vec::new();
        formula.free_vars(&mut vars);
        for var in vars {
            if let Some(rest) = var.strip_prefix("state.") {
                // Clone `var` for the map key so the borrow on
                // `strip_prefix` does not conflict with the move.
                env.insert(var.clone(), Expr::var(VarNS::Horizon, &format!("{}_{}", rest, k)));
            }
        }
        formula.substitute(&env)
    }

    /// Expand a base hazard formula across the full horizon, producing:
    ///
    /// `hazard(state_0) OR hazard(his_1) OR ... OR hazard(his_N)`
    ///
    /// SAT means the hazard may occur at some point in the predicted path.
    pub fn expand_over_horizon(&self, base: &Formula) -> Formula {
        let mut steps = Vec::with_capacity(self.horizon + 1);
        // k = 0 is the current state (state.* vars already match env).
        steps.push(base.clone());
        for k in 1..=self.horizon {
            steps.push(Self::to_horizon_step(base, k));
        }
        Formula::any(steps)
    }

    /// Expand the standard envelope escape across the horizon.
    pub fn envelope_escape_over_horizon(&self) -> Formula {
        self.expand_over_horizon(&EnvelopeHazards::envelope_escape())
    }

    /// Build a horizon env: map `state.name -> his.name_k` for all base
    /// names appearing in `base` (a snapshot environment).
    pub fn horizon_env(base: &BTreeMap<String, f64>, k: usize) -> BTreeMap<String, f64> {
        let mut out = BTreeMap::new();
        for (name, value) in base {
            if let Some(rest) = name.strip_prefix("state.") {
                out.insert(
                    format!("{}.{}_{}", VarNS::Horizon.prefix(), rest, k),
                    *value,
                );
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::{namespaced_env, SMTBackend, StubSolver};

    #[test]
    fn horizon_substitution_renames_state_vars() {
        let f = EnvelopeHazards::overspeed_vne();
        let h = PathConstraints::to_horizon_step(&f, 3);
        let mut vars = Vec::new();
        h.free_vars(&mut vars);
        assert!(vars.iter().any(|v| v == &"his.ias_knots_3".to_string()));
        assert!(!vars.iter().any(|v| v == &"state.ias_knots".to_string()));
    }

    #[test]
    fn expansion_sat_when_future_step_dangerous() {
        let pc = PathConstraints { horizon: 4 };
        let base = EnvelopeHazards::envelope_escape();
        let formula = pc.expand_over_horizon(&base);

        let mut env = namespaced_env(
            VarNS::State,
            &[
                ("ias_knots", 250.0),
                ("alpha_deg", 4.0),
                ("nz", 1.0),
                ("mach", 0.6),
                ("altitude", 3000.0),
                ("terrain_clearance", 3000.0),
            ],
        );
        for (k, v) in [
            ("mag.envelope.vne_knots", 340.0),
            ("mag.envelope.vs_clean_knots", 120.0),
            ("mag.envelope.alpha_max_deg", 14.0),
            ("mag.envelope.nz_max", 2.5),
            ("mag.envelope.nz_min", -1.0),
            ("mag.envelope.mmo", 0.82),
            ("mag.envelope.max_altitude", 13000.0),
            ("mag.envelope.min_terrain_clearance", 150.0),
        ] {
            env.insert(k.to_string(), v);
        }
        let base_state = env.clone();
        for step in 1..=4 {
            env.extend(PathConstraints::horizon_env(&base_state, step));
        }
        // No hazard yet (current + horizon safe) -> UNSAT.
        assert_eq!(
            StubSolver::new().check(&formula, &env),
            crate::solver::CheckResult::Unsat
        );

        // Override step 3 ias to overspeed.
        env.insert("his.ias_knots_3".to_string(), 345.0);
        assert_eq!(
            StubSolver::new().check(&formula, &env),
            crate::solver::CheckResult::Sat
        );
    }

    #[test]
    fn horizon_env_maps_all_state_vars() {
        let base = namespaced_env(VarNS::State, &[("ias_knots", 250.0), ("altitude", 3000.0)]);
        let h = PathConstraints::horizon_env(&base, 2);
        assert_eq!(h.get("his.ias_knots_2"), Some(&250.0));
        assert_eq!(h.get("his.altitude_2"), Some(&3000.0));
    }
}