//! # Smart Matter Decentralized Dead-Drop Storage
//!
//! High-level dead-drop storage manager providing fragmentation, encryption,
//! and reassembly of messages stored across the Smart Matter mesh network.

pub use smart_matter_core::crypto::*;
pub use smart_matter_core::storage::*;
pub use smart_matter_core::types::*;

use heapless::Vec;

/// Manages dead-drop storage operations across the mesh network.
#[derive(Debug, Clone)]
pub struct DropStorageManager {
    store: DeadDropStore,
    crypto: CryptoProvider,
    key: [u8; 32],
}

impl DropStorageManager {
    pub fn new(key: [u8; 32]) -> Self {
        Self {
            store: DeadDropStore::new(),
            crypto: CryptoProvider::new(),
            key,
        }
    }

    /// Split a message into encrypted fragments and store them locally.
    pub fn store_message(
        &mut self,
        msg_id: u32,
        message: &[u8],
        ttl_sec: u32,
    ) -> Result<Vec<FragmentHeader, MAX_FRAGMENTS>, StorageError> {
        let fragment_count = message.len().div_ceil(MAX_PAYLOAD_SIZE);
        if fragment_count > MAX_FRAGMENTS {
            return Err(StorageError::FragmentTooLarge);
        }

        let mut headers: Vec<FragmentHeader, MAX_FRAGMENTS> = Vec::new();
        let now = TimeMs::now();

        for i in 0..fragment_count {
            let start = i * MAX_PAYLOAD_SIZE;
            let end = (start + MAX_PAYLOAD_SIZE).min(message.len());
            let chunk = &message[start..end];

            let nonce = self.crypto.generate_nonce(&msg_id.to_le_bytes());
            let mut plain: Vec<u8, 128> = Vec::new();
            plain.extend_from_slice(chunk).ok();

            let ciphertext = self
                .crypto
                .encrypt(&self.key, &nonce, &plain)
                .map_err(|_| StorageError::InvalidHeader)?;

            let header = FragmentHeader {
                msg_id,
                total: fragment_count as u8,
                index: i as u8,
                ttl_sec,
                created: now,
            };

            let fragment = DeadDropFragment {
                header,
                ciphertext,
                nonce,
            };

            self.store.store(fragment)?;
            headers.push(header).ok();
        }

        Ok(headers)
    }

    /// Assemble a message from all fragments with the given message ID.
    pub fn retrieve_message(
        &mut self,
        msg_id: u32,
    ) -> Result<Option<Vec<u8, { MAX_PAYLOAD_SIZE * MAX_FRAGMENTS }>>, AssemblyError> {
        let fragments = self.store.retrieve(msg_id);
        if fragments.is_empty() {
            return Ok(None);
        }

        let mut assembler = FragmentAssembler::new();
        let mut result = None;

        for fragment in fragments {
            if let Some(msg) = assembler.add_fragment(fragment, &self.crypto, &self.key)? {
                result = Some(msg);
                break;
            }
        }

        self.store.remove(msg_id);
        Ok(result)
    }

    pub fn gc(&mut self) -> usize {
        self.store.gc()
    }

    pub fn fragment_count(&self) -> usize {
        self.store.len()
    }

    pub fn utilization_pct(&self) -> u8 {
        self.store.utilization_pct()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_retrieve_message() {
        let key = [0x42u8; 32];
        let mut manager = DropStorageManager::new(key);

        let message = b"This is a test message for dead-drop storage!";
        let headers = manager
            .store_message(1, message, 3600)
            .expect("Should store message");
        assert!(!headers.is_empty());

        let retrieved = manager
            .retrieve_message(1)
            .expect("Should retrieve")
            .expect("Should have message");
        assert_eq!(&retrieved[..message.len()], message);
    }

    #[test]
    fn test_gc_expired_fragments() {
        let key = [0x42u8; 32];
        let mut manager = DropStorageManager::new(key);

        manager
            .store_message(2, b"expired", 0)
            .expect("Should store");
        assert_eq!(manager.fragment_count(), 1);

        let removed = manager.gc();
        assert_eq!(removed, 1);
        assert_eq!(manager.fragment_count(), 0);
    }
}
