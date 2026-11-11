use axum::{Json, extract::State, extract::Query};
use serde::Deserialize;
use std::sync::Arc;

use sybil_core::NodeId;
use crate::routes::AppState;

#[derive(Deserialize)]
pub struct CreateDidRequest {
    pub node_id: Option<String>,
}

#[derive(Deserialize)]
pub struct ResolveDidQuery {
    pub did: Option<String>,
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateDidRequest>,
) -> Json<serde_json::Value> {
    let node_id = match &req.node_id {
        Some(id) => match uuid::Uuid::parse_str(id) {
            Ok(u) => NodeId(u),
            Err(_) => NodeId::new(),
        },
        None => NodeId::new(),
    };

    let mut did_mgr = state.did_manager.lock().unwrap();
    match did_mgr.create_did(&node_id) {
        Ok(doc) => Json(serde_json::json!({
            "status": "created",
            "did": doc.did,
            "verification_methods": doc.verification_methods.len(),
        })),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

pub async fn resolve(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ResolveDidQuery>,
) -> Json<serde_json::Value> {
    let did = match &query.did {
        Some(d) => d.clone(),
        None => return Json(serde_json::json!({"error": "missing 'did' query parameter"})),
    };

    let did_mgr = state.did_manager.lock().unwrap();
    match did_mgr.resolve_did(&did) {
        Ok(doc) => Json(serde_json::json!({
            "did": doc.did,
            "created_at": doc.created_at.to_rfc3339(),
            "updated_at": doc.updated_at.to_rfc3339(),
            "authentication": doc.authentication,
        })),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

pub async fn resolve_by_path(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(did): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let full_did = format!("did:sybil:{}", did);
    let did_mgr = state.did_manager.lock().unwrap();
    match did_mgr.resolve_did(&full_did) {
        Ok(doc) => Json(serde_json::json!({
            "did": doc.did,
            "created_at": doc.created_at.to_rfc3339(),
            "updated_at": doc.updated_at.to_rfc3339(),
        })),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}