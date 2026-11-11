use axum::{Json, extract::State};
use std::sync::Arc;

use sybil_core::SybilDetectionConfig;
use sybil_sybil::detector::SybilDetector;
use sybil_sybil::detectors::ring_lattice::RingLatticeDetector;
use sybil_graph::cache::GraphCache;

use crate::routes::AppState;

pub async fn detect(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let graph = state.graph.lock().unwrap();
    let config = SybilDetectionConfig::default();

    if graph.node_count() < config.min_graph_size {
        return Json(serde_json::json!({
            "warning": "Graph too small for reliable detection",
            "node_count": graph.node_count(),
            "required": config.min_graph_size
        }));
    }

    let detector = SybilDetector::new(&graph, config);
    match detector.detect() {
        Ok(results) => {
            Json(serde_json::json!({
                "regions_analyzed": results.len(),
                "suspected_sybil": results.iter().filter(|r| r.is_suspected_sybil).count(),
                "details": results.iter().map(|r| serde_json::json!({
                    "region": r.region_id,
                    "node_count": r.node_count,
                    "entropy": r.entropy_score,
                    "anomaly": r.anomaly_score,
                    "confidence": r.confidence,
                    "suspected": r.is_suspected_sybil,
                    "mean_cc": r.details.mean_clustering_coefficient,
                    "internal_density": r.details.internal_edge_density,
                    "external_density": r.details.external_edge_density,
                })).collect::<Vec<_>>()
            }))
        }
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

pub async fn analyze_region(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    // Run ring lattice analysis on the full graph as a demonstration
    let graph = state.graph.lock().unwrap();
    let nodes = graph.all_node_ids();

    let detector = RingLatticeDetector::new(&graph);
    match detector.analyze_region(&nodes) {
        Ok(result) => Json(serde_json::json!({
            "region": result.region_id,
            "node_count": result.node_count,
            "degree_entropy": result.entropy_score,
            "ring_score": result.anomaly_score,
            "suspected_sybil": result.is_suspected_sybil,
            "confidence": result.confidence
        })),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}