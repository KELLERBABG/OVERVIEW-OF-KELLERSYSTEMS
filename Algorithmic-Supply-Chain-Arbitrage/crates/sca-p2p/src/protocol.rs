//! P2P protocol messages and negotiation logic.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The current status of a negotiation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NegotiationStatus {
    /// Offer sent, awaiting response.
    Pending,
    /// Counter-offer made.
    Countered,
    /// Negotiation accepted.
    Accepted,
    /// Negotiation rejected.
    Rejected,
    /// Negotiation completed with a deal.
    Completed,
}

/// A typed message in the P2P protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub msg_type: MessageType,
    pub sender: Uuid,
    pub recipient: Uuid,
    pub payload: Vec<u8>,
}

/// Types of messages in the P2P protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    CapacityOffer,
    CapacityRequest,
    NegotiationAccept,
    NegotiationReject,
    NegotiationCounter,
    SettlementProposal,
    SettlementConfirm,
    Ping,
    Pong,
}

impl Message {
    pub fn new(msg_type: MessageType, sender: Uuid, recipient: Uuid, payload: Vec<u8>) -> Self {
        Self {
            msg_type,
            sender,
            recipient,
            payload,
        }
    }
}

/// A peer advertises available capacity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityOffer {
    /// The offering peer.
    pub peer_id: Uuid,
    /// Available capacity in standard units.
    pub available_capacity: f64,
    /// Unit of capacity.
    pub unit: String,
    /// Price per unit of capacity (USD).
    pub price_per_unit: f64,
    /// Origin node ID.
    pub origin_node: Uuid,
    /// Destination node ID (if known).
    pub destination_node: Option<Uuid>,
    /// Time window for the offer (Unix start/end).
    pub window_start: i64,
    pub window_end: i64,
}

/// A peer requests capacity for a shipment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityRequest {
    /// The requesting peer.
    pub peer_id: Uuid,
    /// Required capacity.
    pub required_capacity: f64,
    /// Maximum price willing to pay per unit.
    pub max_price_per_unit: f64,
    /// Origin node ID.
    pub origin_node: Uuid,
    /// Destination node ID.
    pub destination_node: Uuid,
}

/// Result of a completed negotiation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NegotiationResult {
    pub offer_id: Uuid,
    pub request_id: Uuid,
    pub capacity: f64,
    pub agreed_price: f64,
    pub total_cost: f64,
    pub status: NegotiationStatus,
}

/// A negotiation session between two peers.
#[derive(Debug, Clone)]
pub struct Negotiation {
    pub id: Uuid,
    pub initiator: Uuid,
    pub responder: Uuid,
    pub offer: CapacityOffer,
    pub request: CapacityRequest,
    pub status: NegotiationStatus,
    pub counter_offers: Vec<CapacityOffer>,
    pub created_at: i64,
}

impl Negotiation {
    pub fn new(initiator: Uuid, responder: Uuid, offer: CapacityOffer, request: CapacityRequest) -> Self {
        Self {
            id: Uuid::new_v4(),
            initiator,
            responder,
            offer,
            request,
            status: NegotiationStatus::Pending,
            counter_offers: Vec::new(),
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    /// Check if this negotiation can result in a deal.
    pub fn is_viable(&self) -> bool {
        self.offer.price_per_unit <= self.request.max_price_per_unit
            && self.offer.available_capacity >= self.request.required_capacity
    }

    /// Accept the negotiation and produce a result.
    pub fn accept(&mut self) -> Option<NegotiationResult> {
        if !self.is_viable() {
            self.status = NegotiationStatus::Rejected;
            return None;
        }
        self.status = NegotiationStatus::Accepted;

        let capacity = self.request.required_capacity.min(self.offer.available_capacity);
        let agreed_price = self.offer.price_per_unit;
        let total_cost = capacity * agreed_price;

        Some(NegotiationResult {
            offer_id: self.id,
            request_id: self.id,
            capacity,
            agreed_price,
            total_cost,
            status: NegotiationStatus::Completed,
        })
    }

    /// Make a counter-offer with a different price.
    pub fn counter(&mut self, new_price: f64) {
        self.status = NegotiationStatus::Countered;
        let mut counter = self.offer.clone();
        counter.price_per_unit = new_price;
        self.counter_offers.push(counter);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_offer() -> CapacityOffer {
        CapacityOffer {
            peer_id: Uuid::new_v4(),
            available_capacity: 1000.0,
            unit: "kg".into(),
            price_per_unit: 2.50,
            origin_node: Uuid::new_v4(),
            destination_node: None,
            window_start: 1000,
            window_end: 2000,
        }
    }

    fn make_request() -> CapacityRequest {
        CapacityRequest {
            peer_id: Uuid::new_v4(),
            required_capacity: 500.0,
            max_price_per_unit: 3.00,
            origin_node: Uuid::new_v4(),
            destination_node: Uuid::new_v4(),
        }
    }

    #[test]
    fn test_viable_negotiation() {
        let offer = make_offer();
        let request = make_request();
        let mut neg = Negotiation::new(Uuid::new_v4(), Uuid::new_v4(), offer, request);
        assert!(neg.is_viable());

        let result = neg.accept();
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.capacity, 500.0);
        assert_eq!(result.total_cost, 1250.0);
    }

    #[test]
    fn test_unviable_negotiation() {
        let offer = CapacityOffer {
            price_per_unit: 5.00,
            ..make_offer()
        };
        let request = CapacityRequest {
            max_price_per_unit: 3.00,
            ..make_request()
        };
        let mut neg = Negotiation::new(Uuid::new_v4(), Uuid::new_v4(), offer, request);
        assert!(!neg.is_viable());
        assert!(neg.accept().is_none());
        assert_eq!(neg.status, NegotiationStatus::Rejected);
    }

    #[test]
    fn test_counter_offer() {
        let offer = make_offer();
        let request = make_request();
        let mut neg = Negotiation::new(Uuid::new_v4(), Uuid::new_v4(), offer, request);
        neg.counter(2.00);
        assert_eq!(neg.status, NegotiationStatus::Countered);
        assert_eq!(neg.counter_offers.len(), 1);
        assert_eq!(neg.counter_offers[0].price_per_unit, 2.00);
    }
}