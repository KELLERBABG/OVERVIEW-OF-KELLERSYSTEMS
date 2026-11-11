use axum::{Json, extract::State};
use std::sync::Arc;

use sybil_graph::topology::TopologyAnalyzer;
use sybil_graph::clustering::ClusterAnalyzer;
use sybil_graph::centrality::CentralityAnalyzer;

use crate::routes::AppState;

pub async fn analyze(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let graph = state.graph.lock().unwrap();
    let topo = TopologyAnalyzer::new(&graph);
    let cluster = ClusterAnalyzer::new(&graph);
    let centrality = CentralityAnalyzer::new(&graph);

    match topo.average_clustering_coefficient() {
        Ok(cc) => {
            let is_sw = topo.is_small_world().unwrap_or(false);
            let apl = topo.average_path_length().unwrap_or(0.0);
            let (cc_min, cc_max, cc_mean, cc_std) = cluster.clustering_coefficient_statistics().unwrap_or((0.0, 0.0, 0.0, 0.0));
            let eig = centrality.eigenvector_centrality(100, 1e-6).unwrap_or_default();

            Json(serde_json::json!({
                "small_world": is_sw,
                "clustering_coefficient": cc,
                "average_path_length": apl,
                "cc_statistics": {
                    "min": cc_min,
                    "max": cc_max,
                    "mean": cc_mean,
                    "std_dev": cc_std
                },
                "node_count": graph.node_count(),
                "edge_count": graph.edge_count(),
                "top_centrality": eig.iter().take(5).map(|(id, score)| {
                    serde_json::json!({"node_id": id.to_string(), "centrality": score})
                }).collect::<Vec<_>>()
            }))
        }
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

pub async fn topology(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let graph = state.graph.lock().unwrap();
    let topo = TopologyAnalyzer::new(&graph);
    let is_sw = topo.is_small_world().unwrap_or(false);
    let cc = topo.average_clustering_coefficient().unwrap_or(0.0);
    let apl = topo.average_path_length().unwrap_or(0.0);

    Json(serde_json::json!({
        "small_world": is_sw,
        "clustering_coefficient": cc,
        "average_path_length": apl,
        "expected_path_length": (graph.node_count() as f64).ln() / 6.0_f64.ln(),
        "node_count": graph.node_count(),
        "edge_count": graph.edge_count(),
    }))
}