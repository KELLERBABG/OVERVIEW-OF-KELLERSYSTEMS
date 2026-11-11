use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderStatus {
    Committed,
    Revealed,
    Executed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct EncryptedOrder {
    pub encrypted_payload: Vec<u8>,
    pub commitment_hash: [u8; 32],
    pub submitter_id: u64,
    pub timestamp: u64,
}

impl EncryptedOrder {
    pub fn new(
        encrypted_payload: Vec<u8>,
        commitment_hash: [u8; 32],
        submitter_id: u64,
        timestamp: u64,
    ) -> Self {
        Self {
            encrypted_payload,
            commitment_hash,
            submitter_id,
            timestamp,
        }
    }
}

#[derive(Debug, Error)]
pub enum FrontRunningError {
    #[error("invalid commitment: commitment does not match revealed data")]
    InvalidCommitment,
    #[error("order already revealed")]
    AlreadyRevealed,
    #[error("order not found in mempool")]
    OrderNotFound,
}
