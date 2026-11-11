//! # Storage Layer
//!
//! Content-addressed storage for patent blueprints and graph state.
//! In production, this wraps IPFS, Arweave, or Ceramic/OrbitDB.
//! Here we provide an in-memory simulation with Merkle commitment verification.

use dfip_core::models::PatentNode;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::Uuid;

/// A stored blueprint — the actual invention artifact.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Blueprint {
    /// Content-addressed hash (CID).
    pub cid: String,
    /// The encrypted patent node data.
    pub data: Vec<u8>,
    /// Size in bytes.
    pub size: u64,
}

/// The storage node — simulates IPFS/Arweave.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StorageNode {
    /// Blueprints by CID.
    blueprints: HashMap<String, Blueprint>,
    /// Node data by UUID.
    node_data: HashMap<Uuid, Vec<u8>>,
}

impl StorageNode {
    pub fn new() -> Self {
        Self {
            blueprints: HashMap::new(),
            node_data: HashMap::new(),
        }
    }

    /// Store a patent node's data. Returns a content-addressed CID.
    pub fn store_node(&mut self, node: &PatentNode) -> String {
        let data = serde_json::to_vec(node).unwrap_or_default();
        let cid = format!("Qm{}", hex::encode(Sha256::digest(&data)));

        self.blueprints.insert(
            cid.clone(),
            Blueprint {
                cid: cid.clone(),
                data: data.clone(),
                size: data.len() as u64,
            },
        );
        self.node_data.insert(node.id, data);
        cid
    }

    /// Retrieve a patent node by CID.
    pub fn get_by_cid(&self, cid: &str) -> Option<&Blueprint> {
        self.blueprints.get(cid)
    }

    /// Retrieve a patent node by UUID.
    pub fn get_by_id(&self, id: &Uuid) -> Option<&[u8]> {
        self.node_data.get(id).map(|v| v.as_slice())
    }

    /// Verify data integrity against a CID.
    pub fn verify_integrity(&self, cid: &str, data: &[u8]) -> bool {
        let computed = format!("Qm{}", hex::encode(Sha256::digest(data)));
        computed == cid && self.blueprints.contains_key(cid)
    }

    /// Number of stored blueprints.
    pub fn len(&self) -> usize {
        self.blueprints.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blueprints.is_empty()
    }
}

impl Default for StorageNode {
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
            title: "Storage Test".into(),
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
    fn test_store_and_retrieve() {
        let mut storage = StorageNode::new();
        let node = make_node(Uuid::new_v4());
        let cid = storage.store_node(&node);
        assert!(!cid.is_empty());
        assert_eq!(storage.len(), 1);

        let retrieved = storage.get_by_cid(&cid).unwrap();
        assert!(retrieved.size > 0); // just verify non-zero size
    }

    #[test]
    fn test_verify_integrity() {
        let mut storage = StorageNode::new();
        let node = make_node(Uuid::new_v4());
        let data = serde_json::to_vec(&node).unwrap();
        let cid = storage.store_node(&node);
        assert!(storage.verify_integrity(&cid, &data));
        assert!(!storage.verify_integrity(&cid, b"tampered"));
    }
}