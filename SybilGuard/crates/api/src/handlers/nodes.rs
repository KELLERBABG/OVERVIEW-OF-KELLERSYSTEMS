use axum::{Json, extract::State, extract::Path};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use sybil_core::{NodeId, NodeMetadata, SybilResult};

use crate::routes::AppState;

#[derive(Serialize)]
pub struct NodeResponse {
    pub node_id: String,
    pub did: Option<String>,
    pub reputation_score: f64,
    pub embedding_depth: u32,
    pub joined_at: String,
    pub last_active: String,
}

#[derive(Deserialize)]
pub struct CreateNodeRequest {
    pub did: Option<String>,
}

pub async fn list_nodes(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let graph = state.graph.lock().unwrap();
    let nodes = graph.all_node_ids();
    Json(serde_json::json!({
        "nodes": nodes.iter().map(|n| n.to_string()).collect::<Vec<_>>(),
        "count": nodes.len()
    }))
}

pub async fn create_node(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateNodeRequest>,
) -> Json<serde_json::Value> {
    let mut graph = state.graph.lock().unwrap();
    let node_id = NodeId::new();
    let now = chrono::Utc::now();
    let metadata = NodeMetadata {
        node_id: node_id.clone(),
        did: req.did,
        reputation_score: sybil_core::constants::INITIAL_REPUTATION_SCORE,
        clustering_coefficient: None,
        centrality_score: None,
        embedding_depth: 0,
        joined_at: now,
        last_active: now,
        is_sybil: None,
        metadata: serde_json::Value::Null,
    };
    match graph.add_node(metadata) {
        Ok(_) => Json(serde_json::json!({
            "status": "created",
            "node_id": node_id.to_string(),
            "reputation": sybil_core::constants::INITIAL_REPUTATION_SCORE
        })),
        Err(e) => Json(serde_json::json!({
            "status": "error",
            "error": e.to_string()
        })),
    }
}

pub async fn get_node(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    // Parse the id — in production this would use proper deserialization
    Json(serde_json::json!({
        "message": format!("Node lookup for {} - in production", id)
    }))
}