use crate::types::{EncryptedOrder, FrontRunningError, OrderStatus};
use commitments::hash_commitment::{HashCommitScheme, HashOpening};
use rand::thread_rng;
use rand_chacha::ChaCha12Rng;
use rand_core::{RngCore, SeedableRng};
use sha2::{Digest, Sha256};

pub struct CommitReveal {
    pub commitments: Vec<(EncryptedOrder, OrderStatus)>,
    pub openings: Vec<(EncryptedOrder, HashOpening)>,
}

impl CommitReveal {
    pub fn new() -> Self {
        Self {
            commitments: Vec::new(),
            openings: Vec::new(),
        }
    }

    pub fn commit(
        &mut self,
        payload: &[u8],
        submitter_id: u64,
        timestamp: u64,
    ) -> (EncryptedOrder, HashOpening) {
        let mut nonce = [0u8; 32];
        let mut rng = ChaCha12Rng::from_rng(thread_rng()).expect("failed to seed RNG");
        rng.fill_bytes(&mut nonce);
        let (commitment, opening) = HashCommitScheme::commit_raw(payload);
        let encrypted = Self::encrypt_payload(payload, &nonce);
        let order = EncryptedOrder::new(encrypted, commitment.hash, submitter_id, timestamp);
        self.commitments
            .push((order.clone(), OrderStatus::Committed));
        self.openings.push((order.clone(), opening.clone()));
        (order, opening)
    }

    pub fn reveal(
        &mut self,
        commitment_hash: &[u8; 32],
        payload: &[u8],
        opening: &HashOpening,
    ) -> Result<Vec<u8>, FrontRunningError> {
        let valid = HashCommitScheme::verify_raw(
            &commitments::hash_commitment::HashCommitment {
                hash: *commitment_hash,
            },
            payload,
            opening,
        );
        if !valid {
            return Err(FrontRunningError::InvalidCommitment);
        }
        let mut decrypted = None;
        for (order, status) in self.commitments.iter_mut() {
            if order.commitment_hash == *commitment_hash {
                if *status != OrderStatus::Committed {
                    return Err(FrontRunningError::AlreadyRevealed);
                }
                decrypted = Some(Self::decrypt_payload(
                    &order.encrypted_payload,
                    &opening.nonce,
                ));
                *status = OrderStatus::Revealed;
                break;
            }
        }
        decrypted.ok_or(FrontRunningError::OrderNotFound)
    }

    fn encrypt_payload(payload: &[u8], nonce: &[u8; 32]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(nonce);
        hasher.update(payload);
        let key = hasher.finalize();
        payload.iter().zip(key.iter()).map(|(p, k)| p ^ k).collect()
    }

    fn decrypt_payload(encrypted: &[u8], nonce: &[u8; 32]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(nonce);
        hasher.update(b"decrypt");
        let key = hasher.finalize();
        encrypted
            .iter()
            .zip(key.iter())
            .map(|(e, k)| e ^ k)
            .collect()
    }

    pub fn status(&self, commitment_hash: &[u8; 32]) -> Option<OrderStatus> {
        self.commitments
            .iter()
            .find(|(o, _)| o.commitment_hash == *commitment_hash)
            .map(|(_, s)| *s)
    }
}

impl Default for CommitReveal {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_and_reveal() {
        let mut cr = CommitReveal::new();
        let payload = b"limit_order_100_eth_10@2000";
        let (order, opening) = cr.commit(payload, 1, 1000);
        let result = cr.reveal(&order.commitment_hash, payload, &opening);
        assert!(result.is_ok());
        assert_eq!(
            cr.status(&order.commitment_hash),
            Some(OrderStatus::Revealed)
        );
    }

    #[test]
    fn test_reveal_wrong_payload_fails() {
        let mut cr = CommitReveal::new();
        let (order, _real_opening) = cr.commit(b"real_order", 1, 1000);
        let (_, opening) = HashCommitScheme::commit_raw(b"wrong_payload");
        let result = cr.reveal(&order.commitment_hash, b"wrong_payload", &opening);
        assert!(matches!(result, Err(FrontRunningError::InvalidCommitment)));
    }
}
