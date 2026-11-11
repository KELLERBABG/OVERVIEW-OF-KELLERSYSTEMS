//! Error types for the FHE primitives layer.

/// Errors that can occur in the FHE primitives layer.
///
/// These are deliberately structured so that a caller can distinguish
/// between:
///
/// - **Parametric errors** — the request (e.g. a width mismatch) is
///   fundamentally invalid and should be rejected before any computation.
/// - **Operational errors** — the underlying TFHE library rejected an
///   operation (e.g. a deserialized ciphertext failed its sanity check).
/// - **Serialization errors** — data on the wire or at rest is corrupt or
///   from an incompatible version.
#[derive(Debug, thiserror::Error)]
pub enum FheError {
    /// The plaintext value does not fit in the selected arithmetic width.
    #[error("value {value} does not fit in {width}-bit unsigned integer (max {max})")]
    ValueOutOfRange { value: u64, width: usize, max: u64 },

    /// The ciphertext was created with a different arithmetic width than the
    /// operation or decryptor expects.
    #[error("ciphertext width mismatch: expected {expected}, found {found}")]
    WidthMismatch { expected: usize, found: usize },

    /// An operation was attempted before a server key was installed.
    #[error("no server key installed; call set_server_key before computations")]
    MissingServerKey,

    /// An aggregation was attempted on an empty column.
    #[error("cannot aggregate an empty column")]
    EmptyColumn,

    /// A layer operation was given an empty slice of ciphertexts.
    #[error("operation requires at least one ciphertext")]
    EmptySlice,

    /// Two slices given to a layer operation have different lengths.
    #[error("length mismatch: expected {expected}, found {found}")]
    LengthMismatch { expected: usize, found: usize },

    /// An operation was attempted that is not supported for this ciphertext
    /// width.
    #[error("operation not supported for ciphertext width {0} bits")]
    UnsupportedOperation(usize),

    /// The underlying TFHE library rejected an operation.
    #[error("TFHE error: {0}")]
    Tfhe(String),

    /// A serialization/deserialization failure.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// A ciphertext envelope version is not recognized.
    #[error("unknown ciphertext envelope version {0}")]
    UnknownEnvelopeVersion(u8),

    /// A ciphertext envelope is malformed.
    #[error("malformed ciphertext envelope")]
    MalformedEnvelope,

    /// A ciphertext failed TFHE's own deserialization checks.
    #[error("corrupted ciphertext: {0}")]
    CorruptedCiphertext(String),

    /// A key failed TFHE's own deserialization checks.
    #[error("corrupted key: {0}")]
    CorruptedKey(String),
}

impl From<tfhe::Error> for FheError {
    fn from(e: tfhe::Error) -> Self {
        FheError::Tfhe(e.to_string())
    }
}

impl From<bincode::Error> for FheError {
    fn from(e: bincode::Error) -> Self {
        FheError::Serialization(e.to_string())
    }
}

/// Convenience result alias for the whole crate.
pub type FheResult<T> = Result<T, FheError>;
