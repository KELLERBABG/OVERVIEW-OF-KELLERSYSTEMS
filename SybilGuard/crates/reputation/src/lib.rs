//! # Graph-Embedded Reputation
//!
//! Dynamic scoring algorithm that calculates a node's reputation
//! based on:
//! - Base score (initial grant)
//! - Embedding depth (how deep in the trust graph)
//! - Clustering coefficient (community validation)
//! - Decay over inactivity
//! - Sybil penalties

pub mod engine;
pub mod decay;

pub use engine::*;
pub use decay::*;