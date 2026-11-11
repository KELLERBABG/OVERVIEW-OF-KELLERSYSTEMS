//! HTTP route handlers for the encrypted database REST API.
//!
//! Every handler that touches ciphertexts wraps its work in a
//! [`fhe_core::ServerKeyGuard`] so the TFHE thread-local server key is
//! pinned for the duration of the computation. Handlers **never** unwrap on
//! data from the wire: envelope bytes are validated with
//! [`fhe_core::Ciphertext::from_envelope_bytes`] and rejected with a typed
//! error.
//!
//! ## DoS posture
//!
//! The router enforces request-size caps ([`Limits`]), a per-IP rate limit,
//! and work guards (max aggregation rows) so a single caller cannot exhaust
//! memory or burn unbounded homomorphic CPU. Violations return `413
//! PAYLOAD_TOO_LARGE` / `429 RATE_LIMITED` with a stable [`crate::ServerError`]
//! `code`.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, Path, State};
use axum::http::{header, StatusCode};
use axum::middleware::{from_fn_with_state, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use serde::Deserialize;
use serde_json::json;
use tokio::sync::RwLock;

use fhe_core::nn::{Activation, DenseLayer, FixedPoint};
use fhe_core::{
    Ciphertext, CiphertextAlgebra, EncryptedColumn, ScalarWidth, ServerKey, ServerKeyGuard,
};
use fhe_db::{ColumnDef, ColumnType, EncryptedDatabase, QueryEngine, TableSchema, Wal, WalRecord};

use crate::error::{ServerError, ServerResult};

/// Base64 engine used for all envelope encoding on the wire.
const B64: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;

// ---------------------------------------------------------------------------
// Limits (DoS guards)
// ---------------------------------------------------------------------------

/// Configurable request-size and work guards enforced by the router.
///
/// Defaults are generous for a single-tenant local deployment; operators can
/// tighten them per deployment (see `SECURITY.md`).
#[derive(Debug, Clone)]
pub struct Limits {
    /// Maximum JSON request body for every route except `POST /keys/server`
    /// (which has its own [`Limits::max_server_key_bytes`]).
    pub max_json_body_bytes: usize,
    /// Maximum raw server-key upload body (server keys are megabytes).
    pub max_server_key_bytes: usize,
    /// Maximum number of base64 envelopes in a single insert batch.
    pub max_insert_envelopes: usize,
    /// Maximum decoded size of a single envelope in an insert batch.
    pub max_envelope_bytes: usize,
    /// Maximum number of rows a homomorphic aggregation may process (the
    /// "max price" guard for expensive FHE endpoints).
    pub max_aggregation_rows: usize,
    /// Maximum table-name length in characters.
    pub max_table_name_len: usize,
    /// Maximum column-name length in characters.
    pub max_column_name_len: usize,
    /// Maximum number of columns per table schema.
    pub max_columns_per_table: usize,
    /// Maximum number of bias entries in an installed XNOR model.
    pub max_model_bias_entries: usize,
    /// Fixed-window length for the per-IP rate limiter.
    pub rate_window: Duration,
    /// Allowed requests per window per IP for general endpoints.
    pub rate_quota: u64,
    /// Allowed requests per window per IP for expensive endpoints
    /// (aggregations and homomorphic inference).
    pub rate_quota_expensive: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_json_body_bytes: 2 * 1024 * 1024,
            max_server_key_bytes: 256 * 1024 * 1024,
            max_insert_envelopes: 10_000,
            max_envelope_bytes: 4 * 1024 * 1024,
            max_aggregation_rows: 1_000_000,
            max_table_name_len: 128,
            max_column_name_len: 128,
            max_columns_per_table: 256,
            max_model_bias_entries: 64,
            rate_window: Duration::from_secs(60),
            rate_quota: 1000,
            rate_quota_expensive: 120,
        }
    }
}

// ---------------------------------------------------------------------------
// Metrics
// ---------------------------------------------------------------------------

/// Dependency-free Prometheus-style counters exposed at `GET /metrics`.
#[derive(Debug, Default)]
pub struct Metrics {
    requests_total: AtomicU64,
    responses_2xx: AtomicU64,
    responses_4xx: AtomicU64,
    responses_5xx: AtomicU64,
    rate_limited_total: AtomicU64,
    envelope_bytes_total: AtomicU64,
    rows_inserted_total: AtomicU64,
    key_uploads_total: AtomicU64,
    tables_created_total: AtomicU64,
    columns_added_total: AtomicU64,
    aggregations_total: AtomicU64,
    inferences_total: AtomicU64,
    models_installed_total: AtomicU64,
    readbacks_total: AtomicU64,
    // -- write-ahead log observability --
    wal_recoveries_total: AtomicU64,
    wal_recovery_failures_total: AtomicU64,
    wal_recovery_records_replayed_total: AtomicU64,
    wal_recovery_record_bytes_total: AtomicU64,
    wal_torn_tail_bytes_truncated_total: AtomicU64,
    wal_log_bytes: AtomicU64,
    wal_checkpoints_total: AtomicU64,
    wal_checkpoint_failures_total: AtomicU64,
    wal_checkpoint_records_compacted_total: AtomicU64,
    wal_checkpoint_snapshot_bytes_total: AtomicU64,
    wal_checkpoint_duration_micros_total: AtomicU64,
}

impl Metrics {
    /// Record one completed response by status class.
    pub fn record_response(&self, status: StatusCode) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
        if status.is_success() {
            self.responses_2xx.fetch_add(1, Ordering::Relaxed);
        } else if status.is_client_error() {
            self.responses_4xx.fetch_add(1, Ordering::Relaxed);
        } else {
            self.responses_5xx.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Add decoded envelope bytes to the total.
    fn add_envelope_bytes(&self, n: u64) {
        self.envelope_bytes_total.fetch_add(n, Ordering::Relaxed);
    }

    /// Record one successful boot recovery (the `wal.recover` event).
    pub fn record_recovery(&self, report: &fhe_db::RecoveryReport) {
        self.wal_recoveries_total.fetch_add(1, Ordering::Relaxed);
        self.wal_recovery_records_replayed_total
            .fetch_add(report.records_replayed, Ordering::Relaxed);
        self.wal_recovery_record_bytes_total
            .fetch_add(report.record_bytes_replayed, Ordering::Relaxed);
        self.wal_torn_tail_bytes_truncated_total
            .fetch_add(report.torn_tail_bytes_truncated, Ordering::Relaxed);
        self.wal_log_bytes
            .store(report.log_bytes, Ordering::Relaxed);
    }

    /// Record one failed boot recovery.
    pub fn record_recovery_failure(&self) {
        self.wal_recovery_failures_total
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record one successful checkpoint (the `wal.checkpoint` event).
    pub fn record_checkpoint(&self, report: &fhe_db::CheckpointReport) {
        self.wal_checkpoints_total.fetch_add(1, Ordering::Relaxed);
        self.wal_checkpoint_records_compacted_total
            .fetch_add(report.records_compacted, Ordering::Relaxed);
        self.wal_checkpoint_snapshot_bytes_total
            .fetch_add(report.snapshot_bytes, Ordering::Relaxed);
        self.wal_checkpoint_duration_micros_total.fetch_add(
            report.duration.as_micros().min(u128::from(u64::MAX)) as u64,
            Ordering::Relaxed,
        );
        self.wal_log_bytes
            .store(report.log_bytes_after, Ordering::Relaxed);
    }

    /// Record one failed checkpoint attempt.
    pub fn record_checkpoint_failure(&self) {
        self.wal_checkpoint_failures_total
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Publish the current on-disk log size for the `fhe_wal_log_bytes` gauge.
    pub fn set_wal_log_bytes(&self, bytes: u64) {
        self.wal_log_bytes.store(bytes, Ordering::Relaxed);
    }

    /// Render the counters in Prometheus text exposition format.
    pub fn render(&self) -> String {
        fn c(name: &str, value: u64) -> String {
            format!("{name} {value}\n")
        }
        let mut out = String::new();
        out.push_str("# HELP fhe_requests_total Total HTTP requests processed by the server.\n");
        out.push_str("# TYPE fhe_requests_total counter\n");
        out.push_str(&c(
            "fhe_requests_total",
            self.requests_total.load(Ordering::Relaxed),
        ));
        out.push_str("# HELP fhe_responses_total HTTP responses by status class.\n");
        out.push_str("# TYPE fhe_responses_total counter\n");
        out.push_str(&c(
            "fhe_responses_total{class=\"2xx\"}",
            self.responses_2xx.load(Ordering::Relaxed),
        ));
        out.push_str(&c(
            "fhe_responses_total{class=\"4xx\"}",
            self.responses_4xx.load(Ordering::Relaxed),
        ));
        out.push_str(&c(
            "fhe_responses_total{class=\"5xx\"}",
            self.responses_5xx.load(Ordering::Relaxed),
        ));
        out.push_str("# HELP fhe_rate_limited_total Requests rejected by the rate limiter.\n");
        out.push_str("# TYPE fhe_rate_limited_total counter\n");
        out.push_str(&c(
            "fhe_rate_limited_total",
            self.rate_limited_total.load(Ordering::Relaxed),
        ));
        out.push_str("# HELP fhe_envelope_bytes_total Ciphertext envelope bytes accepted.\n");
        out.push_str("# TYPE fhe_envelope_bytes_total counter\n");
        out.push_str(&c(
            "fhe_envelope_bytes_total",
            self.envelope_bytes_total.load(Ordering::Relaxed),
        ));
        out.push_str("# HELP fhe_rows_inserted_total Encrypted rows inserted.\n");
        out.push_str("# TYPE fhe_rows_inserted_total counter\n");
        out.push_str(&c(
            "fhe_rows_inserted_total",
            self.rows_inserted_total.load(Ordering::Relaxed),
        ));
        out.push_str("# HELP fhe_key_uploads_total Server keys installed.\n");
        out.push_str("# TYPE fhe_key_uploads_total counter\n");
        out.push_str(&c(
            "fhe_key_uploads_total",
            self.key_uploads_total.load(Ordering::Relaxed),
        ));
        out.push_str("# HELP fhe_tables_created_total Tables created.\n");
        out.push_str("# TYPE fhe_tables_created_total counter\n");
        out.push_str(&c(
            "fhe_tables_created_total",
            self.tables_created_total.load(Ordering::Relaxed),
        ));
        out.push_str("# HELP fhe_columns_added_total Columns added.\n");
        out.push_str("# TYPE fhe_columns_added_total counter\n");
        out.push_str(&c(
            "fhe_columns_added_total",
            self.columns_added_total.load(Ordering::Relaxed),
        ));
        out.push_str("# HELP fhe_aggregations_total Homomorphic aggregations run.\n");
        out.push_str("# TYPE fhe_aggregations_total counter\n");
        out.push_str(&c(
            "fhe_aggregations_total",
            self.aggregations_total.load(Ordering::Relaxed),
        ));
        out.push_str("# HELP fhe_inferences_total Homomorphic inferences run.\n");
        out.push_str("# TYPE fhe_inferences_total counter\n");
        out.push_str(&c(
            "fhe_inferences_total",
            self.inferences_total.load(Ordering::Relaxed),
        ));
        out.push_str("# HELP fhe_models_installed_total Encrypted models installed.\n");
        out.push_str("# TYPE fhe_models_installed_total counter\n");
        out.push_str(&c(
            "fhe_models_installed_total",
            self.models_installed_total.load(Ordering::Relaxed),
        ));
        out.push_str("# HELP fhe_readbacks_total Ciphertext read-back requests.\n");
        out.push_str("# TYPE fhe_readbacks_total counter\n");
        out.push_str(&c(
            "fhe_readbacks_total",
            self.readbacks_total.load(Ordering::Relaxed),
        ));
        out.push_str("# HELP fhe_wal_recoveries_total Successful boot recoveries from the write-ahead log.\n");
        out.push_str("# TYPE fhe_wal_recoveries_total counter\n");
        out.push_str(&c(
            "fhe_wal_recoveries_total",
            self.wal_recoveries_total.load(Ordering::Relaxed),
        ));
        out.push_str("# HELP fhe_wal_recovery_failures_total Boot recoveries that failed (the process refuses to serve).\n");
        out.push_str("# TYPE fhe_wal_recovery_failures_total counter\n");
        out.push_str(&c(
            "fhe_wal_recovery_failures_total",
            self.wal_recovery_failures_total.load(Ordering::Relaxed),
        ));
        out.push_str("# HELP fhe_wal_recovery_records_replayed_total WAL records replayed during boot recovery.\n");
        out.push_str("# TYPE fhe_wal_recovery_records_replayed_total counter\n");
        out.push_str(&c(
            "fhe_wal_recovery_records_replayed_total",
            self.wal_recovery_records_replayed_total
                .load(Ordering::Relaxed),
        ));
        out.push_str("# HELP fhe_wal_recovery_record_bytes_total WAL record bytes replayed during boot recovery.\n");
        out.push_str("# TYPE fhe_wal_recovery_record_bytes_total counter\n");
        out.push_str(&c(
            "fhe_wal_recovery_record_bytes_total",
            self.wal_recovery_record_bytes_total.load(Ordering::Relaxed),
        ));
        out.push_str("# HELP fhe_wal_torn_tail_bytes_truncated_total Torn-tail bytes healed (truncated) at boot.\n");
        out.push_str("# TYPE fhe_wal_torn_tail_bytes_truncated_total counter\n");
        out.push_str(&c(
            "fhe_wal_torn_tail_bytes_truncated_total",
            self.wal_torn_tail_bytes_truncated_total
                .load(Ordering::Relaxed),
        ));
        out.push_str(
            "# HELP fhe_wal_log_bytes On-disk WAL size at boot and after each checkpoint.\n",
        );
        out.push_str("# TYPE fhe_wal_log_bytes gauge\n");
        out.push_str(&c(
            "fhe_wal_log_bytes",
            self.wal_log_bytes.load(Ordering::Relaxed),
        ));
        out.push_str(
            "# HELP fhe_wal_checkpoints_total Successful WAL checkpoints (log compactions).\n",
        );
        out.push_str("# TYPE fhe_wal_checkpoints_total counter\n");
        out.push_str(&c(
            "fhe_wal_checkpoints_total",
            self.wal_checkpoints_total.load(Ordering::Relaxed),
        ));
        out.push_str("# HELP fhe_wal_checkpoint_failures_total WAL checkpoints that failed.\n");
        out.push_str("# TYPE fhe_wal_checkpoint_failures_total counter\n");
        out.push_str(&c(
            "fhe_wal_checkpoint_failures_total",
            self.wal_checkpoint_failures_total.load(Ordering::Relaxed),
        ));
        out.push_str("# HELP fhe_wal_checkpoint_records_compacted_total WAL records dropped by checkpoint compaction.\n");
        out.push_str("# TYPE fhe_wal_checkpoint_records_compacted_total counter\n");
        out.push_str(&c(
            "fhe_wal_checkpoint_records_compacted_total",
            self.wal_checkpoint_records_compacted_total
                .load(Ordering::Relaxed),
        ));
        out.push_str("# HELP fhe_wal_checkpoint_snapshot_bytes_total Snapshot bytes written by checkpoints.\n");
        out.push_str("# TYPE fhe_wal_checkpoint_snapshot_bytes_total counter\n");
        out.push_str(&c(
            "fhe_wal_checkpoint_snapshot_bytes_total",
            self.wal_checkpoint_snapshot_bytes_total
                .load(Ordering::Relaxed),
        ));
        out.push_str(
            "# HELP fhe_wal_checkpoint_duration_seconds_total Cumulative checkpoint duration.\n",
        );
        out.push_str("# TYPE fhe_wal_checkpoint_duration_seconds_total counter\n");
        out.push_str(&format!(
            "fhe_wal_checkpoint_duration_seconds_total {:.6}\n",
            self.wal_checkpoint_duration_micros_total
                .load(Ordering::Relaxed) as f64
                / 1_000_000.0
        ));
        out
    }
}

// ---------------------------------------------------------------------------
// Rate limiter
// ---------------------------------------------------------------------------

/// Fixed-window per-IP request buckets.
#[derive(Default)]
struct RateLimitState {
    buckets: HashMap<IpAddr, RateBucket>,
}

/// One fixed-window bucket for a single peer.
struct RateBucket {
    window_start: Instant,
    count: u64,
}

impl RateLimitState {
    /// Allow or reject one request from `ip` under `quota` per `window`.
    fn check(&mut self, ip: IpAddr, quota: u64, window: Duration) -> bool {
        let now = Instant::now();
        // Opportunistically prune expired buckets so the map cannot grow
        // without bound under IP spoofing/rotation.
        if self.buckets.len() > 4096 {
            self.buckets
                .retain(|_, b| now.duration_since(b.window_start) < window);
        }
        let bucket = self.buckets.entry(ip).or_insert(RateBucket {
            window_start: now,
            count: 0,
        });
        if now.duration_since(bucket.window_start) >= window {
            *bucket = RateBucket {
                window_start: now,
                count: 0,
            };
        }
        if bucket.count >= quota {
            return false;
        }
        bucket.count += 1;
        true
    }
}

/// Peer IP used when the router was not served with `ConnectInfo`
/// (e.g. in-process `oneshot` tests). All such requests share one bucket.
fn fallback_peer() -> IpAddr {
    IpAddr::from([127, 0, 0, 1])
}

/// Whether a path runs expensive homomorphic work (stricter rate quota).
fn is_expensive_path(path: &str) -> bool {
    path.ends_with("/aggregations/sum")
        || path.ends_with("/aggregations/avg")
        || path.contains("/models/xnor")
}

/// Per-IP rate limiting middleware. Applied inside `build_router`; requires
/// the service to be served with `into_make_service_with_connect_info` so the
/// real peer address is present (falls back to a shared bucket otherwise).
/// Buckets key on the IP only, so opening a new TCP connection does not reset
/// the quota.
async fn rate_limit_middleware(
    State(state): State<AppState>,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0.ip())
        .unwrap_or_else(fallback_peer);
    let (quota, window) = if is_expensive_path(request.uri().path()) {
        (state.limits.rate_quota_expensive, state.limits.rate_window)
    } else {
        (state.limits.rate_quota, state.limits.rate_window)
    };
    let allowed = state.rate_limit.write().await.check(peer, quota, window);
    if !allowed {
        state
            .metrics
            .rate_limited_total
            .fetch_add(1, Ordering::Relaxed);
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "code": "RATE_LIMITED",
                "message": "rate limit exceeded; slow down and retry",
            })),
        )
            .into_response();
    }
    next.run(request).await
}

/// Request/response accounting middleware for `GET /metrics`. Applied
/// outermost so rate-limited requests are counted too.
async fn metrics_middleware(
    State(state): State<AppState>,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    let response = next.run(request).await;
    state.metrics.record_response(response.status());
    response
}

/// Constant-time byte equality: the time to compare does not depend on the
/// position of the first differing byte, so a remote caller cannot learn the
/// configured token (or a prefix of it) by timing `401` responses.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    let max_len = a.len().max(b.len());
    let mut diff = 0u8;
    for i in 0..max_len {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        diff |= x ^ y;
    }
    diff == 0
}

/// Bearer-token authentication middleware.
///
/// When the server was started with `--auth-token` (or `FHE_DB_AUTH_TOKEN`),
/// every request except `GET /health` must carry `Authorization: Bearer
/// <token>`. The token is compared in constant time; missing or wrong tokens
/// get `401 UNAUTHORIZED`. Without auth configured this middleware is a
/// pass-through, preserving the loopback-only default.
async fn auth_middleware(
    State(state): State<AppState>,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    let Some(expected) = state.auth_token.as_ref() else {
        return next.run(request).await;
    };
    // Liveness probes must work without a secret; everything else is gated.
    if request.method() == axum::http::Method::GET && request.uri().path() == "/health" {
        return next.run(request).await;
    }
    // Accept `Bearer <token>` case-insensitively on the scheme (RFC 7235
    // schemes are case-insensitive); the token itself compares byte-exact.
    let provided = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim())
        .and_then(|v| {
            let (scheme, token) = v.split_once(' ')?;
            if scheme.eq_ignore_ascii_case("bearer") {
                Some(token.trim())
            } else {
                None
            }
        })
        .unwrap_or("");
    if constant_time_eq(provided.as_bytes(), expected.as_bytes()) {
        return next.run(request).await;
    }
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({ "code": "UNAUTHORIZED", "message": "missing or invalid bearer token" })),
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    /// The encrypted database (guarded by a tokio read-write lock).
    pub db: Arc<RwLock<EncryptedDatabase>>,
    /// The installed TFHE server key, if any.
    pub server_key: Arc<RwLock<Option<ServerKey>>>,
    /// The query engine (stateless; shared freely).
    pub query: QueryEngine,
    /// The write-ahead log, when the server was started with `--data-dir`.
    ///
    /// Every mutation is journaled here **before** being applied to `db`
    /// (write-ahead ordering). A crash mid-operation is recovered by
    /// [`fhe_db::Wal::recover`] on the next boot; the periodic checkpoint
    /// task compacts the log with an atomic replace.
    ///
    /// Lock order: `db` before `wal` (see [`AppState::checkpoint_wal`]).
    pub wal: Arc<RwLock<Option<Wal>>>,
    /// The installed encrypted XNOR model, if any. Weights are ciphertexts;
    /// the bias and layer shape are plaintext.
    pub xnor_model: Arc<RwLock<Option<InstalledXnorModel>>>,
    /// DoS / work guards.
    pub limits: Limits,
    /// Request and operation counters exposed at `GET /metrics`.
    pub metrics: Arc<Metrics>,
    /// Per-IP rate-limit buckets (private; only the middleware touches it).
    rate_limit: Arc<RwLock<RateLimitState>>,
    /// Optional bearer token required on every request except `GET /health`.
    /// `None` (the default) leaves the server open, preserving the
    /// loopback-only posture.
    auth_token: Option<Arc<str>>,
}

impl AppState {
    /// Create an empty [`AppState`] with default limits and no server key.
    pub fn new() -> Self {
        Self::with_limits_and_db(Limits::default(), EncryptedDatabase::new())
    }

    /// Create an [`AppState`] over a pre-loaded database (used by the CLI
    /// `--data-dir` restore path and by tests).
    pub fn with_db(db: EncryptedDatabase) -> Self {
        Self::with_limits_and_db(Limits::default(), db)
    }

    /// Create an [`AppState`] with custom [`Limits`] (used by tests to
    /// exercise the DoS guards deterministically).
    pub fn with_limits(limits: Limits) -> Self {
        Self::with_limits_and_db(limits, EncryptedDatabase::new())
    }

    /// Create an [`AppState`] with custom limits over a pre-loaded database.
    pub fn with_limits_and_db(limits: Limits, db: EncryptedDatabase) -> Self {
        Self {
            db: Arc::new(RwLock::new(db)),
            server_key: Arc::new(RwLock::new(None)),
            query: QueryEngine::new(),
            wal: Arc::new(RwLock::new(None)),
            xnor_model: Arc::new(RwLock::new(None)),
            limits,
            metrics: Arc::new(Metrics::default()),
            rate_limit: Arc::new(RwLock::new(RateLimitState::default())),
            auth_token: None,
        }
    }

    /// Create an [`AppState`] with an attached write-ahead log.
    ///
    /// All subsequent mutations are journaled to `wal` before being applied,
    /// so a crash loses nothing acknowledged to a client.
    pub fn with_limits_and_db_and_wal(limits: Limits, db: EncryptedDatabase, wal: Wal) -> Self {
        Self {
            db: Arc::new(RwLock::new(db)),
            server_key: Arc::new(RwLock::new(None)),
            query: QueryEngine::new(),
            wal: Arc::new(RwLock::new(Some(wal))),
            xnor_model: Arc::new(RwLock::new(None)),
            limits,
            metrics: Arc::new(Metrics::default()),
            rate_limit: Arc::new(RwLock::new(RateLimitState::default())),
            auth_token: None,
        }
    }
}

impl AppState {
    /// Checkpoint (compact) the attached write-ahead log, if any.
    ///
    /// This is the single checkpoint implementation used by the periodic task
    /// and by graceful shutdown.
    ///
    /// # Lock order
    ///
    /// Acquires the **database read lock before the WAL write lock**. Every
    /// operation that needs both locks must follow that order: the mutation
    /// handlers take `db.write` and then `wal.write`; a checkpoint takes
    /// `db.read` and then `wal.write`. No code may hold the WAL while waiting
    /// for the database, because a mutation can hold the database while
    /// waiting for the WAL (a deadlock).
    ///
    /// Taking the database read lock first also freezes the snapshot against
    /// concurrent mutations while the WAL is compacted, so the checkpoint and
    /// log cannot disagree.
    ///
    /// # Errors
    ///
    /// Propagates [`fhe_db::DbError`] from the checkpoint write, sync, or
    /// atomic rename. Returns `Ok(())` when no WAL is attached.
    pub async fn checkpoint_wal(&self) -> fhe_db::DbResult<()> {
        // The invariant is enforced by ordering these two acquisitions. Do
        // not reorder them; see the method docs and
        // `checkpoint_acquires_database_before_wal`.
        let db = self.db.read().await;
        let mut wal_guard = self.wal.write().await;
        let Some(wal) = wal_guard.as_mut() else {
            return Ok(());
        };
        match wal.checkpoint_with_report(&db) {
            Ok(report) => {
                self.metrics.record_checkpoint(&report);
                // One structured line per checkpoint: the operator can see
                // compaction happened and what it reclaimed, instead of
                // inferring it from a stable log size.
                tracing::info!(
                    target: "fhe_server::wal",
                    event = "wal.checkpoint",
                    outcome = "ok",
                    path = %wal.path().display(),
                    snapshot_bytes = report.snapshot_bytes,
                    records_compacted = report.records_compacted,
                    record_bytes_compacted = report.record_bytes_compacted,
                    log_bytes_before = report.log_bytes_before,
                    log_bytes_after = report.log_bytes_after,
                    duration_micros = report.duration.as_micros() as u64,
                    "write-ahead log checkpointed"
                );
                Ok(())
            }
            Err(e) => {
                self.metrics.record_checkpoint_failure();
                tracing::error!(
                    target: "fhe_server::wal",
                    event = "wal.checkpoint",
                    outcome = "error",
                    error = %e,
                    "write-ahead log checkpoint failed"
                );
                Err(e)
            }
        }
    }

    /// Require a bearer token on every request except `GET /health`.
    ///
    /// When set, requests without `Authorization: Bearer <token>` receive
    /// `401 UNAUTHORIZED`. The comparison is constant-time so the token
    /// cannot be recovered by timing. Liveness probes (`/health`) stay open
    /// so orchestrators can check the process without holding a secret.
    pub fn with_auth(mut self, token: impl Into<String>) -> Self {
        self.auth_token = Some(Arc::from(token.into()));
        self
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Build the axum router with all FHE-DB routes mounted.
pub fn build_router(state: AppState) -> Router {
    use axum::extract::DefaultBodyLimit;

    let limits = state.limits.clone();

    Router::new()
        .route("/health", get(health))
        .route(
            "/keys/server",
            // Server keys are megabytes; raise the body cap for this route
            // only (all JSON routes keep `max_json_body_bytes`).
            post(upload_server_key).layer(DefaultBodyLimit::max(limits.max_server_key_bytes)),
        )
        .route("/tables", get(list_tables).post(create_table))
        .route("/tables/{table}/columns", post(add_column))
        .route("/tables/{table}/rows", post(insert_rows))
        .route("/tables/{table}/aggregations/sum", post(aggregate_sum))
        .route("/tables/{table}/aggregations/avg", post(aggregate_avg))
        .route("/tables/{table}/aggregations/count", post(aggregate_count))
        // Phase 2: encrypted neural inference over HTTP. The client uploads
        // the *encrypted* weights once, then submits encrypted inputs; the
        // server evaluates XNOR = (x0 + x1 - 1)^2 over ciphertexts only.
        .route("/models/xnor", post(install_xnor_model))
        .route("/models/xnor/infer", post(infer_xnor))
        // Read-back of raw ciphertexts for zero-trust auditors and for the
        // phone shell's encrypted messaging app. Returns only ciphertexts;
        // plaintext never exists on this server.
        .route(
            "/tables/{table}/ciphertexts/{column}",
            axum::routing::get(get_column_ciphertexts),
        )
        // Prometheus-style counters for operations observability.
        .route("/metrics", get(metrics))
        // Default body cap for every JSON route (the /keys/server route
        // overrides it with its own, closer layer).
        .layer(DefaultBodyLimit::max(limits.max_json_body_bytes))
        // Layer order: the last `.layer()` is the outermost middleware, so
        // `metrics_middleware` wraps `auth_middleware` wraps
        // `rate_limit_middleware`. Metrics therefore also counts
        // rate-limited and unauthorized rejections. All middlewares embed a
        // clone of `state` (Arc-based, so all clones share the same data);
        // the handlers read the state installed by `with_state` below.
        .layer(from_fn_with_state(state.clone(), rate_limit_middleware))
        .layer(from_fn_with_state(state.clone(), auth_middleware))
        .layer(from_fn_with_state(state.clone(), metrics_middleware))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `GET /health` — liveness probe.
async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}

/// `GET /metrics` — Prometheus-text operation counters.
async fn metrics(State(state): State<AppState>) -> impl axum::response::IntoResponse {
    (
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        state.metrics.render(),
    )
}

/// `POST /keys/server` — install the TFHE evaluation key.
///
/// Body: raw server key bytes (`application/octet-stream`).
async fn upload_server_key(
    State(state): State<AppState>,
    body: axum::body::Bytes,
) -> ServerResult<StatusCode> {
    let server_key = ServerKey::from_bytes(&body)
        .map_err(|e| ServerError::BadRequest(format!("invalid server key: {e}")))?;
    // The server key is public, but install once and pin it for all future
    // requests. Replace any previous key atomically.
    *state.server_key.write().await = Some(server_key);
    state
        .metrics
        .key_uploads_total
        .fetch_add(1, Ordering::Relaxed);
    Ok(StatusCode::NO_CONTENT)
}

/// Request body for creating a table.
#[derive(Debug, Deserialize)]
pub struct CreateTableRequest {
    /// Table name (plaintext metadata).
    pub name: String,
    /// Column definitions.
    pub columns: Vec<ColumnDefJson>,
}

/// JSON-friendly column definition in request bodies.
#[derive(Debug, Deserialize)]
pub struct ColumnDefJson {
    /// Column name.
    pub name: String,
    /// Column value type.
    pub column_type: ColumnType,
}

impl From<ColumnDefJson> for ColumnDef {
    fn from(j: ColumnDefJson) -> Self {
        ColumnDef {
            name: j.name,
            column_type: j.column_type,
        }
    }
}

/// Validate schema-size guards shared by table creation and column addition.
fn check_schema_shape(
    limits: &Limits,
    table_name: Option<&str>,
    columns: &[ColumnDefJson],
) -> ServerResult<()> {
    if let Some(name) = table_name {
        if name.trim().is_empty() {
            return Err(ServerError::SchemaInvalid(
                "table name must not be empty".into(),
            ));
        }
        if name.chars().count() > limits.max_table_name_len {
            return Err(ServerError::SchemaInvalid(format!(
                "table name exceeds {} characters",
                limits.max_table_name_len
            )));
        }
    }
    if columns.len() > limits.max_columns_per_table {
        return Err(ServerError::SchemaInvalid(format!(
            "schema exceeds {} columns",
            limits.max_columns_per_table
        )));
    }
    for col in columns {
        if col.name.trim().is_empty() {
            return Err(ServerError::SchemaInvalid(
                "column name must not be empty".into(),
            ));
        }
        if col.name.chars().count() > limits.max_column_name_len {
            return Err(ServerError::SchemaInvalid(format!(
                "column name exceeds {} characters",
                limits.max_column_name_len
            )));
        }
    }
    Ok(())
}

/// `POST /tables` — create a table from a schema.
async fn create_table(
    State(state): State<AppState>,
    Json(req): Json<CreateTableRequest>,
) -> ServerResult<Json<TableSchema>> {
    check_schema_shape(&state.limits, Some(&req.name), &req.columns)?;
    let schema = TableSchema::new(req.name)
        .columns_checked(req.columns.into_iter().map(Into::into).collect())?;
    let mut db = state.db.write().await;
    if let Some(wal) = state.wal.write().await.as_mut() {
        let record = WalRecord::CreateTable {
            schema: schema.clone(),
        };
        // Reject a duplicate before the append: a rejected mutation must not
        // leave a record that makes the next boot fail replay.
        record.validate(&db).map_err(ServerError::from)?;
        wal.append(&record)
            .map_err(|e| ServerError::Io(e.to_string()))?;
        if let Ok(bytes) = wal.len() {
            state.metrics.set_wal_log_bytes(bytes);
        }
    }
    db.create_table(schema.clone())?;
    state
        .metrics
        .tables_created_total
        .fetch_add(1, Ordering::Relaxed);
    Ok(Json(schema))
}

/// `GET /tables` — list all table schemas.
async fn list_tables(State(state): State<AppState>) -> ServerResult<Json<Vec<TableSchema>>> {
    let db = state.db.read().await;
    Ok(Json(db.tables()))
}

/// `POST /tables/{table}/columns` — add a column to an existing table.
async fn add_column(
    State(state): State<AppState>,
    Path(table): Path<String>,
    Json(column): Json<ColumnDefJson>,
) -> ServerResult<Json<TableSchema>> {
    check_schema_shape(&state.limits, None, std::slice::from_ref(&column))?;
    let mut db = state.db.write().await;
    let schema = db.table_schema(&table)?;
    // Re-check the full column count (the new column may overflow the cap).
    if schema.columns.len() >= state.limits.max_columns_per_table {
        return Err(ServerError::SchemaInvalid(format!(
            "schema exceeds {} columns",
            state.limits.max_columns_per_table
        )));
    }
    let column_def: fhe_db::ColumnDef = column.into();
    if let Some(wal) = state.wal.write().await.as_mut() {
        let record = WalRecord::AddColumn {
            table: table.clone(),
            column: column_def.clone(),
        };
        record.validate(&db).map_err(ServerError::from)?;
        wal.append(&record)
            .map_err(|e| ServerError::Io(e.to_string()))?;
        if let Ok(bytes) = wal.len() {
            state.metrics.set_wal_log_bytes(bytes);
        }
    }
    db.add_column(&table, column_def)?;
    state
        .metrics
        .columns_added_total
        .fetch_add(1, Ordering::Relaxed);
    Ok(Json(db.table_schema(&table)?))
}

/// Request body for inserting ciphertext rows into a column.
#[derive(Debug, Deserialize)]
pub struct InsertRowsRequest {
    /// Target column name.
    pub column: String,
    /// Base64-encoded [`Ciphertext`] envelopes.
    pub ciphertexts: Vec<String>,
}

/// `POST /tables/{table}/rows` — append encrypted values to a column.
async fn insert_rows(
    State(state): State<AppState>,
    Path(table): Path<String>,
    Json(req): Json<InsertRowsRequest>,
) -> ServerResult<Json<serde_json::Value>> {
    if req.ciphertexts.len() > state.limits.max_insert_envelopes {
        return Err(ServerError::PayloadTooLarge(format!(
            "insert batch of {} envelopes exceeds the cap of {}",
            req.ciphertexts.len(),
            state.limits.max_insert_envelopes
        )));
    }
    // Decode and validate every envelope before touching the database so a
    // single corrupt packet cannot cause a partial insert.
    let mut ciphertexts = Vec::with_capacity(req.ciphertexts.len());
    let mut envelope_bytes_total: usize = 0;
    for encoded in &req.ciphertexts {
        let bytes = B64
            .decode(encoded)
            .map_err(|e| ServerError::InvalidBase64(format!("invalid base64 envelope: {e}")))?;
        if bytes.len() > state.limits.max_envelope_bytes {
            return Err(ServerError::PayloadTooLarge(format!(
                "envelope of {} bytes exceeds the cap of {}",
                bytes.len(),
                state.limits.max_envelope_bytes
            )));
        }
        envelope_bytes_total += bytes.len();
        let ct = Ciphertext::from_envelope_bytes(&bytes).map_err(|e| {
            ServerError::InvalidEnvelope(format!("invalid ciphertext envelope: {e}"))
        })?;
        ciphertexts.push(ct);
    }

    let mut db = state.db.write().await;
    // The server key is not needed for inserts (no homomorphic work), only
    // for aggregations; widths are validated by the database layer. The
    // write-ahead ordering is preserved: the WAL record commits before the
    // in-memory apply.
    if let Some(wal) = state.wal.write().await.as_mut() {
        let record = WalRecord::InsertRows {
            table: table.clone(),
            column: req.column.clone(),
            ciphertexts: ciphertexts.clone(),
        };
        // Width and table/column checks happen before the append, so a
        // rejected batch leaves no record for replay to choke on.
        record.validate(&db).map_err(ServerError::from)?;
        wal.append(&record)
            .map_err(|e| ServerError::Io(e.to_string()))?;
        if let Ok(bytes) = wal.len() {
            state.metrics.set_wal_log_bytes(bytes);
        }
    }
    let inserted = db.insert_ciphertexts(&table, &req.column, ciphertexts)?;
    state
        .metrics
        .rows_inserted_total
        .fetch_add(inserted as u64, Ordering::Relaxed);
    state
        .metrics
        .add_envelope_bytes(envelope_bytes_total as u64);
    Ok(Json(json!({ "inserted": inserted })))
}

/// Request body for an aggregation query.
#[derive(Debug, Deserialize)]
pub struct AggregateRequest {
    /// The column to aggregate.
    pub column: String,
}

/// Guard: reject aggregations over columns larger than the configured cap.
fn check_aggregation_size(state: &AppState, column: &EncryptedColumn) -> ServerResult<()> {
    if column.len() > state.limits.max_aggregation_rows {
        return Err(ServerError::PayloadTooLarge(format!(
            "aggregation over {} rows exceeds the cap of {}",
            column.len(),
            state.limits.max_aggregation_rows
        )));
    }
    Ok(())
}

/// An installed encrypted XNOR model.
///
/// Layout mirrors [`fhe_core::nn::DenseLayer`]: `weights` is row-major
/// `[input_count][output_count]` and `bias` holds one signed bias per output
/// neuron in the fixed-point scale. The weights are ciphertexts — the server
/// holds no plaintext model.
#[derive(Clone)]
pub struct InstalledXnorModel {
    /// Encrypted weights `[2][1]`.
    pub weights: Vec<Vec<Ciphertext>>,
    /// Plainttext bias per neuron, in the fixed-point scale.
    pub bias: Vec<i64>,
}

/// Request body for `POST /models/xnor`.
#[derive(Debug, Deserialize)]
pub struct InstallXnorRequest {
    /// Base64-encoded [`Ciphertext`] envelopes of the encrypted weights.
    pub weights: Vec<String>,
    /// Signed bias per output neuron in the fixed-point scale.
    pub bias: Vec<i64>,
}

/// Request body for `POST /models/xnor/infer`.
#[derive(Debug, Deserialize)]
pub struct InferXnorRequest {
    /// Base64-encoded [`Ciphertext`] envelopes of the encrypted inputs.
    pub inputs: Vec<String>,
}

/// Fixed-point format used for the XNOR model. Must match the client's
/// encoding: Q15.0 over 16-bit storage, pure-integer scale.
const XNOR_FIXED: FixedPoint = FixedPoint {
    width: ScalarWidth::U16,
    frac_bits: 0,
};

/// Clone the installed server key and pin it for this thread, returning a
/// guard that keeps it installed for the caller's scope.
///
/// Returns [`ServerError::NoServerKey`] if no server key has been installed
/// yet.
async fn pin_server_key(state: &AppState) -> ServerResult<ServerKeyGuard> {
    let key = state
        .server_key
        .read()
        .await
        .clone()
        .ok_or(ServerError::NoServerKey)?;
    Ok(ServerKeyGuard::new(&key))
}

/// `POST /tables/{table}/aggregations/sum` — homomorphic SUM over a column.
async fn aggregate_sum(
    State(state): State<AppState>,
    Path(table): Path<String>,
    Json(req): Json<AggregateRequest>,
) -> ServerResult<Json<serde_json::Value>> {
    let _guard = pin_server_key(&state).await?;
    let db = state.db.read().await;
    let column = db.column(&table, &req.column)?;
    check_aggregation_size(&state, column)?;
    let column_type = db.column_type(&table, &req.column)?;
    let result = state.query.sum(column, column_type)?;
    let bytes = result.value.to_envelope_bytes()?;
    state
        .metrics
        .aggregations_total
        .fetch_add(1, Ordering::Relaxed);
    Ok(Json(json!({
        "sum": B64.encode(bytes),
        "count": result.count,
    })))
}

/// `POST /tables/{table}/aggregations/avg` — homomorphic AVG over a column.
///
/// Returns the encrypted sum + count; the client divides after decryption.
async fn aggregate_avg(
    State(state): State<AppState>,
    Path(table): Path<String>,
    Json(req): Json<AggregateRequest>,
) -> ServerResult<Json<serde_json::Value>> {
    let _guard = pin_server_key(&state).await?;
    let db = state.db.read().await;
    let column = db.column(&table, &req.column)?;
    check_aggregation_size(&state, column)?;
    let column_type = db.column_type(&table, &req.column)?;
    let result = state.query.avg(column, column_type)?;
    let bytes = result.value.to_envelope_bytes()?;
    state
        .metrics
        .aggregations_total
        .fetch_add(1, Ordering::Relaxed);
    Ok(Json(json!({
        "sum": B64.encode(bytes),
        "count": result.count,
    })))
}

/// `POST /tables/{table}/aggregations/count` — public COUNT over a column.
async fn aggregate_count(
    State(state): State<AppState>,
    Path(table): Path<String>,
    Json(req): Json<AggregateRequest>,
) -> ServerResult<Json<serde_json::Value>> {
    let db = state.db.read().await;
    let column = db.column(&table, &req.column)?;
    check_aggregation_size(&state, column)?;
    let count = state.query.count(column);
    state
        .metrics
        .aggregations_total
        .fetch_add(1, Ordering::Relaxed);
    Ok(Json(json!({ "count": count })))
}

/// `POST /models/xnor` — install the encrypted XNOR weights.
///
/// The client encrypts `w0=1`, `w1=1` (and sends bias `-1` in the fixed-point
/// scale) so the server never sees the weight plaintext. The bias and layer
/// shape are plaintext inputs. The server pins the weights and uses them for
/// every subsequent [`infer_xnor`] call.
///
/// # Errors
///
/// - [`ServerError::BadRequest`] if a weight envelope is malformed or the
///   weight count differs from 2.
async fn install_xnor_model(
    State(state): State<AppState>,
    Json(req): Json<InstallXnorRequest>,
) -> ServerResult<StatusCode> {
    if req.weights.len() != 2 {
        return Err(ServerError::BadRequest(format!(
            "expected exactly 2 encrypted weights, got {}",
            req.weights.len()
        )));
    }
    if req.bias.is_empty() {
        return Err(ServerError::BadRequest(
            "bias must contain at least one element".into(),
        ));
    }
    if req.bias.len() > state.limits.max_model_bias_entries {
        return Err(ServerError::SchemaInvalid(format!(
            "bias exceeds {} entries",
            state.limits.max_model_bias_entries
        )));
    }
    let mut weights = Vec::with_capacity(2);
    for encoded in &req.weights {
        let bytes = B64
            .decode(encoded)
            .map_err(|e| ServerError::InvalidBase64(format!("invalid base64 weight: {e}")))?;
        let ct = Ciphertext::from_envelope_bytes(&bytes)
            .map_err(|e| ServerError::InvalidEnvelope(format!("invalid weight envelope: {e}")))?;
        weights.push(ct);
    }
    // Row-major layout for a 2-input → 1-output layer.
    let rows = vec![vec![weights[0].clone()], vec![weights[1].clone()]];
    *state.xnor_model.write().await = Some(InstalledXnorModel {
        weights: rows,
        bias: req.bias.clone(),
    });
    state
        .metrics
        .models_installed_total
        .fetch_add(1, Ordering::Relaxed);
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /models/xnor/infer` — run encrypted XNOR inference.
///
/// Evaluates `h = (x0 + x1 + bias)^2` entirely over ciphertexts using the
/// installed encrypted weights. Returns only the encrypted output; the client
/// decrypts locally. The input and weight ciphertexts stay secret; the
/// plaintext bias and model shape are visible to the server.
///
/// # Errors
///
/// - [`ServerError::NoModel`] if no model is installed.
/// - [`ServerError::NoServerKey`] if no server key is installed.
/// - [`ServerError::BadRequest`] if an input envelope is malformed or the
///   input count differs from 2.
async fn infer_xnor(
    State(state): State<AppState>,
    Json(req): Json<InferXnorRequest>,
) -> ServerResult<Json<serde_json::Value>> {
    let _guard = pin_server_key(&state).await?;
    let model = state
        .xnor_model
        .read()
        .await
        .clone()
        .ok_or(ServerError::NoModel)?;
    if req.inputs.len() != 2 {
        return Err(ServerError::BadRequest(format!(
            "expected exactly 2 encrypted inputs, got {}",
            req.inputs.len()
        )));
    }
    let mut inputs = Vec::with_capacity(2);
    for encoded in &req.inputs {
        let bytes = B64
            .decode(encoded)
            .map_err(|e| ServerError::InvalidBase64(format!("invalid base64 input: {e}")))?;
        let ct = Ciphertext::from_envelope_bytes(&bytes)
            .map_err(|e| ServerError::InvalidEnvelope(format!("invalid input envelope: {e}")))?;
        inputs.push(ct);
    }

    let algebra = CiphertextAlgebra;
    let layer = DenseLayer {
        obfuscated_weights: &model.weights,
        bias: &model.bias,
        activation: &Activation::poly_square(),
    };
    let outputs = layer
        .eval(&algebra, &XNOR_FIXED, &inputs)
        .map_err(|e| ServerError::HomomorphicEval(format!("homomorphic inference failed: {e}")))?;
    let bytes = outputs[0]
        .to_envelope_bytes()
        .map_err(|e| ServerError::Serialization(format!("failed to serialize output: {e}")))?;
    state
        .metrics
        .inferences_total
        .fetch_add(1, Ordering::Relaxed);
    Ok(Json(json!({
        "model": "xnor",
        "output": B64.encode(bytes),
    })))
}

/// `GET /tables/{table}/ciphertexts/{column}` — read back the stored
/// ciphertexts of a column.
///
/// Returns only ciphertext envelopes (base64). The server never holds the
/// plaintext, so this is the safe read-back path for zero-trust auditors and
/// for clients that want to decrypt rows locally (e.g. an encrypted
/// messaging app). No server key is required: no computation happens.
///
/// # Errors
///
/// - [`ServerError::Serialization`] if a stored ciphertext cannot be
///   serialized (corrupt database state).
/// - Table/column lookup errors propagate as 404 via [`fhe_db::DbError`].
async fn get_column_ciphertexts(
    State(state): State<AppState>,
    Path((table, column)): Path<(String, String)>,
) -> ServerResult<Json<serde_json::Value>> {
    let db = state.db.read().await;
    let encrypted = db.column(&table, &column)?;
    let mut envelopes = Vec::with_capacity(encrypted.len());
    for ct in encrypted.iter() {
        let bytes = ct.to_envelope_bytes().map_err(|e| {
            ServerError::Serialization(format!("failed to serialize ciphertext: {e}"))
        })?;
        envelopes.push(B64.encode(bytes));
    }
    state
        .metrics
        .readbacks_total
        .fetch_add(1, Ordering::Relaxed);
    Ok(Json(json!({
        "column": column,
        "count": envelopes.len(),
        "ciphertexts": envelopes,
    })))
}

/// Extension trait on [`TableSchema`] to validate column uniqueness when
/// building from a request.
trait TableSchemaExt {
    fn columns_checked(self, columns: Vec<ColumnDef>) -> ServerResult<TableSchema>;
}

impl TableSchemaExt for TableSchema {
    fn columns_checked(mut self, columns: Vec<ColumnDef>) -> ServerResult<TableSchema> {
        let mut seen = std::collections::HashSet::new();
        for col in columns {
            if !seen.insert(col.name.clone()) {
                return Err(ServerError::SchemaInvalid(format!(
                    "duplicate column `{}`",
                    col.name
                )));
            }
            self.columns.push(col);
        }
        Ok(self)
    }
}
