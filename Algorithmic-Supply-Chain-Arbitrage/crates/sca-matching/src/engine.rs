//! Core matching engine.
//!
//! Finds matches between asks and bids by price-time priority,
//! considering routing costs through the supply chain graph.

use uuid::Uuid;

use crate::arbitrage::ArbitrageSolver;
use crate::order::{Order, OrderBook, OrderSide};
use sca_graph::GraphEngine;

/// Configuration for the matching engine.
#[derive(Debug, Clone)]
pub struct MatchingConfig {
    /// Minimum arbitrage spread to execute a match (USD per unit).
    pub min_spread: f64,
    /// Maximum number of matches per run.
    pub max_matches_per_run: usize,
    /// Whether to enforce graph routing costs.
    pub use_routing_costs: bool,
}

impl Default for MatchingConfig {
    fn default() -> Self {
        Self {
            min_spread: 5.0,
            max_matches_per_run: 100,
            use_routing_costs: true,
        }
    }
}

/// The result of a single match.
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// The ask order ID.
    pub ask_id: Uuid,
    /// The bid order ID.
    pub bid_id: Uuid,
    /// Quantity matched.
    pub quantity: f64,
    /// Price per unit (USD).
    pub price: f64,
    /// Estimated routing cost per unit.
    pub route_cost: f64,
    /// Total value of the match.
    pub total_value: f64,
    /// Product being traded.
    pub product: String,
}

/// Summary of a matching run.
#[derive(Debug, Clone)]
pub struct MatchingSummary {
    pub matches: Vec<MatchResult>,
    pub total_matches: usize,
    pub total_value: f64,
    pub total_quantity: f64,
}

/// The supply-demand matching engine.
///
/// Takes an order book and a graph topology, finds profitable
/// arbitrage opportunities, and executes matches.
pub struct MatchingEngine {
    config: MatchingConfig,
    graph: GraphEngine,
    solver: ArbitrageSolver,
}

impl MatchingEngine {
    /// Get a reference to the underlying graph.
    pub fn graph(&self) -> &GraphEngine {
        &self.graph
    }

    /// Create a new matching engine.
    pub fn new(graph: GraphEngine) -> Self {
        let solver = ArbitrageSolver::new(graph.clone());
        Self {
            config: MatchingConfig::default(),
            graph,
            solver,
            book: OrderBook::new(),
        }
    }

    /// Create with custom config.
    pub fn with_config(graph: GraphEngine, config: MatchingConfig) -> Self {
        let solver = ArbitrageSolver::new(graph.clone());
        Self {
            config,
            graph,
            solver,
            book: OrderBook::new(),
        }
    }

    /// Submit an order to the engine's internal order book.
    pub fn submit(&mut self, order: Order) {
        self.book.add(order);
    }

    /// Get a reference to the internal order book.
    pub fn book(&self) -> &OrderBook {
        &self.book
    }

    /// Best (highest-price) active bid for a product.
    pub fn best_bid(&self, product: &str) -> Option<&Order> {
        self.book
            .bids(product)
            .into_iter()
            .max_by(|a, b| a.price.partial_cmp(&b.price).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// Best (lowest-price) active ask for a product.
    pub fn best_ask(&self, product: &str) -> Option<&Order> {
        self.book
            .asks(product)
            .into_iter()
            .min_by(|a, b| a.price.partial_cmp(&b.price).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// Return all resting (active) orders as a flat list.
    ///
    /// Each tuple is `(order_id, side, price, remaining_qty, product, node_id)`.
    /// This avoids exposing the internal `OrderBook` structure directly.
    pub fn snapshot(&self) -> Vec<(Uuid, OrderSide, f64, f64, String, Uuid)> {
        self.book
            .orders()
            .iter()
            .filter(|o| o.is_active())
            .map(|o| {
                (
                    o.id,
                    o.side.clone(),
                    o.price,
                    o.remaining,
                    o.product.clone(),
                    o.node_id,
                )
            })
            .collect()
    }

    /// Run the matching engine on the order book.
    pub fn match_orders(&self, book: &OrderBook) -> MatchingSummary {
        let mut matches = Vec::new();

        for product in book.products() {
            let asks = book.asks(&product);
            let bids = book.bids(&product);

            // Sort asks by price ascending (cheapest first)
            let mut sorted_asks: Vec<&Order> = asks.into_iter().collect();
            sorted_asks.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap_or(std::cmp::Ordering::Equal));

            // Sort bids by price descending (highest first)
            let mut sorted_bids: Vec<&Order> = bids.into_iter().collect();
            sorted_bids.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap_or(std::cmp::Ordering::Equal));

            // Match cross
            let mut ask_idx = 0;
            let mut bid_idx = 0;

            while ask_idx < sorted_asks.len()
                && bid_idx < sorted_bids.len()
                && matches.len() < self.config.max_matches_per_run
            {
                let ask = sorted_asks[ask_idx];
                let bid = sorted_bids[bid_idx];

                // Calculate route cost if graph is available
                let route_cost = if self.config.use_routing_costs {
                    self.solver
                        .estimate_route_cost(&ask.node_id, &bid.node_id)
                        .unwrap_or(0.0)
                } else {
                    0.0
                };

                // Check if this is a profitable arbitrage
                // bid_price - ask_price - route_cost > min_spread
                let spread = bid.price - ask.price - route_cost;

                if spread > self.config.min_spread {
                    let quantity = ask.remaining.min(bid.remaining);
                    let total_value = quantity * bid.price;

                    matches.push(MatchResult {
                        ask_id: ask.id,
                        bid_id: bid.id,
                        quantity,
                        price: bid.price,
                        route_cost,
                        total_value,
                        product: product.clone(),
                    });

                    // Move past filled orders
                    if ask.remaining <= bid.remaining {
                        ask_idx += 1;
                    }
                    if bid.remaining <= ask.remaining {
                        bid_idx += 1;
                    }
                } else {
                    // No profitable match possible with this combination
                    // Move to next ask or bid depending on spread
                    if bid.price - ask.price <= 0.0 {
                        // Bids are all below asks — no more matches
                        break;
                    }
                    bid_idx += 1;
                }
            }
        }

        let total_matches = matches.len();
        let total_value: f64 = matches.iter().map(|m| m.total_value).sum();
        let total_quantity: f64 = matches.iter().map(|m| m.quantity).sum();

        MatchingSummary {
            matches,
            total_matches,
            total_value,
            total_quantity,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sca_graph::GraphEngine;
    use sca_types::{Edge, EdgeType, Node, NodeType};

    fn setup_graph() -> GraphEngine {
        let mut graph = GraphEngine::new();
        let producer = graph.add_node(Node::new(NodeType::Producer, "Farm A"));
        let hub = graph.add_node(Node::new(NodeType::TransportHub, "Port"));
        let consumer = graph.add_node(Node::new(NodeType::Consumer, "Store B"));

        graph
            .add_edge(Edge::new(EdgeType::Shipment, producer, hub, "A→Port").with_weight(10.0))
            .unwrap();
        graph
            .add_edge(Edge::new(EdgeType::Shipment, hub, consumer, "Port→B").with_weight(5.0))
            .unwrap();

        graph
    }

    #[test]
    fn test_simple_match() {
        let graph = setup_graph();
        let engine = MatchingEngine::new(graph);

        let mut book = OrderBook::new();
        let producer_id = engine.graph.list_nodes()[0].id;
        let consumer_id = engine.graph.list_nodes()[2].id;

        book.add(Order::ask(producer_id, "Wheat", 100.0, 200.0));
        book.add(Order::bid(consumer_id, "Wheat", 100.0, 250.0));

        let summary = engine.match_orders(&book);

        // bid 250 - ask 200 - route (10+5) = 35 > 5 min_spread => should match
        assert_eq!(summary.total_matches, 1);
        assert_eq!(summary.total_quantity, 100.0);
    }

    #[test]
    fn test_no_match_when_unprofitable() {
        let graph = setup_graph();
        let engine = MatchingEngine::new(graph);

        let mut book = OrderBook::new();
        let producer_id = engine.graph.list_nodes()[0].id;
        let consumer_id = engine.graph.list_nodes()[2].id;

        // bid 205 - ask 200 - route 15 = -10 — not profitable
        book.add(Order::ask(producer_id, "Wheat", 100.0, 200.0));
        book.add(Order::bid(consumer_id, "Wheat", 100.0, 205.0));

        let summary = engine.match_orders(&book);
        assert_eq!(summary.total_matches, 0);
    }

    #[test]
    fn test_multiple_matches() {
        let graph = setup_graph();
        let engine = MatchingEngine::new(graph);

        let mut book = OrderBook::new();
        let producer_id = engine.graph.list_nodes()[0].id;
        let consumer_id = engine.graph.list_nodes()[2].id;

        book.add(Order::ask(producer_id, "Wheat", 100.0, 200.0));
        book.add(Order::ask(producer_id, "Corn", 50.0, 150.0));
        book.add(Order::bid(consumer_id, "Wheat", 80.0, 260.0));
        book.add(Order::bid(consumer_id, "Corn", 40.0, 200.0));

        let summary = engine.match_orders(&book);
        assert_eq!(summary.total_matches, 2);
    }
}