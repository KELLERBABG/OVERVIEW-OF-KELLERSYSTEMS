//! **City incentivized node hosting** — proof-of-uptime and energy-credit
//! rewards.
//!
//! A node earns credits for staying online. Each reward epoch, the node's
//! uptime is recorded as a MAC-signed `UptimeClaim`. Credits accrue from the
//! verified claims and are paid out via a ledger (the DAG in production; a
//! fixed table here). A claim is rejected if:
//!   - the MAC is invalid (forged),
//!   - the node was not registered, or
//!   - the claim window is out of order (replay).
//!
//! All state is static; no allocation.

#![no_std]

#[cfg(test)]
extern crate std;

use city_crypto::mac::{city_mac, MacKey};

pub const INCENT_MAX_NODES: usize = 8;
pub const INCENT_MAX_CLAIMS: usize = 8;
/// Domain label for the incentive ledger's MAC key (host profile).
pub const INCENT_LABEL: u32 = 0xC01DE;
pub const INCENT_CREDITS_PER_UPTIME: u32 = 10;

fn incent_key() -> MacKey {
    MacKey::derive(INCENT_LABEL)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct UptimeClaim {
    pub node: u32,
    pub window: u32,
    pub uptime_ticks: u32,
    pub reward: u32,
    pub mac: u64,
}

impl UptimeClaim {
    fn body(&self) -> [u8; 12] {
        let mut b = [0u8; 12];
        b[0..4].copy_from_slice(&self.node.to_le_bytes());
        b[4..8].copy_from_slice(&self.window.to_le_bytes());
        b[8..12].copy_from_slice(&self.uptime_ticks.to_le_bytes());
        b
    }

    pub fn compute_mac(&self) -> u64 {
        city_mac(&incent_key(), self.node, 5, self.window, &self.body())
    }

    pub fn is_valid(&self) -> bool {
        self.mac == self.compute_mac()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct HostNode {
    pub id: u32,
    pub credits: u32,
    pub last_window: u32,
}

impl HostNode {
    pub const fn empty() -> Self {
        Self {
            id: 0,
            credits: 0,
            last_window: 0,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct IncentiveLedger {
    nodes: [HostNode; INCENT_MAX_NODES],
    count: usize,
    claims: [UptimeClaim; INCENT_MAX_CLAIMS],
    claim_count: usize,
}

impl Default for IncentiveLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl IncentiveLedger {
    pub const fn new() -> Self {
        Self {
            nodes: [HostNode::empty(); INCENT_MAX_NODES],
            count: 0,
            claims: [UptimeClaim {
                node: 0,
                window: 0,
                uptime_ticks: 0,
                reward: 0,
                mac: 0,
            }; INCENT_MAX_CLAIMS],
            claim_count: 0,
        }
    }

    pub fn register(&mut self, id: u32) -> bool {
        if self.count >= INCENT_MAX_NODES || self.nodes.iter().any(|n| n.id == id) {
            return false;
        }
        self.nodes[self.count] = HostNode {
            id,
            credits: 0,
            last_window: 0,
        };
        self.count += 1;
        true
    }

    fn node_index(&self, id: u32) -> Option<usize> {
        self.nodes.iter().take(self.count).position(|n| n.id == id)
    }

    /// Submit a signed uptime claim; on success credits are credited to the
    /// node's account. Returns the new credit balance, or None if rejected.
    pub fn submit(&mut self, claim: &UptimeClaim) -> Option<u32> {
        if !claim.is_valid() {
            return None;
        }
        let idx = self.node_index(claim.node)?;
        if claim.window <= self.nodes[idx].last_window {
            return None; // replay / out-of-order window
        }
        if self.claim_count >= INCENT_MAX_CLAIMS {
            return None;
        }
        self.claims[self.claim_count] = *claim;
        self.claim_count += 1;

        let reward = claim.uptime_ticks * INCENT_CREDITS_PER_UPTIME;
        self.nodes[idx].credits += reward;
        self.nodes[idx].last_window = claim.window;
        Some(self.nodes[idx].credits)
    }

    pub fn credits(&self, id: u32) -> Option<u32> {
        self.node_index(id).map(|i| self.nodes[i].credits)
    }

    pub fn claims(&self) -> impl Iterator<Item = &UptimeClaim> {
        self.claims.iter().take(self.claim_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uptime_claim_rewards_and_rejects_forgery() {
        let mut ledger = IncentiveLedger::new();
        assert!(ledger.register(1));

        let mut claim = UptimeClaim {
            node: 1,
            window: 1,
            uptime_ticks: 100,
            reward: 0,
            mac: 0,
        };
        claim.reward = claim.uptime_ticks * INCENT_CREDITS_PER_UPTIME;
        claim.mac = claim.compute_mac();
        assert_eq!(ledger.submit(&claim), Some(1000));

        // Replay of the same window is rejected.
        assert!(ledger.submit(&claim).is_none());

        // A forged claim (wrong MAC) is rejected.
        let mut forged = claim;
        forged.uptime_ticks = 1000;
        forged.mac = forged.compute_mac();
        forged.mac ^= 1;
        assert!(ledger.submit(&forged).is_none());

        // Out-of-order window rejected.
        let mut back = claim;
        back.window = 0;
        back.mac = back.compute_mac();
        assert!(ledger.submit(&back).is_none());
    }
}
