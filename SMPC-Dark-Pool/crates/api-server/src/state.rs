use crate::types::TradeHistoryEntry;
use matching_engine::engine::MatchingEngine;
use settlement::engine::SettlementEngine;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicU64;
use std::time::Instant;
use tokio::sync::RwLock;

/// Matching execution model for a dark-pool node.
///
/// - `Immediate` matches crossing orders as soon as each order is submitted.
/// - `Periodic { interval_ms }` buffers orders and runs the match loop on a
///   scheduled cycle, then broadcasts the resulting fills to clients.
#[derive(Debug, Clone, Copy, Default)]
pub enum MatchMode {
    #[default]
    Immediate,
    Periodic {
        interval_ms: u64,
    },
}

pub struct RateLimiterState {
    pub requests: Mutex<HashMap<String, Vec<Instant>>>,
    pub max_requests: usize,
    pub window_secs: u64,
}

impl RateLimiterState {
    pub fn new(max_requests: usize, window_secs: u64) -> Self {
        Self {
            requests: Mutex::new(HashMap::new()),
            max_requests,
            window_secs,
        }
    }
}

impl Default for RateLimiterState {
    fn default() -> Self {
        Self::new(100, 60)
    }
}

pub struct AppState {
    pub engine: RwLock<MatchingEngine>,
    pub settlement: RwLock<SettlementEngine>,
    pub trades: RwLock<Vec<TradeHistoryEntry>>,
    pub ws_clients: RwLock<Vec<tokio::sync::mpsc::UnboundedSender<String>>>,
    pub orders_total: AtomicU64,
    pub matches_total: AtomicU64,
    pub active_connections: AtomicU64,
    pub http_requests_total: AtomicU64,
    pub api_key: Option<String>,
    pub rate_limiter: RateLimiterState,
    pub match_mode: MatchMode,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            engine: RwLock::new(MatchingEngine::new()),
            settlement: RwLock::new(SettlementEngine::new()),
            trades: RwLock::new(Vec::new()),
            ws_clients: RwLock::new(Vec::new()),
            orders_total: AtomicU64::new(0),
            matches_total: AtomicU64::new(0),
            active_connections: AtomicU64::new(0),
            http_requests_total: AtomicU64::new(0),
            api_key: std::env::var("API_KEY").ok(),
            rate_limiter: RateLimiterState::default(),
            match_mode: MatchMode::default(),
        }
    }

    pub fn shared() -> Arc<Self> {
        Arc::new(Self::new())
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
