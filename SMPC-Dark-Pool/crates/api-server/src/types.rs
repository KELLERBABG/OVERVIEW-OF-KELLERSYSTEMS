use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRequest {
    pub party_id: u64,
    pub side: String,
    pub price: String,
    pub quantity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResponse {
    pub order_id: String,
    pub status: String,
    pub submitted_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderStatusResponse {
    pub order_id: String,
    pub side: String,
    pub status: String,
    pub matched: bool,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookSnapshot {
    pub best_bid: Option<String>,
    pub best_ask: Option<String>,
    pub spread: Option<String>,
    pub bid_depth: usize,
    pub ask_depth: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeHistoryEntry {
    pub trade_id: String,
    pub buy_order_id: String,
    pub sell_order_id: String,
    pub price: String,
    pub quantity: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: u16,
    pub message: String,
}
