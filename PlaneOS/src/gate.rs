//! Trustless execution gate.
//!
//! Per the development plan, the execution gate blocks a physical command
//! to the hydraulics/servos or throttles unless a valid mathematical proof
//! for the absolute safety of the action exists.
//!
//! In this prototype, a [`SafetyCase`] is a named hazard [`Formula`] with a
//! human-readable description. The [`ExecutionGate`] checks every hazard
//! against the current control-cycle environment; if **any** hazard is SAT
//! (or the solver cannot prove UNSAT), the candidate command is **blocked**
//! and the aircraft freezes — fail-safe by default.

use crate::formula::Formula;
use crate::solver::{CheckResult, SMTBackend, StubSolver};
use std::collections::BTreeMap;

/// Verdict for one safety case.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaseVerdict {
    /// Hazard is provably absent. Safe to proceed.
    ProvenSafe,
    /// Hazard formula is satisfiable — command is dangerous.
    HazardSat,
    /// Solver produced UNKNOWN — treat as unsafe.
    Unproven,
}

impl CaseVerdict {
    pub fn safe(self) -> bool {
        self == CaseVerdict::ProvenSafe
    }
}

impl std::fmt::Display for CaseVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaseVerdict::ProvenSafe => write!(f, "PROVEN_SAFE"),
            CaseVerdict::HazardSat => write!(f, "HAZARD_SAT"),
            CaseVerdict::Unproven => write!(f, "UNPROVEN"),
        }
    }
}

/// A named hazard: a formula whose SAT means danger.
#[derive(Clone, Debug, PartialEq)]
pub struct SafetyCase {
    pub name: &'static str,
    pub description: &'static str,
    pub formula: Formula,
}

impl SafetyCase {
    pub fn new(name: &'static str, description: &'static str, formula: Formula) -> Self {
        SafetyCase {
            name,
            description,
            formula,
        }
    }
}

/// Verdict report for one case.
#[derive(Clone, Debug, PartialEq)]
pub struct CaseReport {
    pub name: &'static str,
    pub verdict: CaseVerdict,
    pub result: CheckResult,
    /// The hazard formula in canonical form.
    pub formula: String,
}

/// Full gate report for one control cycle.
#[derive(Clone, Debug, PartialEq)]
pub struct GateReport {
    pub cycle: u64,
    pub approved: bool,
    pub blocked_reason: Option<&'static str>,
    pub cases: Vec<CaseReport>,
    /// Number of checked hazard formulas.
    pub case_count: usize,
    /// Solver wall time in microseconds (mock: StubSolver is instant).
    pub check_time_us: u64,
}

impl GateReport {
    pub fn blocked(&self) -> bool {
        !self.approved
    }
}

/// Aggregate status of the gate across all cases of a cycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationStatus {
    /// Every case proved safe and trust is enabled.
    Approved,
    /// At least one case is SAT — command blocked.
    Blocked,
    /// Trust condition failed or proof was unproven.
    Unverified,
}

impl VerificationStatus {
    pub fn from_report(report: &GateReport) -> Self {
        if report.approved {
            VerificationStatus::Approved
        } else if report.blocked_reason.is_some() {
            VerificationStatus::Blocked
        } else {
            VerificationStatus::Unverified
        }
    }
}

impl std::fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerificationStatus::Approved => write!(f, "APPROVED"),
            VerificationStatus::Blocked => write!(f, "BLOCKED"),
            VerificationStatus::Unverified => write!(f, "UNVERIFIED"),
        }
    }
}

/// The execution gate.
///
/// `Clone` is implemented manually because the solver is stored as a boxed
/// trait object; cloning re-boxes via [`SMTBackend::clone_box`].
#[derive(Debug)]
pub struct ExecutionGate {
    /// Hazard cases to verify (envelope, thrust, taint, invariants).
    pub cases: Vec<SafetyCase>,
    /// The verification backend.
    pub backend: Box<dyn SMTBackend>,
    /// Counts of decisions for diagnostics.
    pub approved_cycles: u64,
    pub blocked_cycles: u64,
    /// Set to `false` to simulate a failed trustless condition.
    pub trust_enabled: bool,
}

impl Clone for ExecutionGate {
    fn clone(&self) -> Self {
        ExecutionGate {
            cases: self.cases.clone(),
            backend: self.backend.clone_box(),
            approved_cycles: self.approved_cycles,
            blocked_cycles: self.blocked_cycles,
            trust_enabled: self.trust_enabled,
        }
    }
}

impl ExecutionGate {
    /// Create a gate with a StubSolver (default backend).
    pub fn stub(cases: Vec<SafetyCase>) -> Self {
        ExecutionGate {
            cases,
            backend: Box::new(StubSolver::new()),
            approved_cycles: 0,
            blocked_cycles: 0,
            trust_enabled: true,
        }
    }

    /// Evaluate all safety cases against an environment.
    pub fn evaluate(&mut self, env: &BTreeMap<String, f64>, cycle: u64) -> GateReport {
        let mut reports = Vec::with_capacity(self.cases.len());
        let mut approved = true;
        let mut first_blocked: Option<&'static str> = None;
        let start = std::time::Instant::now();

        for case in &self.cases {
            let result = self.backend.check(&case.formula, env);
            let verdict = match result {
                CheckResult::Unsat => CaseVerdict::ProvenSafe,
                CheckResult::Sat => CaseVerdict::HazardSat,
                CheckResult::Unknown => CaseVerdict::Unproven,
            };
            if !verdict.safe() && approved {
                approved = false;
                first_blocked = Some(case.name);
            }
            reports.push(CaseReport {
                name: case.name,
                verdict,
                result,
                formula: case.formula.to_canonical(),
            });
        }

        let check_time_us = start.elapsed().as_micros() as u64;
        if approved {
            self.approved_cycles += 1;
        } else {
            self.blocked_cycles += 1;
        }

        GateReport {
            cycle,
            approved: approved && self.trust_enabled,
            blocked_reason: if approved { None } else { first_blocked },
            cases: reports,
            case_count: self.cases.len(),
            check_time_us,
        }
    }
}

/// Convenience constructors for the full safety-case set of the prototype.
pub struct StandardCases;

impl StandardCases {
    /// The canonical set of lethal invariants checked every cycle.
    ///
    /// Merges envelope escape, V-N maneuver limit, thrust hazards, taint
    /// hazards, and trustless channel validity.
    pub fn all() -> Vec<SafetyCase> {
        vec![
            SafetyCase::new(
                "envelope_escape",
                "Aircraft outside certified flight envelope (V-N, Mach, altitude, terrain)",
                crate::envelope::EnvelopeHazards::envelope_escape(),
            ),
            SafetyCase::new(
                "vn_maneuver_limit",
                "Load factor exceeds speed-scaled structural limit (V-N corner)",
                crate::envelope::vn::maneuver_limit_exceeded(),
            ),
            SafetyCase::new(
                "thrust_hazard",
                "Engine N1/N2/EGT/oil limit violated or reverser overspeed",
                crate::thrust::ThrustHazards::thrust_hazard(),
            ),
            SafetyCase::new(
                "taint_hazard",
                "Command routed through a failed/tainted actuation channel",
                crate::sensors::SensorTaintGraph::taint_hazard_formula(),
            ),
            SafetyCase::new(
                "throttle_on_failed_engine",
                "Throttle commanded on an unauthorized/failed engine",
                crate::thrust::ThrustHazards::throttle_on_failed_engine(),
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formula::VarNS;
    use crate::solver::namespaced_env;

    fn healthy_env() -> BTreeMap<String, f64> {
        let mut env = namespaced_env(
            VarNS::State,
            &[
                ("ias_knots", 250.0),
                ("alpha_deg", 4.0),
                ("nz", 1.0),
                ("mach", 0.6),
                ("altitude", 3000.0),
                ("terrain_clearance", 3000.0),
                ("n1_L", 0.9),
                ("n1_R", 0.9),
                ("n2_L", 0.95),
                ("n2_R", 0.95),
                ("egt_L", 520.0),
                ("egt_R", 520.0),
                ("oil_pressure_L", 55.0),
                ("oil_pressure_R", 55.0),
                ("reverser_armed", 0.0),
                ("aileron_ok", 1.0),
                ("elevator_ok", 1.0),
                ("rudder_ok", 1.0),
                ("throttle_ok", 1.0),
            ],
        );
        env.insert("cmd.throttle".to_string(), 0.3);
        env.insert("cmd.elevator".to_string(), 0.1);
        env.insert("cmd.aileron".to_string(), 0.0);
        env.insert("cmd.rudder".to_string(), 0.0);
        for (k, v) in [
            ("mag.envelope.vne_knots", 340.0),
            ("mag.envelope.vs_clean_knots", 120.0),
            ("mag.envelope.alpha_max_deg", 14.0),
            ("mag.envelope.nz_max", 2.5),
            ("mag.envelope.nz_min", -1.0),
            ("mag.envelope.mmo", 0.82),
            ("mag.envelope.max_altitude", 13000.0),
            ("mag.envelope.min_terrain_clearance", 150.0),
            ("mag.engine.n1_max", 1.05),
            ("mag.engine.n2_max", 1.12),
            ("mag.engine.egt_max", 950.0),
            ("mag.engine.oil_pressure_min", 20.0),
            ("mag.engine.n1_reverse_max", 0.32),
        ] {
            env.insert(k.to_string(), v);
        }
        env
    }

    #[test]
    fn safe_command_approved() {
        let mut gate = ExecutionGate::stub(StandardCases::all());
        let env = healthy_env();
        let report = gate.evaluate(&env, 1);
        assert!(report.approved, "expected approval, got {:?}", report.cases);
        assert_eq!(
            VerificationStatus::from_report(&report),
            VerificationStatus::Approved
        );
        assert_eq!(gate.approved_cycles, 1);
    }

    #[test]
    fn overspeed_blocked() {
        let mut gate = ExecutionGate::stub(StandardCases::all());
        let mut env = healthy_env();
        env.insert("state.ias_knots".to_string(), 350.0);
        let report = gate.evaluate(&env, 1);
        assert!(report.blocked());
        assert_eq!(report.blocked_reason, Some("envelope_escape"));
        assert_eq!(
            VerificationStatus::from_report(&report),
            VerificationStatus::Blocked
        );
    }

    #[test]
    fn trust_disabled_blocks_all() {
        let mut gate = ExecutionGate::stub(StandardCases::all());
        gate.trust_enabled = false;
        let env = namespaced_env(VarNS::State, &[]);
        let report = gate.evaluate(&env, 1);
        assert!(report.blocked());
    }

    #[test]
    fn gate_clones_independently() {
        let mut gate = ExecutionGate::stub(StandardCases::all());
        let env = healthy_env();
        let _ = gate.evaluate(&env, 1);
        let clone = gate.clone();
        assert_eq!(clone.approved_cycles, 1);
        assert_eq!(clone.cases.len(), gate.cases.len());
    }
}