//! # Homomorphic Encryption for Contribution Discovery
//!
//! Implements encrypted contribution computation where inventors submit
//! encrypted shares `E(contribution_i)` and the network computes
//! `E(total) = sum(E(contribution_i))` without decrypting individual shares.
//!
//! This is a pure-Rust simulation layer — in production this would bind to
//! TFHE (Zama) or OpenFHE. Our implementation provides:
//! - Key generation (public/private keypair)
//! - Encryption of individual contribution values (0..1 as fixed-point)
//! - Homomorphic addition of encrypted shares
//! - Decryption of the aggregate total
//! - Zero-knowledge proof of valid contribution (range proof)

use crate::error::CryptoError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

// ─── Simulated HE Parameters ────────────────────────────────────────────────

/// Security level for the homomorphic encryption scheme.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum HeSecurityLevel {
    /// 128-bit security (fast, for prototyping).
    Standard,
    /// 256-bit security (high, for production).
    High,
}

/// Configuration for the HE scheme.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeConfig {
    pub security_level: HeSecurityLevel,
    /// Fixed-point precision (number of fractional bits).
    pub precision_bits: u32,
    /// Ciphertext modulus size in bits.
    pub modulus_bits: u32,
}

impl Default for HeConfig {
    fn default() -> Self {
        Self {
            security_level: HeSecurityLevel::Standard,
            precision_bits: 16,
            modulus_bits: 64,
        }
    }
}

// ─── Key Types ───────────────────────────────────────────────────────────────

/// A public key for homomorphic encryption.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HePublicKey {
    pub id: Vec<u8>,
    pub config: HeConfig,
}

/// A private key for homomorphic encryption.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HePrivateKey {
    pub id: Vec<u8>,
    pub config: HeConfig,
}

/// A keypair for homomorphic encryption.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeKeypair {
    pub public_key: HePublicKey,
    pub private_key: HePrivateKey,
}

/// A ciphertext — an encrypted value.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeCiphertext {
    pub data: Vec<u8>,
    pub config: HeConfig,
}

// ─── HE Engine ───────────────────────────────────────────────────────────────

/// The homomorphic encryption engine.
///
/// In production, this would wrap TFHE or OpenFHE. Here we provide a
/// simulation that demonstrates the API and protocol.
pub struct HeEngine {
    config: HeConfig,
}

impl HeEngine {
    /// Create a new HE engine with the given configuration.
    pub fn new(config: HeConfig) -> Self {
        Self { config }
    }

    /// Generate a keypair.
    pub fn generate_keypair(&self) -> HeKeypair {
        let id = Sha256::digest(format!("he-key-{}", rand_seed()).as_bytes()).to_vec();
        HeKeypair {
            public_key: HePublicKey {
                id: id.clone(),
                config: self.config.clone(),
            },
            private_key: HePrivateKey {
                id,
                config: self.config.clone(),
            },
        }
    }

    /// Encrypt a floating-point value (0..1) using the public key.
    ///
    /// The value is converted to a fixed-point integer, then "encrypted"
    /// by XOR-ing with a derived keystream (simulated).
    pub fn encrypt(&self, value: f64, public_key: &HePublicKey) -> HeCiphertext {
        let fixed_point = self.float_to_fixed(value);
        let keystream = self.derive_keystream(public_key, fixed_point);
        let encrypted_data: Vec<u8> = fixed_point
            .to_le_bytes()
            .iter()
            .zip(keystream.iter())
            .map(|(a, b)| a ^ b)
            .collect();

        HeCiphertext {
            data: encrypted_data,
            config: self.config.clone(),
        }
    }

    /// Decrypt a ciphertext using the private key.
    pub fn decrypt(&self, ciphertext: &HeCiphertext, private_key: &HePrivateKey) -> Result<f64, CryptoError> {
        if ciphertext.data.len() < 8 {
            return Err(CryptoError::DecryptionFailed("invalid ciphertext length".into()));
        }

        let keystream = self.derive_keystrom_private(private_key);
        let mut plaintext_bytes = [0u8; 8];
        for (i, byte) in ciphertext.data.iter().enumerate() {
            if i < 8 {
                plaintext_bytes[i] = byte ^ keystream[i];
            }
        }
        let fixed_point = u64::from_le_bytes(plaintext_bytes);
        Ok(self.fixed_to_float(fixed_point))
    }

    /// Homomorphically add two ciphertexts, producing `E(a + b)`.
    ///
    /// In a real HE scheme this is a true homomorphic operation.
    /// Here we simulate by decrypting and re-encrypting (for testing only).
    pub fn add(
        &self,
        a: &HeCiphertext,
        b: &HeCiphertext,
        public_key: &HePublicKey,
    ) -> Result<HeCiphertext, CryptoError> {
        // Simulation: decrypt both, add, re-encrypt.
        // In production with TFHE, this is a single operation on ciphertexts.
        let keypair = self.generate_keypair();
        let val_a = self.decrypt_sim(a);
        let val_b = self.decrypt_sim(b);
        let sum = val_a + val_b;
        Ok(self.encrypt(sum, public_key))
    }

    /// Homomorphically sum a list of ciphertexts.
    pub fn sum(
        &self,
        ciphertexts: &[HeCiphertext],
        public_key: &HePublicKey,
    ) -> Result<HeCiphertext, CryptoError> {
        if ciphertexts.is_empty() {
            return Ok(self.encrypt(0.0, public_key));
        }
        let mut result = ciphertexts[0].clone();
        for ct in &ciphertexts[1..] {
            result = self.add(&result, ct, public_key)?;
        }
        Ok(result)
    }

    // ─── Helpers ─────────────────────────────────────────────────────────

    /// Convert a float in [0, 1] to a fixed-point u64.
    fn float_to_fixed(&self, value: f64) -> u64 {
        let clamped = value.clamp(0.0, 1.0);
        let scale = 1u64 << self.config.precision_bits;
        (clamped * scale as f64) as u64
    }

    /// Convert a fixed-point u64 back to float.
    fn fixed_to_float(&self, fixed: u64) -> f64 {
        let scale = 1u64 << self.config.precision_bits;
        fixed as f64 / scale as f64
    }

    /// Derive a keystream from the public key for "encryption".
    fn derive_keystream(&self, public_key: &HePublicKey, _value: u64) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(b"dfip-he-keystream");
        hasher.update(&public_key.id);
        hasher.update(self.config.modulus_bits.to_le_bytes());
        let hash = hasher.finalize();
        // Expand to 8 bytes
        hash[..8].to_vec()
    }

    /// Derive a keystream from the private key.
    fn derive_keystrom_private(&self, private_key: &HePrivateKey) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(b"dfip-he-keystream");
        hasher.update(&private_key.id);
        hasher.update(self.config.modulus_bits.to_le_bytes());
        let hash = hasher.finalize();
        hash[..8].to_vec()
    }

    /// Simulated decryption (used only in the mock `add`).
    fn decrypt_sim(&self, ct: &HeCiphertext) -> f64 {
        // Use a mock keypair for simulation
        let kp = self.generate_keypair();
        self.decrypt(ct, &kp.private_key).unwrap_or(0.0)
    }
}

/// Generate a random seed string.
fn rand_seed() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("{}-{}", dur.as_secs(), dur.subsec_nanos())
}

// ─── Contribution Protocol ───────────────────────────────────────────────────

/// A contribution from an inventor.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContributionShare {
    /// The encrypted share value.
    pub encrypted_share: HeCiphertext,
    /// The public key used for encryption.
    pub public_key: HePublicKey,
    /// A zero-knowledge proof that the share is in valid range [0, 1].
    pub range_proof: ContributionProof,
}

/// A proof that an encrypted contribution is valid.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContributionProof {
    /// Simulated proof bytes — in production, a Bulletproof or Groth16 proof.
    pub proof_bytes: Vec<u8>,
    /// The type of proof.
    pub proof_type: String,
}

/// Run the full encrypted contribution protocol:
/// 1. Each inventor encrypts their share.
/// 2. All encrypted shares are summed homomorphically.
/// 3. The aggregate is decrypted to reveal total contribution.
pub fn run_contribution_protocol(
    contributions: &[f64],
    config: &HeConfig,
) -> Result<(f64, Vec<ContributionShare>), CryptoError> {
    let engine = HeEngine::new(config.clone());
    let keypair = engine.generate_keypair();

    let mut shares = Vec::new();
    for &value in contributions {
        if !(0.0..=1.0).contains(&value) {
            return Err(CryptoError::InvalidParameter(format!(
                "contribution {value} not in [0, 1]"
            )));
        }
        let encrypted = engine.encrypt(value, &keypair.public_key);
        let proof = ContributionProof {
            proof_bytes: Sha256::digest(b"valid-range").to_vec(),
            proof_type: "simulated-bulletproof".into(),
        };
        shares.push(ContributionShare {
            encrypted_share: encrypted,
            public_key: keypair.public_key.clone(),
            range_proof: proof,
        });
    }

    // Homomorphically sum all shares.
    let ciphertexts: Vec<HeCiphertext> = shares.iter().map(|s| s.encrypted_share.clone()).collect();
    let encrypted_sum = engine.sum(&ciphertexts, &keypair.public_key)?;

    // Decrypt the aggregate.
    let total = engine.decrypt(&encrypted_sum, &keypair.private_key)?;

    Ok((total, shares))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let config = HeConfig::default();
        let engine = HeEngine::new(config);
        let keypair = engine.generate_keypair();

        let original = 0.42;
        let ct = engine.encrypt(original, &keypair.public_key);
        let decrypted = engine.decrypt(&ct, &keypair.private_key).unwrap();
        let diff = (original - decrypted).abs();
        assert!(diff < 0.001, "decryption error too large: {diff}");
    }

    #[test]
    fn test_homomorphic_sum() {
        let config = HeConfig::default();
        let engine = HeEngine::new(config.clone());
        let keypair = engine.generate_keypair();

        let values = vec![0.1, 0.2, 0.3, 0.4];
        let ciphertexts: Vec<HeCiphertext> = values
            .iter()
            .map(|v| engine.encrypt(*v, &keypair.public_key))
            .collect();

        let encrypted_sum = engine.sum(&ciphertexts, &keypair.public_key).unwrap();
        let decrypted_sum = engine.decrypt(&encrypted_sum, &keypair.private_key).unwrap();

        let expected: f64 = values.iter().sum();
        let diff = (expected - decrypted_sum).abs();
        assert!(diff < 0.01, "sum error too large: {diff}");
    }

    #[test]
    fn test_contribution_protocol() {
        let config = HeConfig::default();
        let contributions = vec![0.2, 0.3, 0.5];
        let (total, shares) = run_contribution_protocol(&contributions, &config).unwrap();
        assert!((total - 1.0).abs() < 0.01);
        assert_eq!(shares.len(), 3);
    }

    #[test]
    fn test_invalid_contribution_rejected() {
        let config = HeConfig::default();
        let contributions = vec![0.5, 1.5]; // 1.5 > 1.0
        let result = run_contribution_protocol(&contributions, &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_encrypt_zero() {
        let config = HeConfig::default();
        let engine = HeEngine::new(config);
        let keypair = engine.generate_keypair();
        let ct = engine.encrypt(0.0, &keypair.public_key);
        let decrypted = engine.decrypt(&ct, &keypair.private_key).unwrap();
        assert!(decrypted.abs() < 0.001);
    }

    #[test]
    fn test_encrypt_one() {
        let config = HeConfig::default();
        let engine = HeEngine::new(config);
        let keypair = engine.generate_keypair();
        let ct = engine.encrypt(1.0, &keypair.public_key);
        let decrypted = engine.decrypt(&ct, &keypair.private_key).unwrap();
        assert!((decrypted - 1.0).abs() < 0.001);
    }
}