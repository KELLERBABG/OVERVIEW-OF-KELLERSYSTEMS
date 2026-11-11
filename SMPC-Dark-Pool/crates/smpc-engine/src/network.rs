use sss::Share;
use std::collections::HashMap;

pub type PartyId = u64;

#[derive(Debug, Clone)]
pub struct NetworkMessage {
    pub from: PartyId,
    pub to: PartyId,
    pub share: Share,
}

pub struct Node {
    pub id: PartyId,
    pub shares: HashMap<String, Share>,
}

impl Node {
    pub fn new(id: PartyId) -> Self {
        Self {
            id,
            shares: HashMap::new(),
        }
    }

    pub fn store_share(&mut self, key: impl Into<String>, share: Share) {
        self.shares.insert(key.into(), share);
    }

    pub fn get_share(&self, key: impl Into<String>) -> Option<&Share> {
        self.shares.get(&key.into())
    }

    pub fn send_share(&self, to: PartyId, key: impl Into<String>) -> NetworkMessage {
        let key_str = key.into();
        NetworkMessage {
            from: self.id,
            to,
            share: self.shares[&key_str].clone(),
        }
    }
}

pub struct Network {
    pub nodes: Vec<Node>,
}

impl Network {
    pub fn new(num_nodes: u64) -> Self {
        let nodes = (1..=num_nodes).map(Node::new).collect();
        Self { nodes }
    }

    pub fn broadcast(&self, from_id: PartyId, key: impl Into<String>) -> Vec<NetworkMessage> {
        let key_str = key.into();
        let from = &self.nodes[from_id as usize - 1];
        self.nodes
            .iter()
            .filter(|n| n.id != from_id)
            .map(|_| from.send_share(from_id, key_str.clone()))
            .collect()
    }

    pub fn node_mut(&mut self, id: PartyId) -> &mut Node {
        &mut self.nodes[id as usize - 1]
    }

    pub fn node(&self, id: PartyId) -> &Node {
        &self.nodes[id as usize - 1]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::thread_rng;
    use sss::field::random_scalar;
    use sss::sss::split_secret;

    #[test]
    fn test_network_distribution() {
        let mut rng = thread_rng();
        let secret = random_scalar(&mut rng);
        let shares = split_secret(secret, 5, 3).unwrap();
        let mut net = Network::new(5);
        for (i, share) in shares.into_iter().enumerate() {
            let id = (i + 1) as u64;
            net.node_mut(id).store_share("order_price", share);
        }
        for id in 1..=5 {
            let s = net.node(id).get_share("order_price");
            assert!(s.is_some());
        }
    }
}
