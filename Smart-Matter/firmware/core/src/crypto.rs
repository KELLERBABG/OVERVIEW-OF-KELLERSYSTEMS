//! # Cryptographic Primitives
//!
//! Authenticated encryption using **RFC 8439 ChaCha20-Poly1305** (the
//! RustCrypto `chacha20poly1305` crate — a constant-time, widely audited
//! implementation) for dead-drop fragment encryption and integrity
//! verification, plus deployment-scoped key derivation.
//!
//! Designed for constrained embedded targets — no heap allocation,
//! no platform-specific crypto hardware required.
//!
//! ## Key management
//!
//! Keys are **derived**, never hardcoded: `derive_key(deployment_secret,
//! node_id)` computes HMAC-SHA256 over the node ID using a deployment-level
//! secret (see the desktop tool `--deployment-secret` and
//! `SMART_MATTER_DEPLOYMENT_SECRET`). A different secret per deployment
//! means a captured key never decrypts another deployment's traffic.

use chacha20poly1305::aead::generic_array::GenericArray;
use chacha20poly1305::aead::{AeadInPlace, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Tag};
use hmac::{Hmac, Mac};
use sha2::Sha256;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Key size in bytes (256-bit).
pub const KEY_SIZE: usize = 32;

/// Nonce size in bytes (96-bit).
pub const NONCE_SIZE: usize = 12;

/// Tag size in bytes (128-bit Poly1305).
pub const TAG_SIZE: usize = 16;

/// Maximum plaintext size for a single fragment (fits one protocol slice).
pub const MAX_PLAINTEXT_SIZE: usize = 128;

/// Maximum ciphertext + tag size.
pub const MAX_SEALED_SIZE: usize = MAX_PLAINTEXT_SIZE + TAG_SIZE;

/// Domain-separation label for key derivation.
const KDF_CONTEXT: &[u8] = b"smart-matter-v1";

// ---------------------------------------------------------------------------
// Crypto Provider
// ---------------------------------------------------------------------------

/// Provides RFC 8439 ChaCha20-Poly1305 authenticated encryption.
#[derive(Debug, Clone)]
pub struct CryptoProvider;

impl Default for CryptoProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CryptoProvider {
    pub fn new() -> Self {
        Self
    }

    /// ChaCha20-Poly1305 encrypt (RFC 8439).
    ///
    /// Returns `ciphertext || tag` (plaintext_len + 16 bytes).
    /// No AAD is used; callers that need to bind message metadata to the
    /// ciphertext should use [`Self::encrypt_with_aad`].
    pub fn encrypt(
        &self,
        key: &[u8; KEY_SIZE],
        nonce: &[u8; NONCE_SIZE],
        plaintext: &[u8],
    ) -> Result<heapless::Vec<u8, MAX_SEALED_SIZE>, CryptoError> {
        self.encrypt_with_aad(key, nonce, plaintext, b"")
    }

    /// ChaCha20-Poly1305 encrypt with associated data (RFC 8439).
    pub fn encrypt_with_aad(
        &self,
        key: &[u8; KEY_SIZE],
        nonce: &[u8; NONCE_SIZE],
        plaintext: &[u8],
        aad: &[u8],
    ) -> Result<heapless::Vec<u8, MAX_SEALED_SIZE>, CryptoError> {
        if plaintext.len() > MAX_PLAINTEXT_SIZE {
            return Err(CryptoError::PlaintextTooLong);
        }

        let cipher = ChaCha20Poly1305::new(GenericArray::from_slice(key));
        let nonce_ga = GenericArray::from_slice(nonce);

        let mut buffer = heapless::Vec::<u8, MAX_PLAINTEXT_SIZE>::new();
        buffer
            .extend_from_slice(plaintext)
            .map_err(|_| CryptoError::PlaintextTooLong)?;

        // encrypt_in_place_detached requires exactly 16-byte aligned tag
        let tag: Tag = cipher
            .encrypt_in_place_detached(nonce_ga, aad, &mut buffer)
            .map_err(|_| CryptoError::EncryptionFailed)?;

        // ciphertext || tag
        let mut result = heapless::Vec::<u8, MAX_SEALED_SIZE>::new();
        result
            .extend_from_slice(&buffer)
            .map_err(|_| CryptoError::CiphertextTooLong)?;
        result
            .extend_from_slice(&tag)
            .map_err(|_| CryptoError::CiphertextTooLong)?;
        Ok(result)
    }

    /// ChaCha20-Poly1305 decrypt and verify (RFC 8439).
    pub fn decrypt(
        &self,
        key: &[u8; KEY_SIZE],
        nonce: &[u8; NONCE_SIZE],
        data: &[u8],
    ) -> Result<heapless::Vec<u8, MAX_PLAINTEXT_SIZE>, CryptoError> {
        self.decrypt_with_aad(key, nonce, data, b"")
    }

    /// ChaCha20-Poly1305 decrypt and verify with associated data (RFC 8439).
    pub fn decrypt_with_aad(
        &self,
        key: &[u8; KEY_SIZE],
        nonce: &[u8; NONCE_SIZE],
        data: &[u8],
        aad: &[u8],
    ) -> Result<heapless::Vec<u8, MAX_PLAINTEXT_SIZE>, CryptoError> {
        if data.len() < TAG_SIZE {
            return Err(CryptoError::CiphertextTooShort);
        }
        if data.len() > MAX_SEALED_SIZE {
            return Err(CryptoError::CiphertextTooLong);
        }

        let ct_len = data.len() - TAG_SIZE;
        let ciphertext = &data[..ct_len];
        let tag = Tag::from_slice(&data[ct_len..]);

        let cipher = ChaCha20Poly1305::new(GenericArray::from_slice(key));
        let nonce_ga = GenericArray::from_slice(nonce);

        let mut buffer = heapless::Vec::<u8, MAX_PLAINTEXT_SIZE>::new();
        buffer
            .extend_from_slice(ciphertext)
            .map_err(|_| CryptoError::CiphertextTooLong)?;

        cipher
            .decrypt_in_place_detached(nonce_ga, aad, &mut buffer, tag)
            .map_err(|_| CryptoError::AuthenticationFailed)?;

        Ok(buffer)
    }

    /// Generate a nonce from entropy input.
    ///
    /// The entropy pool (`sensor::EntropyPool`) mixes ADC noise, vibration
    /// floor and temperature samples. On embedded targets this is combined
    /// with the MCU tick counter before use.
    pub fn generate_nonce(&self, entropy: &[u8]) -> [u8; NONCE_SIZE] {
        let mut nonce = [0u8; NONCE_SIZE];
        let len = entropy.len().min(NONCE_SIZE);
        nonce[..len].copy_from_slice(&entropy[..len]);
        nonce
    }
}

// ---------------------------------------------------------------------------
// Key Derivation
// ---------------------------------------------------------------------------

/// Derive a deployment-scoped 256-bit node key.
///
/// `key = HMAC-SHA256(deployment_secret, "smart-matter-v1" || node_id)`
///
/// Each deployment picks a fresh random secret (e.g. `openssl rand -hex 32`);
/// node keys are derived per node ID so no key material is shared verbatim
/// and a leaked key compromises only its own node's traffic under that
/// deployment secret.
pub fn derive_key(deployment_secret: &[u8], node_id: &crate::types::NodeId) -> [u8; KEY_SIZE] {
    type HmacSha256 = Hmac<Sha256>;
    let mut mac =
        <HmacSha256 as KeyInit>::new_from_slice(deployment_secret).expect("HMAC accepts any key");
    mac.update(KDF_CONTEXT);
    mac.update(node_id.as_bytes());
    let result = mac.finalize().into_bytes();
    let mut key = [0u8; KEY_SIZE];
    key.copy_from_slice(&result);
    key
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoError {
    PlaintextTooLong,
    CiphertextTooShort,
    CiphertextTooLong,
    AuthenticationFailed,
    EncryptionFailed,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_node_id(id: u8) -> crate::types::NodeId {
        let mut bytes = [0u8; 8];
        bytes[0] = id;
        crate::types::NodeId(bytes)
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let crypto = CryptoProvider::new();
        let key = [0x42u8; 32];
        let nonce = [0x01u8; 12];
        let plaintext = b"Hello Smart Matter! This is a test message.";

        let ciphertext = crypto.encrypt(&key, &nonce, plaintext).unwrap();
        let decrypted = crypto.decrypt(&key, &nonce, &ciphertext).unwrap();

        assert_eq!(&decrypted[..], plaintext);
    }

    #[test]
    fn test_encrypt_decrypt_with_aad() {
        let crypto = CryptoProvider::new();
        let key = [0x42u8; 32];
        let nonce = [0x01u8; 12];
        let plaintext = b"bound to metadata";
        let aad = b"msg_id=7;fragment=2/4";

        let ciphertext = crypto
            .encrypt_with_aad(&key, &nonce, plaintext, aad)
            .unwrap();

        // Correct AAD decrypts
        let decrypted = crypto
            .decrypt_with_aad(&key, &nonce, &ciphertext, aad)
            .unwrap();
        assert_eq!(&decrypted[..], plaintext);

        // Wrong AAD fails authentication
        let result = crypto.decrypt_with_aad(&key, &nonce, &ciphertext, b"tampered");
        assert_eq!(result, Err(CryptoError::AuthenticationFailed));
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let crypto = CryptoProvider::new();
        let key = [0x42u8; 32];
        let nonce = [0x01u8; 12];
        let plaintext = b"Test";

        let mut ciphertext = crypto.encrypt(&key, &nonce, plaintext).unwrap();
        if !ciphertext.is_empty() {
            ciphertext[0] ^= 0xFF;
        }
        let result = crypto.decrypt(&key, &nonce, &ciphertext);
        assert_eq!(result, Err(CryptoError::AuthenticationFailed));
    }

    #[test]
    fn test_wrong_key_fails() {
        let crypto = CryptoProvider::new();
        let key1 = [0x42u8; 32];
        let key2 = [0x24u8; 32];
        let nonce = [0x01u8; 12];
        let plaintext = b"Secret data";

        let ciphertext = crypto.encrypt(&key1, &nonce, plaintext).unwrap();
        let result = crypto.decrypt(&key2, &nonce, &ciphertext);
        assert_eq!(result, Err(CryptoError::AuthenticationFailed));
    }

    #[test]
    fn test_plaintext_too_long_rejected() {
        let crypto = CryptoProvider::new();
        let key = [0x42u8; 32];
        let nonce = [0x01u8; 12];
        let oversized = [0xAAu8; MAX_PLAINTEXT_SIZE + 1];
        assert_eq!(
            crypto.encrypt(&key, &nonce, &oversized),
            Err(CryptoError::PlaintextTooLong)
        );
    }

    #[test]
    fn test_key_derivation_deterministic_and_scoped() {
        let secret = b"deployment-secret-0123456789abcdef";
        let n1 = test_node_id(1);
        let n2 = test_node_id(2);

        let k1 = derive_key(secret, &n1);
        let k1_again = derive_key(secret, &n1);
        let k2 = derive_key(secret, &n2);
        let k_other_secret = derive_key(b"different-secret", &n1);

        assert_eq!(k1, k1_again); // Deterministic
        assert_ne!(k1, k2); // Scoped per node
        assert_ne!(k1, k_other_secret); // Scoped per deployment
    }

    #[test]
    fn test_derived_key_decrypts_own_fragments_only() {
        let crypto = CryptoProvider::new();
        let secret = b"deployment-secret";
        let n1 = test_node_id(1);
        let n2 = test_node_id(2);
        let nonce = [0x07u8; 12];

        let ciphertext = crypto
            .encrypt(&derive_key(secret, &n1), &nonce, b"fragment payload")
            .unwrap();

        // Node 1's derived key decrypts
        assert!(crypto
            .decrypt(&derive_key(secret, &n1), &nonce, &ciphertext)
            .is_ok());
        // Node 2's derived key does not
        assert_eq!(
            crypto.decrypt(&derive_key(secret, &n2), &nonce, &ciphertext),
            Err(CryptoError::AuthenticationFailed)
        );
    }
}
