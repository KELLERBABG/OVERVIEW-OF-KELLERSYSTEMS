//! Library entry point for the Atmospheric Resonance simulation.
//!
//! Exposes the simulation modules so integration tests and benchmarks outside
//! the binary can drive the sim. The binary (`main.rs`) is a thin CLI wrapper
//! over these modules.

pub mod config;
pub mod physics;
pub mod simulation;
pub mod subsystems;
