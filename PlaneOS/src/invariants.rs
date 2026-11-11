//! Lethal invariant enforcement.
//!
//! Per the development plan, "lethal invariants" are unbreakable safety
//! rules — e.g., the physical impossibility of the aircraft leaving its
//! certified flight envelope or violating terrain/altitude corridors —
//! proven over the entire runtime.
//!
//! In this prototype, [`LethalInvariants::standard()`] returns the set of
//! invariant hazard formulas that the execution gate checks every cycle.
//! The invariant verifier additionally provides:
//!
//! * a **never-satisfied monitor** — a library-side invariant that must
//!   never appear as SAT over a run,
//! * warning margin checks computed as formulas (stall-margin, terrain
//!   margin, alpha-margin) for advisory purposes.
//!
//! This module is intentionally thin: the actual proof obligation is
//! discharged by the gate in [`crate::gate`].

use crate::envelope::{EnvelopeHazards, FlightEnvelope};
use crate::formula::{Expr, Formula, FormulaBuilder};
use crate::gate::SafetyCase;
use crate::solver::{CheckResult, SMTBackend, StubSolver};
use std::collections::BTreeMap;

/// Named margin violations (advisory, not blocking).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarginKind {
    Stall,
    Terrain,
    Alpha,
    Overspeed,
}

impl MarginKind {
    pub fn name(self) -> &'static str {
        match self {
            MarginKind::Stall => "stall_margin",
            MarginKind::Terrain => "terrain_margin",
            MarginKind::Alpha => "alpha_margin",
            MarginKind::Overspeed => "overspeed_margin",
        }
    }
}

/// The lethal invariant enforcement module.
#[derive(Clone, Debug)]
pub struct LethalInvariants;

impl LethalInvariants {
    /// Standard lethal hazards for the gate: envelope + V-N corner.
    pub fn standard() -> Vec<SafetyCase> {
        vec![
            SafetyCase::new(
                "lethal.invariant.envelope",
                "Certified flight envelope must never be violated",
                EnvelopeHazards::envelope_escape(),
            ),
            SafetyCase::new(
                "lethal.invariant.vn",
                "V-N maneuver limit must never be exceeded",
                crate::envelope::vn::maneuver_limit_exceeded(),
            ),
        ]
    }

    /// Build advisory margin formulas for the envelope.
    ///
    /// Returns a list of `(kind, formula)` where SAT means the margin is
    /// below the given threshold.
    pub fn margin_hazards(
        envelope: &FlightEnvelope,
        stall_margin_knots: f64,
        terrain_margin_m: f64,
        alpha_margin_deg: f64,
        overspeed_margin_knots: f64,
    ) -> Vec<(MarginKind, Formula)> {
        let _ = envelope; // active envelope reserves flap/gear scaling
        // Stall margin: IAS - vs < stall_margin_knots -> SAT hazard.
        let stall = (Expr::state("ias_knots")
            - Expr::magic("envelope.vs_clean_knots"))
        .lt(Expr::Const(stall_margin_knots));
        // Terrain margin: clearance < terrain_margin_m.
        let terrain = Expr::state("terrain_clearance").lt(Expr::Const(terrain_margin_m));
        // Alpha margin: alpha > alpha_max - alpha_margin_deg.
        let alpha = (Expr::magic("envelope.alpha_max_deg") - Expr::Const(alpha_margin_deg))
            .lt(Expr::state("alpha_deg"));
        // Overspeed margin: VNE - IAS < overspeed_margin_knots.
        let overspeed =
            (Expr::magic("envelope.vne_knots") - Expr::state("ias_knots"))
                .lt(Expr::Const(overspeed_margin_knots));

        vec![
            (MarginKind::Stall, stall),
            (MarginKind::Terrain, terrain),
            (MarginKind::Alpha, alpha),
            (MarginKind::Overspeed, overspeed),
        ]
    }

    /// Verify that an invariant set is never satisfiable across a run of
    /// control-cycle environments.
    ///
    /// If any environment makes an invariant SAT, the run is `false`
    /// (invariant broken). This is the property the development plan calls
    /// "proven over the entire runtime" — sampled here over the recorded
    /// environments.
    pub fn verify_run<I>(envs: I) -> bool
    where
        I: IntoIterator<Item = BTreeMap<String, f64>>,
    {
        let envs: Vec<BTreeMap<String, f64>> = envs.into_iter().collect();
        let solver = StubSolver::new();
        for case in Self::standard() {
            for env in &envs {
                // Standard cases reference state vars that may be missing
                // in a partial env -> Unknown (conservative). Missing data
                // is treated as a failed proof.
                if solver.check(&case.formula, env) != CheckResult::Unsat {
                    return false;
                }
            }
        }
        true
    }

    /// Stall-margin advisory expression (knots).
    pub fn stall_margin_expr() -> Expr {
        EnvelopeHazards::stall_margin_knots()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formula::VarNS;
    use crate::solver::namespaced_env;

    fn safe_env() -> BTreeMap<String, f64> {
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
        env
    }

    #[test]
    fn run_of_safe_envs_verifies_true() {
        let env = safe_env();
        assert!(LethalInvariants::verify_run([env.clone(), env.clone()]));
    }

    #[test]
    fn run_with_overspeed_breaks_invariant() {
        let mut env = safe_env();
        env.insert("state.ias_knots".to_string(), 350.0);
        assert!(!LethalInvariants::verify_run([env]));
    }

    #[test]
    fn margin_hazards_detected() {
        let env = safe_env();
        let solver = StubSolver::new();
        let hazards = LethalInvariants::margin_hazards(
            &FlightEnvelope::from_config(&crate::envelope::ConfigEnvelope::default(), 0),
            10.0,
            100.0,
            3.0,
            10.0,
        );
        // Safe env: no margin hazard should be SAT.
        for (_kind, f) in hazards {
            assert_eq!(solver.check(&f, &env), CheckResult::Unsat);
        }
    }

    #[test]
    fn stall_margin_expression_supported() {
        let e = LethalInvariants::stall_margin_expr();
        let mut env = namespaced_env(VarNS::State, &[("ias_knots", 250.0)]);
        env.insert("mag.envelope.vs_clean_knots".to_string(), 120.0);
        assert!(e.eval(&env).unwrap() > 100.0);
    }

    #[test]
    fn margin_hazard_triggers_on_low_speed() {
        let mut env = safe_env();
        env.insert("state.ias_knots".to_string(), 125.0);
        let solver = StubSolver::new();
        let hazards = LethalInvariants::margin_hazards(
            &FlightEnvelope::from_config(&crate::envelope::ConfigEnvelope::default(), 0),
            10.0,
            100.0,
            3.0,
            10.0,
        );
        // Stall margin 5 < 10 -> SAT.
        let stall = hazards
            .iter()
            .find(|(k, _)| *k == MarginKind::Stall)
            .map(|(_, f)| f)
            .expect("stall hazard present");
        assert_eq!(solver.check(stall, &env), CheckResult::Sat);
    }
}