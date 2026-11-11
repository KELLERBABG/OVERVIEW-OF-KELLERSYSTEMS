//! Path constraint generation for driving maneuvers.
//!
//! Collects boolean constraints for each driving maneuver and builds
//! SMT-LIB2-compatible constraint sets. Supports temporal constraints
//! (always, eventually) and universal quantification for exhaustive
//! safety proving.
//!
//! # Architecture
//! - [`constraint`] — Constraint enum, ConstraintSet, evaluation
//! - [`maneuvers`] — Maneuver-specific constraint factories
//! - [`evaluate`] — Batch constraint checking
//! - [`smt_lib`] — SMT-LIB2 serialization for Z3
//! - [`rss`] — RSS fault model (v1.0.0): FaultParty, safe distances, violator detection, MRM triggers

pub mod constraint;
pub mod evaluate;
pub mod maneuvers;
pub mod rss;
pub mod smt_lib;

pub use constraint::*;
pub use evaluate::*;
pub use maneuvers::*;
pub use rss::*;
pub use smt_lib::*;
