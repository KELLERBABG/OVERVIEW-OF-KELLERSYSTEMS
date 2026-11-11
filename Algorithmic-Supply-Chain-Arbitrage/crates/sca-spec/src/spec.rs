//! Logistics specification data structures.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Status of a logistics specification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SpecStatus {
    /// Draft — being generated.
    Draft,
    /// Finalized and signed.
    Final,
    /// In transit.
    InTransit,
    /// Delivered and confirmed.
    Delivered,
    /// Disputed.
    Disputed,
}

/// A single leg/carrier segment of a multi-modal shipment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipmentLeg {
    /// Sequence number in the shipment.
    pub sequence: usize,
    /// Origin node ID.
    pub origin_node: Uuid,
    /// Destination node ID.
    pub destination_node: Uuid,
    /// Carrier/transport provider.
    pub carrier: String,
    /// Transport mode (e.g., "truck", "rail", "ship", "air").
    pub mode: String,
    /// Estimated departure time (Unix seconds).
    pub estimated_departure: i64,
    /// Estimated arrival time (Unix seconds).
    pub estimated_arrival: i64,
    /// Cost for this leg (USD).
    pub cost: f64,
}

/// Cryptographic evidence of custody at a transfer point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodyEvidence {
    /// The custody record index in the payload chain.
    pub custody_index: usize,
    /// Node ID where custody was transferred.
    pub node_id: Uuid,
    /// Custodian name.
    pub custodian: String,
    /// Timestamp of transfer.
    pub timestamp: i64,
    /// SHA-256 hash of the custody record.
    pub record_hash: String,
}

/// A complete, machine-readable logistics specification.
///
/// Generated automatically from graph topology and payload data,
/// serves as the formal reference for bills of lading, customs,
/// and smart contract conditions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogisticsSpecification {
    /// Unique specification ID.
    pub id: Uuid,
    /// Related payload/shipment ID.
    pub payload_id: Uuid,
    /// Product description.
    pub product: String,
    /// Total quantity.
    pub quantity: f64,
    /// Unit of measure.
    pub unit: String,
    /// Origin node ID.
    pub origin: Uuid,
    /// Destination node ID.
    pub destination: Uuid,
    /// The shipment legs forming the route.
    pub legs: Vec<ShipmentLeg>,
    /// Custody evidence chain.
    pub custody_evidence: Vec<CustodyEvidence>,
    /// Total estimated cost (USD).
    pub total_cost: f64,
    /// Total estimated transit time (seconds).
    pub total_transit_time_secs: i64,
    /// Current status.
    pub status: SpecStatus,
    /// Generation timestamp.
    pub generated_at: i64,
    /// Perishability window (seconds), if applicable.
    pub perishability_window_secs: Option<i64>,
    /// Whether perishability is proven satisfied.
    pub perishability_proven: bool,
}

impl LogisticsSpecification {
    /// Create a new spec with the given payload and route.
    pub fn new(
        payload_id: Uuid,
        product: impl Into<String>,
        quantity: f64,
        unit: impl Into<String>,
        origin: Uuid,
        destination: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            payload_id,
            product: product.into(),
            quantity,
            unit: unit.into(),
            origin,
            destination,
            legs: Vec::new(),
            custody_evidence: Vec::new(),
            total_cost: 0.0,
            total_transit_time_secs: 0,
            status: SpecStatus::Draft,
            generated_at: chrono::Utc::now().timestamp(),
            perishability_window_secs: None,
            perishability_proven: false,
        }
    }

    /// Add a shipment leg.
    pub fn add_leg(&mut self, leg: ShipmentLeg) {
        self.total_cost += leg.cost;
        self.total_transit_time_secs += leg.estimated_arrival - leg.estimated_departure;
        self.legs.push(leg);
    }

    /// Add custody evidence.
    pub fn add_custody_evidence(&mut self, evidence: CustodyEvidence) {
        self.custody_evidence.push(evidence);
    }

    /// Set perishability parameters.
    pub fn set_perishability(&mut self, window_secs: i64, proven: bool) {
        self.perishability_window_secs = Some(window_secs);
        self.perishability_proven = proven;
    }

    /// Finalize the specification (mark as Final).
    pub fn finalize(&mut self) {
        self.status = SpecStatus::Final;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_spec() {
        let mut spec =
            LogisticsSpecification::new(Uuid::new_v4(), "Fresh Bananas", 10000.0, "kg", Uuid::new_v4(), Uuid::new_v4());

        spec.add_leg(ShipmentLeg {
            sequence: 1,
            origin_node: Uuid::new_v4(),
            destination_node: Uuid::new_v4(),
            carrier: "FastShip Inc.".into(),
            mode: "ship".into(),
            estimated_departure: 1000,
            estimated_arrival: 2000,
            cost: 5000.0,
        });

        assert_eq!(spec.legs.len(), 1);
        assert_eq!(spec.total_cost, 5000.0);
        assert_eq!(spec.total_transit_time_secs, 1000);
        assert_eq!(spec.product, "Fresh Bananas");
    }

    #[test]
    fn test_finalize_spec() {
        let mut spec =
            LogisticsSpecification::new(Uuid::new_v4(), "Wheat", 50000.0, "kg", Uuid::new_v4(), Uuid::new_v4());
        assert_eq!(spec.status, SpecStatus::Draft);
        spec.finalize();
        assert_eq!(spec.status, SpecStatus::Final);
    }
}
