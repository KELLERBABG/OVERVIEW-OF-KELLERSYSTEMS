//! # Web of Trust Anchoring
//!
//! Cryptographic vouching protocol that builds trust chains
//! between nodes. Implements:
//! - ed25519-based vouching with signature verification
//! - Trust chain construction and verification
//! - Recursive vouching limits (max depth, max power)
//! - Vouch expiry and revocation

pub mod vouching;
pub mod trust_chain;
pub mod limits;
pub mod bootstrap;

pub use vouching::*;
pub use trust_chain::*;
pub use limits::*;
pub use bootstrap::*;
