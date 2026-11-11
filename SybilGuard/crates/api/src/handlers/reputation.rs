use axum::{Json, extract::State, extract::Path};
use std::sync::Arc;

use crate::routes::AppState;

pub async fn get_score(
    State(state): State<Arc<AppState>>,
    Path(_id): Path<String>,
) -> Json<serde_json::Value> {
    // In production this would look up the node and calculate a real score
    let score = state.reputation.calculate_score(1.0, 0, None, 0, Some(false));
    match score {
        Ok(s) => Json(serde_json::json!({
            "base_score": s.base_score,
            "embedding_bonus": s.embedding_bonus,
            "vouching_power": s.vouching_power,
            "decay_factor": s.decay_factor,
        })),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

pub async fn apply_decay(
    State(state): State<Arc<AppState>>,
    Path(_id): Path<String>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "message": "Decay applied - full implementation requires persistent storage"
    }))
}