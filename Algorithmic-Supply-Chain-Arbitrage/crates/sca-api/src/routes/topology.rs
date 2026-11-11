use crate::AppState;
use axum::{Json, extract::State, http::StatusCode};
use sca_types::{Node, NodeId};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SubgraphRequest {
    pub node_ids: Vec<NodeId>,
}

pub async fn topological_sort(State(state): State<AppState>) -> Result<Json<Vec<NodeId>>, StatusCode> {
    let engine = state.engine.lock().unwrap();
    engine.topological_sort().map(Json).ok_or(StatusCode::BAD_REQUEST)
}

pub async fn subgraph(
    State(state): State<AppState>,
    Json(req): Json<SubgraphRequest>,
) -> Result<Json<Vec<Node>>, StatusCode> {
    let engine = state.engine.lock().unwrap();
    engine.subgraph(&req.node_ids).map(Json).map_err(|_| StatusCode::BAD_REQUEST)
}
