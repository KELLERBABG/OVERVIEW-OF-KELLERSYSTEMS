//! SMT solver integration for real-time collision detection.
//!
//! Provides a pluggable solver backend architecture with a built-in
//! analytic geometric solver. Collision queries use swept-path circle
//! intersection for passenger cars and oriented-rectangle (SAT) collision
//! for long vehicles. Generates SMT-LIB2 output for external Z3 verification.
//!
//! # Architecture
//! - [`solver`] — SolverBackend trait, SmtSolver, GeometricSolver
//! - [`query`] — CollisionQuery, Obstacle, SolverResult
//! - [`smt_lib`] — SMT-LIB2 query generation
//! - [`collision`] — Swept path collision checking

pub mod collision;
pub mod query;
pub mod smt_lib;
pub mod solver;
pub mod z3_backend;

pub use collision::*;
pub use query::*;
pub use smt_lib::*;
pub use solver::*;
pub use z3_backend::*;
