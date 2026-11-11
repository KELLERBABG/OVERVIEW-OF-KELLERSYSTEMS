//! Formal verification invariants for unbreakable safety properties.
//!
//! Proves that the vehicle always satisfies critical safety properties:
//! braking distance ≤ safety gap, lateral acceleration ≤ rollover threshold,
//! speed ≤ limit, slip angle within stable region, etc. Supports invariant
//! composition and runtime monitoring.
//!
//! # Architecture
//! - [`invariants`] — SafetyInvariant trait, 7 concrete invariants, CompositeInvariant
//! - [`proof`] — ProofObligation, batch verification
//! - [`monitor`] — RuntimeMonitor with violation history

pub mod invariants;
pub mod monitor;
pub mod proof;

pub use invariants::*;
pub use monitor::*;
pub use proof::*;
