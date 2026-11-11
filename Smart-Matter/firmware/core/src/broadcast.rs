//! # Sovereign Localized Broadcaster
//!
//! Local broadcast with RSSI-based geo-fencing. Ensures data locality:
//! messages only propagate through nodes within a physical RSSI threshold,
//! preventing data from leaving the intended geographic area.

use crate::mesh::DedupCache;
use crate::types::{BroadcastMessage, NodeId, ProtocolSlice, SliceType};
use heapless::Vec;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default RSSI threshold (dBm) for geo-fencing.
/// Nodes with RSSI below this threshold will not relay.
pub const DEFAULT_RSSI_THRESHOLD_DBM: i16 = -80;

/// Maximum broadcast TTL (hops).
pub const MAX_BROADCAST_TTL: u8 = 16;

// ---------------------------------------------------------------------------
// Localized Broadcaster
// ---------------------------------------------------------------------------

/// Handles sovereign localized broadcast with RSSI geo-fencing.
#[derive(Debug, Clone)]
pub struct LocalizedBroadcaster {
    node_id: NodeId,
    rssi_threshold: i16,
    dedup: DedupCache,
    seqno: u16,
}

impl LocalizedBroadcaster {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            rssi_threshold: DEFAULT_RSSI_THRESHOLD_DBM,
            dedup: DedupCache::new(),
            seqno: 0,
        }
    }

    /// Set the RSSI threshold for geo-fencing.
    pub fn set_rssi_threshold(&mut self, threshold: i16) {
        self.rssi_threshold = threshold;
    }

    /// Get the current RSSI threshold.
    pub fn rssi_threshold(&self) -> i16 {
        self.rssi_threshold
    }

    /// Create a broadcast message for transmission.
    pub fn create_broadcast(&mut self, payload: Vec<u8, 128>, ttl: u8) -> BroadcastMessage {
        let seqno = self.seqno;
        self.seqno = self.seqno.wrapping_add(1);

        BroadcastMessage {
            origin: self.node_id,
            rssi_threshold: self.rssi_threshold,
            seqno,
            payload,
            ttl: ttl.min(MAX_BROADCAST_TTL),
        }
    }

    /// Convert a broadcast message to a protocol slice.
    pub fn to_slice(&self, msg: &BroadcastMessage, energy: crate::EnergyLevel) -> ProtocolSlice {
        let mut payload: Vec<u8, 128> = Vec::new();
        // Encode broadcast header into payload
        payload.extend_from_slice(msg.origin.as_bytes()).ok();
        payload
            .extend_from_slice(&msg.rssi_threshold.to_le_bytes())
            .ok();
        payload.extend_from_slice(&msg.seqno.to_le_bytes()).ok();
        payload.extend_from_slice(&msg.ttl.to_le_bytes()).ok();

        ProtocolSlice {
            slice_type: SliceType::Broadcast,
            src: self.node_id,
            dst: NodeId([0xff; 8]), // Broadcast
            hops: 0,
            seqno: msg.seqno,
            payload,
            energy,
        }
    }

    /// Process an incoming broadcast slice.
    ///
    /// Returns `Some(broadcast)` if the message should be relayed, `None` if
    /// it should be dropped (duplicate, out of range, TTL expired).
    pub fn process_incoming(
        &mut self,
        slice: &ProtocolSlice,
        rssi_dbm: i16,
    ) -> Option<BroadcastMessage> {
        // Deduplicate
        if !self.dedup.check_and_insert(slice.src, slice.seqno) {
            return None;
        }

        // Check TTL
        if slice.hops >= MAX_BROADCAST_TTL {
            return None;
        }

        // Decode broadcast header from payload
        if slice.payload.len() < 8 + 2 + 2 + 1 {
            return None; // Malformed
        }

        let mut origin = [0u8; 8];
        origin.copy_from_slice(&slice.payload[0..8]);
        let origin_id = NodeId(origin);

        let mut rssi_bytes = [0u8; 2];
        rssi_bytes.copy_from_slice(&slice.payload[8..10]);
        let msg_rssi_threshold = i16::from_le_bytes(rssi_bytes);

        let mut seqno_bytes = [0u8; 2];
        seqno_bytes.copy_from_slice(&slice.payload[10..12]);
        let _msg_seqno = u16::from_le_bytes(seqno_bytes);

        let ttl = slice.payload[12];

        // RSSI geo-fencing check
        if rssi_dbm < msg_rssi_threshold {
            return None; // Signal too weak — don't relay
        }

        // If we're the origin, don't re-relay
        if origin_id == self.node_id {
            return None;
        }

        // If TTL expired, stop
        if ttl == 0 {
            return None;
        }

        Some(BroadcastMessage {
            origin: origin_id,
            rssi_threshold: msg_rssi_threshold,
            seqno: slice.seqno,
            payload: slice.payload.clone(),
            ttl: ttl.saturating_sub(1),
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EnergyLevel;

    fn test_node_id(id: u8) -> NodeId {
        let mut bytes = [0u8; 8];
        bytes[0] = id;
        NodeId(bytes)
    }

    #[test]
    fn test_broadcast_creation() {
        let n1 = test_node_id(1);
        let mut bc = LocalizedBroadcaster::new(n1);

        let mut payload: Vec<u8, 128> = Vec::new();
        payload.extend_from_slice(b"hello").ok();

        let msg = bc.create_broadcast(payload, 5);
        assert_eq!(msg.origin, n1);
        assert_eq!(msg.ttl, 5);
        assert_eq!(&msg.payload[..], b"hello");
    }

    #[test]
    fn test_rssi_geofencing() {
        let n1 = test_node_id(1);
        let n2 = test_node_id(2);
        let mut bc = LocalizedBroadcaster::new(n2);

        // Create a broadcast from n1
        let mut payload: Vec<u8, 128> = Vec::new();
        payload.extend_from_slice(b"data").ok();

        let msg = BroadcastMessage {
            origin: n1,
            rssi_threshold: -70,
            seqno: 1,
            payload,
            ttl: 3,
        };

        let slice = {
            let mut p: Vec<u8, 128> = Vec::new();
            p.extend_from_slice(&msg.origin.as_bytes()).ok();
            p.extend_from_slice(&msg.rssi_threshold.to_le_bytes()).ok();
            p.extend_from_slice(&msg.seqno.to_le_bytes()).ok();
            p.extend_from_slice(&msg.ttl.to_le_bytes()).ok();
            ProtocolSlice {
                slice_type: SliceType::Broadcast,
                src: n1,
                dst: NodeId([0xff; 8]),
                hops: 1,
                seqno: 1,
                payload: p,
                energy: EnergyLevel::Nominal,
            }
        };

        // Strong RSSI — should relay
        let result = bc.process_incoming(&slice, -60);
        assert!(result.is_some());

        // Weak RSSI — should drop
        let result2 = bc.process_incoming(&slice, -90);
        assert!(result2.is_none());
    }

    #[test]
    fn test_broadcast_dedup() {
        let n1 = test_node_id(1);
        let n2 = test_node_id(2);
        let mut bc = LocalizedBroadcaster::new(n2);

        let slice = ProtocolSlice {
            slice_type: SliceType::Broadcast,
            src: n1,
            dst: NodeId([0xff; 8]),
            hops: 1,
            seqno: 1,
            payload: {
                let mut p: Vec<u8, 128> = Vec::new();
                p.extend_from_slice(&n1.as_bytes()).ok();
                p.extend_from_slice(&(-70i16).to_le_bytes()).ok();
                p.extend_from_slice(&1u16.to_le_bytes()).ok();
                p.extend_from_slice(&3u8.to_le_bytes()).ok();
                p
            },
            energy: EnergyLevel::Nominal,
        };

        // First time — relay
        assert!(bc.process_incoming(&slice, -60).is_some());
        // Duplicate — drop
        assert!(bc.process_incoming(&slice, -60).is_none());
    }
}
