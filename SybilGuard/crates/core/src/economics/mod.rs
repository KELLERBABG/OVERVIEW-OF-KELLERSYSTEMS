//! # Attack-Vector Economics Module
//!
//! Economic and temporal barriers that raise the cost of Sybil attacks:
//!
//! 1. **Vouching Stake:** A node must stake tokens (or commit to a time lock)
//!    before initiating vouching. Stake is slashed if the vouched node is
//!    later confirmed as Sybil.
//!
//! 2. **Time Locks:** New nodes must wait 30 days before vouching. Vouch
//!    changes have a 7-day cooldown.
//!
//! 3. **Slashing Conditions:** False vouching (vouching for a confirmed Sybil)
//!    results in stake slashing and reputation penalty.
//!
//! 4. **Scalability Cost:** The economic cost of creating a Sybil cluster
//!    grows linearly with cluster size (each node needs stake + time),
//!    making large-scale attacks economically infeasible.

use chrono::{DateTime, Duration, Utc};
use crate::{constants::*, NodeId, SybilError, SybilResult};

/// Represents a locked stake for vouching.
#[derive(Debug, Clone)]
pub struct VouchingStake {
    pub node_id: NodeId,
    pub amount: u64,
    pub locked_at: DateTime<Utc>,
    pub unlocks_at: DateTime<Utc>,
    pub is_slashed: bool,
}

/// A slash event recorded when a false vouch is detected.
#[derive(Debug, Clone)]
pub struct SlashEvent {
    pub node_id: NodeId,
    pub reason: String,
    pub amount_lost: u64,
    pub reputation_penalty: f64,
    pub occurred_at: DateTime<Utc>,
}

/// Manages economic Sybil resistance through staking, time locks, and slashing.
pub struct EconomicsEngine {
    stakes: Vec<VouchingStake>,
    slash_history: Vec<SlashEvent>,
    /// Total stake required to initiate vouching.
    stake_requirement: u64,
    /// Time lock duration for new nodes (in days).
    new_node_lock_days: i64,
    /// Cooldown between vouch changes (in days).
    vouch_cooldown_days: i64,
    /// Track last vouch change time per node.
    last_vouch_change: std::collections::HashMap<NodeId, DateTime<Utc>>,
}

impl EconomicsEngine {
    pub fn new(stake_requirement: u64, new_node_lock_days: i64, vouch_cooldown_days: i64) -> Self {
        Self {
            stakes: Vec::new(),
            slash_history: Vec::new(),
            stake_requirement,
            new_node_lock_days,
            vouch_cooldown_days,
            last_vouch_change: std::collections::HashMap::new(),
        }
    }

    /// A node locks stake to enable vouching.
    pub fn lock_stake(&mut self, node_id: NodeId, amount: u64) -> SybilResult<VouchingStake> {
        if amount < self.stake_requirement {
            return Err(SybilError::InvalidParameter(
                format!("Stake {} below minimum {}", amount, self.stake_requirement)
            ));
        }
        let now = Utc::now();
        let stake = VouchingStake {
            node_id,
            amount,
            locked_at: now,
            unlocks_at: now + Duration::days(self.new_node_lock_days),
            is_slashed: false,
        };
        self.stakes.push(stake.clone());
        Ok(stake)
    }

    /// Check if a node can initiate a vouch.
    pub fn can_vouch(&self, node_id: &NodeId) -> bool {
        // Check stake exists and is not slashed
        let has_stake = self.stakes.iter().any(|s| s.node_id == *node_id && !s.is_slashed);
        if !has_stake {
            return false;
        }
        // Check time lock has expired
        let now = Utc::now();
        let lock_expired = self.stakes.iter()
            .filter(|s| s.node_id == *node_id)
            .all(|s| now >= s.unlocks_at);
        if !lock_expired {
            return false;
        }
        // Check cooldown
        if let Some(last_change) = self.last_vouch_change.get(node_id) {
            if now < *last_change + Duration::days(self.vouch_cooldown_days) {
                return false;
            }
        }
        true
    }

    /// Record a vouch change (for cooldown tracking).
    pub fn record_vouch_change(&mut self, node_id: NodeId) {
        self.last_vouch_change.insert(node_id, Utc::now());
    }

    /// Slash a node's stake for false vouching.
    pub fn slash(&mut self, node_id: &NodeId, reason: &str, reputation_penalty: f64) -> SybilResult<SlashEvent> {
        let mut slashed_amount = 0u64;
        for stake in &mut self.stakes {
            if stake.node_id == *node_id && !stake.is_slashed {
                stake.is_slashed = true;
                slashed_amount += stake.amount;
            }
        }
        if slashed_amount == 0 {
            return Err(SybilError::Vouching(format!("No active stake found for {}", node_id)));
        }
        let event = SlashEvent {
            node_id: node_id.clone(),
            reason: reason.to_string(),
            amount_lost: slashed_amount,
            reputation_penalty,
            occurred_at: Utc::now(),
        };
        self.slash_history.push(event.clone());
        Ok(event)
    }

    /// Get total stake locked by a node.
    pub fn total_stake(&self, node_id: &NodeId) -> u64 {
        self.stakes.iter()
            .filter(|s| s.node_id == *node_id && !s.is_slashed)
            .map(|s| s.amount)
            .sum()
    }

    /// Get slash history for a node.
    pub fn slash_history(&self, node_id: &NodeId) -> Vec<&SlashEvent> {
        self.slash_history.iter().filter(|e| e.node_id == *node_id).collect()
    }

    /// Check if a node has been slashed.
    pub fn is_slashed(&self, node_id: &NodeId) -> bool {
        self.stakes.iter().any(|s| s.node_id == *node_id && s.is_slashed)
    }
}

impl Default for EconomicsEngine {
    fn default() -> Self {
        Self::new(VOUCHING_STAKE_REQUIREMENT, NEW_NODE_VOUCHING_LOCK_DAYS, VOUCH_CHANGE_COOLDOWN_DAYS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stake_locking() {
        let mut engine = EconomicsEngine::default();
        let node = NodeId::new();
        let stake = engine.lock_stake(node, 100).unwrap();
        assert_eq!(stake.amount, 100);
        assert!(!stake.is_slashed);
    }

    #[test]
    fn test_insufficient_stake() {
        let mut engine = EconomicsEngine::new(100, 0, 0);
        let node = NodeId::new();
        assert!(engine.lock_stake(node, 50).is_err());
    }

    #[test]
    fn test_cannot_vouch_without_stake() {
        let engine = EconomicsEngine::default();
        let node = NodeId::new();
        assert!(!engine.can_vouch(&node));
    }

    #[test]
    fn test_slashing() {
        let mut engine = EconomicsEngine::default();
        let node = NodeId::new();
        engine.lock_stake(node.clone(), 100).unwrap();
        let event = engine.slash(&node, "false vouch detected", 5.0).unwrap();
        assert_eq!(event.amount_lost, 100);
        assert_eq!(event.reputation_penalty, 5.0);
        assert!(engine.is_slashed(&node));
    }

    #[test]
    fn test_double_slash_not_allowed() {
        let mut engine = EconomicsEngine::default();
        let node = NodeId::new();
        engine.lock_stake(node.clone(), 100).unwrap();
        engine.slash(&node, "reason", 1.0).unwrap();
        assert!(engine.slash(&node, "again", 1.0).is_err());
    }
}