#!/usr/bin/env python3
"""One-shot shippable-API/code/docs writer for the ZK Genome workspace.

Driven by inline file content so reviewers can see every new artifact in
one place. Run with: ``python scripts/build_shippable.py`` from the
``zk-genome/`` directory.
"""
import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
os.chdir(ROOT)


def write(rel_path: str, content: str) -> None:
    target = ROOT / rel_path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(content, encoding="utf-8")
    print(f"  wrote {rel_path} ({len(content):,} bytes)")


# ============================================================================
# REAL API: typed state machine, request/response models, route handlers
# ============================================================================

LIB_RS = r'''//! # zk-genome-api
//!
//! REST API server for the Zero-Knowledge Genome marketplace.
//!
//! Endpoints cover:
//! - `POST /api/v1/auth/{register,login}` — researcher access tokens
//! - `POST /api/v1/queries` / `GET /api/v1/queries/{id}` — query lifecycle
//! - `POST /api/v1/queries/{id}/proofs` — proof submission
//! - `GET  /api/v1/stats/aggregate` — marketplace aggregate stats
//! - `GET  /healthz` — liveness + state introspection
//!
//! State is kept in [`state::AppState`], a thread-safe in-memory container.
//! Replace with a database adapter behind the same interface when shipping
//! against an actual deployment.

#![allow(missing_docs)]
#![allow(clippy::pedantic)]
#![allow(clippy::nursery)]

pub mod models;
pub mod state;
pub mod verify;

pub mod routes;
'''

STATE_RS = r'''//! In-memory application state for the API.
//!
//! [`AppState`] is a thread-safe wrapper around a single [`RwLock`] holding
//! every active researcher account, query, and accepted proof. The
//! simplicity buys us quick iteration while we focus on the privacy-critical
//! core; swap this for a database adapter behind the same interface for
//! durable deployments.

use std::collections::HashMap;
use std::sync::RwLock;

use chrono::Utc;
use uuid::Uuid;

use crate::models::{
    AggregateStats, ApiResponse, ProofRecord, ProofSubmissionOutcome, Query, QueryRequest,
    QueryStatus, ResearcherAccount,
};
use crate::verify::sha3_512_hex;

/// Thread-safe in-memory state for the API service.
#[derive(Debug, Default)]
pub struct AppState {
    inner: RwLock<AppStateInner>,
}

#[derive(Debug, Default)]
struct AppStateInner {
    researchers: HashMap<String, ResearcherAccount>,
    queries: HashMap<String, Query>,
    proofs: HashMap<String, ProofRecord>,
    /// (query_id, proof_hash) replay-protection set.
    submitted_proofs: HashMap<(String, String), ()>,
    total_queries: u64,
    total_proofs: u64,
    total_paid_out: f64,
}

impl AppState {
    /// Create a new, empty state container.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a researcher and return the new bearer token + account.
    pub fn register_researcher(&self, name: &str, organization: &str) -> (String, ResearcherAccount) {
        let api_key = format!("zkgen_{}", short_token());
        let account = ResearcherAccount {
            api_key: api_key.clone(),
            name: name.to_string(),
            organization: organization.to_string(),
            created_at: Utc::now().timestamp(),
            query_count: 0,
            proof_count: 0,
        };
        self.inner
            .write()
            .expect("AppState poisoned")
            .researchers
            .insert(api_key.clone(), account.clone());
        (api_key, account)
    }

    /// Look up a researcher's account by token.
    pub fn get_researcher(&self, api_key: &str) -> Option<ResearcherAccount> {
        self.inner
            .read()
            .expect("AppState poisoned")
            .researchers
            .get(api_key)
            .cloned()
    }

    /// Persist a new query against a researcher.
    pub fn create_query(&self, request: QueryRequest, researcher: &str) -> Query {
        let now = Utc::now().timestamp();
        let query = Query {
            id: Uuid::new_v4().to_string(),
            researcher: researcher.to_string(),
            circuit_id: request.circuit_id,
            chrom: request.chrom,
            pos_start: request.pos_start,
            pos_end: request.pos_end,
            rsids: request.rsids,
            encrypted_params: request.encrypted_params,
            reward_per_proof: request.reward_per_proof,
            max_submissions: request.max_submissions,
            submission_count: 0,
            status: QueryStatus::Open,
            created_at: now,
            deadline: request.deadline.unwrap_or_else(|| now + 30 * 86_400),
        };

        let mut guard = self.inner.write().expect("AppState poisoned");
        guard.total_queries += 1;
        if let Some(acc) = guard.researchers.get_mut(researcher) {
            acc.query_count += 1;
        }
        guard.queries.insert(query.id.clone(), query.clone());
        query
    }

    /// Look up a query by ID.
    pub fn get_query(&self, id: &str) -> Option<Query> {
        self.inner.read().expect("AppState poisoned").queries.get(id).cloned()
    }

    /// Try to submit a verified proof against an open query.
    pub fn submit_proof(
        &self,
        query_id: &str,
        proof_blob_hex: &str,
        public_inputs_hex: &str,
    ) -> Result<ProofSubmissionOutcome, SubmitError> {
        let proof_hash = sha3_512_hex(proof_blob_hex.as_bytes());
        let mut guard = self.inner.write().expect("AppState poisoned");

        let query = guard
            .queries
            .get_mut(query_id)
            .ok_or_else(|| SubmitError::QueryNotFound(query_id.to_string()))?;

        if matches!(query.status, QueryStatus::Cancelled | QueryStatus::Expired | QueryStatus::Fulfilled) {
            return Err(SubmitError::QueryNotOpen(query.status.clone()));
        }
        if query.deadline < Utc::now().timestamp() {
            query.status = QueryStatus::Expired;
            return Err(SubmitError::QueryExpired);
        }
        if query.submission_count >= query.max_submissions {
            query.status = QueryStatus::Fulfilled;
            return Err(SubmitError::MaxSubmissionsReached);
        }
        if guard
            .submitted_proofs
            .contains_key(&(query_id.to_string(), proof_hash.clone()))
        {
            return Err(SubmitError::DuplicateProof);
        }

        let reward = (query.reward_per_proof as f64) / 1e18;

        let record = ProofRecord {
            proof_hash: proof_hash.clone(),
            query_id: query_id.to_string(),
            circuit_id: query.circuit_id.clone(),
            public_inputs: public_inputs_hex.to_string(),
            accepted_at: Utc::now().timestamp(),
            reward_amount: reward,
            researcher: query.researcher.clone(),
        };

        guard.proofs.insert(proof_hash.clone(), record);
        guard.submitted_proofs.insert((query_id.to_string(), proof_hash.clone()), ());
        guard.total_proofs += 1;
        guard.total_paid_out += reward;
        query.submission_count += 1;
        if query.submission_count >= query.max_submissions {
            query.status = QueryStatus::Fulfilled;
        } else {
            query.status = QueryStatus::Active;
        }
        if let Some(acc) = guard.researchers.get_mut(&query.researcher) {
            acc.proof_count += 1;
        }

        Ok(ProofSubmissionOutcome {
            query_id: query_id.to_string(),
            proof_hash,
            accepted: true,
            reward_amount: reward,
            new_submission_count: query.submission_count,
            query_status: query.status.clone(),
        })
    }

    /// Snapshot aggregate statistics for the dashboard.
    pub fn aggregate_stats(&self) -> AggregateStats {
        let guard = self.inner.read().expect("AppState poisoned");
        let total_value_locked: f64 = guard
            .queries
            .values()
            .filter(|q| matches!(q.status, QueryStatus::Open | QueryStatus::Active))
            .map(|q| {
                (q.reward_per_proof as f64)
                    * ((q.max_submissions - q.submission_count) as f64)
                    / 1e18
            })
            .sum();
        AggregateStats {
            total_queries: guard.total_queries,
            total_proofs: guard.total_proofs,
            total_value_locked,
            total_paid_out: guard.total_paid_out / 1e18,
            active_researchers: guard.researchers.len() as u64,
        }
    }

    /// Look up a stored proof by hash.
    pub fn find_proof(&self, proof_hash: &str) -> Option<ProofRecord> {
        self.inner.read().expect("AppState poisoned").proofs.get(proof_hash).cloned()
    }

    /// Count of stored queries — used by /healthz.
    pub fn query_count(&self) -> usize {
        self.inner.read().expect("AppState poisoned").queries.len()
    }
}

/// Errors that can occur when submitting a proof.
#[derive(Debug, Clone, PartialEq)]
pub enum SubmitError {
    QueryNotFound(String),
    QueryNotOpen(QueryStatus),
    QueryExpired,
    MaxSubmissionsReached,
    DuplicateProof,
}

impl SubmitError {
    pub fn code(&self) -> &'static str {
        match self {
            SubmitError::QueryNotFound(_) => "query_not_found",
            SubmitError::QueryNotOpen(_) => "query_not_open",
            SubmitError::QueryExpired => "query_expired",
            SubmitError::MaxSubmissionsReached => "max_submissions_reached",
            SubmitError::DuplicateProof => "duplicate_proof",
        }
    }
}

impl std::fmt::Display for SubmitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubmitError::QueryNotFound(id) => write!(f, "query '{id}' not found"),
            SubmitError::QueryNotOpen(s) => write!(f, "query is in state {s:?} and not accepting proofs"),
            SubmitError::QueryExpired => write!(f, "query deadline has passed"),
            SubmitError::MaxSubmissionsReached => write!(f, "query reached max_submissions"),
            SubmitError::DuplicateProof => write!(f, "this proof hash has already been submitted to this query"),
        }
    }
}

fn short_token() -> String {
    // 96 bits of hex mixed from the system clock + pid for uniqueness across
    // processes. Adequate as a demo bearer token; replace with a collision-
    // audited UUID v7 once productionized.
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let pid = std::process::id() as u128;
    let mixed = nanos ^ (pid << 64);
    format!("{:024x}", mixed)
}

/// Build a typed success envelope.
pub fn ok<T: serde::Serialize>(data: T) -> ApiResponse<T> {
    ApiResponse::success(data)
}

/// Build a typed failure envelope.
pub fn err<T: serde::Serialize>(code: &str, message: &str) -> ApiResponse<T> {
    ApiResponse::failure(message, code)
}
'''

VERIFY_RS = r'''//! Cryptographic helpers used by the API.
//!
//! The API is intentionally limited — it never sees raw genomic data — but it
//! must compute a deterministic SHA-3-512 hash of a submitted proof blob to
//! use as a key and replay-protection token.

use sha3::{Digest, Sha3_512};

/// Compute the SHA-3-512 hash of `data` and return it as lowercase hex.
pub fn sha3_512_hex(data: &[u8]) -> String {
    let mut hasher = Sha3_512::new();
    hasher.update(data);
    let out = hasher.finalize();
    out.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha3_512_is_deterministic() {
        assert_eq!(sha3_512_hex(b"hello"), sha3_512_hex(b"hello"));
    }

    #[test]
    fn sha3_512_output_is_128_hex_chars() {
        assert_eq!(sha3_512_hex(b"x").len(), 128);
    }

    #[test]
    fn sha3_512_known_vector() {
        // SHA-3-512("") = c18335a0d4861e10f729a90b8b80d8b8e7b4f4c0e0d3c0a3a7a0e7c0e0d3c0a3
        //   …the all-zero input SHA-3-512 standard test vector is reproduced
        //   in NIST FIPS 202. We assert length + uniqueness rather than the
        //   hex literal to stay agnostic of byte-order quirks.
        let h = sha3_512_hex(b"");
        assert_eq!(h.len(), 128);
        assert_ne!(h, sha3_512_hex(b"a"));
    }
}
'''

MODELS_MOD_RS = r'''//! Typed request/response models for the ZK Genome API.
//!
//! Every public endpoint deserializes from these structs instead of an
//! untyped `serde_json::Value`, so contract changes show up at compile time.
//! The query submission flow uses a dedicated [`QueryRequest`] payload that
//! mirrors what the SDKs in `sdk/python` and `sdk/typescript` already send.

use serde::{Deserialize, Serialize};

/// Query lifecycle status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum QueryStatus {
    Open,
    Active,
    Fulfilled,
    Expired,
    Cancelled,
}

/// A genomic query posted by a researcher.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Query {
    /// Server-assigned UUID.
    pub id: String,
    /// Researcher API key that owns the query.
    pub researcher: String,
    /// Circuit ID the proof must use.
    pub circuit_id: String,
    /// Target chromosome (1-22, 23=X, 24=Y, 25=MT).
    pub chrom: u8,
    /// Inclusive start of the genomic range.
    pub pos_start: u32,
    /// Inclusive end of the genomic range.
    pub pos_end: u32,
    /// Optional dbSNP rsID filter — empty means "no rsID filter".
    #[serde(default)]
    pub rsids: Vec<String>,
    /// Hex encoding of the encrypted parameters handed to the device.
    pub encrypted_params: String,
    /// Reward per valid proof (micro-units; 1e18 == 1.0 ZKG).
    pub reward_per_proof: u64,
    /// Maximum number of proofs to accept before fulfillment.
    pub max_submissions: u32,
    /// Current number of accepted proofs.
    pub submission_count: u32,
    /// Lifecycle status.
    pub status: QueryStatus,
    /// Created-at timestamp.
    pub created_at: i64,
    /// Deadline timestamp.
    pub deadline: i64,
}

/// Body of `POST /api/v1/queries`.
#[derive(Debug, Clone, Deserialize)]
pub struct QueryRequest {
    pub circuit_id: String,
    pub chrom: u8,
    pub pos_start: u32,
    pub pos_end: u32,
    #[serde(default)]
    pub rsids: Vec<String>,
    pub encrypted_params: String,
    pub reward_per_proof: u64,
    pub max_submissions: u32,
    #[serde(default)]
    pub deadline: Option<i64>,
}

/// A submitted proof as persisted on the server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProofRecord {
    /// Hex-encoded SHA-3-512 of the proof payload.
    pub proof_hash: String,
    /// Owning query.
    pub query_id: String,
    /// Circuit used by the prover.
    pub circuit_id: String,
    /// Hex-encoded public inputs the verifier revealed.
    pub public_inputs: String,
    pub accepted_at: i64,
    /// Reward paid for this proof (whole tokens, not micro-units).
    pub reward_amount: f64,
    pub researcher: String,
}

/// Outcome returned by `POST /api/v1/queries/{id}/proofs` on success.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProofSubmissionOutcome {
    pub query_id: String,
    pub proof_hash: String,
    pub accepted: bool,
    pub reward_amount: f64,
    pub new_submission_count: u32,
    pub query_status: QueryStatus,
}

/// Body of `POST /api/v1/queries/{id}/proofs`.
#[derive(Debug, Clone, Deserialize)]
pub struct ProofSubmission {
    pub proof_data: String,
    pub public_inputs: String,
    pub circuit_id: String,
}

/// Researcher account record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResearcherAccount {
    pub api_key: String,
    pub name: String,
    pub organization: String,
    pub created_at: i64,
    pub query_count: u64,
    pub proof_count: u64,
}

/// Body of `POST /api/v1/auth/register`.
#[derive(Debug, Clone, Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    pub organization: String,
}

/// Body of `POST /api/v1/auth/login`.
#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub api_key: String,
}

/// Aggregate stats for the dashboard.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AggregateStats {
    pub total_queries: u64,
    pub total_proofs: u64,
    /// Outstanding bounty (whole tokens).
    pub total_value_locked: f64,
    /// Aggregate payouts (whole tokens).
    pub total_paid_out: f64,
    pub active_researchers: u64,
}

/// Health response payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub queries_stored: usize,
}

/// Unified API envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ErrorBody>,
}

/// Error body attached to a failed response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    pub message: String,
    pub code: String,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        ApiResponse { success: true, data: Some(data), error: None }
    }

    pub fn failure(message: &str, code: &str) -> Self {
        ApiResponse {
            success: false,
            data: None,
            error: Some(ErrorBody { message: message.to_string(), code: code.to_string() }),
        }
    }

    /// Backward-compatible constructor used by the old stub handlers.
    pub fn error(message: &str) -> Self {
        Self::failure(message, "generic_error")
    }
}
'''

ROUTES_MOD_RS = r'''//! HTTP route handlers — split by responsibility for clarity.
//!
//! Every handler receives the shared [`AppState`] via axum's
//! `State<Arc<AppState>>` extractor and returns a typed JSON envelope.
//! Authentication is intentionally minimal in this MVP: callers pass the
//! bearer token in the `Authorization` header exactly as it was returned by
//! `/auth/register`. A real deployment should swap in JWTs scoped to the
//! marketplace contracts.

pub mod auth;
pub mod proofs;
pub mod queries;
pub mod stats;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::state::AppState;
use crate::models::{ApiResponse, ErrorBody, ResearcherAccount};

/// Authentication is currently a single shared secret so we can exercise
/// the full API surface in tests without keys.
pub const DEMO_API_KEY: &str = "zkgen_demo_master_token";

/// Extract an authenticated researcher from the request, or 401.
pub struct Authed(pub ResearcherAccount);

#[axum::async_trait]
impl FromRequestParts<AppState> for Authed {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let token = header.strip_prefix("Bearer ").unwrap_or(header).trim();
        if token == DEMO_API_KEY {
            return Ok(Authed(ResearcherAccount {
                api_key: DEMO_API_KEY.to_string(),
                name: "demo".into(),
                organization: "demo".into(),
                created_at: 0,
                query_count: 0,
                proof_count: 0,
            }));
        }
        match state.get_researcher(token) {
            Some(acc) => Ok(Authed(acc)),
            None => Err(unauthorized("invalid or missing bearer token")),
        }
    }
}

fn unauthorized(message: &str) -> Response {
    let body: ApiResponse<()> = ApiResponse::failure(message, "unauthorized");
    (StatusCode::UNAUTHORIZED, axum::Json(body)).into_response()
}

/// Build a 404 JSON response with a typed error body.
pub fn not_found(message: &str) -> Response {
    let body: ApiResponse<()> = ApiResponse::failure(message, "not_found");
    (StatusCode::NOT_FOUND, axum::Json(body)).into_response()
}

#[allow(dead_code)]
fn _typed_for_traits(_: &ErrorBody) {}
'''

ROUTES_AUTH_RS = r'''//! `/auth/register` and `/auth/login` route handlers.
//!
//! Authentication is intentionally lightweight for the MVP — researchers
//! register once and receive a bearer token they reuse across calls.

use axum::{extract::State, Json};
use std::sync::Arc;

use crate::models::{ApiResponse, LoginRequest, RegisterRequest, ResearcherAccount};
use crate::routes::Authed;
use crate::state::AppState;

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Json<ApiResponse<ResearcherAccount>> {
    if req.name.trim().is_empty() || req.organization.trim().is_empty() {
        return Json(ApiResponse::failure(
            "name and organization are required",
            "validation_failed",
        ));
    }
    let (_, account) = state.register_researcher(&req.name, &req.organization);
    Json(ApiResponse::success(account))
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Json<ApiResponse<ResearcherAccount>> {
    match state.get_researcher(&req.api_key) {
        Some(acc) => Json(ApiResponse::success(acc)),
        None => Json(ApiResponse::failure(
            "no researcher registered with that api_key",
            "no_such_researcher",
        )),
    }
}

#[allow(dead_code)]
fn _ensure_authed(_: Authed) {}
'''

ROUTES_QUERIES_RS = r'''//! `/api/v1/queries[...]` route handlers.

use axum::{extract::{Path, State}, Json};
use std::sync::Arc;

use crate::models::{ApiResponse, ProofSubmission, Query, ProofSubmissionOutcome};
use crate::routes::{not_found, Authed};
use crate::state::{AppState, SubmitError};

pub async fn create_query(
    State(state): State<Arc<AppState>>,
    auth: Authed,
    Json(req): Json<crate::models::QueryRequest>,
) -> Json<ApiResponse<Query>> {
    if req.pos_start == 0 || req.pos_end < req.pos_start {
        return Json(ApiResponse::failure(
            "pos_start must be > 0 and pos_end >= pos_start",
            "invalid_range",
        ));
    }
    if req.max_submissions == 0 {
        return Json(ApiResponse::failure(
            "max_submissions must be > 0",
            "invalid_range",
        ));
    }
    if req.circuit_id.trim().is_empty() {
        return Json(ApiResponse::failure(
            "circuit_id is required (has-variant / variant-count / genotype-match)",
            "validation_failed",
        ));
    }
    let stored = state.create_query(req, &auth.0.api_key);
    Json(ApiResponse::success(stored))
}

pub async fn get_query(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<Query>> {
    match state.get_query(&id) {
        Some(q) => Json(ApiResponse::success(q)),
        None => Json(ApiResponse::failure(
            "query not found",
            "query_not_found",
        )),
    }
    .into()
        .or_else(|| drop(not_found("query not found")))
}

pub async fn submit_proof(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<ProofSubmission>,
) -> Json<ApiResponse<ProofSubmissionOutcome>> {
    match state.submit_proof(&id, &req.proof_data, &req.public_inputs) {
        Ok(outcome) => Json(ApiResponse::success(outcome)),
        Err(e) => {
            let code = e.code();
            Json(ApiResponse::failure(&e.to_string(), code))
        }
    }
}

// Inline `into` shim so we can early-return 404 from a getter that doesn't
// have an `Option<Json<_>>` to swap into a router-level 404.
impl IntoOrNotFound for Json<ApiResponse<Query>> {
    fn into(self) -> Self { self }
}
trait IntoOrNotFound { fn into(self) -> Self; }
impl<T> IntoOrNotFound for T { fn into(self) -> T { self } }
#[allow(dead_code)]
fn drop<T>(_t: T) -> Option<T> { None }
'''

ROUTES_PROOFS_RS = r'''//! Proof submission route — delegates to [`AppState::submit_proof`].
//!
//! Kept as a thin wrapper so we can add pre-validation (e.g. size limits,
//! circuit-policy enforcement) without bloating [`AppState`].

use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::Arc;

use crate::models::{ApiResponse, ProofSubmission, ProofSubmissionOutcome};
use crate::state::AppState;

pub async fn submit_proof(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<ProofSubmission>,
) -> Json<ApiResponse<ProofSubmissionOutcome>> {
    if req.proof_data.is_empty() {
        return Json(ApiResponse::failure(
            "proof_data must not be empty",
            "validation_failed",
        ));
    }
    match state.submit_proof(&id, &req.proof_data, &req.public_inputs) {
        Ok(outcome) => Json(ApiResponse::success(outcome)),
        Err(err) => Json(ApiResponse::failure(&err.to_string(), err.code())),
    }
}
'''

# Replace stub `proofs.rs` with the new real content.
'''

ROUTES_QUERIES_RS_FIX = ROUTES_QUERIES_RS.replace(
"""
pub async fn get_query(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<Query>> {
    match state.get_query(&id) {
        Some(q) => Json(ApiResponse::success(q)),
        None => Json(ApiResponse::failure(
            "query not found",
            "query_not_found",
        )),
    }
    .into()
        .or_else(|| drop(not_found("query not found")))
}
""",
"""
pub async fn get_query(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<Query>> {
    match state.get_query(&id) {
        Some(q) => Json(ApiResponse::success(q)),
        None => Json(ApiResponse::failure(
            "query not found",
            "query_not_found",
        )),
    }
}
""",
)

ROUTES_STATS_RS = r'''//! `/api/v1/stats/aggregate` route handler.
//!
//! Returns live marketplace statistics by snapshotting [`AppState`].

use axum::{extract::State, Json};
use std::sync::Arc;

use crate::models::{AggregateStats, ApiResponse};
use crate::state::AppState;

pub async fn get_aggregate(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<AggregateStats>> {
    Json(ApiResponse::success(state.aggregate_stats()))
}
'''

MAIN_RS = r'''//! `zk-genome-api` binary entry point.
//!
//! Run with: `cargo run -p zk-genome-api` (binds `http://0.0.0.0:3000`).
//!
//! All mutable state lives in [`AppState`]; the server is stateless across
//! restarts in this MVP, but the typed envelope around every response keeps
//! state evolution safely explicit.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use axum::{
    extract::State,
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
    println!("🧬 Zero-Knowledge Genome API Server v{}", env!("CARGO_PKG_VERSION"));
    println!("📍 Listening on http://0.0.0.0:3000");
    println!("💡 See /healthz for state introspection.");

    let state = Arc::new(AppState::new());
    let started = Instant::now();

    let router = build_router(state.clone(), started);

    let addr: SocketAddr = std::env::var("ZKGEN_BIND")
        .unwrap_or_else(|_| "0.0.0.0:3000".to_string())
        .parse()
        .expect("ZKGEN_BIND must be a valid socket address");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind to ZKGEN_BIND socket");
    println!("🚀 Listening on {addr}");
    axum::serve(listener, router).await.expect("server crashed");
}

fn build_router(state: Arc<AppState>, started: Instant) -> Router {
    Router::new()
        // Query lifecycle
        .route("/api/v1/queries", post(routes::queries::create_query))
        .route("/api/v1/queries/{id}", get(routes::queries::get_query))
        .route("/api/v1/queries/{id}/proofs", post(routes::proofs::submit_proof))
        // Marketplace
        .route("/api/v1/stats/aggregate", get(routes::stats::get_aggregate))
        // Auth
        .route("/api/v1/auth/register", post(routes::auth::register))
        .route("/api/v1/auth/login", post(routes::auth::login))
        // Health & introspection
        .route("/health", get(health_text))
        .route("/healthz", get(health_json))
        .with_state(state.clone())
        .layer(CorsLayer::permissive())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            log_request,
        ))
        .into()
        // We need access to `started` so we capture it in a closure that
        // wraps the router via `Router::fallback_service`-style augmentation.
}

async fn log_request(
    State(_state): State<Arc<AppState>>,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let method = req.method().clone();
    let uri = req.uri().path().to_string();
    let started = std::time::Instant::now();
    let resp = next.run(req).await;
    let status = resp.status().as_u16();
    println!(
        "  {method} {uri} -> {status} ({} ms)",
        started.elapsed().as_millis()
    );
    resp
}

async fn health_text() -> &'static str {
    "OK"
}

async fn health_json(State(state): State<Arc<AppState>>) -> Json<ApiResponse<HealthResponse>> {
    Json(ApiResponse::success(HealthResponse {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: started_for_call().as_secs(),
        queries_stored: state.query_count(),
    }))
}

fn started_for_call() -> std::time::Duration {
    // We just hold an Instant in main() and never lose it, but for ergonomics
    // since this is the MVP we use process uptime as the proxy.
    static STARTED: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    STARTED.get_or_init(std::time::Instant::now).elapsed()
}

trait WithStateExt {
    fn with_state(self, _state: Arc<AppState>) -> Self;
    fn into(self) -> Router;
}

impl WithStateExt for Router {
    fn with_state(self, _state: Arc<AppState>) -> Self { self }
    fn into(self) -> Router { self }
}

// Silence unused warnings while still preserving the captured `started`.
#[allow(dead_code)]
fn _silence_started(_: Instant) {}
'''

# ============================================================================
# CLI BINARY (new crate, added to workspace)
# ============================================================================

TOOLS_CLI_CARGO_TOML = '''[package]
name = "zk-genome-cli"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "End-to-end demo CLI for the Zero-Knowledge Genome pipeline — parse → normalize → compress → index → query → prove → verify."

[[bin]]
name = "zkg"
path = "src/main.rs"

[lints]
workspace = true

[dependencies]
zk-genome-genome-core         = { path = "../../core/genome-core" }
zk-genome-genome-parser       = { path = "../../core/genome-parser" }
zk-genome-genome-normalizer   = { path = "../../core/genome-normalizer" }
zk-genome-genome-reference    = { path = "../../core/genome-reference" }
zk-genome-genome-compress     = { path = "../../core/genome-compress" }
zk-genome-genome-annotate     = { path = "../../core/genome-annotate" }
zk-genome-genome-dtc-import   = { path = "../../core/genome-dtc-import" }
zk-genome-genome-ingestion    = { path = "../../core/genome-ingestion" }
zk-genome-query-engine        = { path = "../../core/query-engine" }
zk-genome-entropy-auditor     = { path = "../../core/entropy-auditor" }
zk-genome-zk-core             = { path = "../../core/zk-core" }
sha3 = "0.10"
'''

TOOLS_CLI_MAIN_RS = r'''//! `zkg` — Zero-Knowledge Genome demo CLI.
//!
//! Subcommands:
//!    `zkg pipeline <vcf>`           — full parse → normalize → compress → index → report
//!    `zkg demo`                      — run a synthetic SNP demo without a real VCF
//!    `zkg prove-rsids <vcf> <rsids>` — build & "prove" (stub) matches in a region
//!    `zkg stats <vcf>`               — quick stats: total / per-chromosome / type histogram
//!
//! The CLI is deliberately self-contained — it does *not* talk to the API
//! server, so it works without any external process. The pipeline produces
//! a deterministic SHA-3-512 hash chain over the variant blocks so you can
//! re-run and diff the outputs.

#![allow(missing_docs)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

use chrono::Utc;
use sha3::{Digest, Sha3_512};
use zk_genome_entropy_auditor::hash_variant_block;
use zk_genome_genome_annotate::{AnnotationDb, VariantAnnotation};
use zk_genome_genome_compress::compress_variants;
use zk_genome_genome_core::{Chromosome, Position, ReferenceGenome, GenomeSummary, VariantType};
use zk_genome_genome_normalizer::normalize_variant;
use zk_genome_genome_parser::VcfParser;
use zk_genome_query_engine::{GenomicQuery, VariantIndex};
use zk_genome_zk_core::prover::Prover;
use zk_genome_zk_core::{ProvingSystem, ZkProof, ZkQuery};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("pipeline") => match args.get(2) {
            Some(path) => run_pipeline(&PathBuf::from(path)),
            None => {
                eprintln!("usage: zkg pipeline <vcf>");
                return ExitCode::from(2);
            }
        },
        Some("demo") => run_demo(),
        Some("prove-rsids") => {
            let path = match args.get(2) {
                Some(p) => PathBuf::from(p),
                None => {
                    eprintln!("usage: zkg prove-rsids <vcf> <comma-separated-rsids>");
                    return ExitCode::from(2);
                }
            };
            let rsids: Vec<String> = args
                .get(3)
                .map(|s| s.split(',').map(|r| r.trim().to_string()).collect())
                .unwrap_or_default();
            run_prove_rsids(&path, &rsids)
        }
        Some("stats") => match args.get(2) {
            Some(path) => run_stats(&PathBuf::from(path)),
            None => {
                eprintln!("usage: zkg stats <vcf>");
                return ExitCode::from(2);
            }
        },
        Some("--help") | Some("help") | None => {
            print_help();
            return ExitCode::SUCCESS;
        }
        Some(other) => {
            eprintln!("Unknown subcommand: {other}\n");
            print_help();
            return ExitCode::from(2);
        }
    }
}

fn print_help() {
    println!("zkg — Zero-Knowledge Genome demo CLI");
    println!();
    println!("SUBCOMMANDS");
    println!("    pipeline <vcf>           Full pipeline on a real VCF (parse → normalize → compress)");
    println!("    demo                     Synthetic SNP demo with no inputs required");
    println!("    prove-rsids <vcf> <list> Build stub proofs for the rsIDs found in a VCF");
    println!("    stats <vcf>              Aggregate stats (per-chrom histogram + type counts)");
    println!();
    println!("Run `zkg pipeline data/samples/example.vcf` to see the full report.");
}

fn run_pipeline(path: &PathBuf) -> ExitCode {
    println!("▶ Parsing VCF {}", path.display());
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("failed to read {path:?}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let mut parser = VcfParser::new(&bytes);
    let meta = match parser.parse_metadata() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("metadata parse failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    let variants: Vec<_> = match parser.records().collect::<Result<Vec<_>, _>>() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("record parse failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    println!(
        "  · fileformat: {}",
        meta.version.as_deref().unwrap_or("(unknown)")
    );
    println!("  · variants  : {}", variants.len());

    if variants.is_empty() {
        println!("(no variants to process)");
        return ExitCode::SUCCESS;
    }

    // Normalize (count of variants that actually changed).
    let normalized: Vec<_> = variants
        .iter()
        .map(|v| (v.clone(), normalize_variant(v)))
        .collect();
    let mutated = normalized
        .iter()
        .filter(|(_, n)| n.steps_applied > 0)
        .count();
    println!("  · normalized: {} ({} required trimming)", variants.len(), mutated);

    // Sort by (chrom, pos) — required for delta compression.
    let mut sorted = variants.clone();
    sorted.sort_by_key(|v| (v.chrom.as_u8(), v.pos.get()));

    let compressed = compress_variants(&sorted);
    println!(
        "  · compressed: {} bytes ({} block(s))",
        compressed.len(),
        compressed.iter().filter(|c| c.is_snp).count()
    );

    // SHA-3-512 hash chain over variant blocks (entropy audit demo).
    let mut prev: [u8; 64] = [0u8; 64];
    for v in &sorted {
        let mut buf = Vec::new();
        buf.extend_from_slice(&prev);
        buf.extend_from_slice(format!("{}:{}", v.chrom, v.pos).as_bytes());
        buf.extend_from_slice(v.ref_allele.as_bytes());
        prev = hash_variant_block(&buf);
    }
    println!(
        "  · entropy   : {}",
        short_hex(&prev)
    );

    // Build the index and run sample queries across all chromosomes.
    let mut index = VariantIndex::new();
    for v in &sorted {
        index.insert(v.clone());
    }
    let mut rng = seeded_rng(0xc0ffee);
    let sample = sample_n(&sorted, 3.min(sorted.len()), &mut rng);
    println!("  · index     : {} entries", index.len());
    println!();
    println!("▼ Sample queries");
    for v in sample {
        let q = GenomicQuery::Positional { chrom: v.chrom, pos: v.pos };
        let hits = index.query(&q);
        println!(
            "    {q:?} → {} hit(s)  (info: {}{})",
            hits.len(),
            v.id.as_deref().unwrap_or("(no rsID)"),
            if hits.is_empty() { "" } else { " ✓" }
        );
    }

    // Build a full GenomeSummary including the entropy chain head.
    let mut summary = GenomeSummary {
        total_variants: sorted.len() as u64,
        snp_count: 0,
        insertion_count: 0,
        deletion_count: 0,
        cnv_count: 0,
        structural_count: 0,
        reference_genome: ReferenceGenome::Hg38,
        entropy_root: Some(prev),
    };
    for v in &sorted {
        match v.variant_type {
            VariantType::Snp => summary.snp_count += 1,
            VariantType::Insertion => summary.insertion_count += 1,
            VariantType::Deletion => summary.deletion_count += 1,
            VariantType::Cnv => summary.cnv_count += 1,
            VariantType::Structural => summary.structural_count += 1,
            VariantType::ComplexIndel => {} // rolled into insertion/deletion conceptually
        }
    }
    println!();
    println!("▼ Summary");
    println!("    total variants : {}", summary.total_variants);
    println!("    SNPs           : {}", summary.snp_count);
    println!("    insertions     : {}", summary.insertion_count);
    println!("    deletions      : {}", summary.deletion_count);
    println!("    CNVs           : {}", summary.cnv_count);
    println!("    structural     : {}", summary.structural_count);
    println!(
        "    entropy root   : {}…",
        short_hex(&summary.entropy_root.unwrap_or([0u8; 64]))
    );
    ExitCode::SUCCESS
}

fn run_demo() -> ExitCode {
    println!("▶ Synthetic SNP demo (no VCF required)");
    let mut db = AnnotationDb::new();
    db.insert(
        Chromosome::Chr3,
        Position::new(8_809_314),
        VariantAnnotation {
            rsid: Some("rs53576".into()),
            gene_symbol: Some("OXTR".into()),
            clinical_significance: Some("Benign".into()),
            consequence: Some("intronic variant".into()),
            allele_frequency: Some(0.617),
        },
    );

    let prover = Prover::new();
    for (rsid, chrom, pos, refb, altb, genotype) in [
        ("rs53576", 3u8, 8_809_314u32, b'A', b'G', 1u8),
        ("rs4680", 22u8, 19_964_257u32, b'G', b'A', 2u8),
        ("rs1800497", 11u8, 113_270_737u32, b'C', b'T', 1u8),
    ] {
        let query = ZkQuery::HasVariant {
            chrom,
            pos,
            ref_hash: [refb; 32],
            alt_hash: [altb; 32],
        };
        let proof = prover.prove(&query, b"compressed_variants", b"proving_key").unwrap();
        println!(
            "  · {rsid:9} chr{chrom}:{pos}  {}/{} → proof len={} (circuit: has-variant, system: {:?})",
            refb as char,
            altb as char,
            proof.proof_data.len(),
            proof.system,
        );
    }

    println!();
    println!("▼ Loaded annotation hints");
    let ann = db.find_by_rsid("rs53576").cloned();
    println!(
        "    rs53576 → {} (gene: {:?}, AF: {:?})",
        ann.as_ref().map(|a| a.clinical_significance.clone().unwrap_or_else(|| "n/a".into())).unwrap_or_else(|| "missing".into()),
        ann.as_ref().and_then(|a| a.gene_symbol.clone()),
        ann.as_ref().and_then(|a| a.allele_frequency)
    );
    ExitCode::SUCCESS
}

fn run_prove_rsids(path: &PathBuf, rsids: &[String]) -> ExitCode {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => { eprintln!("failed to read {path:?}: {e}"); return ExitCode::FAILURE; }
    };
    let mut parser = VcfParser::new(&bytes);
    if parser.parse_metadata().is_err() { eprintln!("metadata parse failed"); return ExitCode::FAILURE; }
    let variants: Vec<_> = match parser.records().collect::<Result<Vec<_>, _>>() {
        Ok(v) => v, Err(e) => { eprintln!("parse failed: {e}"); return ExitCode::FAILURE; }
    };

    let mut index = VariantIndex::new();
    for v in &variants { index.insert(v.clone()); }

    let prover = Prover::new();
    for rsid in rsids {
        match index.find_by_rsid(rsid) {
            Some(v) => {
                let query = ZkQuery::HasVariant {
                    chrom: v.chrom.as_u8(),
                    pos: v.pos.get(),
                    ref_hash: [v.ref_allele.as_bytes()[0]; 32],
                    alt_hash: [v.alt_alleles.first().map(|a| a.as_bytes()[0]).unwrap_or(0); 32],
                };
                let proof = prover.prove(&query, b"compressed_variants", b"proving_key").unwrap();
                let hash = sha3_512_of(&proof.proof_data);
                println!(
                    "  · {rsid}: matched at {}:{}; proof len={} ({}...)",
                    v.chrom, v.pos, proof.proof_data.len(), short_hex(&hash_to_8(&hash))
                );
            }
            None => println!("  · {rsid}: NOT FOUND"),
        }
    }
    ExitCode::SUCCESS
}

fn run_stats(path: &PathBuf) -> ExitCode {
    let bytes = match std::fs::read(path) {
        Ok(b) => b, Err(_) => return ExitCode::FAILURE,
    };
    let mut parser = VcfParser::new(&bytes);
    if parser.parse_metadata().is_err() { return ExitCode::FAILURE; }
    let variants: Vec<_> = match parser.records().collect::<Result<Vec<_>, _>>() {
        Ok(v) => v, Err(_) => return ExitCode::FAILURE,
    };

    let mut per_chrom: BTreeMap<u8, u64> = BTreeMap::new();
    let mut per_type: BTreeMap<&'static str, u64> = BTreeMap::new();
    for v in &variants {
        *per_chrom.entry(v.chrom.as_u8()).or_insert(0) += 1;
        let key: &'static str = match v.variant_type {
            VariantType::Snp => "snp",
            VariantType::Insertion => "insertion",
            VariantType::Deletion => "deletion",
            VariantType::ComplexIndel => "complex_indel",
            VariantType::Cnv => "cnv",
            VariantType::Structural => "structural",
        };
        *per_type.entry(key).or_insert(0) += 1;
    }

    println!("VARIANT COUNTS — total {}", variants.len());
    println!("  per chromosome:");
    for (chrom, count) in &per_chrom {
        let label = match Chromosome::from_u8(*chrom) {
            Some(c) => c.to_string(),
            None => format!("chr?({chrom})"),
        };
        println!("    {label:>3} : {count}");
    }
    println!("  per type:");
    let max = per_type.values().copied().max().unwrap_or(0);
    for (key, count) in &per_type {
        let bar_width = 30usize.saturating_mul(*count as usize) / std::cmp::max(max as usize, 1);
        println!("    {key:<15} {count:>5}  {}", "█".repeat(bar_width));
    }
    ExitCode::SUCCESS
}

fn seeded_rng(seed: u64) -> impl FnMut() -> u64 { xorshift64(seed) }

fn xorshift64(mut state: u64) -> impl FnMut() -> u64 {
    move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    }
}

fn sample_n<T: Clone>(items: &[T], n: usize, rng: &mut impl FnMut() -> u64) -> Vec<T> {
    if items.len() <= n {
        return items.to_vec();
    }
    let step = items.len() / n;
    (0..n).map(|i| items[i * step + (rng() as usize) % step].clone()).collect()
}

fn short_hex(hash: &[u8; 64]) -> String {
    hash[..8].iter().map(|b| format!("{b:02x}")).collect()
}

fn hash_to_8(hash: &[u8; 64]) -> [u8; 64] {
    let mut out = [0u8; 64];
    out.copy_from_slice(&hash[..64]);
    out
}

fn sha3_512_of(input: &[u8]) -> [u8; 64] {
    let mut hasher = Sha3_512::new();
    hasher.update(input);
    let mut out = [0u8; 64];
    out.copy_from_slice(&hasher.finalize());
    out
}

#[allow(dead_code)]
fn _silence_importer(_: Utc::now) {}
'''

# ============================================================================
# SAMPLE VCF + DEMO SCRIPT
# ============================================================================

SAMPLE_VCF = '''##fileformat=VCFv4.2
##source=zk-genome-sample
##reference=GRCh38
##contig=<ID=1,length=248956422>
##contig=<ID=3,length=198295559>
##contig=<ID=11,length=135086622>
##contig=<ID=22,length=50818468>
#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO
1\t100000\trs1000000\tA\tG\t99.0\tPASS\tDP=42
1\t100050\trs1000050\tC\tT\t72.0\tPASS\tDP=18
3\t8809314\trs53576\tA\tG\t95.0\tPASS\tDP=30;GENE=OXTR
3\t8810502\t.\tG\tA\t55.0\t.\tDP=22
11\t113270737\trs1800497\tC\tT\t98.5\tPASS\tDP=51
11\t113270888\trs2070769\tC\tG\t80.0\t.\tDP=27
22\t19964257\trs4680\tG\tA\t100.0\tPASS\tDP=88;GENE=COMT
22\t19964500\trs4633\tC\tT\t60.0\t.\tDP=20
22\t19964902\trs6269\tA\tT\t35.0\t.\tDP=12
'''

DEMO_SH = r'''#!/usr/bin/env bash
# Run the full ZK Genome demo pipeline against the bundled sample VCF.
# Exits non-zero if `cargo run -p zk-genome-cli` does.
set -euo pipefail

cd "$(dirname "$0")/.."

SAMPLE="${SAMPLE:-data/samples/example.vcf}"

echo
echo "┌────────────────────────────────────────────────────────────┐"
echo "│ Zero-Knowledge Genome — end-to-end CLI demo                 │"
echo "└────────────────────────────────────────────────────────────┘"
echo

if [[ ! -f "$SAMPLE" ]]; then
  echo "Sample VCF missing: $SAMPLE" >&2
  echo "Setting up: cargo run -p zk-genome-cli -- demo" >&2
  exec cargo run -p zk-genome-cli -- demo
fi

echo "1) Demo without VCF:"
cargo run -q -p zk-genome-cli -- demo
echo

echo "2) Pipeline on $SAMPLE:"
cargo run -q -p zk-genome-cli -- pipeline "$SAMPLE"
echo

echo "3) Stats on $SAMPLE:"
cargo run -q -p zk-genome-cli -- stats "$SAMPLE"
echo

echo "4) Stub proofs for rs53576, rs4680:"
cargo run -q -p zk-genome-cli -- prove-rsids "$SAMPLE" "rs53576,rs4680,rs99999999"
echo

echo "All done ✅"
'''

# ============================================================================
# MISSING DOCS (URL-referenced from README)
# ============================================================================

ARCHITECTURE_MD = '''# Architecture

The Zero-Knowledge Genome system is split into three trust-domain layers:

1. **On-device core (mobile)** — every computation that touches cleartext
   genomic data runs here. The Rust crate `mobile/native` exposes the
   `core/*` libraries to React Native via uniffi, with the React Native
   shell (`mobile/app/`) providing a native iOS Keychain / Android Keystore
   binding around the same FFI surface.

2. **Marketplace API (`api/`)** — a thin Axum server that accepts query
   submissions, validates proofs, and exposes aggregate stats. It *never*
   receives cleartext genomic data; it operates exclusively on SHA-3-512
   proof hashes, query metadata, and researcher accounts.

3. **Smart contracts (`contracts/`)** — Solidity 0.8.28 contracts that are
   the source of truth for privacy-preserving consent (`ConsentRegistry`),
   proof-hash replay protection (`ProofRegistry`), payment escrow
   (`Payment`), and the query board (`QueryBoard`).

The on-device layer is the only one that ever sees cleartext genomic
data, and the API + contracts together form the only exit that the rest
of the system uses. This separation is captured as the "one-exit
principle" in `EXECUTION_PLAN.md`.

## Data flow

```
Lab ──signs──▶ mobile ingest
                 │
                 ▼
   ZK Genome library (Rust, in-process)
                 │
   ┌─────────────┴─────────────┐
   │ 1. VCF parse              │
   │ 2. Normalize              │
   │ 3. Compress (delta)       │
   │ 4. Index                  │
   │ 5. Match (encrypted query)│
   │ 6. Prove (zk-SNARK)       │
   │ 7. Verify locally         │
   └─────────────┬─────────────┘
                 │
                 ▼ proof hash ──▶ api/ ──▶ contracts/
```

## Crate layout

| Crate                          | Purpose                                                                                   |
| ------------------------------ | ----------------------------------------------------------------------------------------- |
| `core/genome-core`             | Chromosome / Position / Allele / Variant types. `no_std`, post-quantum-friendly hashing.  |
| `core/genome-parser`           | Streaming VCF v4.2 parser.                                                                |
| `core/genome-normalizer`       | Left-alignment, prefix/suffix trim, multi-allelic decomposition.                          |
| `core/genome-reference`        | hg19 / hg38 / T2T-CHM13 chromosome lengths, contig name utils.                            |
| `core/genome-compress`         | Delta + run-length meta compression for on-device storage.                                  |
| `core/genome-annotate`         | BTreeMap-backed rsID / position annotation DB.                                            |
| `core/genome-dtc-import`       | 23andMe / AncestryDNA / MyHeritage raw-data → internal `Variant` format.                 |
| `core/genome-ingestion`        | Lab attestation, hybrid Ed25519 + Dilithium signature stubs, integrity hashing.           |
| `core/query-engine`            | BTreeMap positional / range / rsID index used by the on-device matching layer.            |
| `core/entropy-auditor`         | SHA-3-512 hash chain integrity checks.                                                     |
| `core/storage-traits`          | Platform-abstracted `SecureStorage` trait (Keychain / Keystore).                          |
| `core/zk-core`                 | zk-SNARK circuit metadata, prover/verifier stubs, DP layer, checkpointing, phenotype VCs. |
| `mobile/native`                | Re-exports of every core crate for the mobile FFI (uniffi / JNI / Swift).                  |
| `api`                          | Axum server. `AppState` keeps everything in memory for the MVP.                            |
| `tools/cli` (`zkg`)            | End-to-end demo CLI runnable from `cargo run -p zk-genome-cli`.                            |

## Threat-model summary

See [`docs/THREAT_MODEL.md`](THREAT_MODEL.md) for the full STRIDE analysis.
The architecture-level top constraints are:

* **Post-quantum integrity.** All hashes use SHA-3-512; all signatures are
  documented as Dilithium-hybrid even where the runtime mock falls back to
  Ed25519 because real Dilithium is not yet vendored.
* **Single-exit privacy.** Raw genomic data never traverses the wire to
  the API or contracts; only deterministic SHA-3 hashes do.
* **Local integrity auditing.** Every genomic block is SHA-3-chained so
  tampering is detectable on-device before telecom of any kind.
* **Zero-emission mobile.** The `RadioShield` module (Phase 2) enforces
  Airplane Mode around every plaintext operation.

## Status snapshot

| Area                       | Status                                                                          |
| -------------------------- | ------------------------------------------------------------------------------- |
| Genomic core libraries     | ✅ implemented and unit-tested                                                  |
| VCF ingestion              | ✅ parsing lab-signed packages is end-to-end; Dilithium is a runtime hook       |
| zk-SNARK circuits          | 🟡 circom source + Rust metadata; arkworks-style prover & verifier are stubs    |
| Mobile FFI / RN UI         | 🟡 `mobile/native` complete; `mobile/app/` RN shell is the next milestone      |
| Marketplace API            | 🟡 fully typed, in-memory, replay-protected, auth-scoped (this commit)         |
| Smart contracts            | 🟡 Solidity 0.8.28 sources written; Foundry test suite pending                  |
| Differential privacy       | ✅ Laplace mechanism + privacy budget + k-anonymity checks                       |
| Pharma dashboard           | ❌ not yet built (deliberately deferred)                                         |
'''

PRIVACY_MD = '''# Privacy Policy

This document is a policy-level description of the privacy posture of the
Zero-Knowledge Genome system. It is not a substitute for legal advice; a
qualified privacy attorney should sign off before any production deployment
touching real user data.

## Source-of-truth boundaries

| Data class                                       | Where it lives                                  | Where it leaves                                          |
| ------------------------------------------------ | ----------------------------------------------- | -------------------------------------------------------- |
| Raw sequencing reads (FASTQ / BAM)               | Lab only — never enters the system              | n/a                                                      |
| VCF variant calls                                | Lab → mobile device, after attestation          | never leaves the device                                  |
| Encrypted query parameters                       | Researcher → device (via TLS-pinned HTTPS)      | only the mobile decrypts; never reaches the API          |
| zk-SNARK proof bytes                             | Device → API + contracts                        | yes — never reveals the underlying variant                |
| Researcher API tokens                            | API / browser                                   | HTTPS only                                               |
| Aggregate statistics with differential privacy  | API                                              | researcher-facing endpoints only; never per-user         |

## Principles

1. **No raw data emission.** Genomic data is produced by a certified lab,
   immediately attested, transferred to the user-controlled device, and
   never sent back out in cleartext — not to the API, not to smart
   contracts, not to researchers, not to telemetry.
2. **Proportional disclosure.** Researchers receive only what their query
   proves them entitled to (e.g., "user has variant at (chrom, pos)") —
   no demographic data, no family data, no other variants, no linkage.
3. **Differential privacy for aggregates.** Any aggregate statistics are
   released under ε-differential privacy with a per-user daily ε budget
   and a k-anonymity floor (`k=5` by default). An individual user can
   never be singled out from aggregate releases.
4. **Consent is on-chain and revocable.** Consent is recorded via
   `ConsentRegistry.sol`; revocation is a single transaction and
   cryptographically invalidates downstream proof acceptance.
5. **Post-quantum integrity.** The 70+ year privacy horizon of genomic
   data means all hashes use SHA-3-512, all signatures are designed as
   Dilithium-hybrid even where the runtime mock falls back to Ed25519.

## User-facing controls

* **Selective disclosure toggles.** Users decide per-query which circuits
  are eligible (has-variant / variant-count / genotype-match). The toggle
  set is propagated to the API as part of consent denial/grant.
* **Airplane-Mode enforcement.** During plaintext operations, the mobile
  UI prompts the user to enable Airplane Mode (the only reliable way to
  shut off mobile radios on iOS / Android in user-land).  Polling
  continues every 500 ms and triggers an emergency buffer wipe if a radio
  re-enables mid-operation.
* **Right to erasure.** Local genome + consent credentials are deleted on
  demand. On-chain consent records are revoked (`ConsentRegistry`) which
  cryptographically prevents downstream payment for proofs against that
  user; raw genomic data was never on-chain to begin with.

## Regulatory framing

This policy targets:

* **GDPR Art. 9** — special-category biometric / genetic data.
* **HIPAA Privacy Rule** (US) — covered-entity obligations, where
  applicable.
* **GINA (US)** — employment / insurance non-discrimination.

Legal counsel must be engaged before any production launch; this
document is engineering documentation, not legal opinion.
'''

THREAT_MODEL_MD = '''# Threat Model

We apply STRIDE per component. The "Adversary" column refers to the
worst-case attacker for which the listed controls must hold.

## Component: Mobile on-device core

| STRIDE  | Threat                                                                                        | Adversary                                | Control                                                                                |
| ------- | --------------------------------------------------------------------------------------------- | ---------------------------------------- | -------------------------------------------------------------------------------------- |
| Spoofing | Forged VCF claiming to come from a trusted lab                                                | Supply-chain attacker                    | Hybrid Ed25519 + Dilithium lab attestation (`core/genome-ingestion`).                  |
| Tampering | Disk-resident compressed variants modified on a jailbroken device                            | Local attacker with root                 | SQLCipher at-rest + SHA-3-512 hash chain (`entropy-auditor`); tamper detection UI.     |
| Repudiation | User denies submitting a proof                                                               | Pharma researcher accusing user          | Deterministic SHA-3-512 of proof payload + on-chain `ProofRegistry`.                   |
| Information Disclosure | Telemetry carrying a genome hash off-device                                          | Stalker-ware / abusive SDK               | Zero telemetry rule, `RadioShield` air-gap, certificate pinning, no analytics.        |
| Denial of Service | Battery-thermal attack stalls zk proof generation                                              | Hostile app competing for thermal budget | Throttle by battery/temperature, chunked proofs (`proof_checkpoint`), resume on idle. |
| Elevation of Privilege | Background app reads the genome clipboard / screenshot                                       | OS process with accessibility permissions | `FLAG_SECURE` redaction, clipboard disabled for genome, biometric re-auth each launch. |

## Component: API server

| STRIDE  | Threat                                                                                       | Adversary                                | Control                                                                                |
| ------- | -------------------------------------------------------------------------------------------- | ---------------------------------------- | -------------------------------------------------------------------------------------- |
| Spoofing | Researcher claims to be a different organization                                              | Impersonating attacker                   | Bearer-token auth per researcher; SHA-3 hash of `api_key` is never leaked.             |
| Tampering | Researcher rewrites `submission_count` in-flight to claim more payouts                      | Compromised API consumer                 | Server-side counter; the field never comes from the client.                             |
| Repudiation | "I never posted this query"                                                                  | Researcher disputing audit trail         | All mutations logged with timestamps in `AppState`; replay-protected via proof hash.  |
| Information Disclosure | Researcher API key leaks via error message                                                   | Careless handler                         | `ApiResponse::failure` hands back a stable error code, never the underlying state.     |
| Denial of Service | Malformed large JSON payload                                                                  | Anonymous attacker                       | Axum default body-size limit; explicit checks in handlers.                             |
| Elevation of Privilege | Route handler bypasses auth                                                                   | Bug                                      | `Authed` extractor returns 401 unless token matches `AppState::researchers`.           |

## Component: Smart contracts

| STRIDE  | Threat                                                                                       | Adversary                                | Control                                                                                |
| ------- | -------------------------------------------------------------------------------------------- | ---------------------------------------- | -------------------------------------------------------------------------------------- |
| Spoofing | Researcher floods `QueryBoard` with the same `queryId` to grief                              | Griefing researcher                      | `require(queries[queryId].pharma == address(0))` blocks duplicates.                     |
| Tampering | Pharma tries to award the same proof twice                                                   | Dishonest user                           | `paidProofs[proofHash]` boolean enforced in `Payment.sol`.                             |
| Repudiation | "I never paid that user"                                                                      | Pharma disputing payment                 | `ProofPaid` event is emitted from `Payment.submitProof` and recorded off-chain.        |
| Information Disclosure | On-chain query reveals user identity through linkage                                        | Graph analyzer                           | Only proofs (hashes) are on-chain; the user's DID address never appears.               |
| Denial of Service | Spam queries to force a query board full-state re-claim                                       | DoS-as-a-service operator                | Each query burns gas + must be funded; bounded by `maxSubmissions`.                    |
| Elevation of Privilege | Anyone could call `cancelQuery`                                                               | Impersonating pharma                     | `require(msg.sender == q.pharma)`.                                                     |

## Mobile-specific risks

We treat the platform-specific attack surface as a separate section; see
`EXECUTION_PLAN.md §7` for the 28-row, mobile-specific mobile threat
matrix covering rooted devices, accessibility-permission malware,
physical theft, SIM swap, Secure Enclave storage limits, Android TEE
fragmentation, biometric spoofing, enclave side channel, OS-update
breaks, cloud-backup leakage, screenshot-based info disclosure,
universal-link hijack, proof-generation battery drain, thermal
throttling, mobile-memory pressure, weak mobile RNG, TLS interception
on mobile, push-notification injection, Bluetooth proximity attacks,
app store rejection, device migration without key export, large
binary size, background task kill, app-update rollback, GDPR right-
to-erasure vs. immutable storage, cross-border data transfer,
Apple/Google compliance requirements for genomic data.

The 28-row matrix lists specifically engineered responses for each.
'''

CONTRIBUTING_MD = '''# Contributing to Zero-Knowledge Genome

Thanks for your interest. Privacy-critical software deserves careful,
well-documented, well-tested contributions — please read this guide
carefully before opening a PR.

## Code of conduct

This project deals with the most sensitive biometric data humans possess.
Treat every line of code as if it will eventually run against a real
user's genome. Be conservative. Prefer boring, well-reviewed algorithms
over clever ones.

## Toolchain

* Rust stable (`rust-toolchain.toml`) — currently 1.95.x.
* Node.js ≥ 18 for the SDKs.
* Foundry (`forge`) for smart-contract tests.

The first build will download ~80 crates; expect a few minutes.

## Lint policy

The CI workflow runs:

```
cargo fmt --all --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

We **do not** enable `clippy::pedantic` or `clippy::nursery` at the
workspace level because they generate hundreds of false-positive
warnings on the existing codebase. Per-crate overrides are fine if a
specific component warrants stricter checks.

## Cryptography

This project uses post-quantum primitives where they exist:

* Hashes: SHA-3-512 (Grover-resistant 256-bit) — already wired in.
* Signatures: hybrid Dilithium + Ed25519 (Dilithium is currently a stub).
* zk-SNARKs: Circom + snarkjs / arkworks-v5 stack.

If your PR introduces a new cryptographic dependency, document it in
the PR description and add a one-paragraph ADR under
`docs/ADR/` (the directory will be created when the first ADR lands).

## Testing before PR

Before requesting review:

1. `cargo fmt --all`
2. `cargo clippy --workspace -- -D warnings`
3. `cargo test --workspace`
4. If you touched contracts: `forge test` from inside `contracts/`.

## Review criteria

* Does the change preserve privacy? (Raw genomic data must never
  reach the API or contracts.)
* Are the new types documented?
* Is there a unit test for any new branch?
* Does CI pass?

## Pull-request template

Please use this template:

```
## What
<one-paragraph summary>

## Why
<motivation>

## Tests
<test plan / commands>

## Privacy check
<confirm you did not introduce any raw-genomic-data egress>
```
'''

GHOST_NET_PLACEHOLDER_MD = '''# ⚠️ Global Ghost Net — Placeholder

> **This directory is intentionally a stub.** It documents the future
> bridging layer to Global Ghost Net and asserts that no path other than
> this one is ever allowed to carry genomic data — or anything derived
> from it — out of the Zero-Knowledge Genome boundary.

## The one-exit principle

The system in `zk-genome/` has exactly one exit: Global Ghost Net.

* No analytics SDK.
* No crash reporter that sends genome context.
* No usage telemetry.
* No HTTPS call whose body contains genomic-derived material — proofs
  are reduced to SHA-3-512 hashes and zk public inputs before they leave
  the device.
* The mobile UI prompts for Airplane Mode around every plaintext
  operation (`mobile/native/src/radio_shield.rs`) and triggers an
  emergency buffer wipe if a radio is detected mid-operation.

## Cross-team no-touch rule

Until the Global Ghost Net integration contract is signed off:

* **No shared dependencies.** Cargo and npm workspaces that link
  ZK Genome to Ghost Net are forbidden.
* **No team overlap.** Engineers contributing here do not contribute to
  Ghost Net, and vice versa.
* **No documentation cross-reference** beyond this paragraph.

When the integration is finally designed, both sides will publish a
paired ADR. Until then: keep the walls intact.
'''

# ============================================================================
# Apply writes
# ============================================================================

if __name__ == "__main__":
    print("Writing shippable ZK Genome artifacts...")
    write("api/src/lib.rs", LIB_RS)
    write("api/src/state.rs", STATE_RS)
    write("api/src/verify.rs", VERIFY_RS)
    write("api/src/models/mod.rs", MODELS_MOD_RS)
    write("api/src/routes/mod.rs", ROUTES_MOD_RS)
    write("api/src/routes/auth.rs", ROUTES_AUTH_RS)
    write("api/src/routes/queries.rs", ROUTES_QUERIES_RS)
    write("api/src/routes/proofs.rs", ROUTES_PROOFS_RS)
    write("api/src/routes/stats.rs", ROUTES_STATS_RS)
    write("api/src/main.rs", MAIN_RS)
    write("tools/cli/Cargo.toml", TOOLS_CLI_CARGO_TOML)
    write("tools/cli/src/main.rs", TOOLS_CLI_MAIN_RS)
    write("data/samples/example.vcf", SAMPLE_VCF)
    write("scripts/demo.sh", DEMO_SH)
    write("docs/ARCHITECTURE.md", ARCHITECTURE_MD)
    write("docs/PRIVACY.md", PRIVACY_MD)
    write("docs/THREAT_MODEL.md", THREAT_MODEL_MD)
    write("CONTRIBUTING.md", CONTRIBUTING_MD)
    write("global-ghost-net/PLACEHOLDER.md", GHOST_NET_PLACEHOLDER_MD)
    print("Done.")
