//! Custody chain data structures.
//!
//! Each record in the chain represents a single custody transfer event,
//! cryptographically signed and linked to the previous event via SHA-256 hash.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A single link in the custody chain.
///
/// Each record contains:
/// - The payload ID being transferred
/// - Location and timestamp of the transfer
/// - The custodian taking possession
/// - The custodian's verifying key (ed25519 public key)
/// - SHA-256 hash of the previous record (links the chain)
/// - Sensor readings at the time of transfer
/// - ed25519 signature over the record contents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodyRecord {
    /// The payload/shipment ID.
    pub payload_id: uuid::Uuid,
    /// Location where custody was transferred.
    pub location_id: uuid::Uuid,
    /// Unix timestamp.
    pub timestamp: i64,
    /// Entity taking custody.
    pub custodian: String,
    /// ed25519 public key of the custodian.
    pub verifying_key: Vec<u8>,
    /// SHA-256 hash of the previous record (empty bytes for first).
    pub previous_hash: Vec<u8>,
    /// Sensor readings at transfer time (e.g., temperature, humidity).
    pub sensor_readings: Vec<(String, String)>,
    /// ed25519 signature over the serialized record data.
    pub signature: Signature,
    /// SHA-256 hash of this record (used by next record).
    pub hash: Vec<u8>,
}

impl CustodyRecord {
    /// Create a new unsigned custody record.
    pub fn new(
        payload_id: uuid::Uuid,
        location_id: uuid::Uuid,
        timestamp: i64,
        custodian: String,
        verifying_key: VerifyingKey,
        previous_hash: Option<Vec<u8>>,
        sensor_readings: Vec<(String, String)>,
    ) -> Self {
        Self {
            payload_id,
            location_id,
            timestamp,
            custodian,
            verifying_key: verifying_key.to_bytes().to_vec(),
            previous_hash: previous_hash.unwrap_or_default(),
            sensor_readings,
            signature: Signature::from_components([0u8; 32], [0u8; 32]),
            hash: Vec::new(),
        }
    }

    /// Sign this record with the custodian's private key.
    /// Computes and stores both the signature and the record hash.
    pub fn sign(mut self, signing_key: &SigningKey) -> Self {
        let data = self.data_to_sign();
        self.hash = Sha256::digest(&data).to_vec();
        self.signature = signing_key.sign(&self.hash);
        self
    }

    /// Verify the signature on this record.
    /// Recomputes the hash from current data and verifies the signature.
    pub fn verify_signature(&self) -> Result<(), ed25519_dalek::SignatureError> {
        // Recompute hash from current data to detect tampering
        let recomputed_hash = Sha256::digest(self.data_to_sign()).to_vec();
        if recomputed_hash != self.hash {
            return Err(ed25519_dalek::SignatureError::from_source("Record data tampered: hash mismatch".to_string()));
        }
        let verifying_bytes: [u8; 32] = self
            .verifying_key
            .as_slice()
            .try_into()
            .map_err(|_| ed25519_dalek::SignatureError::from_source("Invalid key length".to_string()))?;
        let verifying_key = VerifyingKey::from_bytes(&verifying_bytes)?;
        verifying_key.verify(&self.hash, &self.signature)
    }

    /// Serialize record data that is signed (everything except signature and hash).
    pub fn data_to_sign(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&self.payload_id.as_bytes()[..]);
        data.extend_from_slice(&self.location_id.as_bytes()[..]);
        data.extend_from_slice(&self.timestamp.to_le_bytes()[..]);
        data.extend_from_slice(self.custodian.as_bytes());
        data.extend_from_slice(&self.verifying_key);
        data.extend_from_slice(&self.previous_hash);
        for (k, v) in &self.sensor_readings {
            data.extend_from_slice(k.as_bytes());
            data.extend_from_slice(v.as_bytes());
        }
        data
    }
}

/// A complete chain of custody records for a payload.
#[derive(Debug, Clone)]
pub struct CustodyChain {
    records: Vec<CustodyRecord>,
}

impl CustodyChain {
    pub fn new() -> Self {
        Self { records: Vec::new() }
    }

    /// Add a record to the chain.
    pub fn push(&mut self, record: CustodyRecord) {
        self.records.push(record);
    }

    /// Get all records.
    pub fn records(&self) -> &[CustodyRecord] {
        &self.records
    }

    /// The number of custody events.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Check if the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Verify the entire chain: each signature and each hash linkage.
    pub fn verify(&self) -> Result<(), String> {
        for (i, record) in self.records.iter().enumerate() {
            record.verify_signature().map_err(|e| format!("Record {i}: signature verification failed: {e}"))?;

            if i > 0 {
                let expected_hash = &self.records[i - 1].hash;
                if record.previous_hash.as_slice() != expected_hash.as_slice() {
                    return Err(format!("Record {i}: chain broken — hash mismatch at index {i}"));
                }
            }
        }
        Ok(())
    }
}

impl Default for CustodyChain {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoIterator for CustodyChain {
    type Item = CustodyRecord;
    type IntoIter = std::vec::IntoIter<CustodyRecord>;

    fn into_iter(self) -> Self::IntoIter {
        self.records.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracker::generate_test_key;

    #[test]
    fn test_sign_and_verify() {
        let key = generate_test_key();
        let record = CustodyRecord::new(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            1000,
            "Test Custodian".into(),
            key.verifying_key(),
            None,
            vec![],
        );

        let signed = record.sign(&key);
        assert!(signed.verify_signature().is_ok());
    }

    #[test]
    fn test_chain_verification() {
        let key1 = generate_test_key();
        let key2 = generate_test_key();

        let r1 = CustodyRecord::new(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            1000,
            "A".into(),
            key1.verifying_key(),
            None,
            vec![],
        )
        .sign(&key1);

        let r2 = CustodyRecord::new(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            2000,
            "B".into(),
            key2.verifying_key(),
            Some(r1.hash.clone()),
            vec![],
        )
        .sign(&key2);

        let mut chain = CustodyChain::new();
        chain.push(r1);
        chain.push(r2);

        assert!(chain.verify().is_ok());
    }

    #[test]
    fn test_broken_chain() {
        let key = generate_test_key();

        let r1 = CustodyRecord::new(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            1000,
            "A".into(),
            key.verifying_key(),
            None,
            vec![],
        )
        .sign(&key);

        // r2 points to a wrong hash
        let r2 = CustodyRecord::new(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            2000,
            "B".into(),
            key.verifying_key(),
            Some(vec![1, 2, 3, 4]), // wrong hash
            vec![],
        )
        .sign(&key);

        let mut chain = CustodyChain::new();
        chain.push(r1);
        chain.push(r2);

        assert!(chain.verify().is_err());
    }
}
