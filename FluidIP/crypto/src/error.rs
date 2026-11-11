//! Error types for the dfip-crypto crate.

use thiserror::Error;

/// Cryptographic error types.
#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("key generation failed: {0}")]
    KeyGenerationFailed(String),

    #[error("signature verification failed: {0}")]
    SignatureVerificationFailed(String),

    #[error("homomorphic operation failed: {0}")]
    HomomorphicOperationFailed(String),

    #[error("zero-knowledge proof generation failed: {0}")]
    ProofGenerationFailed(String),

    #[error("zero-knowledge proof verification failed: {0}")]
    ProofVerificationFailed(String),

    #[error("invalid parameter: {0}")]
    InvalidParameter(String),
}