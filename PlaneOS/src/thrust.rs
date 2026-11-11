//! Symbolic thrust analysis.
//!
//! Per the development plan, the engine power/thrust delivery is computed
//! symbolically for each control cycle to exclude hardware overload or
//! functional failure in advance. This module expresses engine limits as
//! [`Formula`]s over the canonical engine variables (`state.n1_L/R`,
//! `state.n2_L/R`, `state.egt_L/R`, `state.oil_pressure_L/R`) and the
//! pending throttle command (`cmd.throttle`).

use crate::formula::{Expr, Formula, FormulaBuilder};

/// Engine configuration limits (normalized speeds, temps, pressures).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EngineLimits {
    /// Maximum N1 (normalized, 1.0 = 100%).
    pub n1_max: f64,
    /// Maximum continuous N2 (normalized).
    pub n2_max: f64,
    /// N1 for reverse thrust (normalized).
    pub n1_reverse_max: f64,
    /// Maximum exhaust gas temperature (deg C).
    pub egt_max: f64,
    /// Minimum oil pressure (psi).
    pub oil_pressure_min: f64,
    /// Maximum allowable throttle change per cycle (normalized).
    pub throttle_rate_max: f64,
}

impl Default for EngineLimits {
    fn default() -> Self {
        EngineLimits {
            n1_max: 1.05,
            n2_max: 1.12,
            n1_reverse_max: 0.32,
            egt_max: 950.0,
            oil_pressure_min: 20.0,
            throttle_rate_max: 0.15,
        }
    }
}

impl EngineLimits {
    pub fn seed_env(&self, env: &mut std::collections::BTreeMap<String, f64>) {
        env.insert("mag.engine.n1_max".to_string(), self.n1_max);
        env.insert("mag.engine.n2_max".to_string(), self.n2_max);
        env.insert("mag.engine.n1_reverse_max".to_string(), self.n1_reverse_max);
        env.insert("mag.engine.egt_max".to_string(), self.egt_max);
        env.insert("mag.engine.oil_pressure_min".to_string(), self.oil_pressure_min);
        env.insert(
            "mag.engine.throttle_rate_max".to_string(),
            self.throttle_rate_max,
        );
    }
}

/// Engine-hazard formula constructors.
pub struct ThrustHazards;

impl ThrustHazards {
    /// Left engine fan speed over limit.
    pub fn n1_left_exceeded() -> Formula {
        Expr::state("n1_L").gt(Expr::magic("engine.n1_max"))
    }

    /// Right engine fan speed over limit.
    pub fn n1_right_exceeded() -> Formula {
        Expr::state("n1_R").gt(Expr::magic("engine.n1_max"))
    }

    /// Either engine N1 exceeded.
    pub fn n1_exceeded() -> Formula {
        Formula::or(Self::n1_left_exceeded(), Self::n1_right_exceeded())
    }

    /// Left engine core speed over limit.
    pub fn n2_left_exceeded() -> Formula {
        Expr::state("n2_L").gt(Expr::magic("engine.n2_max"))
    }

    /// Right engine core speed over limit.
    pub fn n2_right_exceeded() -> Formula {
        Expr::state("n2_R").gt(Expr::magic("engine.n2_max"))
    }

    /// Either engine N2 exceeded.
    pub fn n2_exceeded() -> Formula {
        Formula::or(Self::n2_left_exceeded(), Self::n2_right_exceeded())
    }

    /// Left EGT over limit.
    pub fn egt_left_exceeded() -> Formula {
        Expr::state("egt_L").gt(Expr::magic("engine.egt_max"))
    }

    /// Right EGT over limit.
    pub fn egt_right_exceeded() -> Formula {
        Expr::state("egt_R").gt(Expr::magic("engine.egt_max"))
    }

    /// Either engine EGT exceeded.
    pub fn egt_exceeded() -> Formula {
        Formula::or(Self::egt_left_exceeded(), Self::egt_right_exceeded())
    }

    /// Left oil pressure below minimum.
    pub fn oil_left_low() -> Formula {
        Expr::state("oil_pressure_L").lt(Expr::magic("engine.oil_pressure_min"))
    }

    /// Right oil pressure below minimum.
    pub fn oil_right_low() -> Formula {
        Expr::state("oil_pressure_R").lt(Expr::magic("engine.oil_pressure_min"))
    }

    /// Either engine oil pressure low.
    pub fn oil_low() -> Formula {
        Formula::or(Self::oil_left_low(), Self::oil_right_low())
    }

    /// Thrust reverser command at excessive N1.
    ///
    /// SAT iff reverser is armed AND the throttle (reverse thrust) would
    /// push N1 above the reverse limit. Models `cmd.throttle < 0`
    /// (reverse) with `state.reverser_armed` as a 0/1 variable.
    pub fn reverser_overspeed() -> Formula {
        Formula::and(
            Expr::state("reverser_armed").gt(Expr::Const(0.5)),
            Formula::or(
                Expr::state("n1_L").gt(Expr::magic("engine.n1_reverse_max")),
                Expr::state("n1_R").gt(Expr::magic("engine.n1_reverse_max")),
            ),
        )
    }

    /// Engines failed or unauthorized throttles.
    ///
    /// SAT iff the throttle command is applied to an engine that is
    /// marked failed (`engine_failed` variable set to engine index+1 via
    /// `cmd.throttle_l_failed` / `cmd.throttle_r_failed`).
    pub fn throttle_on_failed_engine() -> Formula {
        Formula::and(
            Expr::state("throttle_ok").lt(Expr::Const(0.5)),
            Expr::cmd("throttle").gt(Expr::Const(0.01)),
        )
    }

    /// Combined thrust hazard: any single engine constraint violated.
    pub fn thrust_hazard() -> Formula {
        Formula::any([
            Self::n1_exceeded(),
            Self::n2_exceeded(),
            Self::egt_exceeded(),
            Self::oil_low(),
            Self::reverser_overspeed(),
        ])
    }
}

/// Thrust-reversal legality: reverse thrust is only permitted on the
/// ground or at very low airspeed on a dry runway (mock). The formula is
/// SAT when a reverse command is illegal.
pub struct ThrustReverserPolicy;

impl ThrustReverserPolicy {
    /// SAT iff reverser is armed while airborne above a threshold.
    pub fn illegal_airborne_reverser() -> Formula {
        Formula::and(
            Expr::state("reverser_armed").gt(Expr::Const(0.5)),
            Expr::state("altitude").gt(Expr::magic("env.alt_reverser_max")),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::{namespaced_env, SMTBackend, StubSolver};
    use crate::formula::VarNS;

    #[test]
    fn n1_limit_detected() {
        let f = ThrustHazards::n1_exceeded();
        let mut env = namespaced_env(VarNS::State, &[("n1_L", 1.09), ("n1_R", 0.8)]);
        env.insert("mag.engine.n1_max".to_string(), 1.05);
        assert!(StubSolver::new().check(&f, &env).is_sat());

        env.insert("state.n1_L".to_string(), 1.04);
        assert!(StubSolver::new().check(&f, &env).is_unsat());
    }

    #[test]
    fn egt_limit_detected() {
        let f = ThrustHazards::egt_exceeded();
        let mut env = namespaced_env(VarNS::State, &[("egt_L", 960.0), ("egt_R", 500.0)]);
        env.insert("mag.engine.egt_max".to_string(), 950.0);
        assert!(StubSolver::new().check(&f, &env).is_sat());
    }

    #[test]
    fn reverser_only_when_armed() {
        let f = ThrustHazards::reverser_overspeed();
        let mut env = namespaced_env(
            VarNS::State,
            &[("reverser_armed", 0.0), ("n1_L", 0.5), ("n1_R", 0.5)],
        );
        env.insert("mag.engine.n1_reverse_max".to_string(), 0.32);
        assert!(StubSolver::new().check(&f, &env).is_unsat());

        env.insert("state.reverser_armed".to_string(), 1.0);
        assert!(StubSolver::new().check(&f, &env).is_sat());
    }

    #[test]
    fn throttle_on_failed_engine_detected() {
        let f = ThrustHazards::throttle_on_failed_engine();
        let mut env = namespaced_env(VarNS::State, &[("throttle_ok", 0.0)]);
        env.insert("state.throttle_ok".to_string(), 0.0);
        env.insert("cmd.throttle".to_string(), 0.7);
        assert!(StubSolver::new().check(&f, &env).is_sat());
    }
}