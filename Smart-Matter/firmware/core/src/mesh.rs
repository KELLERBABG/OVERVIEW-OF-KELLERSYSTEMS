//! # Ad-hoc Mesh Formations
//!
//! Mesh networking protocol for Smart Matter nodes. Uses gradient-based
//! routing with minimal state (each node stores only next-hop information
//! for each reachable destination).

use crate::types::{
    EnergyLevel, NodeId, ProtocolSlice, RouteAdvertisement, RouteEntry, RouteFlags, SliceType,
    MAX_HOPS, MAX_PEERS,
};
use heapless::Vec;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Interval between beacon transmissions (ms) when energy is Nominal or above.
pub const BEACON_INTERVAL_MS: u32 = 5_000;

/// Interval between beacon transmissions (ms) when energy is Low.
pub const BEACON_INTERVAL_LOW_MS: u32 = 30_000;

/// Maximum age of a route entry before eviction (seconds).
pub const ROUTE_TIMEOUT_SEC: u32 = 300;

/// Metric penalty per hop.
pub const HOP_PENALTY: u16 = 100;

/// Metric penalty for low-power nodes.
pub const LOW_POWER_PENALTY: u16 = 50;

/// Maximum number of route entries carried in a single RouteAdv slice.
///
/// A serialized `RouteEntry` is 23 bytes; with the advertisement header this
/// keeps each advertisement within the 128-byte slice payload. Nodes rotate
/// which entries they advertise so the full table converges over time.
pub const MAX_ADV_ENTRIES_PER_SLICE: usize = 5;

// ---------------------------------------------------------------------------
// Routing Table
// ---------------------------------------------------------------------------

/// An in-memory routing table with fixed capacity.
#[derive(Debug, Clone)]
pub struct RoutingTable {
    entries: Vec<RouteEntry, MAX_PEERS>,
    sequence_number: u16,
}

impl Default for RoutingTable {
    fn default() -> Self {
        Self::new()
    }
}

impl RoutingTable {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            sequence_number: 0,
        }
    }

    /// Insert or update a route entry.
    pub fn update(&mut self, entry: RouteEntry) {
        if let Some(existing) = self
            .entries
            .iter_mut()
            .find(|e| e.destination == entry.destination)
        {
            // Only update if the new metric is better, or the sequence is newer
            if entry.metric < existing.metric || entry.age_sec < existing.age_sec {
                *existing = entry;
            }
        } else if self.entries.len() < MAX_PEERS {
            let _ = self.entries.push(entry);
        }
    }

    /// Get the next hop for a destination.
    pub fn next_hop(&self, destination: &NodeId) -> Option<NodeId> {
        self.entries
            .iter()
            .find(|e| e.destination == *destination)
            .map(|e| e.next_hop)
    }

    /// Get metric for a destination.
    pub fn metric(&self, destination: &NodeId) -> Option<u16> {
        self.entries
            .iter()
            .find(|e| e.destination == *destination)
            .map(|e| e.metric)
    }

    /// Route advertisement to broadcast to neighbors.
    ///
    /// Carries at most [`MAX_ADV_ENTRIES_PER_SLICE`] entries (rotating by
    /// sequence number) so the serialized advertisement fits the 128-byte
    /// slice payload. `rotation` is the sender's sequence counter — it
    /// selects the window of entries advertised this round.
    pub fn build_advertisement(
        &self,
        origin: NodeId,
        flags: RouteFlags,
        rotation: u16,
    ) -> RouteAdvertisement {
        let start = (rotation as usize) % MAX_ADV_ENTRIES_PER_SLICE.max(1);
        let mut selected = Vec::<RouteEntry, MAX_PEERS>::new();
        // Rotate: take entries starting at `start`, wrapping, capped at the
        // per-slice budget.
        for i in 0..self.entries.len() {
            if selected.len() >= MAX_ADV_ENTRIES_PER_SLICE {
                break;
            }
            let idx = (start + i) % self.entries.len();
            if let Some(e) = self.entries.get(idx) {
                let _ = selected.push(*e);
            }
        }

        RouteAdvertisement {
            origin,
            seqno: self.sequence_number,
            flags,
            entries: selected,
        }
    }

    /// Process an incoming route advertisement from a neighbor.
    pub fn process_advertisement(&mut self, adv: &RouteAdvertisement, neighbor: NodeId) {
        // Update the direct route to the origin
        let direct = RouteEntry {
            destination: adv.origin,
            next_hop: neighbor,
            metric: HOP_PENALTY, // Direct neighbor = 1 hop
            flags: adv.flags,
            age_sec: 0,
        };
        self.update(direct);

        // Process advertised entries
        for entry in adv.entries.iter() {
            let mut metric = entry.metric.saturating_add(HOP_PENALTY);
            if entry.flags.contains(RouteFlags::LOW_POWER) {
                metric = metric.saturating_add(LOW_POWER_PENALTY);
            }

            let new_entry = RouteEntry {
                destination: entry.destination,
                next_hop: neighbor,
                metric,
                flags: entry.flags,
                age_sec: 0,
            };
            self.update(new_entry);
        }

        self.sequence_number = self.sequence_number.wrapping_add(1);
    }

    /// Age all entries and remove stale ones.
    pub fn age_entries(&mut self, delta_sec: u32) {
        let stale: Vec<NodeId, MAX_PEERS> = self
            .entries
            .iter()
            .filter(|e| e.age_sec.saturating_add(delta_sec) > ROUTE_TIMEOUT_SEC)
            .map(|e| e.destination)
            .collect();

        // Remove stale entries
        self.entries.retain(|e| !stale.contains(&e.destination));

        // Age remaining
        for entry in self.entries.iter_mut() {
            entry.age_sec = entry.age_sec.saturating_add(delta_sec);
        }
    }

    /// Number of entries in the routing table.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the table is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Message Deduplication
// ---------------------------------------------------------------------------

/// Simple sequence-number-based deduplication cache.
#[derive(Debug, Clone)]
pub struct DedupCache {
    entries: Vec<(NodeId, u16), 64>, // (source, seqno)
}

impl Default for DedupCache {
    fn default() -> Self {
        Self::new()
    }
}

impl DedupCache {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Check if this (source, seqno) pair has been seen. Returns true if new.
    pub fn check_and_insert(&mut self, src: NodeId, seqno: u16) -> bool {
        if self.entries.iter().any(|(s, n)| *s == src && *n == seqno) {
            false // Duplicate
        } else {
            let _ = self.entries.push((src, seqno));
            if self.entries.len() >= 64 {
                self.entries.remove(0); // Drop oldest
            }
            true
        }
    }
}

// ---------------------------------------------------------------------------
// Mesh Network Interface
// ---------------------------------------------------------------------------

/// Trait representing the wireless transceiver interface.
pub trait Radio {
    /// Send a raw protocol slice.
    fn send(&mut self, slice: &ProtocolSlice) -> Result<(), RadioError>;

    /// Receive a protocol slice (blocking with timeout).
    fn receive(&mut self, timeout_ms: u32) -> Result<Option<ProtocolSlice>, RadioError>;

    /// Get current RSSI of the last received packet (dBm).
    fn last_rssi(&self) -> i16;
}

/// Radio error types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadioError {
    CrcMismatch,
    Collision,
    Underrun,
    Timeout,
    EnergyLow,
    Busy,
}

// ---------------------------------------------------------------------------
// Mesh Node
// ---------------------------------------------------------------------------

/// Main mesh networking logic for a Smart Matter node.
#[derive(Debug, Clone)]
pub struct MeshNode {
    node_id: NodeId,
    routing_table: RoutingTable,
    dedup: DedupCache,
    flags: RouteFlags,
    /// Monotonic message sequence number (beacons, advertisements).
    seqno: u16,
}

impl MeshNode {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            routing_table: RoutingTable::new(),
            dedup: DedupCache::new(),
            flags: RouteFlags::ACCEPT_PEER | RouteFlags::HAS_STORAGE,
            seqno: 0,
        }
    }

    /// Set routing flags.
    pub fn set_flags(&mut self, flags: RouteFlags) {
        self.flags = flags;
    }

    /// This node's identifier.
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// Allocate the next message sequence number (for data slices, acks, …).
    pub fn next_seqno(&mut self) -> u16 {
        let seqno = self.seqno;
        self.seqno = self.seqno.wrapping_add(1);
        seqno
    }

    /// Get a mutable reference to the routing table.
    pub fn routing_table(&mut self) -> &mut RoutingTable {
        &mut self.routing_table
    }

    /// Get an immutable reference to the routing table.
    pub fn routing_table_ref(&self) -> &RoutingTable {
        &self.routing_table
    }

    /// Build a beacon slice for this node.
    pub fn build_beacon(&mut self, energy: EnergyLevel) -> ProtocolSlice {
        let seqno = self.seqno;
        self.seqno = self.seqno.wrapping_add(1);
        ProtocolSlice {
            slice_type: SliceType::Beacon,
            src: self.node_id,
            dst: NodeId([0xff; 8]), // Broadcast
            hops: 0,
            seqno,
            payload: Vec::new(),
            energy,
        }
    }

    /// Build a route advertisement slice.
    ///
    /// The advertisement (origin, flags, up to 5 route entries) is
    /// serialized with `postcard` into the slice payload, so multi-hop
    /// route learning works over the wire.
    pub fn build_route_adv(&mut self, energy: EnergyLevel) -> ProtocolSlice {
        let adv = self
            .routing_table
            .build_advertisement(self.node_id, self.flags, self.seqno);
        let seqno = self.seqno;
        self.seqno = self.seqno.wrapping_add(1);

        let mut payload: Vec<u8, 128> = Vec::new();
        // Serialize with postcard (no_std serde wire format). A stack scratch
        // buffer is required: a heapless `Vec` only derefs to its initialized
        // portion, which postcard would see as a zero-length buffer.
        let mut scratch = [0u8; 128];
        if let Ok(written) = postcard::to_slice(&adv, &mut scratch) {
            let _ = payload.extend_from_slice(written);
        }

        ProtocolSlice {
            slice_type: SliceType::RouteAdv,
            src: self.node_id,
            dst: NodeId([0xff; 8]),
            hops: 0,
            seqno,
            payload,
            energy,
        }
    }

    /// Process an incoming protocol slice from the radio.
    ///
    /// Returns the slice to forward, if any. The returned slice keeps the
    /// original source and final destination (`dst`); the caller resolves the
    /// actual next hop from the routing table at transmit time. This keeps
    /// multi-hop chains correct: rewriting `dst` to the next hop would make
    /// the message die at the penultimate node.
    pub fn process_slice<R: Radio>(
        &mut self,
        slice: &ProtocolSlice,
        _radio: &mut R,
    ) -> Option<ProtocolSlice> {
        // Dedup
        if !self.dedup.check_and_insert(slice.src, slice.seqno) {
            return None; // Already seen
        }

        // If it's for us, no forwarding
        if slice.dst == self.node_id {
            return None;
        }

        // Check hop limit
        if slice.hops >= MAX_HOPS {
            return None;
        }

        match slice.slice_type {
            SliceType::Beacon => {
                // Update routing table with direct route to beacon source
                let entry = RouteEntry {
                    destination: slice.src,
                    next_hop: slice.src,
                    metric: HOP_PENALTY,
                    flags: RouteFlags::empty(),
                    age_sec: 0,
                };
                self.routing_table.update(entry);
                None // Don't forward beacons
            }
            SliceType::RouteAdv => {
                // Update routing from advertisement (wire-decoded entries)
                match postcard::from_bytes::<RouteAdvertisement>(&slice.payload) {
                    Ok(adv) => {
                        self.routing_table.process_advertisement(&adv, slice.src);
                    }
                    Err(_) => {
                        // Malformed advertisement — ignore.
                    }
                }
                None
            }
            SliceType::DataFragment
            | SliceType::Broadcast
            | SliceType::Command
            | SliceType::FwUpdate => {
                // Forward toward destination if a route exists.
                if self.routing_table.next_hop(&slice.dst).is_some() {
                    let mut fwd = slice.clone();
                    fwd.hops += 1;
                    Some(fwd)
                } else {
                    // No route — drop
                    None
                }
            }
            SliceType::DataAck | SliceType::IntegrityReport => {
                // Responses travel back via reverse routing.
                let mut fwd = slice.clone();
                fwd.hops += 1;
                Some(fwd)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_node_id(id: u8) -> NodeId {
        let mut bytes = [0u8; 8];
        bytes[0] = id;
        NodeId(bytes)
    }

    #[test]
    fn test_routing_table_basic() {
        let mut table = RoutingTable::new();
        let n1 = test_node_id(1);
        let n2 = test_node_id(2);

        assert!(table.is_empty());

        let entry = RouteEntry {
            destination: n1,
            next_hop: n2,
            metric: 100,
            flags: RouteFlags::empty(),
            age_sec: 0,
        };
        table.update(entry);

        assert_eq!(table.len(), 1);
        assert_eq!(table.next_hop(&n1), Some(n2));
    }

    #[test]
    fn test_route_advertisement_processing() {
        let mut table = RoutingTable::new();
        let n2 = test_node_id(2);
        let n3 = test_node_id(3);

        // n2 advertises that it can reach n3
        let mut adv_entries: Vec<RouteEntry, MAX_PEERS> = Vec::new();
        adv_entries
            .push(RouteEntry {
                destination: n3,
                next_hop: n3,
                metric: 100,
                flags: RouteFlags::empty(),
                age_sec: 0,
            })
            .ok();

        let adv = RouteAdvertisement {
            origin: n2,
            seqno: 1,
            flags: RouteFlags::empty(),
            entries: adv_entries,
        };

        table.process_advertisement(&adv, n2);

        // Now n1 should know: n2 is direct, n3 via n2
        assert_eq!(table.next_hop(&n2), Some(n2));
        assert_eq!(table.next_hop(&n3), Some(n2));
        assert!(table.metric(&n3).unwrap() >= 100);
    }

    #[test]
    fn test_dedup() {
        let mut cache = DedupCache::new();
        let n1 = test_node_id(1);

        assert!(cache.check_and_insert(n1, 1));
        assert!(!cache.check_and_insert(n1, 1)); // Duplicate
        assert!(cache.check_and_insert(n1, 2)); // New seqno
    }

    #[test]
    fn test_mesh_node_beacon() {
        let n1 = test_node_id(1);
        let mut mesh = MeshNode::new(n1);
        let beacon = mesh.build_beacon(EnergyLevel::Nominal);

        assert_eq!(beacon.src, n1);
        assert_eq!(beacon.slice_type, SliceType::Beacon);
        assert_eq!(beacon.energy, EnergyLevel::Nominal);
    }

    #[test]
    fn test_route_adv_serializes_entries() {
        let mut mesh1 = MeshNode::new(test_node_id(1));
        let n3 = test_node_id(3);
        let n4 = test_node_id(4);
        // Give node 1 a couple of routes to advertise.
        mesh1.routing_table().update(RouteEntry {
            destination: n3,
            next_hop: n3,
            metric: 100,
            flags: RouteFlags::empty(),
            age_sec: 0,
        });
        mesh1.routing_table().update(RouteEntry {
            destination: n4,
            next_hop: n4,
            metric: 200,
            flags: RouteFlags::empty(),
            age_sec: 0,
        });

        let adv_slice = mesh1.build_route_adv(EnergyLevel::Nominal);

        // Node 2 decodes the advertisement from the wire payload and learns
        // both routes via node 1 — proving entries survive serialization.
        let mut mesh2 = MeshNode::new(test_node_id(2));
        let mut radio = crate::mesh::test_radio::TestRadio;
        mesh2.process_slice(&adv_slice, &mut radio);

        assert_eq!(mesh2.routing_table().next_hop(&n3), Some(test_node_id(1)));
        assert_eq!(mesh2.routing_table().next_hop(&n4), Some(test_node_id(1)));
    }

    #[test]
    fn test_multi_hop_forwarding_keeps_destination() {
        // n1 -> n2 -> n3: a message routed through node 2 must keep its
        // final destination (n3) after forwarding — rewriting dst to the
        // next hop would kill the message at the penultimate node.
        let n1 = test_node_id(1);
        let n2 = test_node_id(2);
        let n3 = test_node_id(3);

        let mut mesh2 = MeshNode::new(n2);
        mesh2.routing_table().update(RouteEntry {
            destination: n3,
            next_hop: n3,
            metric: 100,
            flags: RouteFlags::empty(),
            age_sec: 0,
        });

        let slice = ProtocolSlice {
            slice_type: SliceType::DataFragment,
            src: n1,
            dst: n3,
            hops: 1,
            seqno: 7,
            payload: Vec::from_slice(b"frag").unwrap(),
            energy: EnergyLevel::Nominal,
        };

        let mut radio = super::test_radio::TestRadio;
        let forwarded = mesh2.process_slice(&slice, &mut radio).expect("forwarded");
        assert_eq!(forwarded.dst, n3); // Final destination preserved
        assert_eq!(forwarded.src, n1); // Origin preserved (dedup key)
        assert_eq!(forwarded.hops, 2);
    }
}

/// A no-op radio used to exercise `process_slice` in tests.
#[cfg(test)]
pub(crate) mod test_radio {
    use super::*;

    pub struct TestRadio;

    impl Radio for TestRadio {
        fn send(&mut self, _slice: &ProtocolSlice) -> Result<(), RadioError> {
            Ok(())
        }
        fn receive(&mut self, _timeout_ms: u32) -> Result<Option<ProtocolSlice>, RadioError> {
            Ok(None)
        }
        fn last_rssi(&self) -> i16 {
            0
        }
    }
}
