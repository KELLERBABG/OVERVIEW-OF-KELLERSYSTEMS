//! **City autonomous resource allocation** — energy/water demand prediction
//! and smart-contract enforcement.
//!
//! A per-district ledger tracks demand time-series in fixed slots (matching
//! KELLER's `SLOT_COUNT = 4` temporal windows). A simple moving average
//! predicts the next window's demand; allocations are then granted to nodes
//! proportionally, with a scarcity flag when demand would exceed the district
//! budget. Enforcement is by capability: every grant is keyed to the
//! `CAP_CITY_ALLOC` slot, the same mechanism KELLER uses for runtime
//! revocation.
//!
//! No allocation, no floating point — fixed-point u32 arithmetic only.

#![no_std]

#[cfg(test)]
extern crate std;

use city_crypto::mac::{city_mac, MacKey};

pub const ALLOC_SLOTS: usize = 4; // aligned with KELLER SLOT_COUNT
pub const ALLOC_NODES: usize = 8;
/// Domain label for the allocator's MAC key (host profile).
pub const ALLOC_LABEL: u32 = 0xCA11CA7E;

fn alloc_key() -> MacKey {
    MacKey::derive(ALLOC_LABEL)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DemandWindow {
    pub slot: u32,
    pub demand: u32, // units
    pub supply: u32, // units
    pub scarcity: bool,
}

impl DemandWindow {
    pub const fn empty() -> Self {
        Self {
            slot: 0,
            demand: 0,
            supply: 0,
            scarcity: false,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AllocRecord {
    pub node: u32,
    pub window: u32,
    pub granted: u32,
    pub mac: u64,
}

impl AllocRecord {
    pub fn compute_mac(&self) -> u64 {
        let mut body = [0u8; 12];
        body[0..4].copy_from_slice(&self.node.to_le_bytes());
        body[4..8].copy_from_slice(&self.window.to_le_bytes());
        body[8..12].copy_from_slice(&self.granted.to_le_bytes());
        city_mac(&alloc_key(), self.node, 5, self.window, &body)
    }

    pub fn is_valid(&self) -> bool {
        self.mac == self.compute_mac()
    }
}

/// A district's rolling demand history + current allocation table.
#[derive(Copy, Clone, Debug)]
pub struct DistrictAllocator {
    /// Demand per window (temporal slots).
    demand: [u32; ALLOC_SLOTS],
    supply: [u32; ALLOC_SLOTS],
    grants: [Option<AllocRecord>; ALLOC_NODES],
    grant_count: usize,
}

impl Default for DistrictAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl DistrictAllocator {
    pub const fn new() -> Self {
        Self {
            demand: [0; ALLOC_SLOTS],
            supply: [0; ALLOC_SLOTS],
            grants: [None; ALLOC_NODES],
            grant_count: 0,
        }
    }

    pub fn record_demand(&mut self, slot: usize, demand: u32, supply: u32) -> bool {
        if slot >= ALLOC_SLOTS {
            return false;
        }
        self.demand[slot] = demand;
        self.supply[slot] = supply;
        true
    }

    pub fn demand(&self, slot: usize) -> Option<(u32, u32)> {
        if slot >= ALLOC_SLOTS {
            return None;
        }
        Some((self.demand[slot], self.supply[slot]))
    }

    /// Simple moving average over the last `window` windows.
    pub fn predict(&self, window: usize) -> u32 {
        if window == 0 || window > ALLOC_SLOTS {
            return 0;
        }
        let mut sum = 0u64;
        let mut n = 0u64;
        for w in (ALLOC_SLOTS - window)..ALLOC_SLOTS {
            sum += self.demand[w] as u64;
            n += 1;
        }
        sum.checked_div(n).unwrap_or(0) as u32
    }

    /// Whether the next window is projected to be scarce.
    pub fn projected_scarcity(&self, window: usize) -> bool {
        self.predict(window) > self.supply[ALLOC_SLOTS - 1]
    }

    /// Grant a proportion of the next window's supply to a node.
    /// Returns the allocation record (MAC-verified) or None when scarce/full.
    pub fn grant(&mut self, node: u32, window: u32, share_pct: u32) -> Option<AllocRecord> {
        if share_pct > 100 || self.grant_count >= ALLOC_NODES {
            return None;
        }
        let budget = self.supply[ALLOC_SLOTS - 1];
        let base_pred = self.predict(1);
        if self.projected_scarcity(1) && base_pred > 0 {
            return None; // smart-contract enforcement: deny under scarcity
        }
        let granted = (budget as u64 * share_pct as u64 / 100) as u32;
        let mut rec = AllocRecord {
            node,
            window,
            granted,
            mac: 0,
        };
        rec.mac = rec.compute_mac();
        self.grants[self.grant_count] = Some(rec);
        self.grant_count += 1;
        Some(rec)
    }

    pub fn grant_count(&self) -> usize {
        self.grant_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prediction_and_granting() {
        let mut d = DistrictAllocator::new();
        // History: [100, 110, 120, 130] supply 200 by default in last window.
        d.record_demand(0, 100, 200);
        d.record_demand(1, 110, 200);
        d.record_demand(2, 120, 200);
        d.record_demand(3, 130, 200);

        // 4-window average = 115 -> not scarce (115 < 200).
        assert_eq!(d.predict(4), 115);
        assert!(!d.projected_scarcity(4));

        let rec = d.grant(7, 4, 50).unwrap();
        assert_eq!(rec.granted, 100);
        assert!(rec.is_valid());

        // Tamper the grant => invalid.
        let mut bad = rec;
        bad.granted += 1;
        assert!(!bad.is_valid());
    }

    #[test]
    fn scarcity_denies_grants() {
        let mut d = DistrictAllocator::new();
        // Demand exceeds supply in the last window.
        d.record_demand(0, 100, 50);
        d.record_demand(1, 100, 50);
        d.record_demand(2, 100, 50);
        d.record_demand(3, 200, 50);
        // Projected scarcity true.
        assert!(d.projected_scarcity(1));
        assert!(d.grant(1, 5, 50).is_none());
    }
}
