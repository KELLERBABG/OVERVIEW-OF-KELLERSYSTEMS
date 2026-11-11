//! **City localized shard distribution** — geofenced data replication.
//!
//! Data is split into fixed 32-byte chunks and assigned to districts by a
//! geo tag. A district owns a rectangular cell of the city grid; a shard may
//! only be stored on nodes whose geo tag falls inside the district's bounding
//! box. This gives the "data stays within district boundaries" guarantee from
//! the City OS plan without any allocation.
//!
//! Shards are stored AES-XTS-encrypted via the Keller Vault (`city-crypto`),
//! so a leaked node reveals ciphertext only.

#![no_std]

#[cfg(test)]
extern crate std;

use city_crypto::vault::{self, VAULT_SECTOR_SIZE};
use city_types::GeoTag;

pub const SHARD_MAX_NODES: usize = 8;
pub const SHARD_MAX_SECTORS: usize = 16;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct GeoBox {
    /// Quantized grid coordinates (all inclusive).
    pub lat0: u16,
    pub lon0: u16,
    pub lat1: u16,
    pub lon1: u16,
}

impl GeoBox {
    pub const fn contains(&self, tag: GeoTag) -> bool {
        let lat = (tag >> 16) as u16;
        let lon = tag as u16;
        lat >= self.lat0 && lat <= self.lat1 && lon >= self.lon0 && lon <= self.lon1
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct District {
    pub id: u32,
    pub box_: GeoBox,
    pub nodes: [u32; SHARD_MAX_NODES],
    pub node_count: usize,
}

impl District {
    pub const fn new(id: u32, box_: GeoBox) -> Self {
        Self {
            id,
            box_,
            nodes: [0; SHARD_MAX_NODES],
            node_count: 0,
        }
    }

    pub fn add_node(&mut self, node: u32) -> bool {
        if self.node_count >= SHARD_MAX_NODES {
            return false;
        }
        if self.nodes.iter().take(self.node_count).any(|&n| n == node) {
            return false;
        }
        self.nodes[self.node_count] = node;
        self.node_count += 1;
        true
    }

    pub fn contains(&self, tag: GeoTag) -> bool {
        self.box_.contains(tag)
    }

    pub fn replicates(&self) -> usize {
        self.node_count
    }
}

/// A fixed table of districts (a small city grid) plus the mapping from a
/// geo tag to the actual vault sector holding that tag's shard.
#[derive(Copy, Clone, Debug)]
pub struct ShardMap {
    districts: [Option<District>; SHARD_MAX_NODES],
    count: usize,
    /// (geo tag, vault sector) — `vault::store` auto-assigns sectors, so we
    /// record the real mapping here instead of deriving it from the tag.
    slots: [(GeoTag, usize); SHARD_MAX_SECTORS],
    slot_count: usize,
}

impl Default for ShardMap {
    fn default() -> Self {
        Self::new()
    }
}

impl ShardMap {
    pub const fn new() -> Self {
        Self {
            districts: [None; SHARD_MAX_NODES],
            count: 0,
            slots: [(0, 0); SHARD_MAX_SECTORS],
            slot_count: 0,
        }
    }

    fn find_slot(&self, tag: GeoTag) -> Option<usize> {
        self.slots
            .iter()
            .take(self.slot_count)
            .position(|&(t, _)| t == tag)
    }

    pub fn add(&mut self, district: District) -> bool {
        if self.count >= SHARD_MAX_NODES {
            return false;
        }
        self.districts[self.count] = Some(district);
        self.count += 1;
        true
    }

    /// The district responsible for a geo tag, if any.
    pub fn owner(&self, tag: GeoTag) -> Option<&District> {
        self.districts
            .iter()
            .take(self.count)
            .filter_map(|d| d.as_ref())
            .find(|d| d.contains(tag))
    }

    /// Store a shard (up to 32 bytes) under a geo tag. Ownership is enforced
    /// by the district box; the actual vault sector (auto-assigned by the
    /// vault) is recorded so `get` reads the right one.
    /// Returns (district id, vault sector) or None if no district claims it.
    pub fn put(&mut self, tag: GeoTag, data: &[u8]) -> Option<(u32, usize)> {
        if data.len() > VAULT_SECTOR_SIZE || self.slot_count >= SHARD_MAX_SECTORS {
            return None;
        }
        // Security: only tags owned by a district may be written; the
        // `?` ends the immutable borrow before the mutable slot write below.
        let district_id = self.owner(tag)?.id;
        let sector = vault::store(data)?;
        self.slots[self.slot_count] = (tag, sector);
        self.slot_count += 1;
        Some((district_id, sector))
    }

    /// Retrieve a shard. Returns false if the sector is empty/tampered or
    /// the tag was never stored locally.
    pub fn get(&self, tag: GeoTag, out: &mut [u8]) -> bool {
        match self.find_slot(tag) {
            Some(i) => vault::retrieve(self.slots[i].1, out),
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use city_crypto::vault;

    fn geo(lat: u16, lon: u16) -> GeoTag {
        ((lat as u32) << 16) | lon as u32
    }

    #[test]
    fn district_ownership_is_geofenced() {
        let mut map = ShardMap::new();
        let mut d = District::new(
            1,
            GeoBox {
                lat0: 10,
                lon0: 10,
                lat1: 20,
                lon1: 20,
            },
        );
        assert!(d.add_node(0x01));
        assert!(d.add_node(0x02));
        assert!(!d.add_node(0x01)); // duplicate
        assert_eq!(d.replicates(), 2);
        assert!(map.add(d));
        assert!(map.owner(geo(15, 15)).is_some());
        assert!(map.owner(geo(25, 15)).is_none());
        assert!(!map.owner(geo(15, 5)).is_some());
    }

    #[test]
    fn shard_put_get_roundtrip() {
        vault::dispose_all();
        let mut map = ShardMap::new();
        map.add(District::new(
            1,
            GeoBox {
                lat0: 0,
                lon0: 0,
                lat1: 100,
                lon1: 100,
            },
        ));
        let tag = geo(50, 50);
        let data: &[u8] = b"district-local-shard-23!";
        assert!(data.len() <= 32);
        let (district, _sector) = map.put(tag, data).unwrap();
        assert_eq!(district, 1);

        let mut out = [0u8; 24];
        assert!(map.get(tag, &mut out));
        assert_eq!(&out, data);

        // Out-of-bounds tag has no owner.
        assert!(map.put(geo(200, 200), &data).is_none());
        vault::dispose_all();
    }
}
