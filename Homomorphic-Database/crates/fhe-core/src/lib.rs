//! # fhe-core — FHE Primitives Layer
//!
//! This crate is the **only** crate in the workspace that directly touches
//! TFHE-rs. It provides a clean, high-level abstraction over the underlying
//! scheme so that higher layers (`fhe-db`, `fhe-server`) never depend on
//! TFHE-rs internals.
//!
//! ## Security Model
//!
//! - The **client** generates and holds the secret [`ClientKey`].
//! - The **server** only receives the public [`ServerKey`] (a.k.a. evaluation
//!   key), which allows computations on encrypted data but reveals nothing.
//! - Encryption is randomized: the same plaintext encrypts to a different
//!   ciphertext every time. Security rests on TFHE-rs's 2M128 parameter set,
//!   which the vendor documents as targeting IND-CPA^D; this is not audited.
//!
//! ## First-class Errors
//!
//! All fallible operations return [`FheError`] instead of panicking so that
//! higher layers can propagate and handle failures (e.g., a corrupted
//! ciphertext arriving over the wire).
//!
//! ## Serialization
//!
//! Keys and ciphertexts implement [`Serialize`]/[`Deserialize`] so encrypted
//! data can persist at rest and travel over the network. A versioned envelope
//! ([`CiphertextEnvelope`]) protects against deserializing a ciphertext with a
//! mismatched arithmetic width.

pub mod algebra;
pub mod ciphertext;
pub mod encrypt;
pub mod error;
pub mod keys;
pub mod nn;

pub use algebra::{CiphertextAlgebra, ColumnAggregate};
pub use ciphertext::{Ciphertext, CiphertextEnvelope, EncryptedColumn, PlaintextValue};
pub use encrypt::Encryptor;
pub use error::FheError;
pub use keys::{ClientKey, SecurityParameters, ServerKey, ServerKeyGuard};

/// The arithmetic width of an encrypted scalar.
///
/// TFHE-rs provides distinct `FheUint*` types per bit width. This enum lets
/// the higher layers choose the width at runtime (e.g. from a database
/// schema) without monomorphizing every code path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ScalarWidth {
    /// 8-bit unsigned integers
    U8,
    /// 16-bit unsigned integers
    U16,
    /// 32-bit unsigned integers
    U32,
    /// 64-bit unsigned integers
    U64,
}

impl ScalarWidth {
    /// Number of bits in this width.
    pub fn bits(self) -> usize {
        match self {
            ScalarWidth::U8 => 8,
            ScalarWidth::U16 => 16,
            ScalarWidth::U32 => 32,
            ScalarWidth::U64 => 64,
        }
    }

    /// The maximum plaintext value representable in this width.
    pub fn max_value(self) -> u64 {
        match self {
            ScalarWidth::U8 => u8::MAX as u64,
            ScalarWidth::U16 => u16::MAX as u64,
            ScalarWidth::U32 => u32::MAX as u64,
            ScalarWidth::U64 => u64::MAX,
        }
    }
}

/// Convenience re-export of the TFHE prelude for users who want the full
/// operator trait set (e.g. `Add`, `Mul`, comparisons) on ciphertexts.
pub mod prelude {
    pub use tfhe::prelude::*;
}
