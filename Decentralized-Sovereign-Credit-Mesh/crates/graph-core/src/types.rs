use serde::{Deserialize, Serialize};

pub type NodeId = String;
pub type TrustWeight = f64;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RelationshipType {
    Friend,
    Family,
    Colleague,
    Guarantor,
    Custom(String),
}

impl std::fmt::Display for RelationshipType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelationshipType::Friend => write!(f, "friend"),
            RelationshipType::Family => write!(f, "family"),
            RelationshipType::Colleague => write!(f, "colleague"),
            RelationshipType::Guarantor => write!(f, "guarantor"),
            RelationshipType::Custom(s) => write!(f, "custom({})", s),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeData {
    pub id: NodeId,
    pub display_name: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeData {
    pub relationship: RelationshipType,
    pub trust_weight: TrustWeight,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSnapshot {
    pub nodes: Vec<NodeData>,
    pub edges: Vec<(NodeId, NodeId, EdgeData)>,
}
