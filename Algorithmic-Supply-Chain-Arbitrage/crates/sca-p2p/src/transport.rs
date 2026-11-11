//! Abstract transport layer for P2P communication.
//!
//! Defines the `Transport` trait that abstracts over libp2p or
//! any other P2P transport. A `MockTransport` is provided for
//! testing without network infrastructure.

use crate::peer::PeerId;
use crate::protocol::Message;
use std::collections::HashMap;

/// Errors that can occur during transport operations.
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("Peer {0} not found")]
    PeerNotFound(PeerId),
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Message delivery failed: {0}")]
    DeliveryFailed(String),
}

/// Abstract P2P transport trait.
/// In production, implemented by libp2p; in tests, by MockTransport.
pub trait Transport: Send + Sync {
    /// Connect to a peer.
    fn connect(&mut self, peer_id: PeerId) -> Result<(), TransportError>;
    /// Disconnect from a peer.
    fn disconnect(&mut self, peer_id: PeerId) -> Result<(), TransportError>;
    /// Send a message to a peer.
    fn send(&self, peer_id: PeerId, message: Message) -> Result<(), TransportError>;
    /// Receive pending messages (non-blocking).
    fn receive(&self) -> Vec<(PeerId, Message)>;
    /// Check if connected to a peer.
    fn is_connected(&self, peer_id: PeerId) -> bool;
}

/// Mock transport for testing without libp2p.
/// Messages are stored in memory and can be inspected.
#[derive(Debug, Clone)]
pub struct MockTransport {
    connections: HashMap<PeerId, bool>,
    inbox: Vec<(PeerId, Message)>,
    outbox: Vec<(PeerId, Message)>,
}

impl MockTransport {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            inbox: Vec::new(),
            outbox: Vec::new(),
        }
    }

    /// Simulate receiving a message from a peer.
    pub fn inject_message(&mut self, from: PeerId, message: Message) {
        self.inbox.push((from, message));
    }

    /// Get all sent messages (outbox).
    pub fn sent_messages(&self) -> &[(PeerId, Message)] {
        &self.outbox
    }

    /// Count sent messages.
    pub fn sent_count(&self) -> usize {
        self.outbox.len()
    }
}

impl Default for MockTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl Transport for MockTransport {
    fn connect(&mut self, peer_id: PeerId) -> Result<(), TransportError> {
        self.connections.insert(peer_id, true);
        Ok(())
    }

    fn disconnect(&mut self, peer_id: PeerId) -> Result<(), TransportError> {
        self.connections.insert(peer_id, false);
        Ok(())
    }

    fn send(&self, peer_id: PeerId, _message: Message) -> Result<(), TransportError> {
        if !self.connections.get(&peer_id).copied().unwrap_or(false) {
            return Err(TransportError::PeerNotFound(peer_id));
        }
        Ok(())
    }

    fn receive(&self) -> Vec<(PeerId, Message)> {
        self.inbox.clone()
    }

    fn is_connected(&self, peer_id: PeerId) -> bool {
        self.connections.get(&peer_id).copied().unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_mock_connect_disconnect() {
        let mut transport = MockTransport::new();
        let pid = PeerId::new_v4();

        assert!(!transport.is_connected(pid));
        transport.connect(pid).unwrap();
        assert!(transport.is_connected(pid));
        transport.disconnect(pid).unwrap();
        assert!(!transport.is_connected(pid));
    }
}