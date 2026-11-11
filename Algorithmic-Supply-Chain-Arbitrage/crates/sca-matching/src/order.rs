//! Order types for the supply-demand matching engine.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Whether this order is a supply (ask) or demand (bid).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrderSide {
    /// Producer offering supply.
    Ask,
    /// Consumer requesting supply.
    Bid,
}

/// Status of an order in the book.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrderStatus {
    /// Open and available for matching.
    Open,
    /// Partially filled.
    Partial,
    /// Fully filled.
    Filled,
    /// Cancelled by the creator.
    Cancelled,
}

/// A supply or demand order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    /// Unique order identifier.
    pub id: Uuid,
    /// The node ID where this order originates.
    pub node_id: Uuid,
    /// Ask (supply) or Bid (demand).
    pub side: OrderSide,
    /// Product name.
    pub product: String,
    /// Quantity (e.g., kg, pallets).
    pub quantity: f64,
    /// Remaining unfilled quantity.
    pub remaining: f64,
    /// Price per unit in USD.
    pub price: f64,
    /// Current status.
    pub status: OrderStatus,
    /// Timestamp (Unix seconds).
    pub created_at: i64,
}

impl Order {
    /// Create a new ask (supply offer).
    pub fn ask(node_id: Uuid, product: impl Into<String>, quantity: f64, price: f64) -> Self {
        Self::new(node_id, OrderSide::Ask, product, quantity, price)
    }

    /// Create a new bid (demand request).
    pub fn bid(node_id: Uuid, product: impl Into<String>, quantity: f64, price: f64) -> Self {
        Self::new(node_id, OrderSide::Bid, product, quantity, price)
    }

    fn new(node_id: Uuid, side: OrderSide, product: impl Into<String>, quantity: f64, price: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            node_id,
            side,
            product: product.into(),
            quantity,
            remaining: quantity,
            price,
            status: OrderStatus::Open,
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    /// Fill a portion of this order. Returns the amount actually filled.
    pub fn fill(&mut self, amount: f64) -> f64 {
        let actual = amount.min(self.remaining);
        self.remaining -= actual;
        self.status = if self.remaining <= 0.0 {
            OrderStatus::Filled
        } else {
            OrderStatus::Partial
        };
        actual
    }

    /// Cancel this order.
    pub fn cancel(&mut self) {
        self.status = OrderStatus::Cancelled;
    }

    /// Whether this order can be matched.
    pub fn is_active(&self) -> bool {
        matches!(self.status, OrderStatus::Open | OrderStatus::Partial)
    }
}

/// A collection of orders, providing matching operations.
#[derive(Debug, Clone)]
pub struct OrderBook {
    orders: Vec<Order>,
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            orders: Vec::new(),
        }
    }

    /// Add an order to the book.
    pub fn add(&mut self, order: Order) {
        self.orders.push(order);
    }

    /// Get all orders.
    pub fn orders(&self) -> &[Order] {
        &self.orders
    }

    /// Get all active ask orders for a product.
    pub fn asks(&self, product: &str) -> Vec<&Order> {
        self.orders
            .iter()
            .filter(|o| o.side == OrderSide::Ask && o.product == product && o.is_active())
            .collect()
    }

    /// Get all active bid orders for a product.
    pub fn bids(&self, product: &str) -> Vec<&Order> {
        self.orders
            .iter()
            .filter(|o| o.side == OrderSide::Bid && o.product == product && o.is_active())
            .collect()
    }

    /// Get all unique products in the order book.
    pub fn products(&self) -> Vec<String> {
        let mut prods: Vec<String> = self
            .orders
            .iter()
            .map(|o| o.product.clone())
            .collect();
        prods.sort();
        prods.dedup();
        prods
    }
}

impl Default for OrderBook {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_ask() {
        let ask = Order::ask(Uuid::new_v4(), "Wheat", 1000.0, 250.0);
        assert_eq!(ask.side, OrderSide::Ask);
        assert_eq!(ask.product, "Wheat");
        assert_eq!(ask.quantity, 1000.0);
        assert_eq!(ask.remaining, 1000.0);
        assert_eq!(ask.status, OrderStatus::Open);
    }

    #[test]
    fn test_create_bid() {
        let bid = Order::bid(Uuid::new_v4(), "Corn", 500.0, 300.0);
        assert_eq!(bid.side, OrderSide::Bid);
    }

    #[test]
    fn test_fill_order() {
        let mut ask = Order::ask(Uuid::new_v4(), "Wheat", 1000.0, 250.0);
        let filled = ask.fill(400.0);
        assert_eq!(filled, 400.0);
        assert_eq!(ask.remaining, 600.0);
        assert_eq!(ask.status, OrderStatus::Partial);
    }

    #[test]
    fn test_fill_complete() {
        let mut ask = Order::ask(Uuid::new_v4(), "Wheat", 1000.0, 250.0);
        ask.fill(1000.0);
        assert_eq!(ask.status, OrderStatus::Filled);
    }

    #[test]
    fn test_order_book_asks_and_bids() {
        let mut book = OrderBook::new();
        book.add(Order::ask(Uuid::new_v4(), "Wheat", 100.0, 200.0));
        book.add(Order::ask(Uuid::new_v4(), "Wheat", 200.0, 210.0));
        book.add(Order::bid(Uuid::new_v4(), "Wheat", 150.0, 220.0));

        assert_eq!(book.asks("Wheat").len(), 2);
        assert_eq!(book.bids("Wheat").len(), 1);
    }

    #[test]
    fn test_products() {
        let mut book = OrderBook::new();
        book.add(Order::ask(Uuid::new_v4(), "Wheat", 100.0, 200.0));
        book.add(Order::bid(Uuid::new_v4(), "Corn", 50.0, 300.0));

        let prods = book.products();
        assert_eq!(prods.len(), 2);
    }
}