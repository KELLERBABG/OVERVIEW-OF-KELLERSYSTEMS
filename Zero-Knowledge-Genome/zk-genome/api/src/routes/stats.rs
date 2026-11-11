//! `/api/v1/stats/aggregate` handler.

use axum::{extract::State, Json};
use std::sync::Arc;

use crate::models::{AggregateStats, ApiResponse};
use crate::state::AppState;

pub async fn get_aggregate(
    State(state): State<AppState>,
) -> Json<ApiResponse<AggregateStats>> {
    Json(ApiResponse::success(state.aggregate_stats()))
}
