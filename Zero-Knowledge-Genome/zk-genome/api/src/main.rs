//! `zk-genome-api` binary entry point.
//!
//! Run with: `cargo run -p zk-genome-api`. Binds 0.0.0.0:3000 (override
//! with `ZKGEN_BIND`). All mutable state lives in `AppState`.

use std::net::SocketAddr;
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use axum::{
    extract::State,
    middleware::{self, Next},
    routing::{get, post},
    Json, Router,
};
use tower_http::cors::CorsLayer;

mod models;
mod routes;
mod state;
mod verify;

use crate::models::{ApiResponse, HealthResponse};
use crate::state::AppState;

#[tokio::main]
async fn main() {
    println!(
        "Zero-Knowledge Genome API Server v{}",
        env!("CARGO_PKG_VERSION")
    );

    let state = AppState::new();
    let router = build_router(state.clone());

    let addr: SocketAddr = std::env::var("ZKGEN_BIND")
        .unwrap_or_else(|_| "0.0.0.0:3000".to_string())
        .parse()
        .expect("ZKGEN_BIND must be a valid socket address");
    println!("Listening on http://{addr}");
    println!("Try: curl http://{addr}/healthz");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind ZKGEN_BIND socket");
    axum::serve(listener, router).await.expect("server crashed");
}

fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/queries", post(routes::queries::create_query))
        .route("/api/v1/queries/{id}", get(routes::queries::get_query))
        .route(
            "/api/v1/queries/{id}/proofs",
            post(routes::proofs::submit_proof),
        )
        .route(
            "/api/v1/stats/aggregate",
            get(routes::stats::get_aggregate),
        )
        .route("/api/v1/auth/register", post(routes::auth::register))
        .route("/api/v1/auth/login", post(routes::auth::login))
        .route("/health", get(|| async { "OK" }))
        .route("/healthz", get(healthz))
        .with_state(state.clone())
        .layer(CorsLayer::permissive())
        .layer(middleware::from_fn_with_state(state.clone(), log_request))
}

async fn log_request(
    State(state): State<AppState>,
    req: axum::http::Request<axum::body::Body>,
    next: Next,
) -> axum::response::Response {
    let method = req.method().clone();
    let uri = req.uri().path().to_string();
    let started_at = Instant::now();
    let resp = next.run(req).await;
    let status = resp.status().as_u16();
    println!(
        "  {method} {uri} -> {status} ({} ms) [queries={}]",
        started_at.elapsed().as_millis(),
        state.query_count(),
    );
    resp
}

async fn healthz(State(state): State<AppState>) -> Json<ApiResponse<HealthResponse>> {
    Json(ApiResponse::success(HealthResponse {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: process_uptime().as_secs(),
        queries_stored: state.query_count(),
    }))
}

fn process_uptime() -> std::time::Duration {
    static STARTED: OnceLock<Instant> = OnceLock::new();
    STARTED.get_or_init(Instant::now).elapsed()
}
