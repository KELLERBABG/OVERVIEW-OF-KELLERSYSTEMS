//! **City self-healing / resilience** — automatic failover when district
//! nodes go offline.
//!
//! Each registered node carries a stationary `offline_ticks` counter. A
//! heartbeat feeds the service; when a node exceeds the failover threshold,
//! the service "fails over" by revoking that node's service role and
//! re-assigning it to the next healthy standby node — the same capability
//! revocation model KELLER uses for sandboxed drivers, applied at the
//! district service level.
//!
//! No allocation; all state is a fixed table.

#![no_std]

#[cfg(test)]
extern crate std;

use city_crypto::{derive_entropy, entropy_seed};

pub const HEAL_MAX_NODES: usize = 8;
pub const HEAL_DEFAULT_THRESHOLD_TICKS: u32 = 10;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NodeHealth {
    pub id: u32,
    /// Which district role this node currently covers.
    pub role: u32,
    pub offline_ticks: u32,
    pub healthy: bool,
    /// Set when this node is marked as the failover standby.
    pub standby: bool,
}

impl NodeHealth {
    pub const fn empty() -> Self {
        Self {
            id: 0,
            role: 0,
            offline_ticks: 0,
            healthy: false,
            standby: false,
        }
    }
}

/// Fixed-capacity health table.
#[derive(Copy, Clone, Debug)]
pub struct HealthTable {
    nodes: [NodeHealth; HEAL_MAX_NODES],
    count: usize,
    threshold: u32,
}

impl Default for HealthTable {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthTable {
    pub const fn new() -> Self {
        Self {
            nodes: [NodeHealth::empty(); HEAL_MAX_NODES],
            count: 0,
            threshold: HEAL_DEFAULT_THRESHOLD_TICKS,
        }
    }

    pub const fn with_threshold(threshold: u32) -> Self {
        Self {
            nodes: [NodeHealth::empty(); HEAL_MAX_NODES],
            count: 0,
            threshold,
        }
    }

    pub fn count(&self) -> usize {
        self.count
    }

    /// Register a node with a district role.
    pub fn register(&mut self, id: u32, role: u32) -> bool {
        if self.count >= HEAL_MAX_NODES || self.nodes.iter().any(|n| n.healthy && n.id == id) {
            return false;
        }
        self.nodes[self.count] = NodeHealth {
            id,
            role,
            offline_ticks: 0,
            healthy: true,
            standby: false,
        };
        self.count += 1;
        true
    }

    pub fn get(&self, id: u32) -> Option<&NodeHealth> {
        self.nodes.iter().take(self.count).find(|n| n.id == id)
    }

    fn slot_of(&self, id: u32) -> Option<usize> {
        self.nodes.iter().take(self.count).position(|n| n.id == id)
    }

    /// A heartbeat from a node marks it alive (resets offline counter).
    pub fn heartbeat(&mut self, id: u32) -> bool {
        match self.slot_of(id) {
            Some(i) => {
                self.nodes[i].offline_ticks = 0;
                self.nodes[i].healthy = true;
                true
            }
            None => false,
        }
    }

    /// Advance the clock one tick; mark stalled *active-role* nodes unhealthy.
    /// Standby nodes are not yet responsible for a role, so they don't accrue
    /// offline time until promoted (that would make failover impossible).
    pub fn tick(&mut self) {
        for i in 0..self.count {
            #[allow(clippy::suspicious_operation_groupings)]
            if self.nodes[i].healthy && !self.nodes[i].standby {
                self.nodes[i].offline_ticks += 1;
                if self.nodes[i].offline_ticks >= self.threshold {
                    self.nodes[i].healthy = false;
                }
            }
        }
    }

    /// Trigger a failover for a role: pick the first healthy *standby* node
    /// that (a) is alive and (b) does not already own the role, and promote
    /// it. Returns the promoted node id.
    pub fn failover(&mut self, role: u32) -> Option<u32> {
        // Demote any dead holder of this role first.
        for i in 0..self.count {
            if self.nodes[i].role == role && !self.nodes[i].healthy {
                // Role is now vacant.
                break;
            }
        }
        for i in 0..self.count {
            if self.nodes[i].healthy
                && self.nodes[i].standby
                && self.nodes[i].role != role
                && !self.nodes.iter().any(|n| n.role == role && n.healthy)
            {
                self.nodes[i].role = role;
                self.nodes[i].standby = false;
                return Some(self.nodes[i].id);
            }
        }
        None
    }

    /// How many healthy nodes cover a role (redundancy for the dashboard).
    pub fn redundancy(&self, role: u32) -> usize {
        self.nodes
            .iter()
            .take(self.count)
            .filter(|n| n.role == role && n.healthy)
            .count()
    }

    /// Mark a node as the designated standby for a role.
    pub fn set_standby(&mut self, id: u32) -> bool {
        match self.slot_of(id) {
            Some(i) => {
                self.nodes[i].standby = true;
                true
            }
            None => false,
        }
    }
}

/// Pseudo-random but deterministic node id for a district failover set —
/// used by the demo to stand up a standby.
pub fn pick_standby(seed: u32) -> u32 {
    (derive_entropy(&entropy_seed(seed))[0] % 900_000) + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failover_promotes_standby() {
        let mut table = HealthTable::with_threshold(5);
        assert!(table.register(1, 10));
        assert!(table.register(2, 0)); // standby coverage for role 10
        assert!(table.set_standby(2));
        assert!(table.heartbeat(1));

        // Node 1 goes silent past the threshold.
        for _ in 0..6 {
            table.tick();
        }
        assert!(table.get(1).map(|n| !n.healthy).unwrap_or(false));

        // Failover promotes node 2 to role 10.
        let promoted = table.failover(10).expect("standby promoted");
        assert_eq!(promoted, 2);
        assert_eq!(table.redundancy(10), 1);
    }

    #[test]
    fn heartbeat_revives_node() {
        let mut table = HealthTable::with_threshold(3);
        table.register(1, 7);
        for _ in 0..4 {
            table.tick();
        }
        assert!(!table.get(1).unwrap().healthy);
        assert!(table.heartbeat(1));
        assert!(table.get(1).unwrap().healthy);
        assert_eq!(table.get(1).unwrap().offline_ticks, 0);
    }
}
