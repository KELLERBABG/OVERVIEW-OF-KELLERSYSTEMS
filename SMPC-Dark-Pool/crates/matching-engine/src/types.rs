use sss::Share;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderSide {
    Bid,
    Ask,
}

#[derive(Debug, Clone)]
pub struct Order {
    pub id: u64,
    pub party_id: u64,
    pub side: OrderSide,
    pub price: Share,
    pub quantity: Share,
    pub timestamp: Share,
}

impl Order {
    pub fn new(
        id: u64,
        party_id: u64,
        side: OrderSide,
        price: Share,
        quantity: Share,
        timestamp: Share,
    ) -> Self {
        Self {
            id,
            party_id,
            side,
            price,
            quantity,
            timestamp,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Match {
    pub buy_order_id: u64,
    pub sell_order_id: u64,
    pub price: Share,
    pub quantity: Share,
    pub timestamp: Share,
}

impl Match {
    pub fn new(
        buy_order_id: u64,
        sell_order_id: u64,
        price: Share,
        quantity: Share,
        timestamp: Share,
    ) -> Self {
        Self {
            buy_order_id,
            sell_order_id,
            price,
            quantity,
            timestamp,
        }
    }
}

#[derive(Debug, Error)]
pub enum MatchError {
    #[error("order not found: {0}")]
    OrderNotFound(u64),
    #[error("no crossing orders available")]
    NoCrossingOrders,
}
