//! State space exploration for kinetic edge case discovery.
//!
//! Systematically explores all possible kinetic states including rare
//! edge cases (aquaplaning, black ice, tire blowout) using configurable
//! search strategies over state regions.
//!
//! # Architecture
//! - [`explorer`] — Grid, Random, Border, MonteCarlo search strategies
//! - [`region`] — StateRegion with presets (nominal, extreme, highway, parking)
//! - [`reachability`] — Forward reachability analysis
//! - [`edge_case`] — Dangerous condition detection (aquaplaning, blowout, ice)

pub mod edge_case;
pub mod explorer;
pub mod reachability;
pub mod region;

pub use edge_case::*;
pub use explorer::*;
pub use reachability::*;
pub use region::*;
