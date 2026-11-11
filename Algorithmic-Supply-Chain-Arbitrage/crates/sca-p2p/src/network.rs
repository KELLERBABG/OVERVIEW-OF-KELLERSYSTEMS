//! P2P network management — peer registry, routing, and message dispatch.

use std::collections::HashMap;
use uuid::Uuid;

use crate::peer::{Peer, PeerId, PeerInfo};
use crate::protocol::{CapacityOffer, CapacityRequest, Negotiation, NegotiationResult};
use crate::transport::{Transport, TransportError};

/// The P2P network manager.
///
/// Maintains a registry of known peers, routes messages between them,
/// and manages capacity offer/request negotiations.
pub struct P2pNetwork {
    /// Known peers indexed by ID.
    peers: HashMap<PeerId, Peer>,
    /// The transport layer for message delivery.
    transport: Box<dyn Transport + Send>,
    /// Active negotiations.
    negotiations: HashMap<Uuid, Negotiation>,
    /// Completed negotiation results.
    completed: Vec<NegotiationResult>,
}

impl P2pNetwork {
    /// Create a new P2P network.
    pub fn new(transport: Box<dyn Transport + Send>) -> Self {
        Self {
            peers: HashMap::new(),
            transport,
            negotiations: HashMap::new(),
            completed: Vec::new(),
        }
    }

    /// Register a peer in the network.
    pub fn register_peer(&mut self, info: PeerInfo) -> PeerId {
        let id = info.id;
        let peer = Peer::new(info);
        self.peers.insert(id, peer);
        id
    }

    /// Get a peer by ID.
    pub fn get_peer(&self, peer_id: &PeerId) -> Option<&Peer> {
        self.peers.get(peer_id)
    }

    /// Get all registered peer IDs.
    pub fn peer_ids(&self) -> Vec<PeerId> {
        self.peers.keys().copied().collect()
    }

    /// Connect to a peer.
    pub fn connect(&mut self, peer_id: PeerId) -> Result<(), TransportError> {
        if !self.peers.contains_key(&peer_id) {
            return Err(TransportError::PeerNotFound(peer_id));
        }
        self.transport.connect(peer_id)
    }

    /// Disconnect from a peer.
    pub fn disconnect(&mut self, peer_id: PeerId) -> Result<(), TransportError> {
        self.transport.disconnect(peer_id)
    }

    /// Broadcast a capacity offer to all online peers in the same region.
    pub fn broadcast_offer(&self, offer: &CapacityOffer) -> Vec<PeerId> {
        self.peers
            .iter()
            .filter(|(_, p)| p.is_online() && p.id() != offer.peer_id)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Find peers with available capacity in a region.
    pub fn find_capacity(
        &self,
        region: &str,
        min_capacity: f64,
    ) -> Vec<(&Peer, &PeerInfo)> {
        self.peers
            .iter()
            .filter(|(_, p)| {
                p.is_online()
                    && p.info.region == region
                    && p.info.available_capacity >= min_capacity
            })
            .map(|(_, p)| (p, &p.info))
            .collect()
    }

    /// Initiate a negotiation between two peers.
    pub fn negotiate(
        &mut self,
        initiator: PeerId,
        responder: PeerId,
        offer: CapacityOffer,
        request: CapacityRequest,
    ) -> Option<NegotiationResult> {
        let mut negotiation = Negotiation::new(initiator, responder, offer, request);
        let result = negotiation.accept();
        if result.is_some() {
            self.completed.push(result.clone().unwrap());
        }
        self.negotiations.insert(negotiation.id, negotiation);
        result
    }

    /// Get completed negotiation results.
    pub fn completed_negotiations(&self) -> &[NegotiationResult] {
        &self.completed
    }

    /// Get the number of active peers.
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// Remove a peer from the network.
    pub fn remove_peer(&mut self, peer_id: &PeerId) {
        self.peers.remove(peer_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::MockTransport;

    fn setup_network() -> P2pNetwork {
        let transport = Box::new(MockTransport::new());
        P2pNetwork::new(transport)
    }

    #[test]
    fn test_register_and_find_peers() {
        let mut network = setup_network();

        let id = network.register_peer(
            PeerInfo::new(Uuid::new_v4(), "Alpha", Uuid::new_v4(), "EU")
                .with_capacity(5000.0, "kg"),
        );

        assert_eq!(network.peer_count(), 1);
        assert!(network.get_peer(&id).is_some());
        assert_eq!(network.get_peer(&id).unwrap().name(), "Alpha");
    }

    #[test]
    fn test_find_capacity() {
        let mut network = setup_network();

        network.register_peer(
            PeerInfo::new(Uuid::new_v4(), "Alpha", Uuid::new_v4(), "EU")
                .with_capacity(10000.0, "kg"),
        );
        network.register_peer(
            PeerInfo::new(Uuid::new_v4(), "Beta", Uuid::new_v4(), "EU")
                .with_capacity(500.0, "kg"),
        );
        network.register_peer(
            PeerInfo::new(Uuid::new_v4(), "Gamma", Uuid::new_v4(), "NA")
                .with_capacity(20000.0, "kg"),
        );

        // Find EU peers with at least 1000 capacity
        let found = network.find_capacity("EU", 1000.0);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0.name(), "Alpha");
    }

    #[test]
    fn test_successful_negotiation() {
        let mut network = setup_network();
        let initiator = network.register_peer(
            PeerInfo::new(Uuid::new_v4(), "Shipper", Uuid::new_v4(), "EU"),
        );
        let responder = network.register_peer(
            PeerInfo::new(Uuid::new_v4(), "Carrier", Uuid::new_v4(), "EU")
                .with_capacity(5000.0, "kg"),
        );

        let offer = CapacityOffer {
            peer_id: responder,
            available_capacity: 5000.0,
            unit: "kg".into(),
            price_per_unit: 2.00,
            origin_node: Uuid::new_v4(),
            destination_node: None,
            window_start: 1000,
            window_end: 2000,
        };

        let request = CapacityRequest {
            peer_id: initiator,
            required_capacity: 1000.0,
            max_price_per_unit: 3.00,
            origin_node: Uuid::new_v4(),
            destination_node: Uuid::new_v4(),
        };

        let result = network.negotiate(initiator, responder, offer, request);
        assert!(result.is_some());
        assert_eq!(result.unwrap().capacity, 1000.0);
        assert_eq!(network.completed_negotiations().len(), 1);
    }
}