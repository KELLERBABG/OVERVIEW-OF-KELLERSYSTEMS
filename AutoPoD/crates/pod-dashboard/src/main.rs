mod store;

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use pod_core::event::TelemetryEvent;
use pod_core::policy::DamagePolicy;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tower_http::services::ServeDir;

use crate::store::{ClaimRecord, ClaimStatus, Store};

#[derive(Clone)]
struct AppState {
    store: Arc<RwLock<Store>>,
}

#[tokio::main]
async fn main() {
    let state = AppState {
        store: Arc::new(RwLock::new(Store::new())),
    };

    let app = Router::new()
        .route("/api/claims", get(list_claims).post(create_claim))
        .route("/api/claims/{id}", get(get_claim))
        .route("/api/claims/{id}/verify", post(verify_claim))
        .route("/api/claims/{id}/classify", post(classify_claim))
        .route("/api/claims/{id}/report", post(report_claim))
        .route("/api/claims/{id}/taint", post(taint_claim))
        .nest_service("/", ServeDir::new("crates/pod-dashboard/static"))
        .with_state(state);

    let addr = "0.0.0.0:8080";
    println!("Dashboard listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[derive(Serialize)]
struct ClaimsList {
    claims: Vec<ClaimRecord>,
}

async fn list_claims(State(state): State<AppState>) -> Json<ClaimsList> {
    let store = state.store.read().await;
    Json(ClaimsList {
        claims: store.all().clone(),
    })
}

#[derive(Deserialize)]
struct CreateClaimRequest {
    event_json: String,
    policy_json: String,
}

#[derive(Serialize)]
struct CreateClaimResponse {
    id: String,
    policy_name: String,
    event_id: String,
    status: ClaimStatus,
}

async fn create_claim(
    State(state): State<AppState>,
    Json(req): Json<CreateClaimRequest>,
) -> Result<Json<CreateClaimResponse>, StatusCode> {
    let event: TelemetryEvent =
        serde_json::from_str(&req.event_json).map_err(|_| StatusCode::BAD_REQUEST)?;
    let policy: DamagePolicy =
        serde_json::from_str(&req.policy_json).map_err(|_| StatusCode::BAD_REQUEST)?;

    let mut store = state.store.write().await;
    let record = store.add(event, policy);
    Ok(Json(CreateClaimResponse {
        id: record.id.clone(),
        policy_name: record.policy_name.clone(),
        event_id: record.event_id.clone(),
        status: record.status.clone(),
    }))
}

async fn get_claim(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ClaimRecord>, StatusCode> {
    let store = state.store.read().await;
    store.get(&id).cloned().ok_or(StatusCode::NOT_FOUND).map(Json)
}

async fn verify_claim(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ClaimRecord>, StatusCode> {
    let mut store = state.store.write().await;
    store.verify(&id).map_err(|_| StatusCode::NOT_FOUND)?;
    store.get(&id).cloned().ok_or(StatusCode::NOT_FOUND).map(Json)
}

async fn classify_claim(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ClaimRecord>, StatusCode> {
    let mut store = state.store.write().await;
    store.classify(&id).map_err(|_| StatusCode::NOT_FOUND)?;
    store.get(&id).cloned().ok_or(StatusCode::NOT_FOUND).map(Json)
}

async fn report_claim(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut store = state.store.write().await;
    store.generate_report_data(&id).map_err(|_| StatusCode::NOT_FOUND)?;
    let record = store.get(&id).ok_or(StatusCode::NOT_FOUND)?;
    let report = record
        .report_json
        .as_ref()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let val: serde_json::Value =
        serde_json::from_str(report).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(val))
}

async fn taint_claim(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ClaimRecord>, StatusCode> {
    let mut store = state.store.write().await;
    store.taint(&id).map_err(|_| StatusCode::NOT_FOUND)?;
    store.get(&id).cloned().ok_or(StatusCode::NOT_FOUND).map(Json)
}
