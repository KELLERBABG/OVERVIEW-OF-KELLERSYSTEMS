use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SettlementStatus {
    Pending,
    Settled,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementProof {
    pub order_id: u64,
    pub match_proof_hex: String,
    pub signature_aggregate_hex: String,
    pub taker_order_id: u64,
    pub maker_order_id: u64,
    pub price_hex: String,
    pub quantity_hex: String,
    pub timestamp: DateTime<Utc>,
}

impl SettlementProof {
    pub fn new(
        order_id: u64,
        match_proof: &[u8],
        signature_aggregate: &[u8],
        taker_order_id: u64,
        maker_order_id: u64,
        price: &[u8],
        quantity: &[u8],
    ) -> Self {
        Self {
            order_id,
            match_proof_hex: hex::encode(match_proof),
            signature_aggregate_hex: hex::encode(signature_aggregate),
            taker_order_id,
            maker_order_id,
            price_hex: hex::encode(price),
            quantity_hex: hex::encode(quantity),
            timestamp: Utc::now(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SettlementRecord {
    pub proof: SettlementProof,
    pub status: SettlementStatus,
    pub settled_at: Option<DateTime<Utc>>,
}

impl SettlementRecord {
    pub fn new(proof: SettlementProof) -> Self {
        Self {
            proof,
            status: SettlementStatus::Pending,
            settled_at: None,
        }
    }
}

#[derive(Debug, Error)]
pub enum SettlementError {
    #[error("invalid proof: {0}")]
    InvalidProof(String),
    #[error("settlement already finalized")]
    AlreadySettled,
    #[error("threshold signature verification failed")]
    InvalidThresholdSignature,
    #[error("match proof does not match order data")]
    ProofMismatch,
}
