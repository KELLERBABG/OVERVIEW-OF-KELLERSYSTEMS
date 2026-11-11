//! Real-time gas and brake analysis for energy verification.
//!
//! Performs symbolic computation of required kinetic energy for maneuvers
//! and verifies that braking systems can deliver enough energy to prevent
//! collisions. Includes comfort constraint checking for passenger safety.
//!
//! # Architecture
//! - [`energy`] — KineticEnergy, BrakingEnergy with regen support
//! - [`analysis`] — Maneuver feasibility (brake, swerve)
//! - [`maneuvers`] — Maneuver envelope from friction limits
//! - [`comfort`] — Comfort constraint limits

pub mod analysis;
pub mod comfort;
pub mod energy;
pub mod maneuvers;

pub use analysis::*;
pub use comfort::*;
pub use energy::*;
pub use maneuvers::*;
