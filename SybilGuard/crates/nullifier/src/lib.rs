//! # Deterministic Identity Nullifiers
//!
//! Prevents double-credential creation by generating unique, deterministic
//! nullifiers from identity seeds. Once a nullifier is registered, the same
//! seed cannot create another credential.
//!
//! Uses BLAKE3 hashing to ensure:
//! - Deterministic: same seed → same nullifier
//! - Unlinkable: nullifier reveals nothing about the seed
//! - Efficient: O(1) generation and verification

pub mod manager;
pub mod store;

pub use manager::*;
pub use store::*;