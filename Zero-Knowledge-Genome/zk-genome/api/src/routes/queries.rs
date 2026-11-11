//! `/api/v1/queries[...]` handlers.

use axum::{extract::{Path, State}, Json};
use std::sync::Arc;

use crate::models::{ApiResponse, Query, QueryRequest};
use crate::routes::Authed;
use crate::state::AppState;

pub async fn create_query(
    State(state): State<AppState>,
    auth: Authed,
    Json(req): Json<QueryRequest>,
) -> Json<ApiResponse<Query>> {
    if req.pos_start == 0 || req.pos_end < req.pos_start {
        return Json(ApiResponse::failure(
            "pos_start must be > 0 and pos_end >= pos_start",
            "invalid_range",
        ));
    }
    if req.max_submissions == 0 {
        return Json(ApiResponse::failure(
            "max_submissions must be > 0",
            "invalid_range",
        ));
    }
    if req.circuit_id.trim().is_empty() {
        return Json(ApiResponse::failure(
            "circuit_id is required (has-variant|variant-count|genotype-match)",
            "validation_failed",
        ));
    }
    let stored = state.create_query(req, &auth.0.api_key);
    Json(ApiResponse::success(stored))
}

pub async fn get_query(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<ApiResponse<Query>> {
    match state.get_query(&id) {
        Some(q) => Json(ApiResponse::success(q)),
        None => Json(ApiResponse::failure("query not found", "query_not_found")),
    }
}
