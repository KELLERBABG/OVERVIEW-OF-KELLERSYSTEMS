use std::collections::HashMap;

use ed25519_dalek::SigningKey;
use sybil_core::{DidDocument};

/// In-memory storage for DID Documents and their associated key material.
///
/// In production, this would be an encrypted local store or hardware-backed keystore.
#[derive(Default)]
pub struct DidStorage {
    docs: HashMap<String, DidDocument>,
    signing_keys: HashMap<String, SigningKey>,
}

impl DidStorage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn store_doc(&mut self, doc: &DidDocument) {
        self.docs.insert(doc.did.clone(), doc.clone());
    }

    pub fn get_doc(&self, did: &str) -> Option<DidDocument> {
        self.docs.get(did).cloned()
    }

    pub fn remove_doc(&mut self, did: &str) {
        self.docs.remove(did);
        self.signing_keys.remove(did);
    }

    pub fn store_signing_key(&mut self, did: &str, key: SigningKey) {
        self.signing_keys.insert(did.to_string(), key);
    }

    pub fn get_signing_key(&self, did: &str) -> Option<&SigningKey> {
        self.signing_keys.get(did)
    }
}