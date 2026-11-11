//! Typed request/response models for the ZK Genome API.
//!
//! Every public endpoint deserializes from these structs instead of an
//! untyped `serde_json::Value`, so contract changes surface at compile-time.
//! Mirrors the request shape that `sdk/python` and `sdk/typescript` send.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum QueryStatus {
    Open,
    Active,
    Fulfilled,
    Expired,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Query {
    pub id: String,
    pub researcher: String,
    pub circuit_id: String,
    pub chrom: u8,
    pub pos_start: u32,
    pub pos_end: u32,
    #[serde(default)]
    pub rsids: Vec<String>,
    pub encrypted_params: String,
    pub reward_per_proof: u64,
    pub max_submissions: u32,
    pub submission_count: u32,
    pub status: QueryStatus,
    pub created_at: i64,
    pub deadline: i64,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProofRecord {
    pub proof_hash: String,
    pub query_id: String,
    pub circuit_id: String,
    pub public_inputs: String,
    pub accepted_at: i64,
    pub reward_amount: f64,
    pub researcher: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProofSubmissionOutcome {
    pub query_id: String,
    pub proof_hash: String,
    pub accepted: bool,
    pub reward_amount: f64,
    pub new_submission_count: u32,
    pub query_status: QueryStatus,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProofSubmission {
    pub proof_data: String,
    pub public_inputs: String,
    pub circuit_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResearcherAccount {
    pub api_key: String,
    pub name: String,
    pub organization: String,
    pub created_at: i64,
    pub query_count: u64,
    pub proof_count: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    pub organization: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AggregateStats {
    pub total_queries: u64,
    pub total_proofs: u64,
    pub total_value_locked: f64,
    pub total_paid_out: f64,
    pub active_researchers: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub queries_stored: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ErrorBody>,
}

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

    /// Back-compat constructor.
    pub fn error(message: &str) -> Self {
        Self::failure(message, "generic_error")
    }
}
