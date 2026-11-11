use axum::{Json, extract::State};
use serde::Deserialize;
use std::sync::Arc;

use crate::routes::AppState;

#[derive(Deserialize)]
pub struct RegisterNullifierRequest {
    pub seed: String,
}

#[derive(Deserialize)]
pub struct VerifyNullifierRequest {
    pub nullifier: String,
}

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterNullifierRequest>,
) -> Json<serde_json::Value> {
    let mut nullifier = state.nullifier.lock().unwrap();
    let nf = nullifier.generate_nullifier(req.seed.as_bytes());
    match nullifier.register_nullifier(nf.clone()) {
        Ok(_) => Json(serde_json::json!({
            "status": "registered",
            "nullifier": nf.0
        })),
        Err(e) => Json(serde_json::json!({"error": e.to_string(), "nullifier": nf.0})),
    }
}

pub async fn verify(
    State(state): State<Arc<AppState>>,
    Json(req): Json<VerifyNullifierRequest>,
) -> Json<serde_json::Value> {
    let nullifier = sybil_core::IdentityNullifier(req.nullifier.clone());
    let nf = state.nullifier.lock().unwrap();
    let exists = nf.nullifier_exists(&nullifier);

    Json(serde_json::json!({
        "nullifier": req.nullifier,
        "exists": exists,
        "valid": !exists
    }))
}