use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NullifierRegistry {
    used_nullifiers: HashSet<String>,
}

impl NullifierRegistry {
    pub fn new() -> Self {
        NullifierRegistry {
            used_nullifiers: HashSet::new(),
        }
    }

    pub fn is_used(&self, nullifier_hex: &str) -> bool {
        self.used_nullifiers.contains(nullifier_hex)
    }

    pub fn register(&mut self, nullifier_hex: String) -> Result<(), String> {
        if self.used_nullifiers.contains(&nullifier_hex) {
            return Err("nullifier already used — double-borrow attempt detected".to_string());
        }
        self.used_nullifiers.insert(nullifier_hex);
        Ok(())
    }

    pub fn used_count(&self) -> usize {
        self.used_nullifiers.len()
    }

    pub fn compute_nullifier(identity_id: &str, public_key_bytes: &[u8], nonce: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"NULLIFIER:v1:");
        hasher.update(identity_id.as_bytes());
        hasher.update(public_key_bytes);
        hasher.update(nonce);
        let result = hasher.finalize();
        hex_encode(&result)
    }

    pub fn check_nullifier(
        &self,
        identity_id: &str,
        public_key_bytes: &[u8],
        nonce: &[u8],
    ) -> Result<(), String> {
        let nullifier = Self::compute_nullifier(identity_id, public_key_bytes, nonce);
        if self.is_used(&nullifier) {
            return Err("identity already has an active credit line".to_string());
        }
        Ok(())
    }

    pub fn register_nullifier(
        &mut self,
        identity_id: &str,
        public_key_bytes: &[u8],
        nonce: &[u8],
    ) -> Result<String, String> {
        let nullifier = Self::compute_nullifier(identity_id, public_key_bytes, nonce);
        self.register(nullifier.clone())?;
        Ok(nullifier)
    }

    /// Release a nullifier once the credit line is fully settled (repaid or
    /// defaulted), so the identity can open a new credit line. No-op if the
    /// nullifier is not registered.
    pub fn release_nullifier(&mut self, identity_id: &str, public_key_bytes: &[u8], nonce: &[u8]) {
        let nullifier = Self::compute_nullifier(identity_id, public_key_bytes, nonce);
        self.used_nullifiers.remove(&nullifier);
    }
}

impl Default for NullifierRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        write!(out, "{:02x}", b).expect("writing to a String cannot fail");
    }
    out
}
