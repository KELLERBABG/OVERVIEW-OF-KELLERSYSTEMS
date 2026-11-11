use chrono::Utc;
use sybil_core::{
    DidDocument, NodeId, ServiceEndpoint, SybilResult, VerificationMethod,
};
use sybil_core::primitives::{encode_did, generate_keypair};

use crate::storage::DidStorage;

/// DID Manager: handles the full lifecycle of `did:sybil:` identifiers.
pub struct DidManager {
    storage: DidStorage,
}

impl DidManager {
    pub fn new() -> Self {
        Self {
            storage: DidStorage::new(),
        }
    }

    /// Create a new DID for a node.
    ///
    /// Generates an ed25519 keypair and constructs a W3C DID Document.
    pub fn create_did(&mut self, node: &NodeId) -> SybilResult<DidDocument> {
        let (signing_key, verifying_key) = generate_keypair();
        let did = encode_did(&verifying_key);

        // Store the signing key (in production this would be encrypted)
        self.storage.store_signing_key(&did, signing_key);

        let doc = DidDocument {
            did: did.clone(),
            verification_methods: vec![VerificationMethod {
                id: format!("{}#keys-1", did),
                controller: did.clone(),
                key_type: "Ed25519VerificationKey2020".to_string(),
                public_key_multibase: bs58_encode(verifying_key.as_bytes()),
            }],
            authentication: vec![format!("{}#keys-1", did)],
            assertion_method: vec![format!("{}#keys-1", did)],
            service_endpoints: vec![ServiceEndpoint {
                id: format!("{}#sybil-node", did),
                service_type: "SybilResistentesProofOfPersonhood".to_string(),
                endpoint_url: format!("node://sybil/{}", node),
            }],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.storage.store_doc(&doc);
        Ok(doc)
    }

    /// Resolve a DID to its document.
    pub fn resolve_did(&self, did: &str) -> SybilResult<DidDocument> {
        if !did.starts_with("did:sybil:") {
            return Err(sybil_core::SybilError::UnsupportedDidMethod(
                did.split(':').nth(1).unwrap_or("unknown").to_string(),
            ));
        }
        self.storage
            .get_doc(did)
            .ok_or_else(|| sybil_core::SybilError::DidNotFound(did.to_string()))
    }

    /// Update a DID document.
    pub fn update_did(
        &mut self,
        did: &str,
        mut document: DidDocument,
    ) -> SybilResult<DidDocument> {
        if !did.starts_with("did:sybil:") {
            return Err(sybil_core::SybilError::UnsupportedDidMethod(
                did.split(':').nth(1).unwrap_or("unknown").to_string(),
            ));
        }
        document.updated_at = Utc::now();
        self.storage.store_doc(&document);
        Ok(document)
    }

    /// Deactivate a DID.
    pub fn deactivate_did(&mut self, did: &str) -> SybilResult<()> {
        if !did.starts_with("did:sybil:") {
            return Err(sybil_core::SybilError::UnsupportedDidMethod(
                did.split(':').nth(1).unwrap_or("unknown").to_string(),
            ));
        }
        self.storage.remove_doc(did);
        Ok(())
    }
}

/// Base58 encode bytes (simple implementation using base64 as placeholder).
fn bs58_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

impl Default for DidManager {
    fn default() -> Self {
        Self::new()
    }
}