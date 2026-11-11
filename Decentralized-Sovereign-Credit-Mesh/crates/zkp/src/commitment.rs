use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commitment {
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opening {
    pub secret: String,
    pub blinding: String,
}

pub fn commit(secret: &[u8], blinding: &[u8]) -> Commitment {
    let mut hasher = Sha256::new();
    hasher.update(b"COMMIT:v1:");
    hasher.update(secret);
    hasher.update(b":");
    hasher.update(blinding);
    let value = format!("{:x}", hasher.finalize());
    Commitment { value }
}

pub fn open(commitment: &Commitment, secret: &[u8], blinding: &[u8]) -> bool {
    let expected = commit(secret, blinding);
    commitment.value == expected.value
}

pub fn generate_blinding() -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; 32];
    rng.fill(&mut bytes);
    bytes.to_vec()
}

pub fn commit_value(value: u64, label: &str) -> (Commitment, Opening) {
    let secret = format!("{}_{}", label, value);
    let blinding_bytes = generate_blinding();
    let blinding = format!("{:x}", Sha256::digest(&blinding_bytes));
    let commitment = commit(secret.as_bytes(), blinding.as_bytes());
    let opening = Opening { secret, blinding };
    (commitment, opening)
}
