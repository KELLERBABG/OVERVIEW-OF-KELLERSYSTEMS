//! Arbitrage opportunity detection and solving.
//!
//! An arbitrage opportunity exists when there's a price discrepancy
//! between ask and bid that exceeds the routing cost between them.

use sca_graph::GraphEngine;
use uuid::Uuid;

/// A detected arbitrage opportunity.
#[derive(Debug, Clone)]
pub struct ArbitrageOpportunity {
    /// Source node (where ask originates).
    pub source: Uuid,
    /// Destination node (where bid originates).
    pub destination: Uuid,
    /// Product being arbitraged.
    pub product: String,
    /// Ask price per unit.
    pub ask_price: f64,
    /// Bid price per unit.
    pub bid_price: f64,
    /// Routing cost per unit.
    pub route_cost: f64,
    /// Net profit per unit.
    pub profit_per_unit: f64,
    /// Available quantity.
    pub quantity: f64,
}

/// Solves for arbitrage opportunities using the supply chain graph.
pub struct ArbitrageSolver {
    graph: GraphEngine,
}

impl ArbitrageSolver {
    pub fn new(graph: GraphEngine) -> Self {
        Self { graph }
    }

    /// Estimate the routing cost between two nodes by finding
    /// the shortest path and summing edge weights.
    pub fn estimate_route_cost(
        &self,
        source: &Uuid,
        destination: &Uuid,
    ) -> Option<f64> {
        let path = self.graph.shortest_path(*source, *destination)?;

        let total_cost: f64 = path
            .iter()
            .filter_map(|eid| {
                let edge = self.graph.get_edge(*eid)?;
                Some(edge.weight)
            })
            .sum();

        Some(total_cost)
    }

    /// Check if a specific pair of ask/bid creates an arbitrage opportunity.
    pub fn check_opportunity(
        &self,
        ask_node: &Uuid,
        bid_node: &Uuid,
        ask_price: f64,
        bid_price: f64,
        quantity: f64,
        product: impl Into<String>,
    ) -> Option<ArbitrageOpportunity> {
        let route_cost = self.estimate_route_cost(ask_node, bid_node)?;
        let profit_per_unit = bid_price - ask_price - route_cost;

        if profit_per_unit > 0.0 {
            Some(ArbitrageOpportunity {
                source: *ask_node,
                destination: *bid_node,
                product: product.into(),
                ask_price,
                bid_price,
                route_cost,
                profit_per_unit,
                quantity,
            })
        } else {
            None
        }
    }

    /// Find all arbitrage opportunities between two nodes for a given product.
    pub fn find_opportunities(
        &self,
        ask_price: f64,
        bid_price: f64,
        asks: &[(Uuid, f64)], // (node_id, quantity) tuples
        bids: &[(Uuid, f64)],
        product: &str,
    ) -> Vec<ArbitrageOpportunity> {
        let mut opportunities = Vec::new();

        for &(ask_node, ask_qty) in asks {
            for &(bid_node, bid_qty) in bids {
                let qty = ask_qty.min(bid_qty);
                if let Some(opp) = self.check_opportunity(
                    &ask_node,
                    &bid_node,
                    ask_price,
                    bid_price,
                    qty,
                    product,
                ) {
                    opportunities.push(opp);
                }
            }
        }

        // Sort by profit per unit descending
        opportunities.sort_by(|a, b| {
            b.profit_per_unit
                .partial_cmp(&a.profit_per_unit)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        opportunities
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sca_graph::GraphEngine;
    use sca_types::{Edge, EdgeType, Node, NodeType};

    fn setup_graph() -> (GraphEngine, Uuid, Uuid) {
        let mut graph = GraphEngine::new();
        let a = graph.add_node(Node::new(NodeType::Producer, "Farm A"));
        let b = graph.add_node(Node::new(NodeType::Consumer, "Store B"));

        graph
            .add_edge(Edge::new(EdgeType::Shipment, a, b, "A→B").with_weight(10.0))
            .unwrap();

        (graph, a, b)
    }

    #[test]
    fn test_route_cost() {
        let (graph, a, b) = setup_graph();
        let solver = ArbitrageSolver::new(graph);
        let cost = solver.estimate_route_cost(&a, &b);
        assert_eq!(cost, Some(10.0));
    }

    #[test]
    fn test_profitable_opportunity() {
        let (graph, a, b) = setup_graph();
        let solver = ArbitrageSolver::new(graph);

        // Ask 200, Bid 250, Route 10 => profit 40
        let opp = solver.check_opportunity(&a, &b, 200.0, 250.0, 100.0, "Wheat");
        assert!(opp.is_some());
        let opp = opp.unwrap();
        assert_eq!(opp.profit_per_unit, 40.0);
    }

    #[test]
    fn test_unprofitable_opportunity() {
        let (graph, a, b) = setup_graph();
        let solver = ArbitrageSolver::new(graph);

        // Ask 200, Bid 205, Route 10 => profit -5
        let opp = solver.check_opportunity(&a, &b, 200.0, 205.0, 100.0, "Wheat");
        assert!(opp.is_none());
    }
}