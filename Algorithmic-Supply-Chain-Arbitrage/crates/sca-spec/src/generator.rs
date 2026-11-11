//! Spec generator — produces logistics specifications from graph + payload.

use sca_graph::GraphEngine;
use sca_payload::PayloadTracker;
use uuid::Uuid;

use crate::spec::{CustodyEvidence, LogisticsSpecification, ShipmentLeg};

/// Generates logistics specifications automatically from graph
/// topology and payload tracking data.
pub struct SpecGenerator {
    graph: GraphEngine,
}

impl SpecGenerator {
    pub fn new(graph: GraphEngine) -> Self {
        Self { graph }
    }

    /// Generate a logistics specification for a payload tracker.
    pub fn generate(&self, tracker: &PayloadTracker) -> LogisticsSpecification {
        let payload = &tracker.payload;
        let mut spec = LogisticsSpecification::new(
            payload.id,
            &payload.product,
            payload.quantity,
            &payload.unit,
            payload.origin,
            payload.destination,
        );

        // Generate legs from shortest path
        self.add_route_legs(&mut spec, &payload.origin, &payload.destination);

        // Add custody evidence from the tracker's chain
        tracker.chain().iter().enumerate().for_each(|(i, record)| {
            spec.add_custody_evidence(CustodyEvidence {
                custody_index: i,
                node_id: record.location_id,
                custodian: record.custodian.clone(),
                timestamp: record.timestamp,
                record_hash: hex::encode(&record.hash),
            });
        });

        // Set perishability info
        if let Some(window) = payload.perishability_window_secs {
            let proven = tracker.check_perishability().is_ok();
            spec.set_perishability(window, proven);
        }

        spec.finalize();
        spec
    }

    /// Add route legs by decomposing the shortest path into segments.
    fn add_route_legs(&self, spec: &mut LogisticsSpecification, origin: &Uuid, destination: &Uuid) {
        let path = match self.graph.shortest_path(*origin, *destination) {
            Some(p) => p,
            None => return,
        };

        for (i, edge_id) in path.iter().enumerate() {
            if let Some(edge) = self.graph.get_edge(*edge_id) {
                let from_label = self.graph.get_node(edge.source).map(|n| n.name.clone()).unwrap_or_default();
                let to_label = self.graph.get_node(edge.target).map(|n| n.name.clone()).unwrap_or_default();

                let mode = self.infer_transport_mode(&edge.source, &edge.target);

                spec.add_leg(ShipmentLeg {
                    sequence: i + 1,
                    origin_node: edge.source,
                    destination_node: edge.target,
                    carrier: format!("Carrier for {}→{}", from_label, to_label),
                    mode,
                    estimated_departure: chrono::Utc::now().timestamp() + (i as i64 * 3600),
                    estimated_arrival: chrono::Utc::now().timestamp() + (i as i64 * 3600) + (edge.weight as i64 * 60),
                    cost: edge.weight,
                });
            }
        }
    }

    /// Infer transport mode based on node types at each end.
    fn infer_transport_mode(&self, source: &Uuid, target: &Uuid) -> String {
        let src_type = self.graph.get_node(*source).map(|n| n.node_type);
        let tgt_type = self.graph.get_node(*target).map(|n| n.node_type);

        match (src_type, tgt_type) {
            (Some(sca_types::NodeType::Producer), Some(sca_types::NodeType::TransportHub))
            | (Some(sca_types::NodeType::TransportHub), Some(sca_types::NodeType::Consumer)) => "truck".into(),
            (Some(sca_types::NodeType::TransportHub), Some(sca_types::NodeType::TransportHub)) => "ship".into(),
            _ => "truck".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sca_graph::GraphEngine;
    use sca_payload::PayloadTracker;
    use sca_payload::tracker::generate_test_key;
    use sca_types::{Edge, EdgeType, Node, NodeType};

    fn setup_test_data() -> (GraphEngine, PayloadTracker) {
        let mut graph = GraphEngine::new();
        let producer = graph.add_node(Node::new(NodeType::Producer, "Farm A"));
        let hub = graph.add_node(Node::new(NodeType::TransportHub, "Port B"));
        let consumer = graph.add_node(Node::new(NodeType::Consumer, "Store C"));

        graph.add_edge(Edge::new(EdgeType::Shipment, producer, hub, "Farm→Port").with_weight(100.0)).unwrap();
        graph.add_edge(Edge::new(EdgeType::Shipment, hub, consumer, "Port→Store").with_weight(50.0)).unwrap();

        let mut tracker =
            PayloadTracker::create("Organic Apples", 5000.0, "kg", producer, consumer).with_perishability(86400);

        let key = generate_test_key();
        tracker.record_transfer(producer, "Farmer John", &key, vec![]).unwrap();
        tracker.record_transfer(hub, "Logistics Co", &key, vec![("temp".into(), "4.5".into())]).unwrap();

        (graph, tracker)
    }

    #[test]
    fn test_generate_spec() {
        let (graph, tracker) = setup_test_data();
        let generator = SpecGenerator::new(graph);
        let spec = generator.generate(&tracker);

        assert_eq!(spec.product, "Organic Apples");
        assert_eq!(spec.quantity, 5000.0);
        assert_eq!(spec.status, crate::spec::SpecStatus::Final);
        assert_eq!(spec.legs.len(), 2);
        assert!(spec.total_cost > 0.0);
        assert_eq!(spec.custody_evidence.len(), 2);
        assert!(spec.perishability_proven);
    }
}
