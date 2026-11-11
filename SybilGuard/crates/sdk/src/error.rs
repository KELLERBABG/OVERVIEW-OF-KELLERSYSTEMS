use sybil_core::SybilError;
use thiserror::Error;

/// SDK-specific error type.
#[derive(Error, Debug)]
pub enum SdkError {
    #[error("API error: {0}")]
    Api(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl From<SdkError> for SybilError {
    fn from(e: SdkError) -> Self {
        match e {
            SdkError::Api(msg) => SybilError::Api(msg),
            SdkError::Network(msg) => SybilError::Network(msg),
            SdkError::Serialization(msg) => SybilError::Serialization(msg),
        }
    }
}