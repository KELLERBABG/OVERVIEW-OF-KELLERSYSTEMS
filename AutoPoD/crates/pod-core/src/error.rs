use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Invalid sensor reading: {0}")]
    InvalidReading(String),
    #[error("Time error: {0}")]
    Time(String),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}
