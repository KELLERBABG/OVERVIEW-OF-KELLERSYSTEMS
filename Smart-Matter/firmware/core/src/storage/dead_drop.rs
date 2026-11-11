//! # Decentralized Dead-Drop Storage
//!
//! Async data fragment storage with TTL, encrypted format. Each node can
//! store and forward dead-drop fragments for decentralized message passing.
//!
//! Fragments are encrypted with ChaCha20-Poly1305 before storage. Only the
//! intended recipient can reassemble and decrypt the full message.
//!
//! ## Design Note: Flash Endurance
//!
//! This is an **in-memory (SRAM) store** for active fragments. Data is
//! volatile and will be lost on power loss. This is intentional:
//!
//! 1. Dead-drop fragments are ephemeral by design (TTL-based expiration)
//! 2. Fragments are replicated across multiple mesh nodes (redundancy)
//! 3. Critical persistent state uses the separate `wear_level` module
//!    with log-structured Flash writing and rate limiting
//!
//! This separation prevents the high-frequency fragment writes from
//! wearing out the STM32G0's 10,000-cycle Flash endurance.

use crate::crypto::CryptoProvider;
use crate::types::{DeadDropFragment, TimeMs, MAX_FRAGMENTS, MAX_PAYLOAD_SIZE};
use heapless::Vec;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum number of dead-drop fragments stored on a single node.
///
/// Each fragment is ~174 bytes on the wire (18-byte header + up to 144-byte
/// ciphertext + 12-byte nonce) plus ~17 bytes of store metadata, so 32
/// fragments ≈ 6.1 KB — leaving room in the STM32G031J6's 8 KB SRAM for the
/// mesh state, entropy pool, and stack. Fragments are ephemeral by design
/// (TTL-based) and replicated across the mesh, so a full store simply stops
/// accepting new fragments until GC runs.
pub const MAX_STORED_FRAGMENTS: usize = 32;

/// Interval (seconds) between garbage collection sweeps.
pub const GC_INTERVAL_SEC: u32 = 60;

// ---------------------------------------------------------------------------
// Dead-Drop Store
// ---------------------------------------------------------------------------

/// In-memory dead-drop fragment store with TTL-based expiration.
#[derive(Debug, Clone)]
pub struct DeadDropStore {
    fragments: Vec<StoredFragment, MAX_STORED_FRAGMENTS>,
}

/// A fragment with metadata for store management.
#[derive(Debug, Clone)]
struct StoredFragment {
    fragment: DeadDropFragment,
    /// When this fragment was stored.
    stored_at: TimeMs,
}

impl Default for DeadDropStore {
    fn default() -> Self {
        Self::new()
    }
}

impl DeadDropStore {
    pub fn new() -> Self {
        Self {
            fragments: Vec::new(),
        }
    }

    /// Store a dead-drop fragment.
    ///
    /// Returns `Err` if the store is full.
    pub fn store(&mut self, fragment: DeadDropFragment) -> Result<(), StorageError> {
        if self.fragments.len() >= MAX_STORED_FRAGMENTS {
            return Err(StorageError::StoreFull);
        }

        let stored = StoredFragment {
            fragment,
            stored_at: TimeMs::now(),
        };
        self.fragments
            .push(stored)
            .map_err(|_| StorageError::StoreFull)?;
        Ok(())
    }

    /// Retrieve fragments for a given message ID.
    pub fn retrieve(&self, msg_id: u32) -> Vec<&DeadDropFragment, MAX_FRAGMENTS> {
        let mut results: Vec<&DeadDropFragment, MAX_FRAGMENTS> = Vec::new();
        for stored in self.fragments.iter() {
            if stored.fragment.header.msg_id == msg_id && results.push(&stored.fragment).is_err() {
                break;
            }
        }
        results
    }

    /// Check if all fragments for a message ID are present.
    pub fn is_complete(&self, msg_id: u32) -> bool {
        let fragments = self.retrieve(msg_id);
        if fragments.is_empty() {
            return false;
        }
        let total = fragments[0].header.total;
        fragments.len() == total as usize
    }

    /// Remove fragments for a given message ID.
    pub fn remove(&mut self, msg_id: u32) {
        self.fragments
            .retain(|s| s.fragment.header.msg_id != msg_id);
    }

    /// Garbage collect expired fragments. Returns number of fragments removed.
    pub fn gc(&mut self) -> usize {
        let now = TimeMs::now();
        let before = self.fragments.len();
        self.fragments.retain(|s| {
            let age_sec = (now.0.saturating_sub(s.stored_at.0)) / 1000;
            age_sec < s.fragment.header.ttl_sec as u64
        });
        before - self.fragments.len()
    }

    /// Number of fragments currently stored.
    pub fn len(&self) -> usize {
        self.fragments.len()
    }

    /// Check if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.fragments.is_empty()
    }

    /// Iterate over all stored fragments (for maintenance / forwarding).
    pub fn iter(&self) -> impl Iterator<Item = &DeadDropFragment> {
        self.fragments.iter().map(|s| &s.fragment)
    }

    /// Total storage utilization as a percentage (0-100).
    pub fn utilization_pct(&self) -> u8 {
        ((self.fragments.len() as u32) * 100 / MAX_STORED_FRAGMENTS as u32) as u8
    }
}

// ---------------------------------------------------------------------------
// Wire Encoding for DataFragment Payloads
// ---------------------------------------------------------------------------

/// Serialize a fragment into a slice payload (postcard).
///
/// `DataFragment` slices carry this payload; receivers decode it with
/// [`payload_to_fragment`].
pub fn fragment_to_payload(
    fragment: &DeadDropFragment,
) -> Result<heapless::Vec<u8, MAX_PAYLOAD_SIZE>, StorageError> {
    let mut scratch = [0u8; MAX_PAYLOAD_SIZE];
    let written =
        postcard::to_slice(fragment, &mut scratch).map_err(|_| StorageError::InvalidHeader)?;
    let mut payload: heapless::Vec<u8, MAX_PAYLOAD_SIZE> = heapless::Vec::new();
    payload
        .extend_from_slice(written)
        .map_err(|_| StorageError::InvalidHeader)?;
    Ok(payload)
}

/// Decode a `DataFragment` slice payload into a fragment.
pub fn payload_to_fragment(payload: &[u8]) -> Result<DeadDropFragment, StorageError> {
    postcard::from_bytes(payload).map_err(|_| StorageError::InvalidHeader)
}

// ---------------------------------------------------------------------------
// Fragment Assembly
// ---------------------------------------------------------------------------

/// Assembles fragments back into the original message.
#[derive(Debug, Clone)]
pub struct FragmentAssembler {
    /// Map msg_id → collected fragments (by index).
    buffers: Vec<(u32, Vec<Option<Vec<u8, MAX_PAYLOAD_SIZE>>, MAX_FRAGMENTS>), 4>,
}

impl Default for FragmentAssembler {
    fn default() -> Self {
        Self::new()
    }
}

impl FragmentAssembler {
    pub fn new() -> Self {
        Self {
            buffers: Vec::new(),
        }
    }

    /// Add a fragment. Returns `Ok(true)` if the message is complete,
    /// `Ok(false)` if still waiting for more fragments.
    pub fn add_fragment(
        &mut self,
        fragment: &DeadDropFragment,
        crypto: &CryptoProvider,
        key: &[u8; 32],
    ) -> Result<Option<Vec<u8, { MAX_PAYLOAD_SIZE * MAX_FRAGMENTS }>>, AssemblyError> {
        let header = &fragment.header;

        // Find or create buffer for this msg_id
        let buf_idx = self.find_or_create_buffer(header.msg_id, header.total as usize)?;
        let (_, buf) = &mut self.buffers[buf_idx];

        let idx = header.index as usize;
        if idx >= buf.len() {
            return Err(AssemblyError::InvalidIndex);
        }

        // Decrypt fragment payload
        let plaintext = crypto
            .decrypt(key, &fragment.nonce, &fragment.ciphertext)
            .map_err(|_| AssemblyError::DecryptionFailed)?;

        buf[idx] = Some(plaintext);

        // Check if complete
        if buf.iter().all(|b| b.is_some()) {
            // Assemble
            let (_, completed_buf) = self.buffers.remove(buf_idx);
            let mut message: Vec<u8, { MAX_PAYLOAD_SIZE * MAX_FRAGMENTS }> = Vec::new();
            for chunk in completed_buf.into_iter().flatten() {
                message.extend_from_slice(&chunk).ok();
            }
            Ok(Some(message))
        } else {
            Ok(None)
        }
    }

    fn find_or_create_buffer(&mut self, msg_id: u32, total: usize) -> Result<usize, AssemblyError> {
        if let Some(idx) = self.buffers.iter().position(|(id, _)| *id == msg_id) {
            return Ok(idx);
        }

        if self.buffers.len() >= 4 {
            // Drop oldest buffer
            self.buffers.remove(0);
        }

        let mut buf: Vec<Option<Vec<u8, MAX_PAYLOAD_SIZE>>, MAX_FRAGMENTS> = Vec::new();
        for _ in 0..total.min(MAX_FRAGMENTS) {
            buf.push(None).ok();
        }

        self.buffers
            .push((msg_id, buf))
            .map_err(|_| AssemblyError::BufferFull)?;
        Ok(self.buffers.len() - 1)
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageError {
    StoreFull,
    FragmentTooLarge,
    InvalidHeader,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssemblyError {
    BufferFull,
    InvalidIndex,
    DecryptionFailed,
    ChecksumMismatch,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::CryptoProvider;
    use crate::types::FragmentHeader;

    #[test]
    fn test_store_retrieve() {
        let mut store = DeadDropStore::new();
        let fragment = DeadDropFragment {
            header: FragmentHeader {
                msg_id: 42,
                total: 3,
                index: 0,
                ttl_sec: 3600,
                created: TimeMs::now(),
            },
            ciphertext: {
                let mut v: Vec<u8, 144> = Vec::new();
                v.push(1).ok();
                v
            },
            nonce: [0u8; 12],
        };

        assert!(store.store(fragment).is_ok());
        assert_eq!(store.len(), 1);

        let retrieved = store.retrieve(42);
        assert_eq!(retrieved.len(), 1);
    }

    #[test]
    fn test_gc_expired() {
        let mut store = DeadDropStore::new();
        let fragment = DeadDropFragment {
            header: FragmentHeader {
                msg_id: 1,
                total: 1,
                index: 0,
                ttl_sec: 0, // Already expired
                created: TimeMs(0),
            },
            ciphertext: Vec::new(),
            nonce: [0u8; 12],
        };

        store.store(fragment).ok();
        assert_eq!(store.len(), 1);
        let removed = store.gc();
        assert_eq!(removed, 1);
        assert!(store.is_empty());
    }

    #[test]
    fn test_fragment_assembly() {
        let crypto = CryptoProvider::new();
        let key = [0x42u8; 32];

        // Create two fragments
        let msg = b"Hello Smart Matter!";
        let chunk_size = msg.len() / 2 + msg.len() % 2; // Ensure all bytes covered

        let mut assembler = FragmentAssembler::new();

        for i in 0..2 {
            let start = i * chunk_size;
            let end = msg.len().min((i + 1) * chunk_size);
            let data = &msg[start..end];
            let mut plain: Vec<u8, 128> = Vec::new();
            plain.extend_from_slice(data).ok();

            let nonce = [i as u8; 12];
            let ciphertext = crypto.encrypt(&key, &nonce, &plain).unwrap();

            let fragment = DeadDropFragment {
                header: FragmentHeader {
                    msg_id: 1,
                    total: 2,
                    index: i as u8,
                    ttl_sec: 3600,
                    created: TimeMs::now(),
                },
                ciphertext,
                nonce,
            };

            let result = assembler.add_fragment(&fragment, &crypto, &key).unwrap();
            if i == 0 {
                assert!(result.is_none()); // Not complete yet
            } else {
                assert!(result.is_some()); // Complete!
                let assembled = result.unwrap();
                assert_eq!(&assembled[..msg.len()], msg);
            }
        }
    }
}
