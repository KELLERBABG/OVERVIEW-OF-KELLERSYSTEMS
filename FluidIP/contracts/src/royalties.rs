//! # RoyaltySplitter
//!
//! Invariant-enforced payment distribution contract.
//! Splits payments according to the royalty invariants computed by
//! `dfip-core::royalties`.

use dfip_core::models::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A payment entry in the royalty splitter.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaymentEntry {
    /// The recipient node ID (inventor).
    pub recipient_node_id: Uuid,
    /// The recipient address.
    pub recipient: String,
    /// The amount owed (in micro-tokens).
    pub amount: u64,
    /// Whether this payment has been claimed.
    pub claimed: bool,
}

/// The royalty splitter contract state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoyaltySplitter {
    /// Map from patent node ID → share entries (fractional weights).
    invariants: HashMap<Uuid, RoyaltyInvariant>,
    /// Pending payments: (node_id, payment_id) → PaymentEntry.
    pending_payments: HashMap<(Uuid, Uuid), PaymentEntry>,
    /// Total balance available for distribution.
    balance: u64,
}

impl RoyaltySplitter {
    /// Create a new empty splitter.
    pub fn new() -> Self {
        Self {
            invariants: HashMap::new(),
            pending_payments: HashMap::new(),
            balance: 0,
        }
    }

    /// Register a royalty invariant for a patent.
    pub fn register_invariant(&mut self, invariant: RoyaltyInvariant) {
        self.invariants.insert(invariant.patent_node_id, invariant);
    }

    /// Distribute a payment for a derivative usage.
    /// Returns the list of payment entries created.
    pub fn distribute(
        &mut self,
        patent_node_id: &Uuid,
        total_amount: u64,
        recipients: &HashMap<Uuid, String>, // node_id → address
    ) -> Vec<PaymentEntry> {
        let mut entries = Vec::new();

        if let Some(invariant) = self.invariants.get(patent_node_id) {
            for share in &invariant.shares {
                let amount = (total_amount as f64 * share.weight) as u64;
                let address = recipients
                    .get(&share.node_id)
                    .cloned()
                    .unwrap_or_else(|| "0xUnknown".to_string());

                let entry = PaymentEntry {
                    recipient_node_id: share.node_id,
                    recipient: address,
                    amount,
                    claimed: false,
                };

                let payment_id = Uuid::new_v4();
                self.pending_payments
                    .insert((*patent_node_id, payment_id), entry.clone());
                entries.push(entry);
            }
        }

        self.balance += total_amount;
        entries
    }

    /// Claim a pending payment.
    pub fn claim(
        &mut self,
        node_id: &Uuid,
        payment_id: &Uuid,
    ) -> Option<PaymentEntry> {
        if let Some(entry) = self.pending_payments.get_mut(&(*node_id, *payment_id)) {
            if !entry.claimed && self.balance >= entry.amount {
                entry.claimed = true;
                self.balance -= entry.amount;
                return Some(entry.clone());
            }
        }
        None
    }

    /// Get all pending payments for a patent node.
    pub fn get_pending(&self, node_id: &Uuid) -> Vec<&PaymentEntry> {
        self.pending_payments
            .iter()
            .filter(|((nid, _), _)| nid == node_id)
            .map(|(_, entry)| entry)
            .collect()
    }

    /// Get the remaining balance.
    pub fn balance(&self) -> u64 {
        self.balance
    }
}

impl Default for RoyaltySplitter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dfip_core::models::{RoyaltyInvariant, ShareEntry};
    use uuid::Uuid;

    #[test]
    fn test_distribute_and_claim() {
        let mut splitter = RoyaltySplitter::new();

        let node_id = Uuid::new_v4();
        let alice = Uuid::new_v4();
        let bob = Uuid::new_v4();

        let invariant = RoyaltyInvariant {
            patent_node_id: node_id,
            shares: vec![
                ShareEntry { node_id: alice, weight: 0.7 },
                ShareEntry { node_id: bob, weight: 0.3 },
            ],
            upstream_nodes: vec![alice, bob],
        };

        splitter.register_invariant(invariant);

        let mut recipients = HashMap::new();
        recipients.insert(alice, "0xAlice".to_string());
        recipients.insert(bob, "0xBob".to_string());

        let entries = splitter.distribute(&node_id, 1000, &recipients);
        assert_eq!(entries.len(), 2);
        assert_eq!(splitter.balance(), 1000);

        // Alice should get 700
        let alice_entry = entries.iter().find(|e| e.recipient == "0xAlice").unwrap();
        assert_eq!(alice_entry.amount, 700);
    }

    #[test]
    fn test_claim() {
        let mut splitter = RoyaltySplitter::new();
        let node_id = Uuid::new_v4();
        let alice = Uuid::new_v4();

        let invariant = RoyaltyInvariant {
            patent_node_id: node_id,
            shares: vec![ShareEntry { node_id: alice, weight: 1.0 }],
            upstream_nodes: vec![alice],
        };
        splitter.register_invariant(invariant);

        let mut recipients = HashMap::new();
        recipients.insert(alice, "0xAlice".to_string());
        let entries = splitter.distribute(&node_id, 500, &recipients);

        // Find the payment ID for Alice
        let payment_id = splitter
            .pending_payments
            .iter()
            .find(|(_, e)| e.recipient == "0xAlice")
            .map(|((_, pid), _)| *pid)
            .unwrap();

        let claimed = splitter.claim(&node_id, &payment_id).unwrap();
        assert_eq!(claimed.amount, 500);
        assert!(claimed.claimed);
        assert_eq!(splitter.balance(), 0);
    }
}