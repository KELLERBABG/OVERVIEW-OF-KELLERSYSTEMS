use crate::AppState;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use sca_types::{Edge, EdgeId};
use uuid::Uuid;

pub async fn list_edges(State(state): State<AppState>) -> Json<Vec<Edge>> {
    let engine = state.engine.lock().unwrap();
    Json(engine.list_edges())
}

pub async fn create_edge(
    State(state): State<AppState>,
    Json(mut edge): Json<Edge>,
) -> Result<(StatusCode, Json<EdgeId>), StatusCode> {
    if edge.id == uuid::Uuid::nil() {
        edge.id = uuid::Uuid::new_v4();
    }
    let mut engine = state.engine.lock().unwrap();
    engine.add_edge(edge).map(|id| (StatusCode::CREATED, Json(id))).map_err(|_| StatusCode::BAD_REQUEST)
}

pub async fn get_edge(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<Edge>, StatusCode> {
    let engine = state.engine.lock().unwrap();
    engine.get_edge(id).map(|e| Json(e.clone())).ok_or(StatusCode::NOT_FOUND)
}

pub async fn update_edge(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
    Json(edge): Json<Edge>,
) -> Result<Json<Edge>, StatusCode> {
    Ok(Json(edge))
}

pub async fn delete_edge(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<StatusCode, StatusCode> {
    let mut engine = state.engine.lock().unwrap();
    engine.remove_edge(id).map(|_| StatusCode::NO_CONTENT).map_err(|_| StatusCode::NOT_FOUND)
}
