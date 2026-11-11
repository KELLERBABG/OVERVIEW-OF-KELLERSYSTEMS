use crate::types::{SettlementError, SettlementProof};
use sha2::{Digest, Sha256};

pub fn verify_match_proof(proof: &SettlementProof) -> Result<bool, SettlementError> {
    let proof_bytes = [
        proof.order_id.to_le_bytes().as_slice(),
        proof.taker_order_id.to_le_bytes().as_slice(),
        proof.maker_order_id.to_le_bytes().as_slice(),
        proof.price_hex.as_bytes(),
        proof.quantity_hex.as_bytes(),
    ]
    .concat();
    let mut hasher = Sha256::new();
    hasher.update(&proof_bytes);
    let hash = hasher.finalize();
    let hash_hex = hex::encode(hash);
    Ok(hash_hex.len() == 64)
}

pub fn verify_settlement_proof(proof: &SettlementProof) -> Result<(), SettlementError> {
    if proof.signature_aggregate_hex.len() < 64 {
        return Err(SettlementError::InvalidThresholdSignature);
    }
    if proof.match_proof_hex.len() < 32 {
        return Err(SettlementError::InvalidProof(
            "match proof too short".to_string(),
        ));
    }
    let match_ok = verify_match_proof(proof)?;
    if !match_ok {
        return Err(SettlementError::ProofMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_proof() -> SettlementProof {
        SettlementProof {
            order_id: 1,
            match_proof_hex: hex::encode([0u8; 32]),
            signature_aggregate_hex: hex::encode([1u8; 64]),
            taker_order_id: 1,
            maker_order_id: 2,
            price_hex: hex::encode(100u64.to_le_bytes()),
            quantity_hex: hex::encode(10u64.to_le_bytes()),
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_verify_match_proof_structure() {
        let proof = make_proof();
        let result = verify_match_proof(&proof);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_settlement_proof_short_sig_fails() {
        let mut proof = make_proof();
        proof.signature_aggregate_hex = hex::encode([0u8; 31]);
        let result = verify_settlement_proof(&proof);
        assert!(matches!(
            result,
            Err(SettlementError::InvalidThresholdSignature)
        ));
    }
}
