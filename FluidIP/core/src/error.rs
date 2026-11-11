//! Error types for the dfip-core crate.

use thiserror::Error;

/// Core error types for the Decoupled Fluid IP Network.
#[derive(Error, Debug)]
pub enum CoreError {
    // ─── Graph errors ──────────────────────────────────────────────────────────
    #[error("node not found: {0}")]
    NodeNotFound(String),

    #[error("edge not found: {0}")]
    EdgeNotFound(String),

    #[error("cycle detected: adding edge from {parent} to {child} would create a cycle")]
    CycleDetected { parent: String, child: String },

    #[error("invalid contribution weight: {0}. Must be in [0, 1].")]
    InvalidContributionWeight(f64),

    #[error("duplicate node: {0}")]
    DuplicateNode(String),

    #[error("duplicate edge: {0}")]
    DuplicateEdge(String),

    // ─── SMT / Verification errors ────────────────────────────────────────────
    #[error("SMT solver error: {0}")]
    SmtSolverError(String),

    #[error("verification timeout after {0}ms")]
    VerificationTimeout(u64),

    #[error("invalid SMT-LIB2 formula: {0}")]
    InvalidFormula(String),

    // ─── Royalty errors ────────────────────────────────────────────────────────
    #[error("royalty invariant violation: sum(weights) = {sum}, expected 1.0")]
    RoyaltySumViolation { sum: f64 },

    #[error("royalty cycle detected: {0}")]
    RoyaltyCycle(String),

    // ─── Taint errors ─────────────────────────────────────────────────────────
    #[error("unlicensed fragment detected at node {0}")]
    UnlicensedFragment(String),

    #[error("taint propagation error: {0}")]
    TaintPropagationError(String),

    // ─── Crypto errors ─────────────────────────────────────────────────────────
    #[error("cryptographic error: {0}")]
    CryptoError(String),

    #[error("proof verification failed: {0}")]
    ProofVerificationFailed(String),

    // ─── Serialization ─────────────────────────────────────────────────────────
    #[error("serialization error: {0}")]
    SerializationError(String),

    #[error("deserialization error: {0}")]
    DeserializationError(String),

    // ─── General ───────────────────────────────────────────────────────────────
    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl From<serde_json::Error> for CoreError {
    fn from(e: serde_json::Error) -> Self {
        CoreError::SerializationError(e.to_string())
    }
}