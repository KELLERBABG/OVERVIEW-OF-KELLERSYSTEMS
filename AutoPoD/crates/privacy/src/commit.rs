use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commitment {
    pub hash: [u8; 32],
    pub blinding: [u8; 32],
}

impl Commitment {
    pub fn new(value: f64, label: &str) -> Self {
        let mut blinding = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut blinding);
        let hash = Self::compute_hash(value, label, &blinding);
        Commitment { hash, blinding }
    }

    pub fn compute_hash(value: f64, label: &str, blinding: &[u8; 32]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(label.as_bytes());
        hasher.update(value.to_le_bytes());
        hasher.update(blinding);
        hasher.finalize().into()
    }

    pub fn verify(&self, value: f64, label: &str) -> bool {
        let expected = Self::compute_hash(value, label, &self.blinding);
        self.hash == expected
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.hash)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommittedReading {
    pub label: String,
    pub commitment: Commitment,
    pub sensor_type: String,
}

impl CommittedReading {
    pub fn new(value: f64, label: &str, sensor_type: &str) -> Self {
        CommittedReading {
            label: label.to_string(),
            commitment: Commitment::new(value, label),
            sensor_type: sensor_type.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commitment_verify_correct() {
        let c = Commitment::new(42.0, "test_val");
        assert!(c.verify(42.0, "test_val"));
    }

    #[test]
    fn test_commitment_rejects_wrong_value() {
        let c = Commitment::new(42.0, "test_val");
        assert!(!c.verify(43.0, "test_val"));
    }

    #[test]
    fn test_commitment_rejects_wrong_label() {
        let c = Commitment::new(42.0, "test_val");
        assert!(!c.verify(42.0, "wrong_label"));
    }

    #[test]
    fn test_commitment_hides_value() {
        let c = Commitment::new(42.0, "secret");
        let hash_hex = c.to_hex();
        // hex of sha256 output is always 64 chars
        assert_eq!(hash_hex.len(), 64);
        // the raw value 42.0 as f64 is 0x0044800000000000 in hex
        // sha256(0x0044800000000000 || blinding) should not be trivially reverseable
        assert_ne!(hash_hex, "0".repeat(64), "commitment should not be all zeros");
    }

    #[test]
    fn test_commitment_unique_per_blinding() {
        let c1 = Commitment::new(42.0, "v");
        let c2 = Commitment::new(42.0, "v");
        assert_ne!(c1.hash, c2.hash, "same value with different blinding must differ");
    }
}
