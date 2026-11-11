use blake3;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};

use crate::error::SybilResult;
use crate::SybilError;

/// Cryptographic primitives used throughout the system.

/// Generate a new ed25519 keypair.
/// Uses OsRng for secure randomness.
pub fn generate_keypair() -> (SigningKey, VerifyingKey) {
    let mut csprng = OsRng {};
    // ed25519-dalek v2: generate via creating a random secret
    let mut secret_bytes = [0u8; 32];
    use rand::RngCore;
    csprng.fill_bytes(&mut secret_bytes);
    let signing_key = SigningKey::from_bytes(&secret_bytes);
    let verifying_key = signing_key.verifying_key();
    (signing_key, verifying_key)
}

/// Sign a message with the given signing key.
pub fn sign_message(signing_key: &SigningKey, message: &[u8]) -> Signature {
    signing_key.sign(message)
}

/// Verify a signature on a message.
pub fn verify_signature(
    verifying_key: &VerifyingKey,
    message: &[u8],
    signature: &Signature,
) -> SybilResult<()> {
    verifying_key
        .verify_strict(message, signature)
        .map_err(|_| SybilError::InvalidSignature)
}

/// Compute a BLAKE3 hash of the input data.
pub fn blake3_hash(data: &[u8]) -> [u8; 32] {
    blake3::hash(data).into()
}

/// Compute a SHA-256 hash of the input data.
pub fn sha256_hash(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Compute a deterministic nullifier from a secret seed.
///
/// The nullifier is `BLAKE3( "nullifier" || seed )`.
/// This ensures the nullifier cannot be linked back to the identity
/// while still being deterministic (same seed → same nullifier).
pub fn compute_nullifier(seed: &[u8]) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"nullifier");
    hasher.update(seed);
    let hash = hasher.finalize();
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(hash.as_bytes())
}

/// Verify that a nullifier matches a seed.
pub fn verify_nullifier(seed: &[u8], nullifier: &str) -> bool {
    compute_nullifier(seed) == nullifier
}

/// Encode a DID string from a verifying key.
pub fn encode_did(verifying_key: &VerifyingKey) -> String {
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(verifying_key.as_bytes());
    format!("did:sybil:{}", encoded)
}

/// Parse a DID string back into its key material (if possible).
pub fn decode_did(did: &str) -> SybilResult<VerifyingKey> {
    use base64::Engine;
    let encoded = did
        .strip_prefix("did:sybil:")
        .ok_or_else(|| SybilError::Did(format!("invalid DID prefix: {}", did)))?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|e| SybilError::Did(format!("base64 decode failed: {}", e)))?;
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| SybilError::Did("invalid key length".to_string()))?;
    VerifyingKey::from_bytes(&arr)
        .map_err(|e| SybilError::Did(format!("invalid verifying key: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_and_verify() {
        let (sk, vk) = generate_keypair();
        let msg = b"hello world";
        let sig = sign_message(&sk, msg);
        assert!(verify_signature(&vk, msg, &sig).is_ok());
    }

    #[test]
    fn test_signature_fails_on_bad_message() {
        let (sk, vk) = generate_keypair();
        let msg = b"hello world";
        let sig = sign_message(&sk, msg);
        assert!(verify_signature(&vk, b"wrong message", &sig).is_err());
    }

    #[test]
    fn test_nullifier_deterministic() {
        let seed = b"my-secret-seed";
        let n1 = compute_nullifier(seed);
        let n2 = compute_nullifier(seed);
        assert_eq!(n1, n2);
    }

    #[test]
    fn test_nullifier_different_for_different_seeds() {
        let n1 = compute_nullifier(b"seed-a");
        let n2 = compute_nullifier(b"seed-b");
        assert_ne!(n1, n2);
    }

    #[test]
    fn test_did_roundtrip() {
        let (_, vk) = generate_keypair();
        let did = encode_did(&vk);
        let decoded = decode_did(&did).unwrap();
        assert_eq!(vk, decoded);
    }

    #[test]
    fn test_blake3_and_sha256_different() {
        let data = b"test data";
        let b3 = blake3_hash(data);
        let s256 = sha256_hash(data);
        assert_ne!(b3, s256);
    }
}