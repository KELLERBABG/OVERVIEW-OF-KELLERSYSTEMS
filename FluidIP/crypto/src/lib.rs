//! # dfip-crypto
//!
//! Cryptographic primitives for the Decoupled Fluid IP Network.
//! Includes homomorphic encryption wrappers, ZK proof helpers, and
//! taint-tracking hash constructions.

pub mod error;
pub mod he;
pub mod zk;

pub use error::*;
pub use he::*;
pub use zk::*;

pub fn init_crypto() {
    tracing::info!("dfip-crypto initialized");
}