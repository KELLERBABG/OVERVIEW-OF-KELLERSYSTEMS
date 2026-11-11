use crate::AppState;
use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct ShortestPathRequest {
    pub source: Uuid,
    pub target: Uuid,
}

#[derive(Debug, Serialize)]
pub struct ShortestPathResponse {
    pub cost: f64,
    pub path: Vec<Uuid>,
}

pub async fn find_shortest_path(
    State(state): State<AppState>,
    Json(req): Json<ShortestPathRequest>,
) -> Result<Json<ShortestPathResponse>, StatusCode> {
    let engine = state.engine.lock().unwrap();
    engine
        .find_shortest_path(req.source, req.target)
        .map(|(cost, path)| Json(ShortestPathResponse { cost, path }))
        .ok_or(StatusCode::NOT_FOUND)
}
