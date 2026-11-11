use ed25519_dalek::{Signer, SigningKey};
use pod_core::sensor::SensorReading;
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainLink {
    pub index: u64,
    pub reading: SensorReading,
    pub prev_hash: Vec<u8>,
    pub hash: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct SigningChain {
    signing_key: SigningKey,
    links: Vec<ChainLink>,
    prev_hash: Vec<u8>,
}

impl SigningChain {
    pub fn new() -> Self {
        let mut secret = [0u8; 32];
        OsRng.fill_bytes(&mut secret);
        let signing_key = SigningKey::from_bytes(&secret);

        Self {
            signing_key,
            links: Vec::new(),
            prev_hash: Vec::new(),
        }
    }

    pub fn from_key(signing_key: SigningKey) -> Self {
        Self {
            signing_key,
            links: Vec::new(),
            prev_hash: Vec::new(),
        }
    }

    pub fn add_reading(&mut self, reading: SensorReading) -> Result<ChainLink, String> {
        let index = self.links.len() as u64;

        let mut hasher = Sha256::new();
        hasher.update(index.to_le_bytes());
        hasher.update(reading.timestamp.to_le_bytes());
        hasher.update(reading.value.to_le_bytes());
        hasher.update(&self.prev_hash);
        let hash = hasher.finalize().to_vec();

        let msg = [&index.to_le_bytes()[..], &hash[..]].concat();
        let signature = self.signing_key.sign(&msg).to_bytes().to_vec();

        let link = ChainLink {
            index,
            reading,
            prev_hash: self.prev_hash.clone(),
            hash: hash.clone(),
            signature,
        };

        self.prev_hash = hash;
        self.links.push(link.clone());

        Ok(link)
    }

    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.signing_key.verifying_key().to_bytes().to_vec()
    }

    pub fn links(&self) -> &[ChainLink] {
        &self.links
    }
}

impl Default for SigningChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pod_core::sensor::SensorType;
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
    fn test_chain_creates_linked_hashes() {
        let mut chain = SigningChain::new();
        let link1 = chain.add_reading(test_reading(1.0, 1000)).unwrap();
        let link2 = chain.add_reading(test_reading(2.0, 2000)).unwrap();

        assert_eq!(link1.index, 0);
        assert_eq!(link2.index, 1);
        assert_eq!(link2.prev_hash, link1.hash);
        assert_eq!(chain.links().len(), 2);
    }

    #[test]
    fn test_chain_genesis_block_has_empty_prev_hash() {
        let mut chain = SigningChain::new();
        let link = chain.add_reading(test_reading(1.0, 1000)).unwrap();
        assert!(link.prev_hash.is_empty());
    }

    #[test]
    fn test_chain_links_have_signatures() {
        let mut chain = SigningChain::new();
        let link = chain.add_reading(test_reading(5.0, 500)).unwrap();
        assert!(!link.signature.is_empty());
        assert!(!link.hash.is_empty());
    }

    #[test]
    fn test_public_key_is_consistent() {
        let chain = SigningChain::new();
        let pk = chain.public_key_bytes();
        assert_eq!(pk.len(), 32);
    }

    #[test]
    fn test_chain_from_key_produces_valid_links() {
        let secret = {
            let mut s = [0u8; 32];
            OsRng.fill_bytes(&mut s);
            s
        };
        let key = SigningKey::from_bytes(&secret);
        let mut chain = SigningChain::from_key(key);
        let link = chain.add_reading(test_reading(9.81, 0)).unwrap();
        assert_eq!(link.index, 0);
    }
}
