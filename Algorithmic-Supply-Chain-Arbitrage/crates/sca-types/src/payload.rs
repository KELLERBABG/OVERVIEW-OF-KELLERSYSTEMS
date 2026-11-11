//! Payload tracking types for taint-coded cargo integrity.
//!
//! Defines the cryptographic integrity chain for freight:
//! each custody transfer is signed, forming an auditable
//! chain-of-custody from producer to final consumer.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single custody event in the chain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CustodyEvent {
    /// The node ID where custody was transferred.
    pub location_id: Uuid,
    /// Timestamp (Unix epoch seconds).
    pub timestamp: i64,
    /// The entity taking custody.
    pub custodian: String,
    /// ed25519 signature of the previous event hash + this event data.
    pub signature: Option<Vec<u8>>,
    /// Hash of the previous custody event (linking the chain).
    pub previous_hash: Option<Vec<u8>>,
}

/// The full payload manifest for a shipment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Payload {
    /// Unique shipment identifier.
    pub id: Uuid,
    /// Product name/description.
    pub product: String,
    /// Quantity (kg, pallets, etc.).
    pub quantity: f64,
    /// Unit of measure.
    pub unit: String,
    /// Origin node ID.
    pub origin: Uuid,
    /// Destination node ID.
    pub destination: Uuid,
    /// The chain of custody events.
    pub custody_chain: Vec<CustodyEvent>,
    /// Perishability window in seconds (None if non-perishable).
    pub perishability_window_secs: Option<i64>,
}

impl Payload {
    /// Create a new payload.
    pub fn new(
        product: impl Into<String>,
        quantity: f64,
        unit: impl Into<String>,
        origin: Uuid,
        destination: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            product: product.into(),
            quantity,
            unit: unit.into(),
            origin,
            destination,
            custody_chain: Vec::new(),
            perishability_window_secs: None,
        }
    }

    /// Mark this payload as perishable with a maximum transit time (seconds).
    pub fn with_perishability(mut self, window_secs: i64) -> Self {
        self.perishability_window_secs = Some(window_secs);
        self
    }

    /// Add a custody event to the chain.
    pub fn add_custody_event(&mut self, event: CustodyEvent) {
        self.custody_chain.push(event);
    }

    /// The total transit time so far (seconds), or None if no events.
    pub fn transit_time_secs(&self) -> Option<i64> {
        let first = self.custody_chain.first()?.timestamp;
        let last = self.custody_chain.last()?.timestamp;
        Some(last - first)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_payload() {
        let p = Payload::new("Bananas", 1000.0, "kg", Uuid::new_v4(), Uuid::new_v4());
        assert_eq!(p.product, "Bananas");
        assert_eq!(p.quantity, 1000.0);
        assert!(p.perishability_window_secs.is_none());
    }

    #[test]
    fn test_perishable_payload() {
        let p = Payload::new("Milk", 500.0, "L", Uuid::new_v4(), Uuid::new_v4())
            .with_perishability(86400); // 24 hours
        assert_eq!(p.perishability_window_secs, Some(86400));
    }

    #[test]
    fn test_custody_chain_transit_time() {
        let mut p = Payload::new("Electronics", 10.0, "pcs", Uuid::new_v4(), Uuid::new_v4());
        p.add_custody_event(CustodyEvent {
            location_id: Uuid::new_v4(),
            timestamp: 1000,
            custodian: "Warehouse A".into(),
            signature: None,
            previous_hash: None,
        });
        p.add_custody_event(CustodyEvent {
            location_id: Uuid::new_v4(),
            timestamp: 5000,
            custodian: "Warehouse B".into(),
            signature: None,
            previous_hash: None,
        });
        assert_eq!(p.transit_time_secs(), Some(4000));
    }
}