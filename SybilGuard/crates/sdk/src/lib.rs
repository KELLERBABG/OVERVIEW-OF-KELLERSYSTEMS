//! # Sybil-Resistentes Client SDK
//!
//! Client library for integrating third-party services with the
//! Proof-of-Personhood system. Provides REST API bindings for:
//! - Node creation and management
//! - Web of Trust operations
//! - Reputation queries
//! - Sybil detection results
//! - DID lifecycle
//! - Nullifier verification

pub mod client;
pub mod types;
pub mod error;

pub use client::*;
pub use types::*;
pub use error::*;