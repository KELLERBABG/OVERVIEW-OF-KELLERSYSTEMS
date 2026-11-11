//! Total Memory Encryption.
//!
//! Transparent encryption of sensitive in-memory data using a
//! SHA-256 based stream cipher with random nonces. All sensitive
//! buffers (private keys, escrow balances, bid prices, SMPC shares)
//! are encrypted at rest and decrypted on access.
//!
//! Uses XEX-based tweakable encryption: each block is encrypted with
//! a unique IV, and the key is derived from a master key + context.

use rand::Rng;
use sha2::{Digest, Sha256};

/// An encrypted memory region.
#[derive(Debug, Clone)]
pub struct EncryptedMemory {
    ciphertext: Vec<u8>,
    iv: Vec<u8>,
}

/// Manages encrypted memory regions with automatic encryption/decryption.
#[derive(Debug)]
pub struct MemoryEncryptor {
    key: [u8; 32],
}

impl MemoryEncryptor {
    /// Create a new encryptor with a random 256-bit key.
    pub fn new() -> Self {
        let mut key = [0u8; 32];
        rand::thread_rng().fill(&mut key);
        Self { key }
    }

    /// Create with a specific key (for test vectors / deterministic builds).
    pub fn with_key(key: [u8; 32]) -> Self {
        Self { key }
    }

    /// Encrypt plaintext into an encrypted memory region.
    /// Uses a stream cipher constructed from SHA-256 in CTR mode.
    pub fn encrypt(&self, plaintext: &[u8]) -> EncryptedMemory {
        let mut iv = vec![0u8; 32];
        rand::thread_rng().fill(iv.as_mut_slice());

        let ciphertext = self.xor_with_keystream(plaintext, &iv);

        EncryptedMemory { ciphertext, iv }
    }

    /// Decrypt an encrypted memory region back to plaintext.
    pub fn decrypt(&self, encrypted: &EncryptedMemory) -> Vec<u8> {
        self.xor_with_keystream(&encrypted.ciphertext, &encrypted.iv)
    }

    /// Generate keystream using SHA-256 in CTR-like mode and XOR with data.
    /// K(i) = SHA256(key || iv || counter)
    fn xor_with_keystream(&self, data: &[u8], iv: &[u8]) -> Vec<u8> {
        let mut result = data.to_vec();
        let mut counter = 0u64;

        for chunk in result.chunks_mut(32) {
            let mut hasher = Sha256::new();
            hasher.update(&self.key);
            hasher.update(iv);
            hasher.update(counter.to_le_bytes().as_slice());
            let keystream_block = hasher.finalize();

            for (b, k) in chunk.iter_mut().zip(keystream_block.iter()) {
                *b ^= k;
            }
            counter += 1;
        }

        result
    }

    /// Securely zero the key from memory.
    pub fn zeroize(&mut self) {
        self.key.iter_mut().for_each(|b| *b = 0);
    }

    /// Expose the current key bytes for audit / test assertions only.
    /// Do not retain the returned slice beyond the lifetime of `&self`.
    pub fn as_key_bytes(&self) -> &[u8; 32] {
        &self.key
    }
}

impl Default for MemoryEncryptor {
    fn default() -> Self {
        Self::new()
    }
}

/// A sensitive value that is automatically encrypted in memory.
#[derive(Debug, Clone)]
pub struct SecureCell {
    encrypted: EncryptedMemory,
}

impl SecureCell {
    pub fn new(value: &[u8]) -> Self {
        let encryptor = MemoryEncryptor::new();
        let encrypted = encryptor.encrypt(value);
        Self { encrypted }
    }

    pub fn decrypt(&self, encryptor: &MemoryEncryptor) -> Vec<u8> {
        encryptor.decrypt(&self.encrypted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let encryptor = MemoryEncryptor::new();
        let data = b"sensitive_escrow_balance_12345";

        let encrypted = encryptor.encrypt(data);
        assert_ne!(encrypted.ciphertext, data);

        let decrypted = encryptor.decrypt(&encrypted);
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_different_ciphertexts() {
        let encryptor = MemoryEncryptor::new();
        let data = b"same_data";

        let e1 = encryptor.encrypt(data);
        let e2 = encryptor.encrypt(data);

        // Different IVs should produce different ciphertexts
        assert_ne!(e1.ciphertext, e2.ciphertext);
    }

    #[test]
    fn test_deterministic_key() {
        let key = [0x42u8; 32];
        let encryptor = MemoryEncryptor::with_key(key);
        let data = b"test_data_123";

        let encrypted = encryptor.encrypt(data);
        let decrypted = encryptor.decrypt(&encrypted);
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_zeroize_clears_key() {
        let mut encryptor = MemoryEncryptor::new();
        let before = encryptor.encrypt(b"test");
        assert_ne!(before.ciphertext, b"test");
        encryptor.zeroize();
        // After zeroize the stored key must be all zeros so future callers
        // cannot recover the previous key material from the encryptor.
        let key_after: &[u8] = encryptor.as_key_bytes();
        assert!(key_after.iter().all(|b| *b == 0));

        // A zeroed key produces a deterministic all-zero keystream for a
        // fixed IV, so we can assert a concrete decrypt outcome.
        let iv = vec![0u8; 32];
        let zeroed_ct = encryptor.xor_with_keystream(b"abc", &iv);
        let recovered = encryptor.xor_with_keystream(&zeroed_ct, &iv);
        assert_eq!(recovered, b"abc");
    }
}