use crate::AppState;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use sca_types::{Node, NodeId};
use uuid::Uuid;

pub async fn list_nodes(State(state): State<AppState>) -> Json<Vec<Node>> {
    let engine = state.engine.lock().unwrap();
    Json(engine.list_nodes())
}

pub async fn create_node(State(state): State<AppState>, Json(mut node): Json<Node>) -> (StatusCode, Json<NodeId>) {
    if node.id == uuid::Uuid::nil() {
        node.id = uuid::Uuid::new_v4();
    }
    let mut engine = state.engine.lock().unwrap();
    let id = engine.add_node(node);
    (StatusCode::CREATED, Json(id))
}

pub async fn get_node(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<Node>, StatusCode> {
    let engine = state.engine.lock().unwrap();
    engine.get_node(id).map(|n| Json(n.clone())).ok_or(StatusCode::NOT_FOUND)
}

pub async fn update_node(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
    Json(node): Json<Node>,
) -> Result<Json<Node>, StatusCode> {
    Ok(Json(node))
}

pub async fn delete_node(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<StatusCode, StatusCode> {
    let mut engine = state.engine.lock().unwrap();
    engine.remove_node(id).map(|_| StatusCode::NO_CONTENT).map_err(|_| StatusCode::NOT_FOUND)
}

pub async fn outgoing_edges(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<sca_types::Edge>>, StatusCode> {
    let engine = state.engine.lock().unwrap();
    engine.outgoing_edges(id).map(Json).ok_or(StatusCode::NOT_FOUND)
}

pub async fn incoming_edges(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<sca_types::Edge>>, StatusCode> {
    let engine = state.engine.lock().unwrap();
    engine.incoming_edges(id).map(Json).ok_or(StatusCode::NOT_FOUND)
}

pub async fn reachable_from(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<NodeId>>, StatusCode> {
    let engine = state.engine.lock().unwrap();
    engine.reachable_from(id).map(Json).ok_or(StatusCode::NOT_FOUND)
}
