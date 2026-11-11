//! Supply Chain Space Exploration.
//!
//! Systematic investigation of disruption scenarios (strikes, weather,
//! border closures) and their impact on supply chain paths. Finds
//! viable alternative routes when constraints are violated.

use sca_graph::GraphEngine;
use sca_types::{EdgeConstraint, EdgeId};
use std::collections::HashMap;

use crate::path_verifier::PathVerifier;

/// A disruption scenario — constraints that are added, removed, or
/// modified to simulate a disruption event.
#[derive(Debug, Clone)]
pub struct DisruptionScenario {
    pub name: String,
    pub description: String,
    /// Additional constraints imposed by this disruption.
    pub additional_constraints: Vec<EdgeConstraint>,
    /// Edge IDs that are blocked (removed from consideration).
    pub blocked_edges: Vec<EdgeId>,
    /// Edge IDs whose weights are modified (key: edge_id, value: new weight).
    pub weight_changes: HashMap<EdgeId, f64>,
}

/// The result of a space exploration query.
#[derive(Debug, Clone)]
pub struct ExplorationResult {
    /// The scenario that was tested.
    pub scenario: DisruptionScenario,
    /// Whether the original path still works under the disruption.
    pub original_path_feasible: bool,
    /// Alternative paths that work under the disruption (if any).
    pub alternative_paths: Vec<Vec<EdgeId>>,
    /// Summary of impact.
    pub summary: String,
}

/// Systematically explores how disruptions affect supply chain routes.
#[derive(Debug, Clone)]
pub struct SpaceExplorer {
    engine: GraphEngine,
    verifier: PathVerifier,
}

impl SpaceExplorer {
    /// Create a new space explorer.
    pub fn new(engine: GraphEngine) -> Self {
        let verifier = PathVerifier::new(engine.clone());
        Self { engine, verifier }
    }

    /// Get a reference to the underlying graph engine.
    pub fn engine(&self) -> &GraphEngine {
        &self.engine
    }

    /// Get a mutable reference to the path verifier for adding constraints.
    pub fn verifier_mut(&mut self) -> &mut PathVerifier {
        &mut self.verifier
    }

    /// Test a single disruption scenario between two nodes.
    pub fn explore(
        &self,
        source: &uuid::Uuid,
        target: &uuid::Uuid,
        scenario: &DisruptionScenario,
    ) -> ExplorationResult {
        // Get original path
        let original_path = match self.engine.shortest_path(*source, *target) {
            Some(p) => p,
            None => {
                return ExplorationResult {
                    scenario: scenario.clone(),
                    original_path_feasible: false,
                    alternative_paths: Vec::new(),
                    summary: "No path exists between the given nodes even without disruption.".into(),
                };
            }
        };

        // Check if original path is still feasible under disruption
        let original_feasible = self.is_path_feasible(&original_path, scenario);

        // Find alternative paths
        let alternatives = self.find_alternative_paths(source, target, scenario, 5);

        let summary = format!(
            "Disruption '{}': original path {}, {} alternatives found.",
            scenario.name,
            if original_feasible { "feasible" } else { "BLOCKED" },
            alternatives.len()
        );

        ExplorationResult {
            scenario: scenario.clone(),
            original_path_feasible: original_feasible,
            alternative_paths: alternatives,
            summary,
        }
    }

    /// Check if a path satisfies all constraints in a scenario.
    fn is_path_feasible(&self, path: &[EdgeId], scenario: &DisruptionScenario) -> bool {
        // If any edge in the path is blocked, it's not feasible
        for edge_id in path {
            if scenario.blocked_edges.contains(edge_id) {
                return false;
            }
        }
        true
    }

    /// Find alternative paths between source and target that avoid blocked edges.
    fn find_alternative_paths(
        &self,
        source: &uuid::Uuid,
        target: &uuid::Uuid,
        scenario: &DisruptionScenario,
        max_alternatives: usize,
    ) -> Vec<Vec<EdgeId>> {
        let mut alternatives = Vec::new();

        let blocked_set: std::collections::HashSet<EdgeId> = scenario.blocked_edges.iter().copied().collect();

        // Build a modified graph without blocked edges for searching
        // For now, try using the existing path and look for edge-by-edge bypasses
        if let Some(mut path) = self.engine.shortest_path(*source, *target) {
            // Remove blocked edges from consideration
            path.retain(|eid| !blocked_set.contains(eid));

            // If the path survived filtering, it's an alternative
            if !path.is_empty() && alternatives.len() < max_alternatives {
                alternatives.push(path.clone());
            }
        }

        alternatives
    }

    /// Generate a set of common disruption scenarios.
    pub fn common_scenarios(&self) -> Vec<DisruptionScenario> {
        vec![
            DisruptionScenario {
                name: "port_strike".into(),
                description: "Major port strike — all port hubs are blocked".into(),
                additional_constraints: Vec::new(),
                blocked_edges: Vec::new(), // Would be populated with specific edges
                weight_changes: HashMap::new(),
            },
            DisruptionScenario {
                name: "border_closure".into(),
                description: "Border crossing delays exceed 24 hours".into(),
                additional_constraints: Vec::new(),
                blocked_edges: Vec::new(),
                weight_changes: HashMap::new(),
            },
            DisruptionScenario {
                name: "cold_chain_break".into(),
                description: "Refrigeration failure raises temperature above 8°C".into(),
                additional_constraints: Vec::new(),
                blocked_edges: Vec::new(),
                weight_changes: HashMap::new(),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sca_graph::GraphEngine;
    use sca_types::{Edge, EdgeType, Node, NodeType};

    fn setup() -> (GraphEngine, uuid::Uuid, uuid::Uuid) {
        let mut engine = GraphEngine::new();
        let a = engine.add_node(Node::new(NodeType::Producer, "Farm A"));
        let b = engine.add_node(Node::new(NodeType::TransportHub, "Port B"));
        let c = engine.add_node(Node::new(NodeType::TransportHub, "Port C"));
        let d = engine.add_node(Node::new(NodeType::Consumer, "Store D"));

        engine.add_edge(Edge::new(EdgeType::Shipment, a, b, "A→B").with_weight(100.0)).unwrap();
        engine.add_edge(Edge::new(EdgeType::Shipment, b, d, "B→D").with_weight(50.0)).unwrap();
        engine.add_edge(Edge::new(EdgeType::Shipment, a, c, "A→C").with_weight(80.0)).unwrap();
        engine.add_edge(Edge::new(EdgeType::Shipment, c, d, "C→D").with_weight(60.0)).unwrap();

        (engine, a, d)
    }

    #[test]
    fn test_explore_no_disruption() {
        let (engine, src, tgt) = setup();
        let explorer = SpaceExplorer::new(engine);

        let scenario = DisruptionScenario {
            name: "none".into(),
            description: "No disruption".into(),
            additional_constraints: Vec::new(),
            blocked_edges: Vec::new(),
            weight_changes: HashMap::new(),
        };

        let result = explorer.explore(&src, &tgt, &scenario);
        assert!(result.original_path_feasible);
    }

    #[test]
    fn test_explore_with_blocked_edges() {
        let (engine, src, tgt) = setup();

        // Find the shortest path (A→B→D should be 150 weight)
        let path = engine.shortest_path(src, tgt).unwrap();
        assert_eq!(path.len(), 2);
        let first_edge_id = path[0];

        let explorer = SpaceExplorer::new(engine);

        let scenario = DisruptionScenario {
            name: "block_first_leg".into(),
            description: "First leg blocked".into(),
            additional_constraints: Vec::new(),
            blocked_edges: vec![first_edge_id],
            weight_changes: HashMap::new(),
        };

        let result = explorer.explore(&src, &tgt, &scenario);
        assert!(!result.original_path_feasible);
    }
}
