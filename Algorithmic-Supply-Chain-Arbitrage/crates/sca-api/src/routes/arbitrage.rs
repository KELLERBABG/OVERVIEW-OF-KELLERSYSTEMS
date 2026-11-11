use crate::AppState;
use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ArbitrageResponse {
    pub cycle: Option<ArbitrageCycleDto>,
}

#[derive(Debug, Serialize)]
pub struct ArbitrageCycleDto {
    pub nodes: Vec<uuid::Uuid>,
    pub node_names: Vec<String>,
    pub edges: Vec<uuid::Uuid>,
    pub edge_labels: Vec<String>,
    pub edge_costs: Vec<f64>,
    pub cycle_cost: f64,
    pub profit: f64,
}

pub async fn detect_arbitrage(
    State(state): State<AppState>,
) -> Result<Json<ArbitrageResponse>, (StatusCode, String)> {
    let engine = state.engine.lock().map_err(|_| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "graph engine is busy".into(),
        )
    })?;
    let cycle = engine.detect_arbitrage().map(|c| ArbitrageCycleDto {
        nodes: c.nodes,
        node_names: c.node_names,
        edges: c.edges,
        edge_labels: c.edge_labels,
        edge_costs: c.edge_costs,
        cycle_cost: c.total_cost,
        profit: c.profit,
    });
    Ok(Json(ArbitrageResponse { cycle }))
}
