//! Path constraint verification.
//!
//! Takes a graph path (sequence of edge IDs) and checks that all
//! constraints on those edges are satisfied.

use sca_graph::GraphEngine;
use sca_types::{EdgeConstraint, ScaError};
use std::collections::HashMap;

use crate::solver::{ExactSolver, Solver, SolverResult};

/// Verifies that a path through the supply chain graph satisfies
/// all edge constraints.
#[derive(Debug, Clone)]
pub struct PathVerifier {
    engine: GraphEngine,
    constraints: HashMap<uuid::Uuid, Vec<EdgeConstraint>>,
    solver: ExactSolver,
}

impl PathVerifier {
    /// Create a new path verifier with the given graph.
    pub fn new(engine: GraphEngine) -> Self {
        Self { engine, constraints: HashMap::new(), solver: ExactSolver::new() }
    }

    /// Register a constraint for a specific edge.
    pub fn add_constraint(&mut self, constraint: EdgeConstraint) {
        self.constraints.entry(constraint.edge_id).or_default().push(constraint);
    }

    /// Register multiple constraints at once.
    pub fn add_constraints(&mut self, constraints: Vec<EdgeConstraint>) {
        for c in constraints {
            self.add_constraint(c);
        }
    }

    /// Get all constraints for a given edge.
    pub fn constraints_for_edge(&self, edge_id: &uuid::Uuid) -> Vec<&EdgeConstraint> {
        self.constraints.get(edge_id).map(|c| c.iter().collect()).unwrap_or_default()
    }

    /// Check if a path (sequence of edge IDs) is feasible.
    /// Returns a `SolverResult` with the outcome.
    pub fn verify_path(&self, edge_ids: &[uuid::Uuid]) -> SolverResult {
        // Collect all constraints for the edges in the path
        let mut all_constraints = Vec::new();

        for eid in edge_ids {
            if let Some(edge_constraints) = self.constraints.get(eid) {
                for c in edge_constraints {
                    all_constraints.push(c.expression.clone());
                }
            }
        }

        if all_constraints.is_empty() {
            return SolverResult {
                satisfiable: true,
                model: HashMap::new(),
                explanation: Some("No constraints on path".into()),
            };
        }

        self.solver.check(&all_constraints)
    }

    /// Verify a path by node IDs (traces the path through the graph).
    pub fn verify_path_between(&self, source: &uuid::Uuid, target: &uuid::Uuid) -> Result<SolverResult, ScaError> {
        let path = self
            .engine
            .shortest_path(*source, *target)
            .ok_or_else(|| ScaError::Graph("No path exists between the given nodes".into()))?;

        Ok(self.verify_path(&path))
    }

    /// Give the solver a set of variable values to use as defaults.
    pub fn with_variable(mut self, var: sca_types::ConstraintVariable, value: f64) -> Self {
        self.solver = self.solver.with_var(var, value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sca_graph::GraphEngine;
    use sca_types::{ComparisonOp, ConstraintExpr, ConstraintVariable, Edge, EdgeType, Node, NodeType};

    fn setup_graph_and_verifier() -> (GraphEngine, PathVerifier, uuid::Uuid, uuid::Uuid) {
        let mut engine = GraphEngine::new();

        let producer = engine.add_node(Node::new(NodeType::Producer, "Farm"));
        let hub = engine.add_node(Node::new(NodeType::TransportHub, "Port"));
        let consumer = engine.add_node(Node::new(NodeType::Consumer, "Store"));

        let e1 = Edge::new(EdgeType::Shipment, producer, hub, "Farm→Port").with_weight(100.0);
        let e2 = Edge::new(EdgeType::Shipment, hub, consumer, "Port→Store").with_weight(50.0);

        let e1_id = engine.add_edge(e1).unwrap();
        let e2_id = engine.add_edge(e2).unwrap();

        let mut verifier = PathVerifier::new(engine.clone());

        // Add constraint: capacity >= 100 on first edge
        verifier.add_constraint(EdgeConstraint {
            edge_id: e1_id,
            name: "Min capacity".into(),
            expression: ConstraintExpr::capacity(ComparisonOp::Ge, 100.0),
        });

        // Add constraint: temperature <= 8 on second edge
        verifier.add_constraint(EdgeConstraint {
            edge_id: e2_id,
            name: "Cold chain".into(),
            expression: ConstraintExpr::Comparison {
                variable: ConstraintVariable::Temperature,
                op: ComparisonOp::Le,
                value: 8.0,
            },
        });

        (engine, verifier, producer, consumer)
    }

    /// The path carries `capacity >= 100` and `temperature <= 8`. Those are
    /// independent variables, so the path is feasible and both bounds hold in
    /// the returned model. The previous solver reported this as unsatisfiable
    /// because it forced a single shared value onto every variable.
    #[test]
    fn test_verify_feasible_path() {
        let (engine, verifier, _p, _c) = setup_graph_and_verifier();

        // Get the path
        let nodes: Vec<_> = engine.list_nodes();
        let producer = nodes.iter().find(|n| n.name == "Farm").unwrap();
        let consumer = nodes.iter().find(|n| n.name == "Store").unwrap();

        let path = engine.shortest_path(producer.id, consumer.id).unwrap();

        let result = verifier.verify_path(&path);
        assert!(result.satisfiable, "capacity>=100 and temperature<=8 are jointly satisfiable");
        assert!(result.model[&ConstraintVariable::Capacity] >= 100.0);
        assert!(result.model[&ConstraintVariable::Temperature] <= 8.0);
    }

    #[test]
    fn test_verify_with_defaults() {
        let (_engine, verifier, producer_id, consumer_id) = setup_graph_and_verifier();
        let verifier = verifier
            .with_variable(ConstraintVariable::Capacity, 200.0)
            .with_variable(ConstraintVariable::Temperature, 5.0);

        let result = verifier.verify_path_between(&producer_id, &consumer_id);
        assert!(result.is_ok());
        assert!(result.unwrap().satisfiable);
    }

    #[test]
    fn test_no_constraints_path() {
        let mut engine = GraphEngine::new();
        let a = engine.add_node(Node::new(NodeType::Producer, "A"));
        let b = engine.add_node(Node::new(NodeType::Consumer, "B"));
        engine.add_edge(Edge::new(EdgeType::Shipment, a, b, "A→B")).unwrap();

        let verifier = PathVerifier::new(engine);
        let result = verifier.verify_path_between(&a, &b);
        assert!(result.is_ok());
        assert!(result.unwrap().satisfiable);
    }
}
