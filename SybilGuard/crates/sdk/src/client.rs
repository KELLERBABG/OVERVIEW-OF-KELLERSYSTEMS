use serde_json::Value;
use sybil_core::{HealthStatus, SybilResult};

/// REST API client for interacting with a Sybil-Resistentes node.
pub struct SybilClient {
    base_url: String,
    client: reqwest::Client,
}

impl SybilClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::Client::new(),
        }
    }

    /// Health check against the API server.
    pub async fn health(&self) -> SybilResult<HealthStatus> {
        let url = format!("{}/health", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| sybil_core::SybilError::Network(e.to_string()))?;
        let status: HealthStatus = resp
            .json()
            .await
            .map_err(|e| sybil_core::SybilError::Api(e.to_string()))?;
        Ok(status)
    }
}