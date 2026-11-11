//! # Smart Matter Sovereign Localized Broadcaster
//!
//! Provides localized broadcast capabilities with RSSI-based geo-fencing.
//! Messages are constrained to a physical area based on signal strength,
//! ensuring data locality — information stays in the geographic region
//! where it was generated.

pub use smart_matter_core::broadcast::*;
pub use smart_matter_core::types::*;

use heapless::Vec;

/// Manages localized broadcast operations across the mesh.
#[derive(Debug)]
pub struct BroadcasterController {
    broadcaster: LocalizedBroadcaster,
}

impl BroadcasterController {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            broadcaster: LocalizedBroadcaster::new(node_id),
        }
    }

    /// Broadcast a message with geographical locality constraints.
    pub fn broadcast(
        &mut self,
        payload: Vec<u8, 128>,
        ttl: u8,
        rssi_threshold: i16,
    ) -> BroadcastMessage {
        self.broadcaster.set_rssi_threshold(rssi_threshold);
        self.broadcaster.create_broadcast(payload, ttl)
    }

    /// Process an incoming broadcast slice with RSSI check.
    pub fn process_incoming(
        &mut self,
        slice: &ProtocolSlice,
        rssi_dbm: i16,
    ) -> Option<BroadcastMessage> {
        self.broadcaster.process_incoming(slice, rssi_dbm)
    }

    pub fn rssi_threshold(&self) -> i16 {
        self.broadcaster.rssi_threshold()
    }

    pub fn set_rssi_threshold(&mut self, threshold: i16) {
        self.broadcaster.set_rssi_threshold(threshold);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_node_id(id: u8) -> NodeId {
        let mut bytes = [0u8; 8];
        bytes[0] = id;
        NodeId(bytes)
    }

    #[test]
    fn test_broadcaster_controller() {
        let n1 = test_node_id(1);
        let mut controller = BroadcasterController::new(n1);

        let mut payload: Vec<u8, 128> = Vec::new();
        payload.extend_from_slice(b"local data").ok();

        let msg = controller.broadcast(payload, 5, -75);
        assert_eq!(msg.origin, n1);
        assert_eq!(msg.rssi_threshold, -75);
        assert_eq!(msg.ttl, 5);
    }

    #[test]
    fn test_rssi_geofencing_controller() {
        let n1 = test_node_id(1);
        let n2 = test_node_id(2);
        let mut controller = BroadcasterController::new(n2);

        let slice = {
            let mut p: Vec<u8, 128> = Vec::new();
            p.extend_from_slice(&n1.as_bytes()).ok();
            p.extend_from_slice(&(-70i16).to_le_bytes()).ok();
            p.extend_from_slice(&1u16.to_le_bytes()).ok();
            p.extend_from_slice(&3u8.to_le_bytes()).ok();
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

        // Strong RSSI should relay
        assert!(controller.process_incoming(&slice, -60).is_some());
        // Weak RSSI should drop
        assert!(controller.process_incoming(&slice, -90).is_none());
    }
}
