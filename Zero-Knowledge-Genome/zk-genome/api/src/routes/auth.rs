//! `/auth/register` and `/auth/login` route handlers.
//!
//! Authentication is intentionally lightweight for the MVP — researchers
//! register once and receive a bearer token they reuse across calls.

use axum::{extract::State, Json};
use std::sync::Arc;

use crate::models::{ApiResponse, LoginRequest, RegisterRequest, ResearcherAccount};
use crate::state::AppState;

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Json<ApiResponse<ResearcherAccount>> {
    if req.name.trim().is_empty() || req.organization.trim().is_empty() {
        return Json(ApiResponse::failure(
            "name and organization are required",
            "validation_failed",
        ));
    }
    let (_, account) = state.register_researcher(&req.name, &req.organization);
    Json(ApiResponse::success(account))
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Json<ApiResponse<ResearcherAccount>> {
    match state.get_researcher(&req.api_key) {
        Some(acc) => Json(ApiResponse::success(acc)),
        None => Json(ApiResponse::failure(
            "no researcher registered with that api_key",
            "no_such_researcher",
        )),
    }
}
