//! # Plane OS — Formally Verified Aerospace Flight Control Prototype
//!
//! **Research / open-source tooling only.** This crate is NOT a deployable
//! avionics product and is not certified to DO-178C (DAL A), EASA CS-25,
//! ARP4754A, or any other airworthiness standard. A solo developer cannot
//! legally certify or deploy safety-critical aviation systems; this project
//! exists for research, education, and open formal-methods tooling (see
//! `development_plan.md`).
//!
//! The architecture implements the core verification chain from the plan:
//!
//! 1. [`formula`] — A tiny symbolic nonlinear-real-arithmetic (NRA) DSL used
//!    to build hazard formulas (boolean expressions over the flight path).
//! 2. [`dynamics`] — 6-DoF rigid-body state with quaternion attitude, wind,
//!    and terrain.
//! 3. [`envelope`] — V-N / Mach / altitude flight envelope models that
//!    produce SMT-friendly hazard formulas.
//! 4. [`constraints`] — Kinetic path constraint generation over a prediction
//!    horizon (critical state-space exploration).
//! 5. [`invariants`] — Lethal invariant enforcement: unbreakable safety rules
//!    proven over the entire runtime.
//! 6. [`thrust`] — Symbolic thrust analysis (N1/N2, EGT, oil pressure,
//!    reverse thrust limits).
//! 7. [`sensors`] — Taint-coded sensor stream audit (pitot, IMU, GPS, ADS-B,
//!    radar → actuation taint tracking).
//! 8. [`gate`] — The trustless execution gate: a physical command is blocked
//!    unless every hazard formula is proven UNSAT.
//! 9. [`spec`] — Automated formal specification generation for the flight
//!    recorder (canonical, hash-chained, machine-readable).
//! 10. [`simulator`] — 6-DoF mock simulator integrating the verification
//!     chain, plus [`controller`] (a prototype fly-by-wire control law).
//!
//! ## Verification backends
//!
//! * `StubSolver` (default) — sound ground evaluation of the concrete
//!   control-cycle state. No external dependencies.
//! * `Z3Backend` (feature `z3`) — real SMT-NRA checks through the Z3 solver
//!   with ground-truth seeding, so SAT/UNSAT is decided on the actual
//!   control-cycle state while Z3 validates the nonlinear constraints
//!   symbolically.

pub mod constraints;
pub mod controller;
pub mod dynamics;
pub mod envelope;
pub mod formula;
pub mod gate;
pub mod invariants;
pub mod sensors;
pub mod simulator;
pub mod solver;
pub mod spec;
pub mod state;
pub mod thrust;

pub use dynamics::{FlightDynamics, FlightState, Quaternion, TerrainModel, Vec3};
pub use envelope::{ConfigEnvelope, FlightEnvelope};
pub use formula::{Comparison, CmpOp, Expr, Formula};
pub use gate::{CaseVerdict, ExecutionGate, GateReport, SafetyCase, VerificationStatus};
pub use solver::{CheckResult, SMTBackend, StubSolver};