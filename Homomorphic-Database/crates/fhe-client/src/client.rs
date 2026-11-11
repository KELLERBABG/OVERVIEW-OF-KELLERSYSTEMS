//! HTTP transport for the FHE-DB REST API.
//!
//! [`FheClient`] wraps `reqwest` and the `fhe-core` types so application code
//! can run the full zero-trust flow: generate keys, upload the public server
//! key, push encrypted columns, run homomorphic aggregations, and decrypt
//! results.

use base64::Engine;
use serde::{Deserialize, Serialize};

use fhe_core::{Ciphertext, ClientKey, Encryptor, ScalarWidth, ServerKey};

use crate::ClientError;

/// Base64 engine used for all envelope encoding on the wire.
const B64: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;

/// Column type as it appears in the server's JSON schema API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ColumnTypeJson {
    /// 8-bit unsigned integer
    U8,
    /// 16-bit unsigned integer
    U16,
    /// 32-bit unsigned integer
    U32,
    /// 64-bit unsigned integer
    U64,
}

impl From<ScalarWidth> for ColumnTypeJson {
    fn from(w: ScalarWidth) -> Self {
        match w {
            ScalarWidth::U8 => ColumnTypeJson::U8,
            ScalarWidth::U16 => ColumnTypeJson::U16,
            ScalarWidth::U32 => ColumnTypeJson::U32,
            ScalarWidth::U64 => ColumnTypeJson::U64,
        }
    }
}

/// A column definition in a table schema request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDefJson {
    /// Column name (plaintext metadata).
    pub name: String,
    /// Column value type.
    pub column_type: ColumnTypeJson,
}

/// A table schema as returned by `POST /tables` / `GET /tables`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchemaJson {
    /// Table name.
    pub name: String,
    /// Column definitions.
    pub columns: Vec<ColumnDefJson>,
}

/// Response body of a SUM/AVG aggregation.
#[derive(Debug, Clone, Deserialize)]
pub struct AggregateResponse {
    /// Base64-encoded [`Ciphertext`] envelope of the encrypted aggregate.
    pub sum: String,
    /// Row count (public metadata).
    pub count: u64,
}

/// Response body of a COUNT aggregation.
#[derive(Debug, Clone, Deserialize)]
pub struct CountResponse {
    /// Row count.
    pub count: u64,
}

/// Response body of an insert.
#[derive(Debug, Clone, Deserialize)]
pub struct InsertResponse {
    /// Number of rows inserted.
    pub inserted: usize,
}

/// An HTTP client for the FHE-DB server.
///
/// All methods are async and talk to the server over HTTP. The client holds
/// no secret key state — callers pass the [`ClientKey`] explicitly to
/// encrypt/decrypt methods. This keeps the SDK lightweight and forces the
/// key-handling policy to stay in the application.
#[derive(Debug, Clone)]
pub struct FheClient {
    /// Underlying HTTP client.
    http: reqwest::Client,
    /// Base URL of the server, e.g. `http://127.0.0.1:8080`.
    base_url: String,
}

impl FheClient {
    /// Create a client pointing at a server.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::Url`] if `base_url` cannot be parsed as a URL.
    pub fn new(base_url: impl Into<String>) -> Result<Self, ClientError> {
        // An empty token means "no auth header", identical to the open
        // default posture.
        Self::with_auth(base_url, "")
    }

    /// Create a client pointing at a server, sending
    /// `Authorization: Bearer <token>` on every request.
    ///
    /// Use this when the server was started with `--auth-token` (or
    /// `FHE_DB_AUTH_TOKEN`). An empty `token` behaves exactly like
    /// [`FheClient::new`] (no header).
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::Url`] if `base_url` cannot be parsed as a URL
    /// or `token` contains characters that cannot appear in an HTTP header
    /// value.
    pub fn with_auth(
        base_url: impl Into<String>,
        token: impl Into<String>,
    ) -> Result<Self, ClientError> {
        let base_url = base_url.into();
        // Validate the base URL early so later calls fail with clear errors.
        let _ = reqwest::Url::parse(&base_url)
            .map_err(|e| ClientError::Url(format!("invalid base URL `{base_url}`: {e}")))?;

        let mut builder = reqwest::Client::builder();
        let token = token.into();
        if !token.is_empty() {
            let mut headers = reqwest::header::HeaderMap::new();
            let value = reqwest::header::HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(|e| ClientError::Url(format!("invalid auth token: {e}")))?;
            headers.insert(reqwest::header::AUTHORIZATION, value);
            builder = builder.default_headers(headers);
        }
        Ok(Self {
            http: builder.build()?,
            base_url,
        })
    }

    /// The configured base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Health check. Returns `true` if the server responds `200 OK`.
    pub async fn is_healthy(&self) -> bool {
        self.http
            .get(format!("{}/health", self.base_url))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Upload the public server key to the compute server.
    ///
    /// The server key is the **only** key material that leaves the client.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::Http`] on transport failures and
    /// [`ClientError::Server`] if the server rejects the key.
    pub async fn upload_server_key(&self, server_key: &ServerKey) -> Result<(), ClientError> {
        let bytes = server_key.to_bytes()?;
        let response = self
            .http
            .post(format!("{}/keys/server", self.base_url))
            .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
            .body(bytes)
            .send()
            .await?;
        check_status(response, &["204"]).await?;
        Ok(())
    }

    /// Create a table from a schema.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::Server`] if the table already exists or the
    /// schema is invalid.
    pub async fn create_table(
        &self,
        schema: &TableSchemaJson,
    ) -> Result<TableSchemaJson, ClientError> {
        let response = self
            .http
            .post(format!("{}/tables", self.base_url))
            .json(schema)
            .send()
            .await?;
        let body = check_status(response, &["200"]).await?;
        Ok(serde_json::from_slice(&body)?)
    }

    /// List all table schemas.
    pub async fn list_tables(&self) -> Result<Vec<TableSchemaJson>, ClientError> {
        let response = self
            .http
            .get(format!("{}/tables", self.base_url))
            .send()
            .await?;
        let body = check_status(response, &["200"]).await?;
        Ok(serde_json::from_slice(&body)?)
    }

    /// Add a column to an existing table.
    pub async fn add_column(
        &self,
        table: &str,
        column: &ColumnDefJson,
    ) -> Result<TableSchemaJson, ClientError> {
        let response = self
            .http
            .post(format!("{}/tables/{table}/columns", self.base_url))
            .json(column)
            .send()
            .await?;
        let body = check_status(response, &["200"]).await?;
        Ok(serde_json::from_slice(&body)?)
    }

    /// Insert ciphertexts into a column.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::Server`] if the table/column does not exist or
    /// any ciphertext width mismatches the column type.
    pub async fn insert_ciphertexts(
        &self,
        table: &str,
        column: &str,
        ciphertexts: Vec<Ciphertext>,
    ) -> Result<usize, ClientError> {
        let mut encoded = Vec::with_capacity(ciphertexts.len());
        for ct in &ciphertexts {
            let bytes = ct.to_envelope_bytes()?;
            encoded.push(B64.encode(bytes));
        }
        let response = self
            .http
            .post(format!("{}/tables/{table}/rows", self.base_url))
            .json(&serde_json::json!({
                "column": column,
                "ciphertexts": encoded,
            }))
            .send()
            .await?;
        let body = check_status(response, &["200"]).await?;
        let parsed: InsertResponse = serde_json::from_slice(&body)?;
        Ok(parsed.inserted)
    }

    /// Encrypt plaintext values locally and insert them.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::Fhe`] if a value does not fit the width.
    pub async fn insert_plaintext(
        &self,
        client_key: &ClientKey,
        table: &str,
        column: &str,
        width: ScalarWidth,
        values: &[u64],
    ) -> Result<usize, ClientError> {
        let encryptor = Encryptor::new(client_key);
        let ciphertexts: Vec<Ciphertext> = values
            .iter()
            .map(|v| encryptor.encrypt_u64(*v, width))
            .collect::<Result<_, _>>()?;
        self.insert_ciphertexts(table, column, ciphertexts).await
    }

    /// Homomorphic SUM over a column.
    ///
    /// Returns the encrypted sum and the public row count. The caller
    /// decrypts with its [`ClientKey`].
    pub async fn sum(&self, table: &str, column: &str) -> Result<(Ciphertext, u64), ClientError> {
        self.aggregate("/sum", table, column).await
    }

    /// Homomorphic AVG over a column.
    ///
    /// Returns the encrypted sum and the public row count; divide after
    /// decryption to obtain the average.
    pub async fn avg(&self, table: &str, column: &str) -> Result<(Ciphertext, u64), ClientError> {
        self.aggregate("/avg", table, column).await
    }

    /// Public COUNT over a column.
    pub async fn count(&self, table: &str, column: &str) -> Result<u64, ClientError> {
        let response = self
            .http
            .post(format!(
                "{}/tables/{table}/aggregations/count",
                self.base_url
            ))
            .json(&serde_json::json!({ "column": column }))
            .send()
            .await?;
        let body = check_status(response, &["200"]).await?;
        let parsed: CountResponse = serde_json::from_slice(&body)?;
        Ok(parsed.count)
    }

    /// Read back the stored ciphertexts of a column.
    ///
    /// Returns the stored ciphertexts in insertion order. This is the
    /// zero-trust read-back path: the server returns only envelopes, which
    /// the caller decrypts locally with its [`ClientKey`].
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::Server`] if the table/column does not exist.
    pub async fn get_ciphertexts(
        &self,
        table: &str,
        column: &str,
    ) -> Result<Vec<Ciphertext>, ClientError> {
        let response = self
            .http
            .get(format!(
                "{}/tables/{table}/ciphertexts/{column}",
                self.base_url
            ))
            .send()
            .await?;
        let body = check_status(response, &["200"]).await?;
        let parsed: serde_json::Value = serde_json::from_slice(&body)?;
        let envelopes = parsed
            .get("ciphertexts")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                ClientError::Protocol("missing `ciphertexts` array in read-back response".into())
            })?;
        let mut ciphertexts = Vec::with_capacity(envelopes.len());
        for encoded in envelopes {
            let encoded = encoded.as_str().ok_or_else(|| {
                ClientError::Protocol("non-string ciphertext entry in read-back response".into())
            })?;
            let bytes = B64.decode(encoded).map_err(|e| {
                ClientError::Protocol(format!("invalid base64 in read-back response: {e}"))
            })?;
            ciphertexts.push(Ciphertext::from_envelope_bytes(&bytes)?);
        }
        Ok(ciphertexts)
    }

    /// Install the encrypted XNOR model weights.
    ///
    /// `weights` must contain exactly two encrypted values (`w0`, `w1`) and
    /// `bias` one signed bias per output neuron in the fixed-point scale.
    /// Weight values leave the client only as ciphertexts; the server does
    /// receive the plaintext `bias` and the layer shape.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::Server`] if the server rejects the model.
    pub async fn install_xnor_model(
        &self,
        weights: Vec<Ciphertext>,
        bias: Vec<i64>,
    ) -> Result<(), ClientError> {
        let mut encoded = Vec::with_capacity(weights.len());
        for w in &weights {
            encoded.push(B64.encode(w.to_envelope_bytes()?));
        }
        let response = self
            .http
            .post(format!("{}/models/xnor", self.base_url))
            .json(&serde_json::json!({
                "weights": encoded,
                "bias": bias,
            }))
            .send()
            .await?;
        check_status(response, &["204"]).await?;
        Ok(())
    }

    /// Run encrypted XNOR inference over two encrypted inputs.
    ///
    /// The server evaluates `(x0 + x1 - 1)^2` homomorphically and returns the
    /// encrypted output; only the caller can decrypt it.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::Server`] if no model is installed or the inputs
    /// are malformed.
    pub async fn infer_xnor(
        &self,
        inputs: (Ciphertext, Ciphertext),
    ) -> Result<Ciphertext, ClientError> {
        let mut encoded = Vec::with_capacity(2);
        for x in [inputs.0, inputs.1] {
            encoded.push(B64.encode(x.to_envelope_bytes()?));
        }
        let response = self
            .http
            .post(format!("{}/models/xnor/infer", self.base_url))
            .json(&serde_json::json!({ "inputs": encoded }))
            .send()
            .await?;
        let body = check_status(response, &["200"]).await?;
        let parsed: serde_json::Value = serde_json::from_slice(&body)?;
        let output = parsed
            .get("output")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ClientError::Protocol("missing `output` in XNOR inference response".into())
            })?;
        let bytes = B64
            .decode(output)
            .map_err(|e| ClientError::Protocol(format!("invalid base64 in XNOR response: {e}")))?;
        Ok(Ciphertext::from_envelope_bytes(&bytes)?)
    }

    /// Decrypted SUM over a column.
    pub async fn sum_plaintext(
        &self,
        client_key: &ClientKey,
        table: &str,
        column: &str,
    ) -> Result<u64, ClientError> {
        let (ct, _count) = self.sum(table, column).await?;
        Ok(ct.decrypt(client_key).as_u64())
    }

    /// Decrypted AVG over a column (client-side division by the public
    /// count).
    pub async fn avg_plaintext(
        &self,
        client_key: &ClientKey,
        table: &str,
        column: &str,
    ) -> Result<u64, ClientError> {
        let (ct, count) = self.avg(table, column).await?;
        let sum = ct.decrypt(client_key).as_u64();
        Ok(sum / count)
    }

    /// Shared implementation for `sum`/`avg`.
    async fn aggregate(
        &self,
        endpoint: &str,
        table: &str,
        column: &str,
    ) -> Result<(Ciphertext, u64), ClientError> {
        let response = self
            .http
            .post(format!(
                "{}/tables/{table}/aggregations{endpoint}",
                self.base_url
            ))
            .json(&serde_json::json!({ "column": column }))
            .send()
            .await?;
        let body = check_status(response, &["200"]).await?;
        let parsed: AggregateResponse = serde_json::from_slice(&body)?;
        let bytes = B64.decode(&parsed.sum).map_err(|e| {
            ClientError::Protocol(format!("invalid base64 in aggregate response: {e}"))
        })?;
        let ct = Ciphertext::from_envelope_bytes(&bytes)?;
        Ok((ct, parsed.count))
    }
}

/// Check the response status code against the accepted set, returning the
/// body bytes on success or a structured [`ClientError::Server`] on failure.
async fn check_status(
    response: reqwest::Response,
    accepted: &[&str],
) -> Result<Vec<u8>, ClientError> {
    let status = response.status();
    let body = response.bytes().await?;
    if accepted.contains(&status.as_str()) {
        return Ok(body.to_vec());
    }
    // Try to extract a server-provided message and stable error code.
    let parsed = serde_json::from_slice::<serde_json::Value>(&body).ok();
    let message = parsed
        .as_ref()
        .and_then(|v| {
            v.get("message")
                .and_then(|m| m.as_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "no error message".to_string());
    let code = parsed
        .as_ref()
        .and_then(|v| v.get("code").and_then(|c| c.as_str()).map(str::to_string));
    Err(ClientError::Server {
        status: status.as_u16(),
        code,
        message,
    })
}
