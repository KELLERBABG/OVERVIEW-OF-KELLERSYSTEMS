use crate::proof::verify_settlement_proof;
use crate::types::{SettlementError, SettlementProof, SettlementRecord, SettlementStatus};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

pub struct SettlementEngine {
    pub settlements: HashMap<u64, SettlementRecord>,
}

impl SettlementEngine {
    pub fn new() -> Self {
        Self {
            settlements: HashMap::new(),
        }
    }

    pub fn submit_proof(
        &mut self,
        proof: SettlementProof,
    ) -> Result<SettlementRecord, SettlementError> {
        if self.settlements.contains_key(&proof.order_id) {
            return Err(SettlementError::AlreadySettled);
        }
        verify_settlement_proof(&proof)?;
        let record = SettlementRecord::new(proof);
        self.settlements
            .insert(record.proof.order_id, record.clone());
        Ok(record)
    }

    pub fn settle(&mut self, order_id: u64) -> Result<SettlementRecord, SettlementError> {
        let record = self
            .settlements
            .get_mut(&order_id)
            .ok_or(SettlementError::InvalidProof("order not found".to_string()))?;
        if record.status == SettlementStatus::Settled {
            return Err(SettlementError::AlreadySettled);
        }
        let tx_hash = Self::generate_tx_hash(&record.proof);
        tracing::info!(
            "Settling order {}: tx_hash={}",
            order_id,
            hex::encode(tx_hash)
        );
        record.status = SettlementStatus::Settled;
        record.settled_at = Some(chrono::Utc::now());
        Ok(record.clone())
    }

    pub fn get_status(&self, order_id: u64) -> Option<SettlementStatus> {
        self.settlements.get(&order_id).map(|r| r.status.clone())
    }

    fn generate_tx_hash(proof: &SettlementProof) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"settle:");
        hasher.update(proof.order_id.to_le_bytes());
        hasher.update(proof.match_proof_hex.as_bytes());
        hasher.update(proof.signature_aggregate_hex.as_bytes());
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    pub fn pending_count(&self) -> usize {
        self.settlements
            .values()
            .filter(|r| r.status == SettlementStatus::Pending)
            .count()
    }

    pub fn total_settled(&self) -> usize {
        self.settlements
            .values()
            .filter(|r| r.status == SettlementStatus::Settled)
            .count()
    }
}

impl Default for SettlementEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_proof(order_id: u64, sig_len: usize) -> SettlementProof {
        SettlementProof {
            order_id,
            match_proof_hex: hex::encode([0u8; 32]),
            signature_aggregate_hex: hex::encode(vec![1u8; sig_len]),
            taker_order_id: order_id,
            maker_order_id: order_id + 1,
            price_hex: hex::encode(100u64.to_le_bytes()),
            quantity_hex: hex::encode(10u64.to_le_bytes()),
            timestamp: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_submit_and_settle() {
        let mut engine = SettlementEngine::new();
        let proof = make_proof(1, 64);
        let record = engine.submit_proof(proof).unwrap();
        assert_eq!(record.status, SettlementStatus::Pending);
        let settled = engine.settle(1).unwrap();
        assert_eq!(settled.status, SettlementStatus::Settled);
        assert!(settled.settled_at.is_some());
    }

    #[test]
    fn test_double_settle_fails() {
        let mut engine = SettlementEngine::new();
        let proof = make_proof(1, 64);
        engine.submit_proof(proof).unwrap();
        engine.settle(1).unwrap();
        let result = engine.settle(1);
        assert!(matches!(result, Err(SettlementError::AlreadySettled)));
    }

    #[test]
    fn test_short_sig_rejected() {
        let mut engine = SettlementEngine::new();
        let proof = make_proof(1, 16);
        let result = engine.submit_proof(proof);
        assert!(result.is_err());
    }

    #[test]
    fn test_pending_and_settled_counts() {
        let mut engine = SettlementEngine::new();
        engine.submit_proof(make_proof(1, 64)).unwrap();
        engine.submit_proof(make_proof(2, 64)).unwrap();
        assert_eq!(engine.pending_count(), 2);
        engine.settle(1).unwrap();
        assert_eq!(engine.pending_count(), 1);
        assert_eq!(engine.total_settled(), 1);
    }
}
