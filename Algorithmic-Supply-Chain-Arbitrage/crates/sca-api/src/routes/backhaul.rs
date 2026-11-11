use crate::AppState;
use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct BackhaulResponse {
    pub paired_corridors: Vec<PairedCorridorDto>,
    pub deadhead_before_km: f64,
    pub deadhead_after_km: f64,
    pub deadhead_reduction_percent: f64,
    pub paired_corridor_count: usize,
    pub total_return_distance_km: f64,
}

#[derive(Debug, Serialize)]
pub struct PairedCorridorDto {
    pub forward_node: uuid::Uuid,
    pub backhaul_node: uuid::Uuid,
    pub forward_label: String,
    pub backhaul_label: String,
    pub forward_distance_km: f64,
    pub backhaul_distance_km: f64,
    pub deadhead_eliminated_km: f64,
    pub effective_backhaul_cost: f64,
}

pub async fn detect_paired_backhaul(
    State(state): State<AppState>,
) -> Result<Json<BackhaulResponse>, (StatusCode, String)> {
    let engine = state.engine.lock().map_err(|_| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "graph engine is busy".into(),
        )
    })?;
    let report = engine.detect_paired_backhauls();
    let paired_corridors: Vec<PairedCorridorDto> = report
        .paired_corridors
        .into_iter()
        .map(|p| PairedCorridorDto {
            forward_node: p.forward.source,
            backhaul_node: p.forward.target,
            forward_label: p.forward.label,
            backhaul_label: p.backhaul.label,
            forward_distance_km: p.forward.distance_km,
            backhaul_distance_km: p.backhaul.distance_km,
            deadhead_eliminated_km: p.deadhead_eliminated_km,
            effective_backhaul_cost: p.effective_backhaul_cost,
        })
        .collect();
    Ok(Json(BackhaulResponse {
        paired_corridors,
        deadhead_before_km: report.deadhead_before_km,
        deadhead_after_km: report.deadhead_after_km,
        deadhead_reduction_percent: report.deadhead_reduction_percent,
        paired_corridor_count: report.paired_corridor_count,
        total_return_distance_km: report.total_return_distance_km,
    }))
}
