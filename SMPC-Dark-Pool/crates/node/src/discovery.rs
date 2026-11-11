use crate::config::{NodeConfig, PeerInfo};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::Uuid;

pub struct NodeDiscovery {
    pub known_peers: HashMap<Uuid, PeerInfo>,
    pub blacklisted: Vec<Uuid>,
}

impl NodeDiscovery {
    pub fn new(config: &NodeConfig) -> Self {
        let known_peers: HashMap<_, _> = config
            .peers
            .iter()
            .map(|p| (p.node_id, p.clone()))
            .collect();
        Self {
            known_peers,
            blacklisted: Vec::new(),
        }
    }

    pub fn register_peer(&mut self, peer: PeerInfo) {
        self.known_peers.insert(peer.node_id, peer);
    }

    pub fn remove_peer(&mut self, node_id: &Uuid) {
        self.known_peers.remove(node_id);
    }

    pub fn blacklist(&mut self, node_id: Uuid) {
        self.known_peers.remove(&node_id);
        if !self.blacklisted.contains(&node_id) {
            self.blacklisted.push(node_id);
        }
    }

    pub fn is_blacklisted(&self, node_id: &Uuid) -> bool {
        self.blacklisted.contains(node_id)
    }

    pub fn active_peers(&self) -> Vec<&PeerInfo> {
        self.known_peers.values().collect()
    }

    pub fn peer_count(&self) -> usize {
        self.known_peers.len()
    }

    pub fn discover_peer_id(address: &str, public_key: &str) -> Uuid {
        let mut hasher = Sha256::new();
        hasher.update(address.as_bytes());
        hasher.update(public_key.as_bytes());
        let hash = hasher.finalize();
        let bytes: [u8; 16] = hash[..16].try_into().unwrap_or([0u8; 16]);
        Uuid::from_bytes(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PeerInfo;

    fn make_config() -> NodeConfig {
        let peer = PeerInfo::new(Uuid::new_v4(), "192.168.1.1:9001".into(), "pk1".into());
        NodeConfig::new("0.0.0.0:9000".into(), "sk_test".into(), vec![peer], 3, 5)
    }

    #[test]
    fn test_discovery_initializes_with_peers() {
        let config = make_config();
        let discovery = NodeDiscovery::new(&config);
        assert_eq!(discovery.peer_count(), 1);
    }

    #[test]
    fn test_register_and_remove() {
        let config = make_config();
        let mut discovery = NodeDiscovery::new(&config);
        let new_peer = PeerInfo::new(Uuid::new_v4(), "10.0.0.1:9002".into(), "pk2".into());
        discovery.register_peer(new_peer.clone());
        assert_eq!(discovery.peer_count(), 2);
        discovery.remove_peer(&new_peer.node_id);
        assert_eq!(discovery.peer_count(), 1);
    }

    #[test]
    fn test_blacklist() {
        let config = make_config();
        let mut discovery = NodeDiscovery::new(&config);
        let peer_id = *discovery.known_peers.keys().next().unwrap();
        discovery.blacklist(peer_id);
        assert!(discovery.is_blacklisted(&peer_id));
        assert_eq!(discovery.peer_count(), 0);
    }

    #[test]
    fn test_discover_peer_id_deterministic() {
        let id1 = NodeDiscovery::discover_peer_id("addr1", "pk1");
        let id2 = NodeDiscovery::discover_peer_id("addr1", "pk1");
        assert_eq!(id1, id2);
    }
}
