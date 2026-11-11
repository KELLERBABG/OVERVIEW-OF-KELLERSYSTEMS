use crate::commit_reveal::CommitReveal;
use crate::types::{EncryptedOrder, FrontRunningError, OrderStatus};
use commitments::hash_commitment::HashOpening;
use std::collections::BTreeMap;

pub struct Mempool {
    orders: BTreeMap<u64, EncryptedOrder>,
    commit_reveal: CommitReveal,
    next_sequence: u64,
}

impl Mempool {
    pub fn new() -> Self {
        Self {
            orders: BTreeMap::new(),
            commit_reveal: CommitReveal::new(),
            next_sequence: 0,
        }
    }

    pub fn submit(&mut self, payload: &[u8], submitter_id: u64) -> (EncryptedOrder, HashOpening) {
        let seq = self.next_sequence;
        self.next_sequence += 1;
        let (order, opening) = self.commit_reveal.commit(payload, submitter_id, seq);
        self.orders.insert(seq, order.clone());
        (order, opening)
    }

    pub fn reveal(
        &mut self,
        commitment_hash: &[u8; 32],
        payload: &[u8],
        opening: &HashOpening,
    ) -> Result<Vec<u8>, FrontRunningError> {
        self.commit_reveal.reveal(commitment_hash, payload, opening)
    }

    pub fn pending_count(&self) -> usize {
        self.commit_reveal
            .commitments
            .iter()
            .filter(|(_, s)| *s == OrderStatus::Committed)
            .count()
    }

    pub fn all_committed(&self) -> Vec<&EncryptedOrder> {
        self.orders.values().collect()
    }

    pub fn is_empty(&self) -> bool {
        self.orders.is_empty()
    }

    pub fn len(&self) -> usize {
        self.orders.len()
    }
}

impl Default for Mempool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mempool_submit_and_pending() {
        let mut mempool = Mempool::new();
        mempool.submit(b"order_1", 1);
        mempool.submit(b"order_2", 2);
        assert_eq!(mempool.pending_count(), 2);
        assert_eq!(mempool.len(), 2);
    }

    #[test]
    fn test_mempool_reveal() {
        let mut mempool = Mempool::new();
        let (order, opening) = mempool.submit(b"secret_order", 1);
        let result = mempool.reveal(&order.commitment_hash, b"secret_order", &opening);
        assert!(result.is_ok());
        assert_eq!(mempool.pending_count(), 0);
    }
}
