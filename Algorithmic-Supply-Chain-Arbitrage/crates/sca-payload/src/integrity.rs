//! Integrity verification utilities.
//!
//! Provides higher-level integrity checks beyond the basic
//! chain verification: cross-referencing with graph data,
//! checking that custody locations match expected path nodes,
//! and validating sensor readings.

use sca_graph::GraphEngine;

use crate::chain::CustodyChain;

/// Result of a full integrity check.
#[derive(Debug)]
pub struct IntegrityReport {
    pub chain_valid: bool,
    pub path_consistent: bool,
    pub all_locations_exist: bool,
    pub event_count: usize,
    pub details: Vec<String>,
}

/// Verifies payload integrity against graph topology and custody chain.
pub struct IntegrityVerifier {
    graph: GraphEngine,
}

impl IntegrityVerifier {
    pub fn new(graph: GraphEngine) -> Self {
        Self { graph }
    }

    /// Perform a full integrity check on a custody chain.
    pub fn verify(&self, chain: &CustodyChain) -> IntegrityReport {
        let mut details = Vec::new();

        // 1. Verify chain signatures and linkage
        let chain_valid = match chain.verify() {
            Ok(()) => {
                details.push("Custody chain: all signatures valid, chain intact".into());
                true
            }
            Err(e) => {
                details.push(format!("Custody chain error: {e}"));
                false
            }
        };

        // 2. Verify all location IDs exist in the graph
        let mut all_locations_exist = true;
        for record in chain.records() {
            if self.graph.get_node(record.location_id).is_none() {
                details.push(format!("Location {} not found in graph", record.location_id));
                all_locations_exist = false;
            }
        }
        if all_locations_exist {
            details.push("All custody locations exist in the graph".into());
        }

        // 3. Path consistency: check that locations follow graph edges
        let mut path_consistent = true;
        for window in chain.records().windows(2) {
            let from = &window[0].location_id;
            let to = &window[1].location_id;
            if self.graph.find_shortest_path(*from, *to).is_none() {
                details.push(format!("No path between custody locations {} → {}", from, to));
                path_consistent = false;
            }
        }
        if path_consistent {
            details.push("Custody path is consistent with graph topology".into());
        }

        IntegrityReport { chain_valid, path_consistent, all_locations_exist, event_count: chain.len(), details }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracker::generate_test_key;
    use sca_graph::GraphEngine;
    use sca_types::{Edge, EdgeType, Node, NodeType};

    #[test]
    fn test_integrity_report_with_graph() {
        let mut graph = GraphEngine::new();
        let a = graph.add_node(Node::new(NodeType::Producer, "Farm A"));
        let b = graph.add_node(Node::new(NodeType::TransportHub, "Hub B"));
        graph.add_edge(Edge::new(EdgeType::Shipment, a, b, "A→B")).unwrap();

        let key = generate_test_key();
        let mut chain = CustodyChain::new();

        let r1 = crate::chain::CustodyRecord::new(
            uuid::Uuid::new_v4(),
            a,
            1000,
            "Farmer".into(),
            key.verifying_key(),
            None,
            vec![],
        )
        .sign(&key);
        chain.push(r1);

        let r2 = crate::chain::CustodyRecord::new(
            uuid::Uuid::new_v4(),
            b,
            2000,
            "Hub".into(),
            key.verifying_key(),
            Some(chain.records()[0].hash.clone()),
            vec![],
        )
        .sign(&key);
        chain.push(r2);

        let verifier = IntegrityVerifier::new(graph);
        let report = verifier.verify(&chain);

        assert!(report.chain_valid);
        assert!(report.path_consistent);
        assert!(report.all_locations_exist);
        assert_eq!(report.event_count, 2);
    }
}
