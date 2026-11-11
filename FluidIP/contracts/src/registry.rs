//! # PatentRegistry
//!
//! On-chain DAG storage for patent nodes. Only Merkle commitments and
//! minimal metadata are stored on-chain; the full patent graph state
//! is stored off-chain (IPFS/Arweave) with integrity proofs.

use dfip_core::models::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::Uuid;

/// A registered patent on-chain.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegisteredPatent {
    /// The patent node UUID.
    pub id: Uuid,
    /// Title (stored on-chain for discoverability).
    pub title: String,
    /// Timestamp of registration (Unix seconds).
    pub registered_at: i64,
    /// The registrant's address (hex string).
    pub registrant: String,
    /// Merkle commitment to the full PatentNode data.
    pub commitment: Vec<u8>,
    /// Hash of the patent's invariant (for clustering).
    pub invariant_hash: Vec<u8>,
    /// Whether the patent is publicly disclosed or secret.
    pub license: License,
}

/// The on-chain patent registry.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatentRegistry {
    patents: HashMap<Uuid, RegisteredPatent>,
    commitments: HashMap<Vec<u8>, Uuid>, // commitment → id
}

impl PatentRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            patents: HashMap::new(),
            commitments: HashMap::new(),
        }
    }

    /// Register a new patent on-chain.
    pub fn register(
        &mut self,
        node: &PatentNode,
        registrant: &str,
    ) -> RegisteredPatent {
        let serialized = serde_json::to_vec(node).unwrap_or_default();
        let commitment = Sha256::digest(&serialized).to_vec();

        let invariant_hash = node
            .invariant
            .as_ref()
            .map(|inv| Sha256::digest(&inv.encoded_ir).to_vec())
            .unwrap_or_else(|| Sha256::digest(b"no-invariant").to_vec());

        let registered = RegisteredPatent {
            id: node.id,
            title: node.title.clone(),
            registered_at: chrono::Utc::now().timestamp(),
            registrant: registrant.to_string(),
            commitment: commitment.clone(),
            invariant_hash,
            license: node.license.clone(),
        };

        self.patents.insert(node.id, registered.clone());
        self.commitments.insert(commitment, node.id);
        registered
    }

    /// Verify a patent node against its on-chain commitment.
    pub fn verify_commitment(&self, node: &PatentNode) -> bool {
        let serialized = serde_json::to_vec(node).unwrap_or_default();
        let computed = Sha256::digest(&serialized).to_vec();

        if let Some(registered) = self.patents.get(&node.id) {
            computed == registered.commitment
        } else {
            false
        }
    }

    /// Get a registered patent by ID.
    pub fn get(&self, id: &Uuid) -> Option<&RegisteredPatent> {
        self.patents.get(id)
    }

    /// Get all registered patents.
    pub fn all(&self) -> Vec<&RegisteredPatent> {
        self.patents.values().collect()
    }

    /// Number of registered patents.
    pub fn len(&self) -> usize {
        self.patents.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.patents.is_empty()
    }
}

impl Default for PatentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dfip_core::models::*;

    fn make_node(id: Uuid) -> PatentNode {
        PatentNode {
            id,
            title: "Test Patent".into(),
            patent_type: PatentType::Software,
            authors: vec!["Alice".into()],
            created_at_unix: 0,
            content_hash: vec![],
            invariant: None,
            tags: vec![],
            license: License::Open,
            meta_commitment: vec![],
        }
    }

    #[test]
    fn test_register_and_verify() {
        let mut registry = PatentRegistry::new();
        let node = make_node(Uuid::new_v4());
        let registered = registry.register(&node, "0xAlice");
        assert_eq!(registered.id, node.id);
        assert_eq!(registered.registrant, "0xAlice");
        assert!(registry.verify_commitment(&node));
    }

    #[test]
    fn test_tampered_node_rejected() {
        let mut registry = PatentRegistry::new();
        let mut node = make_node(Uuid::new_v4());
        registry.register(&node, "0xBob");

        // Tamper with the node
        node.title = "Hacked Patent".into();
        assert!(!registry.verify_commitment(&node));
    }

    #[test]
    fn test_registry_empty() {
        let registry = PatentRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }
}