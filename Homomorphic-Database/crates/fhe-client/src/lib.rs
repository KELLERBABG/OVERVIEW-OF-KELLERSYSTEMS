//! # fhe-client — FHE-DB Client SDK
//!
//! A small async HTTP client for the FHE-DB zero-trust flow:
//!
//! 1. Generate a `(ClientKey, ServerKey)` with [`fhe_core::ClientKey::new`].
//! 2. Upload the **public** [`fhe_core::ServerKey`] to the server.
//! 3. Encrypt values locally with [`fhe_core::Encryptor`].
//! 4. Insert ciphertexts and run homomorphic aggregations with [`FheClient`].
//! 5. Decrypt results with the [`fhe_core::ClientKey`].
//!
//! The secret client key never leaves the process, matching the zero-trust
//! security model.

pub mod client;

pub use client::{
    AggregateResponse, ColumnDefJson, ColumnTypeJson, CountResponse, FheClient, InsertResponse,
    TableSchemaJson,
};

/// Errors that can occur while using the client SDK.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// The base URL is invalid.
    #[error("invalid URL: {0}")]
    Url(String),

    /// A transport-level failure (connection refused, timeout, ...).
    #[error("HTTP transport error: {0}")]
    Http(#[from] reqwest::Error),

    /// The server returned a non-success status with a JSON error body.
    ///
    /// `code` is the server's stable machine-readable error code (e.g.
    /// `UNKNOWN_TABLE`, `PAYLOAD_TOO_LARGE`), when the body carried one.
    #[error("server error {status}{}: {message}", code.as_ref().map(|c| format!(" [{c}]")).unwrap_or_default())]
    Server {
        /// HTTP status code.
        status: u16,
        /// Stable machine-readable error code from the server, if present.
        code: Option<String>,
        /// Human-readable error message.
        message: String,
    },

    /// The server's response could not be deserialized or was malformed.
    #[error("protocol error: {0}")]
    Protocol(String),

    /// A JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// An error from the FHE core layer.
    #[error("FHE error: {0}")]
    Fhe(#[from] fhe_core::FheError),
}

/// Convenience result alias for the client SDK.
pub type ClientResult<T> = Result<T, ClientError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_base_url() {
        assert!(matches!(
            FheClient::new("not a url"),
            Err(ClientError::Url(_))
        ));
    }

    #[test]
    fn accepts_valid_http_url() {
        let client = FheClient::new("http://127.0.0.1:8080").unwrap();
        assert_eq!(client.base_url(), "http://127.0.0.1:8080");
    }

    #[test]
    fn with_auth_accepts_valid_token() {
        let client = FheClient::with_auth("http://127.0.0.1:8080", "tok-123").unwrap();
        assert_eq!(client.base_url(), "http://127.0.0.1:8080");
    }

    #[test]
    fn with_auth_rejects_invalid_header_token() {
        // Header values may not contain control characters; the token must be
        // rejected at construction time, not at request time.
        assert!(matches!(
            FheClient::with_auth("http://127.0.0.1:8080", "bad\ntoken"),
            Err(ClientError::Url(_))
        ));
    }

    #[test]
    fn deserializes_aggregate_response() {
        let json = r#"{"sum":"abc","count":3}"#;
        let parsed: AggregateResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.count, 3);
        assert_eq!(parsed.sum, "abc");
    }

    #[test]
    fn deserializes_count_response() {
        let json = r#"{"count":42}"#;
        let parsed: CountResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.count, 42);
    }

    #[test]
    fn deserializes_insert_response() {
        let json = r#"{"inserted":7}"#;
        let parsed: InsertResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.inserted, 7);
    }

    #[test]
    fn deserializes_schema_json() {
        let json = r#"{"name":"t","columns":[{"name":"c","column_type":"U32"}]}"#;
        let parsed: TableSchemaJson = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.name, "t");
        assert_eq!(parsed.columns.len(), 1);
        assert_eq!(parsed.columns[0].name, "c");
        assert_eq!(parsed.columns[0].column_type, ColumnTypeJson::U32);
    }

    #[test]
    fn column_type_roundtrips_through_serde() {
        let json = serde_json::to_string(&ColumnTypeJson::U16).unwrap();
        assert_eq!(json, "\"U16\"");
        let back: ColumnTypeJson = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ColumnTypeJson::U16);
    }

    #[test]
    fn error_maps_http_to_server_error() {
        let err = ClientError::Server {
            status: 409,
            code: Some("TABLE_EXISTS".into()),
            message: "table `t` already exists".into(),
        };
        assert_eq!(
            err.to_string(),
            "server error 409 [TABLE_EXISTS]: table `t` already exists"
        );
        // The code is optional: bodies without one still format cleanly.
        let legacy = ClientError::Server {
            status: 500,
            code: None,
            message: "boom".into(),
        };
        assert_eq!(legacy.to_string(), "server error 500: boom");
    }
}
