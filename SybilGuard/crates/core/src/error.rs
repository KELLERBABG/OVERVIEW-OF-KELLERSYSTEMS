use thiserror::Error;

/// Top-level error type for the Proof-of-Personhood system.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum SybilError {
    // ── Graph errors ──────────────────────────────────────────────
    #[error("graph error: {0}")]
    Graph(String),

    #[error("node {0} not found in graph")]
    NodeNotFound(String),

    #[error("edge already exists between {0} and {1}")]
    EdgeAlreadyExists(String, String),

    // ── Web of Trust errors ───────────────────────────────────────
    #[error("vouching error: {0}")]
    Vouching(String),

    #[error("max vouching depth exceeded (limit: {limit}, attempted: {attempted})")]
    MaxVouchingDepthExceeded { limit: u32, attempted: u32 },

    #[error("vouching power exhausted for node {0}")]
    VouchingPowerExhausted(String),

    #[error("cannot vouch for self")]
    SelfVouchNotAllowed,

    #[error("target node {0} is not a member of the web of trust")]
    NotInWebOfTrust(String),

    // ── DID errors ────────────────────────────────────────────────
    #[error("DID error: {0}")]
    Did(String),

    #[error("DID method {0} not supported")]
    UnsupportedDidMethod(String),

    #[error("DID {0} not found")]
    DidNotFound(String),

    #[error("DID resolution failed: {0}")]
    DidResolutionFailed(String),

    // ── Nullifier errors ──────────────────────────────────────────
    #[error("nullifier error: {0}")]
    Nullifier(String),

    #[error("duplicate nullifier: identity already used")]
    DuplicateNullifier,

    #[error("nullifier {0} already exists in the set")]
    NullifierAlreadyExists(String),

    // ── Reputation errors ─────────────────────────────────────────
    #[error("reputation error: {0}")]
    Reputation(String),

    #[error("insufficient reputation (have: {have}, required: {required})")]
    InsufficientReputation { have: f64, required: f64 },

    // ── Sybil detection errors ────────────────────────────────────
    #[error("sybil detection error: {0}")]
    SybilDetection(String),

    #[error("graph too small for reliable detection (nodes: {size})")]
    GraphTooSmallForDetection { size: usize },

    #[error("entropy calculation failed: {0}")]
    EntropyCalculationFailed(String),

    // ── zk proof errors ───────────────────────────────────────────
    #[error("zk proof error: {0}")]
    ZkProof(String),

    #[error("proof verification failed: {0}")]
    ProofVerificationFailed(String),

    #[error("circuit generation failed: {0}")]
    CircuitGenerationFailed(String),

    // ── Storage / persistence errors ───────────────────────────────
    #[error("storage error: {0}")]
    Storage(String),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("deserialization error: {0}")]
    Deserialization(String),

    // ── Cryptographic errors ──────────────────────────────────────
    #[error("crypto error: {0}")]
    Crypto(String),

    #[error("invalid signature")]
    InvalidSignature,

    #[error("key generation failed: {0}")]
    KeyGenerationFailed(String),

    // ── API / network errors ──────────────────────────────────────
    #[error("API error: {0}")]
    Api(String),

    #[error("network error: {0}")]
    Network(String),

    #[error("timeout")]
    Timeout,

    // ── Configuration errors ──────────────────────────────────────
    #[error("configuration error: {0}")]
    Config(String),

    #[error("invalid parameter: {0}")]
    InvalidParameter(String),

    // ── Internal / unexpected errors ──────────────────────────────
    #[error("internal error: {0}")]
    Internal(String),

    #[error("not implemented: {0}")]
    NotImplemented(String),
}

/// Convenience result alias.
pub type SybilResult<T> = Result<T, SybilError>;