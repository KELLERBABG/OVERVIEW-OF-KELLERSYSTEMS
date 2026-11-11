//! # dfip-core
//!
//! Canonical data models and graph engine for the Decoupled Fluid IP Network.
//! This crate defines the core types used across all other crates.

pub mod models;
pub mod graph;
pub mod error;
pub mod ir;
pub mod delta;
pub mod solver;
pub mod spec;
pub mod royalties;
pub mod taint;
pub mod explorer;
pub mod fidelity;
pub mod solver_strategy;
pub mod regulation;
pub mod contribution;

pub use models::*;
pub use graph::*;
pub use error::*;
pub use ir::*;
pub use delta::*;
pub use solver::*;
pub use spec::*;
pub use royalties::*;
pub use taint::*;
pub use explorer::*;
pub use fidelity::*;
pub use solver_strategy::*;
pub use regulation::*;
pub use contribution::*;
