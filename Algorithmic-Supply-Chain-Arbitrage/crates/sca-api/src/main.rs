//! # sca-api
//!
//! REST API server for the Algorithmic Supply Chain Arbitrage graph engine.
//!
//! Uses **Axum** for HTTP, exposes CRUD operations on the supply chain
//! graph, and serves OpenAPI documentation.

mod routes;

use axum::{
    Json, Router,
    routing::{get, post},
};
use sca_graph::GraphEngine;
use std::sync::{Arc, Mutex};
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub engine: Arc<Mutex<GraphEngine>>,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let state = AppState { engine: Arc::new(Mutex::new(GraphEngine::new())) };

    let app = Router::new()
        .route("/health", get(health))
        // Node CRUD
        .route("/api/nodes", get(routes::nodes::list_nodes).post(routes::nodes::create_node))
        .route(
            "/api/nodes/{id}",
            get(routes::nodes::get_node).patch(routes::nodes::update_node).delete(routes::nodes::delete_node),
        )
        .route("/api/nodes/{id}/edges/outgoing", get(routes::nodes::outgoing_edges))
        .route("/api/nodes/{id}/edges/incoming", get(routes::nodes::incoming_edges))
        .route("/api/nodes/{id}/reachable", get(routes::nodes::reachable_from))
        // Edge CRUD
        .route("/api/edges", get(routes::edges::list_edges).post(routes::edges::create_edge))
        .route(
            "/api/edges/{id}",
            get(routes::edges::get_edge).patch(routes::edges::update_edge).delete(routes::edges::delete_edge),
        )
        // Pathfinding
        .route("/api/paths", post(routes::paths::find_shortest_path))
        // Topology
        .route("/api/topology/sort", get(routes::topology::topological_sort))
        .route("/api/topology/subgraph", post(routes::topology::subgraph))
        // Arbitrage
        .route("/api/arbitrage", get(routes::arbitrage::detect_arbitrage))
        .route("/api/arbitrage/backhaul", get(routes::backhaul::detect_paired_backhaul))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = "0.0.0.0:3000";
    tracing::info!("Starting server on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// Health check endpoint.
async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "service": "sca-api" }))
}
