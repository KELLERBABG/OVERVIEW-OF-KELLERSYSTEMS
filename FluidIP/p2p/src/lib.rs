//! # Decentralized Network Layer — P2P Networking
//!
//! Abstract P2P networking layer for patent graph propagation,
//! verification request distribution, and consensus on royalty invariants.
//!
//! In production this would use libp2p (gossipsub, kademlia, identify).
//! Here we define the message types and a simulated network node.

use dfip_core::graph::PatentDag;
use dfip_core::models::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

// ─── Network Messages ────────────────────────────────────────────────────────

/// Types of messages exchanged on the P2P network.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NetworkMessage {
    /// Announce a new patent node.
    AnnounceNode(PatentNode),
    /// Announce a new derivation edge.
    AnnounceEdge(Edge),
    /// Request the full patent graph state from a peer.
    SyncRequest { requester_id: Uuid, last_known_hash: Vec<u8> },
    /// Response to a sync request — batch of nodes and edges.
    SyncResponse {
        nodes: Vec<PatentNode>,
        edges: Vec<Edge>,
        merkle_root: Vec<u8>,
    },
    /// Submit a verification task to the network.
    VerificationRequest {
        edge_id: Uuid,
        original_id: Uuid,
        derivative_id: Uuid,
        formula_hash: Vec<u8>,
    },
    /// Verification result from a network verifier.
    VerificationResponse {
        edge_id: Uuid,
        result: VerificationResult,
        verifier_id: Uuid,
        signature: Vec<u8>,
    },
    /// Heartbeat / presence announcement.
    Ping { node_id: Uuid, timestamp: i64 },
}

/// A peer in the network.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Peer {
    pub id: Uuid,
    pub address: String,
    pub known_patents: usize,
    pub is_verifier: bool,
    pub reputation: f64,
}

// ─── Simulated Network Node ──────────────────────────────────────────────────

/// A node in the DFIP P2P network.
pub struct NetworkNode {
    pub id: Uuid,
    pub dag: PatentDag,
    peers: HashMap<Uuid, Peer>,
    message_log: Vec<(NetworkMessage, Uuid)>, // (message, from_peer)
}

impl NetworkNode {
    /// Create a new network node with the given identity.
    pub fn new(id: Uuid) -> Self {
        Self {
            id,
            dag: PatentDag::new(),
            peers: HashMap::new(),
            message_log: Vec::new(),
        }
    }

    /// Connect to a peer.
    pub fn add_peer(&mut self, peer: Peer) {
        self.peers.insert(peer.id, peer);
    }

    /// Broadcast a message to all connected peers.
    pub fn broadcast(&mut self, msg: NetworkMessage) {
        let peer_ids: Vec<Uuid> = self.peers.keys().copied().collect();
        for peer_id in peer_ids {
            self.send_to(peer_id, msg.clone());
        }
    }

    /// Send a message to a specific peer.
    pub fn send_to(&mut self, peer_id: Uuid, msg: NetworkMessage) {
        if self.peers.contains_key(&peer_id) {
            self.message_log.push((msg, peer_id));
        }
    }

    /// Receive a message and process it.
    pub fn receive(&mut self, from: Uuid, msg: NetworkMessage) -> Option<NetworkMessage> {
        match &msg {
            NetworkMessage::AnnounceNode(node) => {
                let _ = self.dag.add_node(node.clone());
                // Relay to other peers
                Some(NetworkMessage::AnnounceNode(node.clone()))
            }
            NetworkMessage::AnnounceEdge(edge) => {
                // Edge only accepted if nodes exist
                let _ = self.dag.add_edge(&edge.parent_id, &edge.child_id, edge.clone());
                Some(NetworkMessage::AnnounceEdge(edge.clone()))
            }
            NetworkMessage::SyncRequest { requester_id, last_known_hash } => {
                let state = self.dag.export_state();
                let nodes: Vec<PatentNode> = state.nodes.into_values().collect();
                let edges = state.edges;
                let merkle_root = self.dag.last_event_hash().to_vec();
                Some(NetworkMessage::SyncResponse {
                    nodes,
                    edges,
                    merkle_root,
                })
            }
            NetworkMessage::VerificationRequest { .. } => {
                // In production: assign to a verifier node.
                // For now: acknowledge.
                None
            }
            _ => None,
        }
    }

    /// Get the message log.
    pub fn message_log(&self) -> &[(NetworkMessage, Uuid)] {
        &self.message_log
    }

    /// Get connected peers.
    pub fn peers(&self) -> &HashMap<Uuid, Peer> {
        &self.peers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dfip_core::models::*;

    fn make_node(id: Uuid) -> PatentNode {
        PatentNode {
            id,
            title: "P2P Test".into(),
            patent_type: PatentType::Software,
            authors: vec!["Alice".into()],
            created_at_unix: 0,
            content_hash: vec![],
            invariant: None,
            tags: vec![],
            license: License::Open,
            meta_commitment: vec![],
        }
    }

    #[test]
    fn test_announce_node_propagation() {
        let mut alice = NetworkNode::new(Uuid::new_v4());
        let mut bob = NetworkNode::new(Uuid::new_v4());

        alice.add_peer(Peer {
            id: bob.id,
            address: "127.0.0.1:9001".into(),
            known_patents: 0,
            is_verifier: true,
            reputation: 1.0,
        });

        let node = make_node(Uuid::new_v4());
        let msg = NetworkMessage::AnnounceNode(node.clone());

        // Alice broadcasts to Bob
        alice.broadcast(msg.clone());
        assert_eq!(alice.message_log().len(), 1);

        // Bob receives and adds to his DAG
        let response = bob.receive(alice.id, msg).unwrap();
        assert!(bob.dag.contains_node(&node.id));
        assert!(matches!(response, NetworkMessage::AnnounceNode(_)));
    }

    #[test]
    fn test_sync_request() {
        let mut alice = NetworkNode::new(Uuid::new_v4());
        let mut bob = NetworkNode::new(Uuid::new_v4());

        // Alice has a node
        let node = make_node(Uuid::new_v4());
        alice.dag.add_node(node).unwrap();

        // Bob sends a SyncRequest to Alice.
        // Alice receives it and responds.
        let response = alice
            .receive(
                bob.id,
                NetworkMessage::SyncRequest {
                    requester_id: bob.id,
                    last_known_hash: vec![],
                },
            )
            .unwrap();

        match response {
            NetworkMessage::SyncResponse { nodes, edges, .. } => {
                assert_eq!(nodes.len(), 1);
                assert_eq!(edges.len(), 0);
            }
            _ => panic!("expected SyncResponse"),
        }
    }
}