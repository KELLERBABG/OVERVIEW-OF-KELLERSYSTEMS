//! Peer identity and discovery types.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique peer identifier.
pub type PeerId = Uuid;

/// Information about a peer in the P2P network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Unique peer identifier.
    pub id: PeerId,
    /// Human-readable name.
    pub name: String,
    /// Graph node ID this peer represents.
    pub node_id: Uuid,
    /// Supported transport addresses (e.g., "/ip4/.../tcp/...").
    pub addresses: Vec<String>,
    /// Current available capacity in standard units.
    pub available_capacity: f64,
    /// Capacity unit (e.g., "kg", "pallets", "TEU").
    pub capacity_unit: String,
    /// Geographic region tag for routing.
    pub region: String,
    /// Whether this peer is currently online.
    pub online: bool,
}

impl PeerInfo {
    pub fn new(
        id: PeerId,
        name: impl Into<String>,
        node_id: Uuid,
        region: impl Into<String>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            node_id,
            addresses: Vec::new(),
            available_capacity: 0.0,
            capacity_unit: "kg".into(),
            region: region.into(),
            online: true,
        }
    }

    /// Set available capacity.
    pub fn with_capacity(mut self, capacity: f64, unit: impl Into<String>) -> Self {
        self.available_capacity = capacity;
        self.capacity_unit = unit.into();
        self
    }

    /// Add a transport address.
    pub fn with_address(mut self, addr: impl Into<String>) -> Self {
        self.addresses.push(addr.into());
        self
    }
}

/// A peer in the P2P network with full protocol state.
#[derive(Debug, Clone)]
pub struct Peer {
    pub info: PeerInfo,
}

impl Peer {
    pub fn new(info: PeerInfo) -> Self {
        Self { info }
    }

    pub fn id(&self) -> PeerId {
        self.info.id
    }

    pub fn name(&self) -> &str {
        &self.info.name
    }

    pub fn node_id(&self) -> Uuid {
        self.info.node_id
    }

    pub fn is_online(&self) -> bool {
        self.info.online
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_peer() {
        let info = PeerInfo::new(Uuid::new_v4(), "Transport Alpha", Uuid::new_v4(), "EU")
            .with_capacity(10000.0, "kg");
        let peer = Peer::new(info);
        assert_eq!(peer.name(), "Transport Alpha");
        assert_eq!(peer.info.region, "EU");
        assert!(peer.is_online());
    }
}