use crate::types::{SettlementError, SettlementProof};
use serde::{Deserialize, Serialize};
use serde_json;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractCall {
    pub function: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractEvent {
    pub name: String,
    pub data: String,
    pub block_number: u64,
    pub tx_hash: String,
}

pub fn encode_settle_call(proof: &SettlementProof) -> ContractCall {
    ContractCall {
        function: "settleMatch".to_string(),
        args: vec![
            proof.order_id.to_string(),
            proof.taker_order_id.to_string(),
            proof.maker_order_id.to_string(),
            format!("0x{}", proof.match_proof_hex),
            format!("0x{}", proof.signature_aggregate_hex),
            format!("0x{}", proof.price_hex),
            format!("0x{}", proof.quantity_hex),
        ],
    }
}

pub fn encode_verify_proof_call(proof: &SettlementProof) -> ContractCall {
    ContractCall {
        function: "verifyMatchProof".to_string(),
        args: vec![
            format!("0x{}", proof.match_proof_hex),
            format!("0x{}", proof.price_hex),
            format!("0x{}", proof.quantity_hex),
        ],
    }
}

pub fn simulate_settle(proof: &SettlementProof) -> Result<ContractEvent, SettlementError> {
    let mut hasher = Sha256::new();
    hasher.update(b"settle:");
    hasher.update(proof.order_id.to_le_bytes());
    let tx_hash = hex::encode(hasher.finalize());
    Ok(ContractEvent {
        name: "MatchSettled".to_string(),
        data: serde_json::to_string(proof).unwrap_or_default(),
        block_number: 1,
        tx_hash,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_proof() -> SettlementProof {
        SettlementProof {
            order_id: 42,
            match_proof_hex: "abcd".to_string(),
            signature_aggregate_hex: hex::encode([1u8; 64]),
            taker_order_id: 1,
            maker_order_id: 2,
            price_hex: "64".to_string(),
            quantity_hex: "0a".to_string(),
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_encode_settle_call() {
        let proof = make_proof();
        let call = encode_settle_call(&proof);
        assert_eq!(call.function, "settleMatch");
        assert_eq!(call.args.len(), 7);
    }

    #[test]
    fn test_simulate_settle() {
        let proof = make_proof();
        let event = simulate_settle(&proof).unwrap();
        assert_eq!(event.name, "MatchSettled");
    }
}
