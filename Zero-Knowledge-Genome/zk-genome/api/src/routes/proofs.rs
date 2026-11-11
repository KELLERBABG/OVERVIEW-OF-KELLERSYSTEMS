//! Proof submission handler.

use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::Arc;

use crate::models::{ApiResponse, ProofSubmission, ProofSubmissionOutcome};
use crate::routes::Authed;
use crate::state::AppState;

pub async fn submit_proof(
    State(state): State<AppState>,
    auth: Authed,
    Path(id): Path<String>,
    Json(req): Json<ProofSubmission>,
) -> Json<ApiResponse<ProofSubmissionOutcome>> {
    if req.proof_data.is_empty() {
        return Json(ApiResponse::failure(
            "proof_data must not be empty",
            "validation_failed",
        ));
    }
    match state.submit_proof(
        &id,
        &auth.0.api_key,
        &req.proof_data,
        &req.public_inputs,
    ) {
        Ok(outcome) => Json(ApiResponse::success(outcome)),
        Err(err) => Json(ApiResponse::failure(&err.to_string(), err.code())),
    }
}
