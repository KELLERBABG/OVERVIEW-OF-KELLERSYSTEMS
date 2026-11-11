//! Real-time P2P supply-demand matching engine for arbitrage balancing.

pub mod arbitrage;
pub mod engine;
pub mod order;

pub use crate::order::{Order, OrderBook, OrderSide, OrderStatus};
pub use crate::arbitrage::{ArbitrageOpportunity, ArbitrageSolver};
pub use crate::engine::{
    EngineOrder, Fill, MatchingEngine, OrderId, Price, Side,
};
