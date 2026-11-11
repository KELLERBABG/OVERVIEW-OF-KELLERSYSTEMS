//! Core domain types for the Algorithmic Supply Chain Arbitrage platform.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for a supply chain node.
pub type NodeId = Uuid;

/// Unique identifier for a supply chain edge.
pub type EdgeId = Uuid;

/// Unique identifier for a freight order.
pub type OrderId = Uuid;

/// Comparison operators for constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonOp {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}

impl ComparisonOp {
    pub fn evaluate(&self, a: f64, b: f64) -> bool {
        match self {
            ComparisonOp::Lt => a < b,
            ComparisonOp::Le => a <= b,
            ComparisonOp::Gt => a > b,
            ComparisonOp::Ge => a >= b,
            ComparisonOp::Eq => (a - b).abs() < 1e-9,
            ComparisonOp::Ne => (a - b).abs() >= 1e-9,
        }
    }
}

/// Variables that can be constrained in the SMT engine.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConstraintVariable {
    Cost,
    Time,
    Carbon,
    Reliability,
    Capacity,
    Temperature,
    BorderTime,
    FuelEfficiency,
    Custom(String),
}

/// SMT Constraint expression over supply chain variables.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConstraintExpr {
    Literal(bool),
    Comparison { variable: ConstraintVariable, op: ComparisonOp, value: f64 },
    And(Box<ConstraintExpr>, Box<ConstraintExpr>),
    Or(Box<ConstraintExpr>, Box<ConstraintExpr>),
    Not(Box<ConstraintExpr>),
}

impl ConstraintExpr {
    pub fn always_true() -> Self {
        ConstraintExpr::Literal(true)
    }

    pub fn always_false() -> Self {
        ConstraintExpr::Literal(false)
    }

    pub fn cost(op: ComparisonOp, value: f64) -> Self {
        ConstraintExpr::Comparison { variable: ConstraintVariable::Cost, op, value }
    }

    pub fn time(op: ComparisonOp, value: f64) -> Self {
        ConstraintExpr::Comparison { variable: ConstraintVariable::Time, op, value }
    }

    pub fn carbon(op: ComparisonOp, value: f64) -> Self {
        ConstraintExpr::Comparison { variable: ConstraintVariable::Carbon, op, value }
    }

    pub fn reliability(op: ComparisonOp, value: f64) -> Self {
        ConstraintExpr::Comparison { variable: ConstraintVariable::Reliability, op, value }
    }

    pub fn capacity(op: ComparisonOp, value: f64) -> Self {
        ConstraintExpr::Comparison { variable: ConstraintVariable::Capacity, op, value }
    }

    pub fn evaluate(&self, vars: &HashMap<ConstraintVariable, f64>) -> Option<bool> {
        match self {
            ConstraintExpr::Literal(b) => Some(*b),
            ConstraintExpr::Comparison { variable, op, value } => {
                vars.get(variable).map(|&actual| op.evaluate(actual, *value))
            }
            ConstraintExpr::And(left, right) => match (left.evaluate(vars), right.evaluate(vars)) {
                (Some(a), Some(b)) => Some(a && b),
                _ => None,
            },
            ConstraintExpr::Or(left, right) => match (left.evaluate(vars), right.evaluate(vars)) {
                (Some(a), Some(b)) => Some(a || b),
                _ => None,
            },
            ConstraintExpr::Not(inner) => inner.evaluate(vars).map(|b| !b),
        }
    }
}

/// An edge constraint linking an edge ID to a constraint expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeConstraint {
    pub edge_id: EdgeId,
    pub name: String,
    pub expression: ConstraintExpr,
}

impl EdgeConstraint {
    pub fn new(edge_id: EdgeId, expression: ConstraintExpr) -> Self {
        Self { edge_id, name: "Constraint".into(), expression }
    }
}

/// Global error type for the SCA platform.
#[derive(Debug, thiserror::Error)]
pub enum ScaError {
    #[error("Graph error: {0}")]
    Graph(String),
    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),
    #[error("Solver error: {0}")]
    Solver(String),
    #[error("Payload error: {0}")]
    Payload(String),
    #[error("Not found: {0}")]
    NotFound(String),
}

/// Payload and custody tracking domain models.
pub mod payload {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub enum CustodyEventType {
        Created,
        Transfer,
        Inspection,
        Delivered,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CustodyEvent {
        pub location_id: NodeId,
        pub timestamp: i64,
        pub custodian: String,
        pub signature: Option<Vec<u8>>,
        pub previous_hash: Option<Vec<u8>>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Payload {
        pub id: Uuid,
        pub product: String,
        pub quantity: f64,
        pub unit: String,
        pub origin: NodeId,
        pub destination: NodeId,
        pub perishable: bool,
        pub perishability_window_secs: Option<i64>,
        pub created_at: DateTime<Utc>,
        pub custody_events: Vec<CustodyEvent>,
    }

    impl Payload {
        pub fn new(
            product: impl Into<String>,
            quantity: f64,
            unit: impl Into<String>,
            origin: NodeId,
            destination: NodeId,
        ) -> Self {
            Self {
                id: Uuid::new_v4(),
                product: product.into(),
                quantity,
                unit: unit.into(),
                origin,
                destination,
                perishable: false,
                perishability_window_secs: None,
                created_at: Utc::now(),
                custody_events: Vec::new(),
            }
        }

        pub fn with_perishability(mut self, window_secs: i64) -> Self {
            self.perishable = true;
            self.perishability_window_secs = Some(window_secs);
            self
        }

        pub fn add_custody_event(&mut self, event: CustodyEvent) {
            self.custody_events.push(event);
        }

        pub fn transit_time_secs(&self) -> Option<i64> {
            if self.custody_events.len() < 2 {
                return None;
            }
            let first = self.custody_events.first()?.timestamp;
            let last = self.custody_events.last()?.timestamp;
            Some(last - first)
        }
    }
}

/// Type of supply chain node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    Producer,
    Consumer,
    TransportHub,
    ProcessingFacility,
    StorageFacility,
    Supplier,
    Warehouse,
    DistributionCenter,
    CarrierHub,
    Retailer,
    Port,
    Airport,
    RailTerminal,
}

/// Node metadata in the topology graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    #[serde(default)]
    pub id: NodeId,
    pub name: String,
    pub node_type: NodeType,
    pub latitude: f64,
    pub longitude: f64,
    pub capacity: f64,
    pub available_capacity: f64,
    pub created_at: DateTime<Utc>,
}

impl Node {
    pub fn new(node_type: NodeType, name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            node_type,
            latitude: 0.0,
            longitude: 0.0,
            capacity: 1000.0,
            available_capacity: 1000.0,
            created_at: Utc::now(),
        }
    }

    pub fn with_geo(mut self, lat: f64, lon: f64) -> Self {
        self.latitude = lat;
        self.longitude = lon;
        self
    }

    pub fn with_capacity(mut self, cap: f64) -> Self {
        self.capacity = cap;
        self.available_capacity = cap;
        self
    }
}

/// Type of supply chain edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeType {
    Shipment,
    Storage,
    Processing,
    Transit,
}

/// Transport mode between nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportMode {
    Road,
    Rail,
    Air,
    Maritime,
    Multimodal,
}

/// Directed edge representing a freight corridor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    #[serde(default)]
    pub id: EdgeId,
    pub source: NodeId,
    pub target: NodeId,
    pub edge_type: EdgeType,
    pub label: String,
    pub weight: f64,
    pub transport_mode: TransportMode,
    pub distance_km: f64,
    pub base_cost: f64,
    pub capacity_tons: f64,
    pub lead_time_hours: f64,
    pub reliability_score: f64,
}

impl Edge {
    pub fn new(edge_type: EdgeType, source: NodeId, target: NodeId, label: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            source,
            target,
            edge_type,
            label: label.into(),
            weight: 1.0,
            transport_mode: TransportMode::Road,
            distance_km: 100.0,
            base_cost: 100.0,
            capacity_tons: 100.0,
            lead_time_hours: 2.0,
            reliability_score: 1.0,
        }
    }

    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self.base_cost = weight;
        self
    }

    pub fn with_mode(mut self, mode: TransportMode) -> Self {
        self.transport_mode = mode;
        self
    }

    pub fn with_distance(mut self, distance_km: f64) -> Self {
        self.distance_km = distance_km;
        self
    }
}

/// Freight shipment commodity requirements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommodityRequirement {
    pub commodity_id: String,
    pub weight_tons: f64,
    pub volume_m3: f64,
    pub requires_refrigeration: bool,
    pub max_temperature_celsius: Option<f64>,
    pub max_delivery_hours: f64,
}

/// Bid or Ask in the supply-demand capacity matching engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityOffer {
    pub id: Uuid,
    pub carrier_id: Uuid,
    pub origin_node_id: NodeId,
    pub destination_node_id: NodeId,
    pub available_weight_tons: f64,
    pub unit_price_per_ton_km: f64,
    pub departure_time: DateTime<Utc>,
    pub arrival_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreightDemand {
    pub id: OrderId,
    pub shipper_id: Uuid,
    pub origin_node_id: NodeId,
    pub destination_node_id: NodeId,
    pub required_weight_tons: f64,
    pub max_budget: f64,
    pub deadline: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let node = Node::new(NodeType::Port, "Hamburg Port").with_capacity(50000.0);
        assert_eq!(node.name, "Hamburg Port");
        assert_eq!(node.node_type, NodeType::Port);
        assert_eq!(node.available_capacity, 50000.0);
    }

    #[test]
    fn test_edge_creation() {
        let n1 = Uuid::new_v4();
        let n2 = Uuid::new_v4();
        let edge = Edge::new(EdgeType::Shipment, n1, n2, "Corridor").with_weight(300.0);
        assert_eq!(edge.source, n1);
        assert_eq!(edge.target, n2);
        assert_eq!(edge.weight, 300.0);
    }
}
