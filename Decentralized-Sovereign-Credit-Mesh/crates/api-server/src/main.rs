use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};

use mesh_credit::constraints::{ConstraintEngine, Exposure};
use mesh_credit::loan::{calculate_interest_rate, Loan, LoanStatus};
use mesh_graph_core::algorithms;
use mesh_graph_core::entropy;
use mesh_graph_core::graph::SocialGraph;
use mesh_graph_core::types::*;
use mesh_reputation::scoring;
use mesh_wot::identity::Identity;
use mesh_wot::keyring::Keyring;
use mesh_wot::limits::VouchingLimits;
use mesh_wot::nullifier::NullifierRegistry;
use mesh_wot::vouching::VouchStatement;

const STATE_FILE: &str = "mesh_server_state.json";
const KEYRING_FILE: &str = "mesh-keys/server.keyring.json";

/// Runtime configuration. All values are overridable via `MESH_*` environment
/// variables so the API can be deployed without code changes (finding #8).
#[derive(Debug, Clone)]
struct Config {
    host: String,
    port: u16,
    state_file: PathBuf,
    keyring_file: PathBuf,
    /// When set, all endpoints except `/health` require `Authorization: Bearer <token>`.
    api_token: Option<String>,
    /// Allowed CORS origins (comma-separated). Empty = same-origin only (no CORS headers).
    cors_origins: Vec<String>,
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

impl Config {
    fn from_env() -> Self {
        let port: u16 = env_or("MESH_API_PORT", "8899").parse().unwrap_or(8899);
        let cors_origins = env_or("MESH_CORS_ORIGINS", "")
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Config {
            host: env_or("MESH_API_HOST", "127.0.0.1"),
            port,
            state_file: PathBuf::from(env_or("MESH_STATE_FILE", STATE_FILE)),
            keyring_file: PathBuf::from(env_or("MESH_KEYRING_FILE", KEYRING_FILE)),
            api_token: match std::env::var("MESH_API_TOKEN") {
                Ok(t) if !t.trim().is_empty() => Some(t.trim().to_string()),
                _ => None,
            },
            cors_origins,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServerState {
    identities: HashMap<NodeId, Identity>,
    graph: GraphSnapshot,
    nullifiers: NullifierRegistry,
    loans: Vec<Loan>,
    repayments: Vec<RepaymentRecord>,
    vouching_limits: VouchingLimits,

    /// Private signing keys — never serialized to the state file (finding #1).
    #[serde(skip)]
    keyring: Keyring,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RepaymentRecord {
    from: String,
    to: String,
    amount: f64,
}

type SharedState = Arc<RwLock<ServerState>>;

/// Shared application context handed to every handler: the state lock plus the
/// persistence targets (state + keyring files), so mutations save immediately.
#[derive(Clone)]
struct AppCtx {
    inner: SharedState,
    state_file: PathBuf,
    keyring_file: PathBuf,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    nodes: usize,
    edges: usize,
    loans: usize,
}

#[derive(Deserialize)]
struct AddUserRequest {
    name: String,
    #[serde(default)]
    id: Option<String>,
}

#[derive(Serialize)]
struct UserResponse {
    id: String,
    display_name: String,
    created_at: i64,
    public_key: String,
}

#[derive(Deserialize)]
struct VouchRequest {
    voucher_id: String,
    target_id: String,
    #[serde(default = "default_weight")]
    weight: f64,
    #[serde(default = "default_relationship")]
    relationship: String,
}

fn default_weight() -> f64 {
    0.5
}
fn default_relationship() -> String {
    "friend".to_string()
}

#[derive(Deserialize)]
struct TrustPathQuery {
    from: String,
    to: String,
}

#[derive(Serialize)]
struct TrustPathResponse {
    found: bool,
    hops: usize,
    path: Vec<PathNode>,
}

#[derive(Serialize)]
struct PathNode {
    id: String,
    name: String,
    relationship: Option<String>,
    weight: Option<f64>,
}

#[derive(Deserialize)]
struct CreateLoanRequest {
    borrower: String,
    lender: String,
    principal: f64,
    purpose: String,
}

#[derive(Serialize)]
struct LoanResponse {
    id: String,
    borrower: String,
    lender: String,
    principal: f64,
    interest_rate: f64,
    status: String,
    constraints_passed: bool,
}

#[derive(Deserialize)]
struct RepayRequest {
    amount: f64,
}

#[derive(Serialize)]
struct ReputationScoreResponse {
    node_id: String,
    display_name: String,
    composite_score: f64,
    pagerank: f64,
    degree_centrality: f64,
    clustering: f64,
    sybil_score: f64,
}

#[derive(Serialize)]
struct StatsResponse {
    nodes: usize,
    edges: usize,
    density: f64,
    avg_clustering: f64,
    pagerank_range: Vec<f64>,
    entropy: f64,
    nullifiers_used: usize,
    active_loans: usize,
    total_loan_volume: f64,
}

#[tokio::main]
async fn main() {
    let config = Config::from_env();

    // Corrupt or unreadable state is never silently reset to empty (finding #7):
    // the offending file is preserved as a timestamped backup and the operator is told.
    let state = load_state_or_backup(&config.state_file);

    let shared_state: SharedState = Arc::new(RwLock::new(state));
    let ctx = AppCtx {
        inner: shared_state.clone(),
        state_file: config.state_file.clone(),
        keyring_file: config.keyring_file.clone(),
    };

    // Load the persisted keyring and re-attach signing keys (they are never in
    // the state file). If the keyring file is corrupt, preserve it as a backup
    // instead of silently starting with an empty store.
    {
        let mut state = shared_state.write().unwrap_or_else(|p| p.into_inner());
        let loaded = match Keyring::load(&config.keyring_file) {
            Ok(k) => k,
            Err(e) => {
                let backup = config
                    .keyring_file
                    .with_extension(format!("corrupt.{}.json", chrono::Utc::now().timestamp()));
                let _ = std::fs::rename(&config.keyring_file, &backup);
                eprintln!(
                    "WARNING: keyring '{}' was corrupt ({}); backed up to '{}' and starting with an empty keyring.",
                    config.keyring_file.display(),
                    e,
                    backup.display()
                );
                Keyring::new()
            }
        };
        state.keyring = loaded;
        let keyring = state.keyring.clone();
        for identity in state.identities.values_mut() {
            if identity.signing_key_bytes.is_none() {
                identity.signing_key_bytes = keyring.get(&identity.id).map(|k| k.to_vec());
            }
        }
    }

    let cors = build_cors(&config);
    let mut router = Router::new()
        .route("/health", get(health))
        .route("/users", get(list_users).post(add_user))
        .route("/vouch", post(create_vouch))
        .route("/trust-path", get(trust_path))
        .route("/trust-score/:user", get(trust_score))
        .route("/stats", get(stats))
        .route("/loans", get(list_loans).post(create_loan))
        .route("/loans/:id/repay", post(repay_loan))
        .route("/nullifiers/check", post(check_nullifier))
        .route("/sybil/:user", get(sybil_score))
        .with_state(ctx.clone());

    if let Some(cors) = cors {
        router = router.layer(cors);
    }
    if config.api_token.is_some() {
        let token_config = Arc::new(config.clone());
        router = router.layer(middleware::from_fn_with_state(token_config, require_auth));
    }

    let addr = format!("{}:{}", config.host, config.port);
    // Refuse to expose an unauthenticated API beyond loopback (CWE-1188):
    // without MESH_API_TOKEN the server will only bind to 127.0.0.1/::1.
    let is_loopback =
        config.host == "127.0.0.1" || config.host == "::1" || config.host == "localhost";
    if config.api_token.is_none() && !is_loopback {
        eprintln!(
            "FATAL: refusing to bind {} without MESH_API_TOKEN — set a Bearer token or bind to 127.0.0.1.",
            addr
        );
        std::process::exit(1);
    }
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| {
            eprintln!("FATAL: cannot bind {} — {}", addr, e);
            std::process::exit(1);
        });
    let auth_note = if config.api_token.is_some() {
        "auth: Bearer token required"
    } else {
        "WARNING: no MESH_API_TOKEN set — API is unauthenticated (bind to localhost only!)"
    };
    println!("API server running on http://{} ({})", addr, auth_note);

    let save_ctx = ctx.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            if let Ok(state) = save_ctx.inner.read() {
                persist_locked(&state, &save_ctx.state_file, &save_ctx.keyring_file);
            }
        }
    });

    axum::serve(listener, router).await.unwrap();
}

/// Persist state + keyring atomically; on failure log and keep running.
fn persist_locked(
    state: &ServerState,
    state_file: &std::path::Path,
    keyring_path: &std::path::Path,
) {
    if let Ok(json) = serde_json::to_string_pretty(state) {
        if let Err(e) = atomic_write(state_file, json.as_bytes()) {
            eprintln!("WARNING: failed to persist state: {}", e);
        }
    }
    if let Err(e) = state.keyring.save(keyring_path) {
        eprintln!("WARNING: failed to persist keyring: {}", e);
    }
}

fn load_state_or_backup(path: &PathBuf) -> ServerState {
    if !path.exists() {
        return ServerState::default();
    }
    match std::fs::read_to_string(path) {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(state) => state,
            Err(e) => {
                let backup =
                    path.with_extension(format!("corrupt.{}.json", chrono::Utc::now().timestamp()));
                let _ = std::fs::rename(path, &backup);
                eprintln!(
                    "WARNING: state file '{}' was corrupt ({}); backed up to '{}' and starting fresh.",
                    path.display(),
                    e,
                    backup.display()
                );
                ServerState::default()
            }
        },
        Err(e) => {
            eprintln!(
                "WARNING: cannot read state file '{}' ({}); starting fresh.",
                path.display(),
                e
            );
            ServerState::default()
        }
    }
}

fn build_cors(config: &Config) -> Option<CorsLayer> {
    if config.cors_origins.is_empty() {
        return None;
    }
    let origins: Vec<axum::http::HeaderValue> = config
        .cors_origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();
    if origins.is_empty() {
        return None;
    }
    Some(
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(origins))
            .allow_methods(AllowMethods::list([
                axum::http::Method::GET,
                axum::http::Method::POST,
            ]))
            .allow_headers(AllowHeaders::list([
                header::CONTENT_TYPE,
                header::AUTHORIZATION,
            ])),
    )
}

/// Bearer-token guard. `/health` stays public so load balancers can probe it.
async fn require_auth(
    State(config): State<Arc<Config>>,
    headers: HeaderMap,
    request: axum::extract::Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    if request.uri().path() == "/health" {
        return Ok(next.run(request).await);
    }
    let expected = config.api_token.as_deref().unwrap_or_default();
    let provided = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or_default();
    if provided.is_empty() || !constant_time_eq(provided.as_bytes(), expected.as_bytes()) {
        return Err((
            StatusCode::UNAUTHORIZED,
            "invalid or missing bearer token".to_string(),
        ));
    }
    Ok(next.run(request).await)
}

/// Simple constant-time comparison for token checks.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

impl Default for ServerState {
    fn default() -> Self {
        ServerState {
            identities: HashMap::new(),
            graph: GraphSnapshot {
                nodes: vec![],
                edges: vec![],
            },
            nullifiers: NullifierRegistry::new(),
            loans: Vec::new(),
            repayments: Vec::new(),
            vouching_limits: VouchingLimits::default(),
            keyring: Keyring::new(),
        }
    }
}

impl ServerState {
    fn to_graph(&self) -> SocialGraph {
        SocialGraph::from_snapshot(self.graph.clone())
    }

    fn apply_graph(&mut self, graph: &SocialGraph) {
        self.graph = graph.to_snapshot();
    }
}

fn read_state(
    state: &SharedState,
) -> Result<std::sync::RwLockReadGuard<'_, ServerState>, (StatusCode, String)> {
    state.read().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "state lock poisoned".to_string(),
        )
    })
}

fn write_state(
    state: &SharedState,
) -> Result<std::sync::RwLockWriteGuard<'_, ServerState>, (StatusCode, String)> {
    state.write().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "state lock poisoned".to_string(),
        )
    })
}

async fn health(State(ctx): State<AppCtx>) -> Json<HealthResponse> {
    let graph_nodes;
    let graph_edges;
    let loans;
    {
        let Ok(state) = ctx.inner.read() else {
            return Json(HealthResponse {
                status: "degraded".to_string(),
                nodes: 0,
                edges: 0,
                loans: 0,
            });
        };
        let graph = state.to_graph();
        graph_nodes = graph.node_count();
        graph_edges = graph.edge_count();
        loans = state.loans.len();
    }
    Json(HealthResponse {
        status: "ok".to_string(),
        nodes: graph_nodes,
        edges: graph_edges,
        loans,
    })
}

async fn add_user(
    State(ctx): State<AppCtx>,
    Json(req): Json<AddUserRequest>,
) -> Result<Json<UserResponse>, (StatusCode, String)> {
    let mut state = write_state(&ctx.inner)?;
    let id = req.id.unwrap_or_else(|| {
        format!(
            "user_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        )
    });
    // Reject duplicate ids so an authenticated caller cannot silently rotate
    // an existing identity's keypair (orphaning its vouches/nullifiers).
    if state.identities.contains_key(&id) {
        return Err((
            StatusCode::CONFLICT,
            format!("user '{}' already exists", id),
        ));
    }
    let identity = Identity::generate(id.clone(), req.name.clone());

    let node = NodeData {
        id: id.clone(),
        display_name: req.name.clone(),
        created_at: chrono::Utc::now().timestamp(),
    };

    let mut graph = state.to_graph();
    graph.add_node(node);
    state.apply_graph(&graph);
    if let Some(key) = identity.signing_key_bytes.clone() {
        state.keyring.insert(id.clone(), key);
    }
    state.identities.insert(id.clone(), identity.clone());
    persist_locked(&state, &ctx.state_file, &ctx.keyring_file);

    Ok(Json(UserResponse {
        id,
        display_name: identity.display_name,
        created_at: chrono::Utc::now().timestamp(),
        public_key: hex_encode(&identity.public_key_bytes),
    }))
}

async fn list_users(State(ctx): State<AppCtx>) -> Json<Vec<UserResponse>> {
    let Ok(state) = ctx.inner.read() else {
        return Json(vec![]);
    };
    let graph = state.to_graph();
    let users: Vec<UserResponse> = graph
        .all_nodes()
        .into_iter()
        .map(|n| UserResponse {
            id: n.id.clone(),
            display_name: n.display_name.clone(),
            created_at: n.created_at,
            public_key: state
                .identities
                .get(&n.id)
                .map(|i| hex_encode(&i.public_key_bytes))
                .unwrap_or_default(),
        })
        .collect();
    Json(users)
}

async fn create_vouch(
    State(ctx): State<AppCtx>,
    Json(req): Json<VouchRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut state = write_state(&ctx.inner)?;

    let voucher = state.identities.get(&req.voucher_id).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            format!("voucher '{}' not found", req.voucher_id),
        )
    })?;

    state
        .vouching_limits
        .can_vouch(&state.to_graph(), &req.voucher_id)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    let rel = parse_relationship(&req.relationship);
    let vouch = VouchStatement::create(voucher, &req.target_id, rel, req.weight)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    let mut graph = state.to_graph();
    graph
        .add_edge(&req.voucher_id, &req.target_id, vouch.to_edge_data())
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    state.apply_graph(&graph);
    persist_locked(&state, &ctx.state_file, &ctx.keyring_file);

    Ok(Json(serde_json::json!({
        "status": "ok",
        "voucher": req.voucher_id,
        "target": req.target_id,
        "weight": req.weight,
        "signature": hex_encode(&vouch.signature_bytes[..8])
    })))
}

async fn trust_path(
    State(ctx): State<AppCtx>,
    Query(query): Query<TrustPathQuery>,
) -> Json<TrustPathResponse> {
    let Ok(state) = ctx.inner.read() else {
        return Json(TrustPathResponse {
            found: false,
            hops: 0,
            path: vec![],
        });
    };
    let graph = state.to_graph();

    match graph.shortest_trust_path(&query.from, &query.to) {
        Some(path) => {
            let mut path_nodes = Vec::new();
            for (i, node_id) in path.iter().enumerate() {
                let name = graph
                    .get_node(node_id)
                    .map(|n| n.display_name.clone())
                    .unwrap_or_default();
                if i < path.len() - 1 {
                    let edge_info = graph
                        .neighbors(node_id)
                        .into_iter()
                        .find(|(id, _)| id == &path[i + 1]);
                    path_nodes.push(PathNode {
                        id: node_id.clone(),
                        name,
                        relationship: edge_info.as_ref().map(|(_, e)| e.relationship.to_string()),
                        weight: edge_info.map(|(_, e)| e.trust_weight),
                    });
                } else {
                    path_nodes.push(PathNode {
                        id: node_id.clone(),
                        name,
                        relationship: None,
                        weight: None,
                    });
                }
            }
            Json(TrustPathResponse {
                found: true,
                hops: path.len() - 1,
                path: path_nodes,
            })
        }
        None => Json(TrustPathResponse {
            found: false,
            hops: 0,
            path: vec![],
        }),
    }
}

async fn trust_score(
    State(ctx): State<AppCtx>,
    Path(user): Path<String>,
) -> Result<Json<ReputationScoreResponse>, (StatusCode, String)> {
    let state = read_state(&ctx.inner)?;
    let graph = state.to_graph();

    if !graph.contains_node(&user) {
        return Err((StatusCode::NOT_FOUND, format!("user '{}' not found", user)));
    }

    let result = scoring::compute_reputation(&graph, &user)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "no reputation data".to_string()))?;
    let sybil = entropy::sybil_score(&graph, &user).unwrap_or(0.0);
    let name = graph
        .get_node(&user)
        .map(|n| n.display_name.clone())
        .unwrap_or_default();

    Ok(Json(ReputationScoreResponse {
        node_id: user,
        display_name: name,
        composite_score: result.composite_score,
        pagerank: result.pagerank_score,
        degree_centrality: result.degree_centrality,
        clustering: result.clustering_coefficient,
        sybil_score: sybil,
    }))
}

async fn stats(State(ctx): State<AppCtx>) -> Json<StatsResponse> {
    let Ok(state) = ctx.inner.read() else {
        return Json(StatsResponse {
            nodes: 0,
            edges: 0,
            density: 0.0,
            avg_clustering: 0.0,
            pagerank_range: vec![],
            entropy: 0.0,
            nullifiers_used: 0,
            active_loans: 0,
            total_loan_volume: 0.0,
        });
    };
    let graph = state.to_graph();
    let n = graph.node_count();
    let e = graph.edge_count();
    let density = if n > 1 {
        2.0 * e as f64 / (n * (n - 1)) as f64
    } else {
        0.0
    };
    let avg_clustering = algorithms::average_clustering(&graph);
    let entropy_val = entropy::graph_entropy(&graph);
    let active_loans = state
        .loans
        .iter()
        .filter(|l| l.status == LoanStatus::Active)
        .count();
    let total_volume: f64 = state.loans.iter().map(|l| l.principal).sum();

    // Guard against an empty graph: PageRank folds over no values yield
    // infinities that serialize as null.
    let pagerank_range = if n > 0 {
        let pr = algorithms::compute_pagerank(&graph, 0.85, 100, 1e-6);
        let max_pr = pr.values().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min_pr = pr.values().cloned().fold(f64::INFINITY, f64::min);
        vec![min_pr, max_pr]
    } else {
        vec![]
    };

    Json(StatsResponse {
        nodes: n,
        edges: e,
        density,
        avg_clustering,
        pagerank_range,
        entropy: entropy_val,
        nullifiers_used: state.nullifiers.used_count(),
        active_loans,
        total_loan_volume: total_volume,
    })
}

async fn create_loan(
    State(ctx): State<AppCtx>,
    Json(req): Json<CreateLoanRequest>,
) -> Result<Json<LoanResponse>, (StatusCode, String)> {
    let mut state = write_state(&ctx.inner)?;
    let graph = state.to_graph();

    let exposures: Vec<Exposure> = state
        .loans
        .iter()
        .map(|l| Exposure {
            borrower: l.borrower.clone(),
            outstanding: l.outstanding,
            active: l.status == LoanStatus::Active,
        })
        .collect();

    let (constraints_pass, _) =
        ConstraintEngine::check_borrower(&graph, &req.borrower, req.principal, &exposures)
            .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    if !constraints_pass {
        return Err((
            StatusCode::BAD_REQUEST,
            "borrower does not meet credit constraints".to_string(),
        ));
    }

    // Double-borrow protection on the API path (finding #3): every loan must
    // register a deterministic nullifier, exactly like the CLI does. The nonce is
    // identity-stable, so a second credit line for the same identity is rejected.
    let borrower_pk = state
        .identities
        .get(&req.borrower)
        .map(|i| i.public_key_bytes.clone())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                format!("borrower '{}' not found", req.borrower),
            )
        })?;

    const CREDIT_NONCE: &[u8] = b"credit-line:v1";
    state
        .nullifiers
        .check_nullifier(&req.borrower, &borrower_pk, CREDIT_NONCE)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    let loan_id = format!(
        "loan_{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
    );
    let interest_rate = calculate_interest_rate(&graph, &req.borrower, 0.05);

    let mut loan = Loan::new(
        loan_id.clone(),
        req.borrower.clone(),
        req.lender.clone(),
        req.principal,
        interest_rate,
        req.purpose.clone(),
    )
    .map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    loan.drawdown().map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    // Only after the loan is valid do we mark the identity's credit line used.
    state
        .nullifiers
        .register_nullifier(&req.borrower, &borrower_pk, CREDIT_NONCE)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    state.loans.push(loan);
    persist_locked(&state, &ctx.state_file, &ctx.keyring_file);

    Ok(Json(LoanResponse {
        id: loan_id,
        borrower: req.borrower,
        lender: req.lender,
        principal: req.principal,
        interest_rate,
        status: "active".to_string(),
        constraints_passed: true,
    }))
}

async fn list_loans(State(ctx): State<AppCtx>) -> Json<Vec<LoanResponse>> {
    let Ok(state) = ctx.inner.read() else {
        return Json(vec![]);
    };
    let loans: Vec<LoanResponse> = state
        .loans
        .iter()
        .map(|l| LoanResponse {
            id: l.id.clone(),
            borrower: l.borrower.clone(),
            lender: l.lender.clone(),
            principal: l.principal,
            interest_rate: l.interest_rate,
            status: format!("{:?}", l.status).to_lowercase(),
            constraints_passed: true,
        })
        .collect();
    Json(loans)
}

async fn repay_loan(
    State(ctx): State<AppCtx>,
    Path(id): Path<String>,
    Json(req): Json<RepayRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut state = write_state(&ctx.inner)?;

    let (borrower, lender, repaid, remaining, status_str, released) = {
        let loan = state
            .loans
            .iter_mut()
            .find(|l| l.id == id)
            .ok_or_else(|| (StatusCode::NOT_FOUND, format!("loan '{}' not found", id)))?;

        let repaid = loan
            .repay(req.amount)
            .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

        let released = loan.status == LoanStatus::Repaid;
        (
            loan.borrower.clone(),
            loan.lender.clone(),
            repaid,
            loan.outstanding,
            format!("{:?}", loan.status).to_lowercase(),
            released,
        )
    };

    state.repayments.push(RepaymentRecord {
        from: borrower.clone(),
        to: lender,
        amount: repaid,
    });

    // Fully repaid → release the identity's credit line so it can borrow again.
    if released {
        if let Some(identity) = state.identities.get(&borrower).cloned() {
            state.nullifiers.release_nullifier(
                &borrower,
                &identity.public_key_bytes,
                b"credit-line:v1",
            );
        }
    }
    persist_locked(&state, &ctx.state_file, &ctx.keyring_file);

    Ok(Json(serde_json::json!({
        "status": "ok",
        "loan_id": id,
        "repaid": repaid,
        "outstanding": remaining,
        "loan_status": status_str,
        "credit_line_released": released
    })))
}

#[derive(Deserialize)]
struct NullifierCheckRequest {
    identity_id: String,
    public_key_hex: String,
    nonce_hex: String,
}

async fn check_nullifier(
    State(ctx): State<AppCtx>,
    Json(req): Json<NullifierCheckRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let state = read_state(&ctx.inner)?;

    // Strict hex decoding (finding #4): malformed input is a 400, never silently
    // re-interpreted as a different (shorter) key.
    let pk_bytes = hex_decode(&req.public_key_hex).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let nonce_bytes = hex_decode(&req.nonce_hex).map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    let nullifier = NullifierRegistry::compute_nullifier(&req.identity_id, &pk_bytes, &nonce_bytes);
    let used = state.nullifiers.is_used(&nullifier);

    Ok(Json(serde_json::json!({
        "nullifier": nullifier,
        "used": used,
        "message": if used {
            "identity already has an active credit line"
        } else {
            "identity is clear"
        }
    })))
}

async fn sybil_score(
    State(ctx): State<AppCtx>,
    Path(user): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let state = read_state(&ctx.inner)?;
    let graph = state.to_graph();

    if !graph.contains_node(&user) {
        return Err((StatusCode::NOT_FOUND, format!("user '{}' not found", user)));
    }

    let score = entropy::sybil_score(&graph, &user).unwrap_or(0.0);
    let local_entropy = entropy::local_entropy(&graph, &user, 2);
    let global_entropy = entropy::graph_entropy(&graph);

    Ok(Json(serde_json::json!({
        "user": user,
        "sybil_score": score,
        "local_entropy": local_entropy,
        "global_entropy": global_entropy,
        "risk": if score > 0.7 { "high" } else if score > 0.4 { "medium" } else { "low" }
    })))
}

fn parse_relationship(s: &str) -> RelationshipType {
    match s.to_lowercase().as_str() {
        "friend" => RelationshipType::Friend,
        "family" => RelationshipType::Family,
        "colleague" => RelationshipType::Colleague,
        "guarantor" => RelationshipType::Guarantor,
        _ => RelationshipType::Custom(s.to_string()),
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        write!(out, "{:02x}", b).expect("writing to a String cannot fail");
    }
    out
}

/// Strict hex decoder: rejects odd lengths and any non-hex character
/// instead of silently truncating or dropping bytes (finding #4).
fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    if s.len() % 2 != 0 {
        return Err(format!(
            "hex string must have even length, got {} chars",
            s.len()
        ));
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    for i in (0..s.len()).step_by(2) {
        let byte = u8::from_str_radix(&s[i..i + 2], 16)
            .map_err(|_| format!("invalid hex character at position {}", i))?;
        out.push(byte);
    }
    Ok(out)
}

/// Atomic write: temp file + rename so a crash never leaves a half-written state.
fn atomic_write(path: &std::path::Path, content: &[u8]) -> Result<(), String> {
    let tmp = PathBuf::from(format!("{}.tmp.{}", path.display(), std::process::id()));
    std::fs::write(&tmp, content).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}
