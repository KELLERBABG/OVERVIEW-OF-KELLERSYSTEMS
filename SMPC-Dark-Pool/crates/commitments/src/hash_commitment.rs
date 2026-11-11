use crate::traits::CommitmentScheme;
use rand::SeedableRng;
use rand::thread_rng;
use rand_chacha::ChaCha12Rng;
use rand_core::RngCore;
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HashCommitError {
    #[error("invalid commitment length")]
    InvalidLength,
}

pub type HashOutput = [u8; 32];

#[derive(Debug, Clone, PartialEq)]
pub struct HashCommitment {
    pub hash: HashOutput,
}

#[derive(Debug, Clone)]
pub struct HashOpening {
    pub nonce: [u8; 32],
}

pub struct HashCommitScheme;

impl HashCommitScheme {
    pub fn commit_raw(value: &[u8]) -> (HashCommitment, HashOpening) {
        let mut nonce = [0u8; 32];
        let mut rng = ChaCha12Rng::from_rng(thread_rng()).expect("failed to seed RNG");
        rng.fill_bytes(&mut nonce);
        let hash = Self::hash_with_nonce(value, &nonce);
        (HashCommitment { hash }, HashOpening { nonce })
    }

    pub fn verify_raw(comm: &HashCommitment, value: &[u8], opening: &HashOpening) -> bool {
        let expected = Self::hash_with_nonce(value, &opening.nonce);
        expected == comm.hash
    }

    fn hash_with_nonce(value: &[u8], nonce: &[u8; 32]) -> HashOutput {
        let mut hasher = Sha256::new();
        hasher.update(nonce);
        hasher.update(value);
        let result = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&result);
        out
    }
}

impl CommitmentScheme for HashCommitScheme {
    type Commitment = HashCommitment;
    type Opening = HashOpening;
    type Error = HashCommitError;

    fn commit(&self, value: &[u8]) -> (Self::Commitment, Self::Opening) {
        Self::commit_raw(value)
    }

    fn verify(
        &self,
        commitment: &Self::Commitment,
        value: &[u8],
        opening: &Self::Opening,
    ) -> Result<bool, Self::Error> {
        Ok(Self::verify_raw(commitment, value, opening))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_commit_verify() {
        let scheme = HashCommitScheme;
        let value = b"order_secret_42";
        let (comm, opening) = scheme.commit(value);
        assert!(scheme.verify(&comm, value, &opening).unwrap());
    }

    #[test]
    fn test_hash_commit_verify_fails_wrong_value() {
        let scheme = HashCommitScheme;
        let (comm, opening) = scheme.commit(b"secret");
        assert!(!scheme.verify(&comm, b"wrong", &opening).unwrap());
    }

    #[test]
    fn test_hash_commit_deterministic_nonce() {
        let value = b"test_value";
        let (comm1, _) = HashCommitScheme::commit_raw(value);
        let (comm2, _) = HashCommitScheme::commit_raw(value);
        assert_ne!(comm1, comm2);
    }
}
