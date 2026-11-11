//! Graph topology types for supply chain mapping.
//!
//! Defines node types (producers, transport hubs, consumers),
//! edge types (shipments, custody transfers), and the overall
//! graph structure representing global freight flow.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a graph node.
pub type NodeId = Uuid;

/// Unique identifier for a graph edge.
pub type EdgeId = Uuid;

/// The role a node plays in the supply chain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NodeType {
    /// A producer of goods (farm, factory, manufacturer).
    Producer,
    /// A logistics hub (warehouse, port, distribution center).
    TransportHub,
    /// A consumer of goods (retailer, end customer).
    Consumer,
}

/// The kind of relationship represented by an edge.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EdgeType {
    /// A physical shipment of goods between two nodes.
    Shipment,
    /// A change of custody or ownership without physical movement.
    CustodyTransfer,
}

/// A node in the supply chain graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Node {
    /// Unique identifier.
    pub id: NodeId,
    /// Human-readable label (e.g., "Port of Rotterdam").
    pub label: String,
    /// The role of this node.
    pub node_type: NodeType,
    /// Geographic coordinates (decimal degrees).
    pub latitude: Option<f64>,
    /// Geographic coordinates (decimal degrees).
    pub longitude: Option<f64>,
    /// Arbitrary metadata key-value pairs.
    pub metadata: Vec<(String, String)>,
}

impl Node {
    /// Create a new node with the given type and label. Generates a random UUID.
    pub fn new(node_type: NodeType, label: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            label: label.into(),
            node_type,
            latitude: None,
            longitude: None,
            metadata: Vec::new(),
        }
    }

    /// Set geographic coordinates.
    pub fn with_coordinates(mut self, lat: f64, lon: f64) -> Self {
        self.latitude = Some(lat);
        self.longitude = Some(lon);
        self
    }

    /// Add a metadata key-value pair.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }
}

/// A directed edge between two nodes in the supply chain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Edge {
    /// Unique identifier.
    pub id: EdgeId,
    /// The source node ID.
    pub source: NodeId,
    /// The target node ID.
    pub target: NodeId,
    /// The kind of relationship.
    pub edge_type: EdgeType,
    /// Human-readable label.
    pub label: String,
    /// Weight/cost value for path optimization (e.g., transit cost in USD).
    pub weight: Option<f64>,
    /// Arbitrary metadata key-value pairs.
    pub metadata: Vec<(String, String)>,
}

impl Edge {
    /// Create a new directed edge.
    pub fn new(
        edge_type: EdgeType,
        source: NodeId,
        target: NodeId,
        label: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            source,
            target,
            edge_type,
            label: label.into(),
            weight: None,
            metadata: Vec::new(),
        }
    }

    /// Set a weight/cost for this edge.
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = Some(weight);
        self
    }

    /// Add a metadata key-value pair.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_producer_node() {
        let node = Node::new(NodeType::Producer, "Farm Alpha");
        assert_eq!(node.node_type, NodeType::Producer);
        assert_eq!(node.label, "Farm Alpha");
        assert!(node.latitude.is_none());
    }

    #[test]
    fn test_create_node_with_coordinates() {
        let node = Node::new(NodeType::TransportHub, "Port of Rotterdam")
            .with_coordinates(51.9225, 4.47917);
        assert_eq!(node.latitude, Some(51.9225));
        assert_eq!(node.longitude, Some(4.47917));
    }

    #[test]
    fn test_create_edge() {
        let src = Uuid::new_v4();
        let tgt = Uuid::new_v4();
        let edge = Edge::new(EdgeType::Shipment, src, tgt, "Shipment #42");
        assert_eq!(edge.source, src);
        assert_eq!(edge.target, tgt);
        assert_eq!(edge.edge_type, EdgeType::Shipment);
    }

    #[test]
    fn test_edge_with_weight_and_metadata() {
        let src = Uuid::new_v4();
        let tgt = Uuid::new_v4();
        let edge = Edge::new(EdgeType::CustodyTransfer, src, tgt, "Transfer")
            .with_weight(1500.0)
            .with_metadata("temp_range", "2-8°C");
        assert_eq!(edge.weight, Some(1500.0));
        assert_eq!(edge.metadata[0], ("temp_range".to_string(), "2-8°C".to_string()));
    }

    #[test]
    fn test_node_id_uniqueness() {
        let n1 = Node::new(NodeType::Producer, "A");
        let n2 = Node::new(NodeType::Producer, "B");
        assert_ne!(n1.id, n2.id);
    }
}