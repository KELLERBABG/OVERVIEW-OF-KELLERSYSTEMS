use axum::{
    routing::{get, post},
    Json, Router,
};
use std::sync::{Arc, Mutex};

use sybil_core::{NodeId, ReputationConfig};
use sybil_graph::graph::PoPGraph;
use sybil_wot::vouching::VouchingEngine;
use sybil_wot::bootstrap::BootstrapManager;
use sybil_nullifier::manager::NullifierManager;
use sybil_did::manager::DidManager;
use sybil_reputation::engine::ReputationEngine;

use crate::handlers::{graph, nodes, wot, sybil, reputation, did, nullifier};

/// Application-wide shared state with all engine instances.
pub struct AppState {
    pub graph: Mutex<PoPGraph>,
    pub vouching: Mutex<VouchingEngine>,
    pub bootstrap: Mutex<BootstrapManager>,
    pub nullifier: Mutex<NullifierManager>,
    pub did_manager: Mutex<DidManager>,
    pub reputation: ReputationEngine,
    pub version: String,
}

impl AppState {
    pub fn new() -> Self {
        let mut boot = BootstrapManager::default();
        let a1 = NodeId::new();
        let a2 = NodeId::new();
        let a3 = NodeId::new();
        boot.initialize(vec![
            (a1, "did:sybil:founding-1".into()),
            (a2, "did:sybil:founding-2".into()),
            (a3, "did:sybil:founding-3".into()),
        ]);
        Self {
            graph: Mutex::new(PoPGraph::new()),
            vouching: Mutex::new(VouchingEngine::new()),
            bootstrap: Mutex::new(boot),
            nullifier: Mutex::new(NullifierManager::new()),
            did_manager: Mutex::new(DidManager::new()),
            reputation: ReputationEngine::new(ReputationConfig::default()),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Build the production API router with all real handlers.
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/nodes", get(nodes::list_nodes).post(nodes::create_node))
        .route("/api/v1/nodes/{id}", get(nodes::get_node))
        .route("/api/v1/graph/analyze", post(graph::analyze))
        .route("/api/v1/graph/topology", get(graph::topology))
        .route("/api/v1/wot/vouch", post(wot::issue_vouch))
        .route("/api/v1/wot/revoke", post(wot::revoke_vouch))
        .route("/api/v1/wot/trust-chain/{id}", get(wot::trust_chain))
        .route("/api/v1/sybil/detect", post(sybil::detect))
        .route("/api/v1/sybil/analyze-region", post(sybil::analyze_region))
        .route("/api/v1/reputation/{id}", get(reputation::get_score))
        .route("/api/v1/reputation/{id}/decay", post(reputation::apply_decay))
        .route("/api/v1/did", post(did::create).get(did::resolve))
        .route("/api/v1/did/{did}", get(did::resolve_by_path))
        .route("/api/v1/nullifier", post(nullifier::register))
        .route("/api/v1/nullifier/verify", post(nullifier::verify))
        .with_state(state)
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "sybil-resistentes",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}