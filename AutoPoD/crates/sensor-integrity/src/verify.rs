use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

use crate::chain::ChainLink;

#[derive(Debug, thiserror::Error)]
pub enum IntegrityError {
    #[error("Invalid signature at link {0}")]
    BadSignature(u64),
    #[error("Hash chain broken at link {index}: expected {expected}, got {actual}")]
    HashMismatch { index: u64, expected: String, actual: String },
    #[error("Key error: {0}")]
    Key(String),
}

pub struct IntegrityVerifier;

impl IntegrityVerifier {
    pub fn verify_chain(links: &[ChainLink], public_key: &[u8]) -> Result<(), IntegrityError> {
        if links.is_empty() {
            return Ok(());
        }
        let verifying_key = VerifyingKey::from_bytes(
            public_key.try_into().map_err(|_| IntegrityError::Key("invalid key length".into()))?,
        )
        .map_err(|e| IntegrityError::Key(e.to_string()))?;

        let mut prev_hash: Vec<u8> = Vec::new();

        for link in links {
            let msg = [&link.index.to_le_bytes()[..], &link.hash[..]].concat();
            let sig = Signature::from_slice(&link.signature)
                .map_err(|_| IntegrityError::BadSignature(link.index))?;

            verifying_key
                .verify(&msg, &sig)
                .map_err(|_| IntegrityError::BadSignature(link.index))?;

            let mut hasher = Sha256::new();
            hasher.update(link.index.to_le_bytes());
            hasher.update(link.reading.timestamp.to_le_bytes());
            hasher.update(link.reading.value.to_le_bytes());
            hasher.update(&prev_hash);
            let computed_hash = hasher.finalize().to_vec();

            if computed_hash != link.hash {
                return Err(IntegrityError::HashMismatch {
                    index: link.index,
                    expected: hex::encode(&link.hash),
                    actual: hex::encode(&computed_hash),
                });
            }

            prev_hash = link.hash.clone();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::SigningChain;
    use pod_core::sensor::{SensorReading, SensorType};
    use uuid::Uuid;

    fn test_reading(value: f64, ts: i64) -> SensorReading {
        SensorReading {
            sensor_id: Uuid::nil(),
            sensor_type: SensorType::Accelerometer,
            timestamp: ts,
            value,
            unit: "g".into(),
        }
    }

    #[test]
    fn test_verify_valid_chain() {
        let mut chain = SigningChain::new();
        chain.add_reading(test_reading(1.0, 1000)).unwrap();
        chain.add_reading(test_reading(2.0, 2000)).unwrap();
        chain.add_reading(test_reading(3.0, 3000)).unwrap();

        let pk = chain.public_key_bytes();
        assert!(IntegrityVerifier::verify_chain(chain.links(), &pk).is_ok());
    }

    #[test]
    fn test_verify_empty_chain() {
        let chain = SigningChain::new();
        let pk = chain.public_key_bytes();
        assert!(IntegrityVerifier::verify_chain(chain.links(), &pk).is_ok());
    }

    #[test]
    fn test_verify_rejects_tampered_hash() {
        let mut chain = SigningChain::new();
        chain.add_reading(test_reading(1.0, 1000)).unwrap();
        chain.add_reading(test_reading(2.0, 2000)).unwrap();

        let mut links: Vec<_> = chain.links().to_vec();
        links[0].hash = vec![0u8; 32];

        let pk = chain.public_key_bytes();
        assert!(IntegrityVerifier::verify_chain(&links, &pk).is_err());
    }

    #[test]
    fn test_verify_rejects_tampered_signature() {
        let mut chain = SigningChain::new();
        chain.add_reading(test_reading(1.0, 1000)).unwrap();

        let mut links: Vec<_> = chain.links().to_vec();
        links[0].signature = vec![0u8; 64];

        let pk = chain.public_key_bytes();
        let result = IntegrityVerifier::verify_chain(&links, &pk);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_rejects_wrong_key() {
        let mut chain = SigningChain::new();
        chain.add_reading(test_reading(1.0, 1000)).unwrap();

        let wrong_key = vec![0u8; 32];
        let result = IntegrityVerifier::verify_chain(chain.links(), &wrong_key);
        assert!(result.is_err());
    }
}
