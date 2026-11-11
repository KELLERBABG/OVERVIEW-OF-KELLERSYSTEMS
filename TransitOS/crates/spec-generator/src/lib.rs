//! Automated kinetic specification generation.
//!
//! Produces formal specifications from observed driving behavior.
//! Supports both parametric specs (max velocity, acceleration bounds)
//! and temporal logic specifications (LTL) mined from trajectories.
//!
//! # Architecture
//! - [`spec`] — KineticSpec with max and p95 statistics
//! - [`generate`] — Specification inference from trajectories
//! - [`temporal`] — LTL specification types and trace checking

pub mod generate;
pub mod spec;
pub mod temporal;

pub use generate::*;
pub use spec::*;
pub use temporal::*;
