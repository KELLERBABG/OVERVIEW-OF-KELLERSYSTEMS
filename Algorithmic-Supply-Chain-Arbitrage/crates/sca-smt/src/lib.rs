//! # sca-smt
//!
//! SMT (Satisfiability Modulo Theories) solver integration for the
//! Algorithmic Supply Chain Arbitrage platform.
//!
//! Provides three capabilities:
//!
//! 1. **Path Constraint Routing** — Verify that a path satisfies
//!    all boolean constraints encoded as edge conditions
//!    (capacity, fuel efficiency, border times, temperature).
//!
//! 2. **Kinetic Invariant Enforcement** — Prove that logistical
//!    invariants hold over the entire path (e.g., perishability
//!    window is never exceeded).
//!
//! 3. **Supply Chain Space Exploration** — Simulate disruption
//!    scenarios (strikes, weather, border closure) and find
//!    viable alternative paths.
//!
//! ## Solver Architecture
//!
//! - **`ExactSolver`** — In-process, sound and complete decision procedure
//!   for the `ConstraintExpr` language. No external dependencies. This is the
//!   solver behind every path verification, invariant check and disruption
//!   exploration.
//! - **Z3 backend** — Planned tier 2 (see ADR-004). `smt_lib` already emits
//!   SMT-LIB 2.0 queries ready for Z3; wiring it up requires an external Z3
//!   installation, which the default build does not assume.

pub mod constraint;
pub mod invariants;
pub mod path_verifier;
pub mod smt_lib;
pub mod solver;
pub mod space_explorer;

pub use invariants::KineticInvariantEngine;
pub use path_verifier::PathVerifier;
pub use solver::{ExactSolver, MockSolver, Solver, SolverResult};
pub use space_explorer::SpaceExplorer;

/// Re-export key constraint types from sca-types for convenience.
pub use sca_types::{ComparisonOp, ConstraintExpr, ConstraintVariable, ScaError};
