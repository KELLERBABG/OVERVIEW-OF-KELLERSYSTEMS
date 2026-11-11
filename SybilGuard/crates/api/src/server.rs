use std::sync::Arc;
use tracing::info;

use crate::routes::{build_router, AppState};

/// Start the production API server.
pub async fn start_server(port: u16) {
    let state = Arc::new(AppState::new());

    let app = build_router(state);

    let addr = format!("0.0.0.0:{}", port);
    info!("🚀 Sybil-Resistentes API server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap_or_else(|e| {
        panic!("Failed to bind to {}: {}", addr, e);
    });
    axum::serve(listener, app).await.unwrap();
}