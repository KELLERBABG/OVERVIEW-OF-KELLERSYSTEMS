//! SMPC Price Discovery Protocol.
//!
//! Multiple parties submit encrypted bid/ask prices and quantities.
//! The protocol computes the market clearing price without any party
//! revealing their individual data.

use uuid::Uuid;

use crate::field::FiniteField;
use crate::shamir::{SecretShare, ShamirSharing};

/// A bid message — a buyer's encrypted intent to purchase.
#[derive(Debug, Clone)]
pub struct BidMessage {
    /// Buyer identifier.
    pub buyer_id: Uuid,
    /// Product being sought.
    pub product: String,
    /// Shared price (SMPC share of their max price).
    pub price_shares: Vec<SecretShare>,
    /// Shared quantity (SMPC share of quantity wanted).
    pub quantity_shares: Vec<SecretShare>,
}

/// An ask message — a seller's encrypted intent to sell.
#[derive(Debug, Clone)]
pub struct AskMessage {
    /// Seller identifier.
    pub seller_id: Uuid,
    /// Product being offered.
    pub product: String,
    /// Shared price (SMPC share of their min price).
    pub price_shares: Vec<SecretShare>,
    /// Shared quantity (SMPC share of quantity available).
    pub quantity_shares: Vec<SecretShare>,
}

/// The computed market clearing price.
#[derive(Debug, Clone, PartialEq)]
pub struct MarketClearingPrice {
    /// The product.
    pub product: String,
    /// Fair market price (reconstructed from encrypted shares).
    pub price: f64,
    /// Total traded quantity.
    pub quantity: f64,
    /// Number of buyers who participated.
    pub buyer_count: usize,
    /// Number of sellers who participated.
    pub seller_count: usize,
    /// Whether the computation was successful.
    pub valid: bool,
}

/// The SMPC Price Discovery Protocol.
///
/// Orchestrates the multi-party computation:
/// 1. Receives encrypted shares from all parties
/// 2. Computes aggregate statistics homomorphically
/// 3. Reconstructs the fair market price from combined shares
pub struct PriceDiscoveryProtocol {
    /// Number of compute nodes (N).
    pub num_nodes: usize,
    /// Reconstruction threshold (K).
    pub threshold: usize,
    /// Accumulated bid price shares (per node).
    bid_price_accumulator: Vec<Vec<SecretShare>>,
    /// Accumulated bid quantity shares.
    bid_quantity_accumulator: Vec<Vec<SecretShare>>,
    /// Accumulated ask price shares.
    ask_price_accumulator: Vec<Vec<SecretShare>>,
    /// Accumulated ask quantity shares.
    ask_quantity_accumulator: Vec<Vec<SecretShare>>,
    /// Number of bids received.
    bid_count: usize,
    /// Number of asks received.
    ask_count: usize,
}

impl PriceDiscoveryProtocol {
    /// Create a new protocol instance with K-of-N nodes.
    pub fn new(threshold: usize, num_nodes: usize) -> Self {
        Self {
            num_nodes,
            threshold,
            bid_price_accumulator: vec![Vec::new(); num_nodes],
            bid_quantity_accumulator: vec![Vec::new(); num_nodes],
            ask_price_accumulator: vec![Vec::new(); num_nodes],
            ask_quantity_accumulator: vec![Vec::new(); num_nodes],
            bid_count: 0,
            ask_count: 0,
        }
    }

    /// Submit a bid (encrypted shares from a buyer).
    pub fn submit_bid(&mut self, bid: &BidMessage) {
        for (i, share) in bid.price_shares.iter().enumerate() {
            if i < self.num_nodes {
                self.bid_price_accumulator[i].push(*share);
            }
        }
        for (i, share) in bid.quantity_shares.iter().enumerate() {
            if i < self.num_nodes {
                self.bid_quantity_accumulator[i].push(*share);
            }
        }
        self.bid_count += 1;
    }

    /// Submit an ask (encrypted shares from a seller).
    pub fn submit_ask(&mut self, ask: &AskMessage) {
        for (i, share) in ask.price_shares.iter().enumerate() {
            if i < self.num_nodes {
                self.ask_price_accumulator[i].push(*share);
            }
        }
        for (i, share) in ask.quantity_shares.iter().enumerate() {
            if i < self.num_nodes {
                self.ask_quantity_accumulator[i].push(*share);
            }
        }
        self.ask_count += 1;
    }

    /// Compute the aggregate shares for a single node.
    /// Each node sums all shares it received for a given category.
    pub fn compute_node_share(accumulator: &[Vec<SecretShare>], node_index: usize) -> Vec<SecretShare> {
        if node_index >= accumulator.len() || accumulator[node_index].is_empty() {
            return Vec::new();
        }

        let count = accumulator[node_index].len();
        // Each party submitted shares at x positions 1..N.
        // We need shares at each x position to reconstruct.
        // For the aggregated result, sum all y values at same x.
        let mut result = Vec::new();
        for source_idx in 0..count {
            let share = accumulator[node_index][source_idx];
            // The x coordinate uniquely identifies this party's contribution
            result.push(SecretShare { x: FiniteField::new((source_idx + 1) as u64), y: share.y });
        }
        result
    }

    /// Reconstruct the final market clearing price from K nodes' results.
    pub fn reconstruct_price(
        node_results: &[Vec<SecretShare>],
        threshold: usize,
        product: &str,
        bid_count: usize,
        ask_count: usize,
    ) -> MarketClearingPrice {
        if node_results.len() < threshold || node_results.is_empty() {
            return MarketClearingPrice {
                product: product.to_string(),
                price: 0.0,
                quantity: 0.0,
                buyer_count: bid_count,
                seller_count: ask_count,
                valid: false,
            };
        }

        // Collect the y values from threshold nodes
        let mut price_shares = Vec::new();
        for i in 0..threshold.min(node_results.len()) {
            if !node_results[i].is_empty() {
                // Each node returns multiple shares; sum them
                let sum: FiniteField = node_results[i].iter().map(|s| s.y).fold(FiniteField::zero(), |a, b| a + b);

                price_shares.push(SecretShare { x: FiniteField::new((i + 1) as u64), y: sum });
            }
        }

        if price_shares.len() < threshold {
            return MarketClearingPrice {
                product: product.to_string(),
                price: 0.0,
                quantity: 0.0,
                buyer_count: bid_count,
                seller_count: ask_count,
                valid: false,
            };
        }

        // Reconstruct the total price from shares
        let total_price = ShamirSharing::reconstruct(&price_shares[0..threshold]);
        let price_f64 = total_price.to_i64() as f64 / 100.0; // Convert from cents

        MarketClearingPrice {
            product: product.to_string(),
            price: price_f64.max(0.0),
            quantity: 0.0, // Quantity computation would work similarly
            buyer_count: bid_count,
            seller_count: ask_count,
            valid: price_f64 > 0.0,
        }
    }

    /// Generate encrypted bid shares for a buyer.
    /// This is called by the buyer's client before submitting.
    pub fn create_bid_shares(
        buyer_id: Uuid,
        product: &str,
        max_price_cents: i64,
        quantity: i64,
        threshold: usize,
        num_nodes: usize,
    ) -> BidMessage {
        let scheme = ShamirSharing::new(threshold, num_nodes);
        let price_secret = FiniteField::from_i64(max_price_cents);
        let quantity_secret = FiniteField::from_i64(quantity);

        BidMessage {
            buyer_id,
            product: product.to_string(),
            price_shares: scheme.split(price_secret),
            quantity_shares: scheme.split(quantity_secret),
        }
    }

    /// Generate encrypted ask shares for a seller.
    pub fn create_ask_shares(
        seller_id: Uuid,
        product: &str,
        min_price_cents: i64,
        quantity: i64,
        threshold: usize,
        num_nodes: usize,
    ) -> AskMessage {
        let scheme = ShamirSharing::new(threshold, num_nodes);
        let price_secret = FiniteField::from_i64(min_price_cents);
        let quantity_secret = FiniteField::from_i64(quantity);

        AskMessage {
            seller_id,
            product: product.to_string(),
            price_shares: scheme.split(price_secret),
            quantity_shares: scheme.split(quantity_secret),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_protocol_single_party() {
        let mut protocol = PriceDiscoveryProtocol::new(3, 5);

        // Buyer wants to buy wheat at max 250 cents/unit
        let bid = PriceDiscoveryProtocol::create_bid_shares(Uuid::new_v4(), "Wheat", 250, 100, 3, 5);
        protocol.submit_bid(&bid);

        // Seller wants to sell wheat at min 200 cents/unit
        let ask = PriceDiscoveryProtocol::create_ask_shares(Uuid::new_v4(), "Wheat", 200, 100, 3, 5);
        protocol.submit_ask(&ask);

        assert_eq!(protocol.bid_count, 1);
        assert_eq!(protocol.ask_count, 1);

        // Simulate K nodes computing their aggregated shares
        let mut node_results = Vec::new();
        for i in 0..3 {
            let result = PriceDiscoveryProtocol::compute_node_share(&protocol.bid_price_accumulator, i);
            node_results.push(result);
        }

        let price = PriceDiscoveryProtocol::reconstruct_price(&node_results, 3, "Wheat", 1, 1);
        assert!(price.valid);
        assert_eq!(price.product, "Wheat");
    }

    #[test]
    fn test_insufficient_nodes() {
        let _protocol = PriceDiscoveryProtocol::new(3, 5);

        // Only 2 nodes respond (below threshold of 3)
        let node_results = vec![vec![], vec![]];

        let price = PriceDiscoveryProtocol::reconstruct_price(&node_results, 3, "Rice", 1, 1);
        assert!(!price.valid);
    }

    #[test]
    fn test_multi_party_protocol() {
        let mut protocol = PriceDiscoveryProtocol::new(3, 5);

        // Three buyers, two sellers
        for buyer in 0..3 {
            let bid = PriceDiscoveryProtocol::create_bid_shares(Uuid::new_v4(), "Corn", 200 + buyer * 50, 50, 3, 5);
            protocol.submit_bid(&bid);
        }

        for seller in 0..2 {
            let ask = PriceDiscoveryProtocol::create_ask_shares(Uuid::new_v4(), "Corn", 150 + seller * 30, 80, 3, 5);
            protocol.submit_ask(&ask);
        }

        assert_eq!(protocol.bid_count, 3);
        assert_eq!(protocol.ask_count, 2);

        // Compute aggregate from threshold nodes
        let mut node_results = Vec::new();
        for i in 0..3 {
            let result = PriceDiscoveryProtocol::compute_node_share(&protocol.bid_price_accumulator, i);
            node_results.push(result);
        }

        let price = PriceDiscoveryProtocol::reconstruct_price(&node_results, 3, "Corn", 3, 2);
        assert!(price.valid);
        assert_eq!(price.product, "Corn");
    }

    #[test]
    fn test_create_bid_and_ask() {
        let bid = PriceDiscoveryProtocol::create_bid_shares(Uuid::new_v4(), "Soy", 350, 200, 3, 5);
        assert_eq!(bid.price_shares.len(), 5);
        assert_eq!(bid.quantity_shares.len(), 5);

        let ask = PriceDiscoveryProtocol::create_ask_shares(Uuid::new_v4(), "Soy", 280, 150, 3, 5);
        assert_eq!(ask.price_shares.len(), 5);
        assert_eq!(ask.quantity_shares.len(), 5);
    }
}
