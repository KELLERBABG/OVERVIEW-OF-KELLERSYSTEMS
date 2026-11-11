use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub struct NetworkConfig {
    pub channel_size: usize,
    pub max_message_size: usize,
    pub initial_retry_interval_ms: u64,
    pub max_retry_interval_ms: u64,
    pub batch_timeout_ms: u64,
    pub enable_pipelining: bool,
    pub enable_batching: bool,
    pub handshake_timeout_ms: u64,
    pub read_timeout_ms: u64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            channel_size: 1024,
            max_message_size: 1 << 20,
            initial_retry_interval_ms: 1000,
            max_retry_interval_ms: 30000,
            batch_timeout_ms: 10,
            enable_pipelining: true,
            enable_batching: true,
            handshake_timeout_ms: 5000,
            read_timeout_ms: 30000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub node_id: Uuid,
    pub address: String,
    pub public_key_hex: String,
    pub reputation_score: f64,
}

impl PeerInfo {
    pub fn new(node_id: Uuid, address: String, public_key_hex: String) -> Self {
        Self {
            node_id,
            address,
            public_key_hex,
            reputation_score: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub node_id: Uuid,
    pub listen_addr: String,
    pub secret_key_hex: String,
    pub peers: Vec<PeerInfo>,
    pub consensus_threshold: usize,
    pub consensus_total: usize,
    #[serde(skip)]
    pub net_config: NetworkConfig,
}

impl NodeConfig {
    pub fn new(
        listen_addr: String,
        secret_key_hex: String,
        peers: Vec<PeerInfo>,
        threshold: usize,
        total: usize,
    ) -> Self {
        Self {
            node_id: Uuid::new_v4(),
            listen_addr,
            secret_key_hex,
            peers,
            consensus_threshold: threshold,
            consensus_total: total,
            net_config: NetworkConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_config_creation() {
        let peer = PeerInfo::new(Uuid::new_v4(), "127.0.0.1:9001".into(), "abc123".into());
        let config = NodeConfig::new("127.0.0.1:9000".into(), "sk_test".into(), vec![peer], 3, 5);
        assert_eq!(config.listen_addr, "127.0.0.1:9000");
        assert_eq!(config.peers.len(), 1);
    }

    #[test]
    fn test_peer_info_score_default() {
        let peer = PeerInfo::new(Uuid::new_v4(), "addr".into(), "pk".into());
        assert_eq!(peer.reputation_score, 1.0);
    }

    #[test]
    fn test_network_config_default() {
        let cfg = NetworkConfig::default();
        assert_eq!(cfg.channel_size, 1024);
        assert_eq!(cfg.max_message_size, 1 << 20);
        assert_eq!(cfg.initial_retry_interval_ms, 1000);
        assert_eq!(cfg.max_retry_interval_ms, 30000);
        assert_eq!(cfg.batch_timeout_ms, 10);
        assert!(cfg.enable_pipelining);
        assert!(cfg.enable_batching);
        assert_eq!(cfg.handshake_timeout_ms, 5000);
        assert_eq!(cfg.read_timeout_ms, 30000);
    }

    #[test]
    fn test_node_config_net_config_default() {
        let config = NodeConfig::new("127.0.0.1:0".into(), "sk".into(), vec![], 1, 1);
        assert_eq!(config.net_config.channel_size, 1024);
    }
}
