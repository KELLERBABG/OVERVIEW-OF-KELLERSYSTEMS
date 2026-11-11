use axum::{Json, extract::State, extract::Path};
use serde::Deserialize;
use std::sync::Arc;

use sybil_core::NodeId;
use crate::routes::AppState;

#[derive(Deserialize)]
pub struct VouchRequest {
    pub voucher: String,
    pub subject: String,
    pub depth: Option<u32>,
}

#[derive(Deserialize)]
pub struct RevokeRequest {
    pub voucher: String,
    pub subject: String,
}

pub async fn issue_vouch(
    State(state): State<Arc<AppState>>,
    Json(req): Json<VouchRequest>,
) -> Json<serde_json::Value> {
    // Parse node IDs from hex strings
    let voucher_id = match uuid::Uuid::parse_str(&req.voucher) {
        Ok(u) => NodeId(u),
        Err(_) => return Json(serde_json::json!({"error": "invalid voucher ID"})),
    };
    let subject_id = match uuid::Uuid::parse_str(&req.subject) {
        Ok(u) => NodeId(u),
        Err(_) => return Json(serde_json::json!({"error": "invalid subject ID"})),
    };
    let depth = req.depth.unwrap_or(1);

    let mut vouching = state.vouching.lock().unwrap();
    match vouching.vouch(voucher_id, subject_id, depth) {
        Ok(record) => Json(serde_json::json!({
            "status": "vouch_issued",
            "voucher": record.voucher.to_string(),
            "subject": record.subject.to_string(),
            "depth": record.depth,
            "expires_at": record.expires_at.to_rfc3339()
        })),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

pub async fn revoke_vouch(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RevokeRequest>,
) -> Json<serde_json::Value> {
    let voucher_id = match uuid::Uuid::parse_str(&req.voucher) {
        Ok(u) => NodeId(u),
        Err(_) => return Json(serde_json::json!({"error": "invalid voucher ID"})),
    };
    let subject_id = match uuid::Uuid::parse_str(&req.subject) {
        Ok(u) => NodeId(u),
        Err(_) => return Json(serde_json::json!({"error": "invalid subject ID"})),
    };

    let mut vouching = state.vouching.lock().unwrap();
    match vouching.revoke(&voucher_id, &subject_id) {
        Ok(_) => Json(serde_json::json!({"status": "vouch_revoked"})),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

pub async fn trust_chain(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let node_id = match uuid::Uuid::parse_str(&id) {
        Ok(u) => NodeId(u),
        Err(_) => return Json(serde_json::json!({"error": "invalid node ID"})),
    };

    let vouching = state.vouching.lock().unwrap();
    let anchors = state.bootstrap.lock().unwrap();
    let anchor_ids: Vec<NodeId> = anchors.active_anchors().iter().map(|a| a.node_id.clone()).collect();

    let verifier = sybil_wot::trust_chain::TrustChainVerifier::new(&vouching);
    match verifier.verify_trust_chain(&node_id, &anchor_ids) {
        Ok(chain) => Json(serde_json::json!({
            "subject": chain.subject.to_string(),
            "total_depth": chain.total_depth,
            "cumulative_weight": chain.cumulative_weight,
            "verified_at": chain.verified_at.to_rfc3339(),
            "anchors": chain.anchors.iter().map(|a| a.to_string()).collect::<Vec<_>>(),
            "path_length": chain.path.len()
        })),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}