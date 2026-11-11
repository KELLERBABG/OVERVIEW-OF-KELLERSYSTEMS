//! Flight envelope models and hazard formulas.
//!
//! Implements the flight-envelope constraints from the development plan —
//! V-N (load factor vs airspeed), Mach limits, altitude corridors — and
//! exposes them as SMT-friendly [`Formula`]s over the canonical control
//! cycle state. A SAT formula means the aircraft is outside (or about to
//! leave) the certified envelope and the command must be blocked.

use crate::formula::{Expr, Formula, FormulaBuilder};

/// Configuration/fixed envelope limits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConfigEnvelope {
    /// Never-exceed speed (knots IAS) — `V_NE`.
    pub vne_knots: f64,
    /// Maximum operating speed (knots IAS) — `V_MO`.
    pub vmo_knots: f64,
    /// Stall speed clean (knots IAS).
    pub vs_clean_knots: f64,
    /// Stall speed with flaps (knots IAS).
    pub vs_flaps_knots: f64,
    /// Maximum Mach — `M_MO`.
    pub mmo: f64,
    /// Maximum positive load factor (g).
    pub nz_max: f64,
    /// Minimum load factor (g, usually negative).
    pub nz_min: f64,
    /// Maximum angle of attack (deg) before stall.
    pub alpha_max_deg: f64,
    /// Maximum certified altitude (m).
    pub max_altitude: f64,
    /// Default minimum terrain clearance (m).
    pub min_terrain_clearance: f64,
    /// Surface-rate limits (deg/s) for position-rate limiting.
    pub elevator_rate_max_deg_s: f64,
    pub aileron_rate_max_deg_s: f64,
    /// Command-rate limits (normalized units/s).
    pub cmd_rate_max: f64,
}

impl Default for ConfigEnvelope {
    fn default() -> Self {
        // Transport-category-like mock limits.
        ConfigEnvelope {
            vne_knots: 340.0,
            vmo_knots: 320.0,
            vs_clean_knots: 120.0,
            vs_flaps_knots: 90.0,
            mmo: 0.82,
            nz_max: 2.5,
            nz_min: -1.0,
            alpha_max_deg: 14.0,
            max_altitude: 13_000.0, // ~42,650 ft
            min_terrain_clearance: 150.0,
            elevator_rate_max_deg_s: 60.0,
            aileron_rate_max_deg_s: 40.0,
            cmd_rate_max: 0.9, // normalized / s
        }
    }
}

impl ConfigEnvelope {
    /// Seed envelope constants into the canonical environment.
    pub fn seed_env(&self, env: &mut std::collections::BTreeMap<String, f64>) {
        env.insert("mag.envelope.vne_knots".to_string(), self.vne_knots);
        env.insert("mag.envelope.vmo_knots".to_string(), self.vmo_knots);
        env.insert("mag.envelope.vs_clean_knots".to_string(), self.vs_clean_knots);
        env.insert("mag.envelope.vs_flaps_knots".to_string(), self.vs_flaps_knots);
        env.insert("mag.envelope.mmo".to_string(), self.mmo);
        env.insert("mag.envelope.nz_max".to_string(), self.nz_max);
        env.insert("mag.envelope.nz_min".to_string(), self.nz_min);
        env.insert("mag.envelope.alpha_max_deg".to_string(), self.alpha_max_deg);
        env.insert("mag.envelope.max_altitude".to_string(), self.max_altitude);
        env.insert(
            "mag.envelope.min_terrain_clearance".to_string(),
            self.min_terrain_clearance,
        );
        env.insert(
            "mag.envelope.elevator_rate_max_deg_s".to_string(),
            self.elevator_rate_max_deg_s,
        );
        env.insert(
            "mag.envelope.aileron_rate_max_deg_s".to_string(),
            self.aileron_rate_max_deg_s,
        );
        env.insert("mag.envelope.cmd_rate_max".to_string(), self.cmd_rate_max);
    }
}

/// The active envelope, derived per-control-cycle (e.g., flap-dependent
/// stall speeds, gear drag).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlightEnvelope {
    /// Effective stall speed (knots) for the current configuration.
    pub vs_knots: f64,
    /// Effective max load factor for the current configuration.
    pub nz_eff_max: f64,
    pub nz_eff_min: f64,
    /// Flap-dependent alpha limit.
    pub alpha_eff_max_deg: f64,
}

impl FlightEnvelope {
    /// Derive the active envelope from config + configuration state.
    pub fn from_config(cfg: &ConfigEnvelope, flap_setting: u32) -> Self {
        let flap_scale = (flap_setting.min(3) as f64) / 3.0;
        let vs = cfg.vs_clean_knots - flap_scale * (cfg.vs_clean_knots - cfg.vs_flaps_knots);
        FlightEnvelope {
            vs_knots: vs,
            nz_eff_max: cfg.nz_max,
            nz_eff_min: cfg.nz_min,
            // Flaps extend the usable alpha range slightly (mock).
            alpha_eff_max_deg: cfg.alpha_max_deg + 2.0 * flap_scale,
        }
    }
}

/// Hazard-formula constructors that operate on the canonical variable
/// namespace emitted by [`crate::state::ControlSnapshot::env`].
pub struct EnvelopeHazards;

impl EnvelopeHazards {
    /// SAT iff airspeed exceeds V_NE.
    pub fn overspeed_vne() -> Formula {
        Expr::state("ias_knots").gt(Expr::magic("envelope.vne_knots"))
    }

    /// SAT iff airspeed exceeds V_MO.
    pub fn overspeed_vmo() -> Formula {
        Expr::state("ias_knots").gt(Expr::magic("envelope.vmo_knots"))
    }

    /// SAT iff airspeed is below effective stall speed.
    pub fn underspeed_stall() -> Formula {
        Expr::state("ias_knots").lt(Expr::magic("envelope.vs_clean_knots"))
    }

    /// SAT iff angle of attack exceeds the effective limit.
    pub fn alpha_exceeded() -> Formula {
        Expr::state("alpha_deg").gt(Expr::magic("envelope.alpha_max_deg"))
    }

    /// SAT iff load factor exceeds positive limit.
    pub fn nz_positive_exceeded() -> Formula {
        Expr::state("nz").gt(Expr::magic("envelope.nz_max"))
    }

    /// SAT iff load factor below negative limit.
    pub fn nz_negative_exceeded() -> Formula {
        Expr::state("nz").lt(Expr::magic("envelope.nz_min"))
    }

    /// Mach overspeed (M > M_MO).
    pub fn mach_exceeded() -> Formula {
        Expr::state("mach").gt(Expr::magic("envelope.mmo"))
    }

    /// Altitude above certified ceiling.
    pub fn altitude_exceeded() -> Formula {
        Expr::state("altitude").gt(Expr::magic("envelope.max_altitude"))
    }

    /// Terrain clearance below minimum.
    pub fn terrain_conflict() -> Formula {
        Expr::state("terrain_clearance")
            .lt(Expr::magic("envelope.min_terrain_clearance"))
    }

    /// Combined envelope escape: any single envelope limit violated.
    pub fn envelope_escape() -> Formula {
        Formula::any([
            Self::overspeed_vne(),
            Self::underspeed_stall(),
            Self::alpha_exceeded(),
            Self::nz_positive_exceeded(),
            Self::nz_negative_exceeded(),
            Self::mach_exceeded(),
            Self::altitude_exceeded(),
            Self::terrain_conflict(),
        ])
    }

    /// Stall margin in knots (positive = safe). Useful for warnings rather
    /// than hard blockade.
    pub fn stall_margin_knots() -> Expr {
        Expr::state("ias_knots") - Expr::magic("envelope.vs_clean_knots")
    }
}

/// NRA approximation of the V-N corner: at low speed, the required load
/// factor cannot exceed the structural limit. Exposed as a formula.
pub mod vn {
    use super::*;

    /// SAT iff `nz` exceeds the speed-scaled structural limit.
    ///
    /// A simple model: `nz_limit(v) = nz_max * min(1, (v / v_a)^2)` where
    /// `v_a` is the maneuver speed. Approximation only for research.
    pub fn maneuver_limit_exceeded() -> Formula {
        let v = Expr::state("ias_knots");
        let va = Expr::magic("envelope.vne_knots") * 0.7;
        // ratio = v/va clamped at 1
        let ratio = Expr::Min(Box::new(v / va), Box::new(Expr::Const(1.0)));
        let limit = Expr::magic("envelope.nz_max") * ratio.clone() * ratio;
        Expr::state("nz").gt(limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formula::VarNS;
    use crate::solver::{namespaced_env, SMTBackend, StubSolver};

    #[test]
    fn overspeed_detected() {
        let f = EnvelopeHazards::overspeed_vne();
        let mut env = namespaced_env(VarNS::State, &[("ias_knots", 350.0)]);
        env.insert("mag.envelope.vne_knots".to_string(), 340.0);
        assert!(StubSolver::new().check(&f, &env).is_sat());
    }

    #[test]
    fn inside_envelope_is_unsat() {
        let f = EnvelopeHazards::envelope_escape();
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
        assert!(StubSolver::new().check(&f, &env).is_unsat());
    }

    #[test]
    fn stall_detected() {
        let f = EnvelopeHazards::underspeed_stall();
        let mut env = namespaced_env(VarNS::State, &[("ias_knots", 100.0)]);
        env.insert("mag.envelope.vs_clean_knots".to_string(), 120.0);
        assert!(StubSolver::new().check(&f, &env).is_sat());
    }

    #[test]
    fn vn_limit_properly_boxed() {
        let f = vn::maneuver_limit_exceeded();
        let mut env = namespaced_env(VarNS::State, &[("ias_knots", 300.0), ("nz", 3.0)]);
        env.insert("mag.envelope.vne_knots".to_string(), 340.0);
        env.insert("mag.envelope.nz_max".to_string(), 2.5);
        // At high speed ratio=(300/238)=1.26 -> clamped 1.0, limit=2.5,
        // nz=3.0 > 2.5 -> SAT.
        assert!(StubSolver::new().check(&f, &env).is_sat());
    }
}