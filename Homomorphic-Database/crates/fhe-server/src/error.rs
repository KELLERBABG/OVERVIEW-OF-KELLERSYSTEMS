//! Errors produced by the HTTP API layer.
//!
//! Every error maps to a **stable, machine-readable `code`** in the JSON
//! response body (`{"code": "...", "message": "..."}`) so clients can branch
//! on the failure class without parsing prose. The `code` strings below are
//! part of the wire contract: they must not change for the same failure.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use fhe_core::ScalarWidth;
use serde_json::json;

/// Errors that can occur while serving HTTP requests.
///
/// Each variant carries a stable [`ServerError::code`] used as the
/// machine-readable `code` field of the JSON error body.
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    /// The request body failed to deserialize or was semantically invalid in
    /// a way that has no more specific variant.
    #[error("bad request: {0}")]
    BadRequest(String),

    /// A base64 string on the wire could not be decoded.
    #[error("invalid base64: {0}")]
    InvalidBase64(String),

    /// A base64-decoded payload is not a valid ciphertext envelope.
    #[error("invalid ciphertext envelope: {0}")]
    InvalidEnvelope(String),

    /// A homomorphic operation failed at evaluation time.
    #[error("homomorphic evaluation failed: {0}")]
    HomomorphicEval(String),

    /// The request exceeds a configured size or work guard (DoS protection).
    #[error("request too large: {0}")]
    PayloadTooLarge(String),

    /// The client exceeded the per-IP request quota (DoS protection).
    #[error("rate limit exceeded: {0}")]
    RateLimited(String),

    /// The request did not carry a valid bearer token (authentication).
    ///
    /// Only reachable when the server was started with `--auth-token` (or
    /// `FHE_DB_AUTH_TOKEN`); without auth configured every request is
    /// accepted, preserving the loopback-only default posture.
    #[error("missing or invalid bearer token")]
    Unauthorized,

    /// A schema (table/column name, column count, ...) violates the limits.
    #[error("invalid schema: {0}")]
    SchemaInvalid(String),

    /// The referenced resource does not exist (fallback; prefer the typed
    /// variants below).
    #[error("not found: {0}")]
    NotFound(String),

    /// The referenced table does not exist.
    #[error("unknown table `{0}`")]
    UnknownTable(String),

    /// The referenced column does not exist in the table schema.
    #[error("unknown column `{column}` in table `{table}`")]
    UnknownColumn { table: String, column: String },

    /// A table with the same name already exists.
    #[error("table `{0}` already exists")]
    TableExists(String),

    /// A column with the same name already exists in the table schema.
    #[error("column `{0}` already exists")]
    ColumnExists(String),

    /// Ciphertext width does not match the column's declared type.
    #[error("column `{column}` expects {expected:?}, got {found:?}")]
    WidthMismatch {
        column: String,
        expected: ScalarWidth,
        found: ScalarWidth,
    },

    /// An operation was attempted on an empty column.
    #[error("column `{0}` is empty")]
    EmptyColumn(String),

    /// A homomorphic operation was attempted before a server key was
    /// installed.
    #[error("no server key installed; POST /keys/server first")]
    NoServerKey,

    /// An inference was attempted before an encrypted model was installed.
    #[error("no XNOR model installed; POST /models/xnor first")]
    NoModel,

    /// The on-disk database file does not match the expected format.
    #[error("corrupted database: {0}")]
    CorruptedDatabase(String),

    /// I/O failure while reading or writing the database file.
    #[error("I/O error: {0}")]
    Io(String),

    /// An FHE-layer error propagated from `fhe-core`.
    #[error("FHE error: {0}")]
    Fhe(#[from] fhe_core::FheError),

    /// A serialization failure (bincode/JSON) on data the server produced or
    /// received.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// An unexpected internal error with no more specific variant.
    #[error("internal error: {0}")]
    Internal(String),
}

impl ServerError {
    /// The stable machine-readable code for this error. This string is part
    /// of the wire contract — do not change it for the same failure class.
    pub fn code(&self) -> &'static str {
        match self {
            ServerError::BadRequest(_) => "BAD_REQUEST",
            ServerError::InvalidBase64(_) => "INVALID_BASE64",
            ServerError::InvalidEnvelope(_) => "INVALID_ENVELOPE",
            ServerError::HomomorphicEval(_) => "HOMOMORPHIC_EVAL",
            ServerError::PayloadTooLarge(_) => "PAYLOAD_TOO_LARGE",
            ServerError::RateLimited(_) => "RATE_LIMITED",
            ServerError::Unauthorized => "UNAUTHORIZED",
            ServerError::SchemaInvalid(_) => "SCHEMA_INVALID",
            ServerError::NotFound(_) => "NOT_FOUND",
            ServerError::UnknownTable(_) => "UNKNOWN_TABLE",
            ServerError::UnknownColumn { .. } => "UNKNOWN_COLUMN",
            ServerError::TableExists(_) => "TABLE_EXISTS",
            ServerError::ColumnExists(_) => "COLUMN_EXISTS",
            ServerError::WidthMismatch { .. } => "WIDTH_MISMATCH",
            ServerError::EmptyColumn(_) => "EMPTY_COLUMN",
            ServerError::NoServerKey => "NO_SERVER_KEY",
            ServerError::NoModel => "NO_MODEL",
            ServerError::CorruptedDatabase(_) => "CORRUPTED_DATABASE",
            ServerError::Io(_) => "IO_ERROR",
            ServerError::Fhe(_) => "FHE_ERROR",
            ServerError::Serialization(_) => "SERIALIZATION_ERROR",
            ServerError::Internal(_) => "INTERNAL",
        }
    }

    /// The HTTP status for this error.
    pub fn status(&self) -> StatusCode {
        match self {
            ServerError::BadRequest(_)
            | ServerError::InvalidBase64(_)
            | ServerError::InvalidEnvelope(_)
            | ServerError::HomomorphicEval(_)
            | ServerError::SchemaInvalid(_)
            | ServerError::WidthMismatch { .. }
            | ServerError::EmptyColumn(_) => StatusCode::BAD_REQUEST,
            ServerError::PayloadTooLarge(_) => StatusCode::PAYLOAD_TOO_LARGE,
            ServerError::RateLimited(_) => StatusCode::TOO_MANY_REQUESTS,
            ServerError::Unauthorized => StatusCode::UNAUTHORIZED,
            ServerError::NotFound(_)
            | ServerError::UnknownTable(_)
            | ServerError::UnknownColumn { .. } => StatusCode::NOT_FOUND,
            ServerError::TableExists(_)
            | ServerError::ColumnExists(_)
            | ServerError::NoServerKey
            | ServerError::NoModel => StatusCode::CONFLICT,
            ServerError::CorruptedDatabase(_)
            | ServerError::Io(_)
            | ServerError::Fhe(_)
            | ServerError::Serialization(_)
            | ServerError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Map an error into a JSON response with the appropriate HTTP status.
    /// The body always carries the stable `code` plus a human `message`.
    pub fn into_response(self) -> Response {
        let status = self.status();
        let body = Json(json!({
            "code": self.code(),
            "message": self.to_string(),
        }));
        (status, body).into_response()
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        self.into_response()
    }
}

impl From<fhe_db::DbError> for ServerError {
    fn from(e: fhe_db::DbError) -> Self {
        use fhe_db::DbError::*;
        match e {
            UnknownTable(t) => ServerError::UnknownTable(t),
            TableExists(t) => ServerError::TableExists(t),
            UnknownColumn { table, column } => ServerError::UnknownColumn { table, column },
            ColumnExists(c) => ServerError::ColumnExists(c),
            ColumnWidthMismatch {
                column,
                expected,
                found,
            } => ServerError::WidthMismatch {
                column,
                expected,
                found,
            },
            EmptyColumn(c) => ServerError::EmptyColumn(c),
            CorruptedDatabase(m) => ServerError::CorruptedDatabase(m),
            Io(m) => ServerError::Io(m),
            Fhe(e) => ServerError::Fhe(e),
        }
    }
}

impl From<bincode::Error> for ServerError {
    fn from(e: bincode::Error) -> Self {
        ServerError::Serialization(e.to_string())
    }
}

/// Result alias used by every HTTP handler.
pub type ServerResult<T> = Result<T, ServerError>;
