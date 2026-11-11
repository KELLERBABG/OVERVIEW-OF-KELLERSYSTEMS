use ed25519_dalek::Verifier;
use mesh_graph_core::types::{NodeId, RelationshipType, TrustWeight};
use serde::{Deserialize, Serialize};

use crate::identity::Identity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VouchStatement {
    pub voucher_id: NodeId,
    pub target_id: NodeId,
    pub relationship: RelationshipType,
    pub trust_weight: TrustWeight,
    pub timestamp: i64,
    pub signature_bytes: Vec<u8>,
}

impl VouchStatement {
    pub fn create(
        voucher: &Identity,
        target_id: &str,
        relationship: RelationshipType,
        trust_weight: TrustWeight,
    ) -> Result<Self, String> {
        if trust_weight <= 0.0 || trust_weight > 1.0 || !trust_weight.is_finite() {
            return Err("trust weight must be a finite value in (0.0, 1.0]".to_string());
        }
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let message = Self::canonical_message(
            &voucher.id,
            target_id,
            &relationship,
            trust_weight,
            timestamp,
        );
        let signature_bytes = voucher.sign(&message)?;

        Ok(VouchStatement {
            voucher_id: voucher.id.clone(),
            target_id: target_id.to_string(),
            relationship,
            trust_weight,
            timestamp,
            signature_bytes,
        })
    }

    pub fn verify(&self, voucher_public_key_bytes: &[u8]) -> Result<bool, String> {
        let message = Self::canonical_message(
            &self.voucher_id,
            &self.target_id,
            &self.relationship,
            self.trust_weight,
            self.timestamp,
        );

        let mut pk_bytes = [0u8; 32];
        if voucher_public_key_bytes.len() != 32 {
            return Err("invalid public key length".to_string());
        }
        pk_bytes.copy_from_slice(voucher_public_key_bytes);

        let vk = ed25519_dalek::VerifyingKey::from_bytes(&pk_bytes)
            .map_err(|e| format!("invalid public key: {}", e))?;

        let mut sig_bytes = [0u8; 64];
        if self.signature_bytes.len() != 64 {
            return Err("invalid signature length".to_string());
        }
        sig_bytes.copy_from_slice(&self.signature_bytes);
        let signature = ed25519_dalek::Signature::from_bytes(&sig_bytes);

        match vk.verify(&message, &signature) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn canonical_message(
        voucher_id: &str,
        target_id: &str,
        relationship: &RelationshipType,
        trust_weight: TrustWeight,
        timestamp: i64,
    ) -> Vec<u8> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(b"VOUCH:v1:");
        hasher.update(voucher_id.as_bytes());
        hasher.update(b":");
        hasher.update(target_id.as_bytes());
        hasher.update(b":");
        hasher.update(relationship.to_string().as_bytes());
        hasher.update(b":");
        hasher.update(trust_weight.to_le_bytes());
        hasher.update(b":");
        hasher.update(timestamp.to_le_bytes());
        hasher.finalize().to_vec()
    }

    pub fn to_edge_data(&self) -> mesh_graph_core::types::EdgeData {
        mesh_graph_core::types::EdgeData {
            relationship: self.relationship.clone(),
            trust_weight: self.trust_weight,
            created_at: self.timestamp,
            expires_at: None,
        }
    }
}

pub fn verify_trust_chain(
    statements: &[VouchStatement],
    public_keys: &std::collections::HashMap<NodeId, Vec<u8>>,
) -> Result<f64, String> {
    if statements.is_empty() {
        return Err("empty trust chain".to_string());
    }

    let mut cumulative_trust = 1.0;

    for stmt in statements {
        let pk = public_keys
            .get(&stmt.voucher_id)
            .ok_or_else(|| format!("public key for {} not found", stmt.voucher_id))?;

        let valid = stmt.verify(pk)?;
        if !valid {
            return Err(format!(
                "invalid vouch signature from {} for {}",
                stmt.voucher_id, stmt.target_id
            ));
        }

        cumulative_trust *= stmt.trust_weight;
    }

    Ok(cumulative_trust)
}
