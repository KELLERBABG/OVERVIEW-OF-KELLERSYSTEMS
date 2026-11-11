//! Canonical data models for the Decoupled Fluid IP Network.
//!
//! These types map directly to the protobuf schema in `proto/dfip/core/v1/models.proto`
//! and provide the Rust-native representation used throughout the system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ─── Patent Graph ─────────────────────────────────────────────────────────────

/// A single invention / IP node in the evolutionary DAG.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PatentNode {
    pub id: Uuid,
    pub title: String,
    pub patent_type: PatentType,
    pub authors: Vec<String>,
    pub created_at_unix: i64,
    pub content_hash: Vec<u8>,
    pub invariant: Option<SymbolicExpression>,
    pub tags: Vec<String>,
    pub license: License,
    pub meta_commitment: Vec<u8>,
}

/// The type of invention artifact.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PatentType {
    Software,
    Chemical,
    Mechanical,
}

/// IP license classification.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum License {
    Open,
    Protected,
    Secret,
}

/// A directed edge in the evolutionary DAG, representing a derivation relationship.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Edge {
    pub id: Uuid,
    pub parent_id: Uuid,
    pub child_id: Uuid,
    pub kind: EdgeKind,
    pub delta: Option<DeltaConstraint>,
    pub contribution_weight: f64,
    pub signature: Vec<u8>,
}

/// The kind of derivation relationship.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EdgeKind {
    Derivative,
    Improvement,
    Fork,
    Merge,
}

// ─── Symbolic / Logical Layer ────────────────────────────────────────────────

/// An abstract symbolic representation of an invention's functional logic.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SymbolicExpression {
    pub format: ExpressionFormat,
    pub encoded_ir: Vec<u8>,
    pub free_variables: Vec<String>,
    pub assumptions: Vec<String>,
}

/// The format or language of the symbolic expression.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ExpressionFormat {
    SsaTrace,
    Molecular,
    Geometric,
    SmtLib2,
}

/// Delta constraint: the logical difference between an original and a derivative.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DeltaConstraint {
    pub original_node_id: Uuid,
    pub derivative_node_id: Uuid,
    pub original_invariant_hash: Vec<u8>,
    pub formula_smt_lib2: Vec<u8>,
    pub assumptions: Vec<String>,
}

// ─── SMT Verification ────────────────────────────────────────────────────────

/// The outcome of a SMT-based verification query on an edge.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct VerificationResult {
    pub edge_id: Uuid,
    pub sat: SatResult,
    pub model: Vec<u8>,
    pub unsat_core: Vec<Vec<u8>>,
    pub severity_score: f64,
    pub solver_time_ms: f64,
    pub proof_commitment: Vec<u8>,
}

/// SMT satisfiability result.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SatResult {
    Sat,
    Unsat,
    Unknown,
}

// ─── Royalties ────────────────────────────────────────────────────────────────

/// A royalty invariant ensuring sum(weights) == 1.0 for all upstream contributions.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RoyaltyInvariant {
    pub patent_node_id: Uuid,
    pub shares: Vec<ShareEntry>,
    pub upstream_nodes: Vec<Uuid>,
}

/// A single share entry mapping a node to its fractional weight.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ShareEntry {
    pub node_id: Uuid,
    pub weight: f64,
}

// ─── Taint Tracking ───────────────────────────────────────────────────────────

/// A taint label tracking the origin license and derivation chain of a fragment.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TaintLabel {
    pub id: Uuid,
    pub origin_license: License,
    pub derivation_chain: Vec<Uuid>,
}

/// A rule defining how taint labels compose under a given operator.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TaintPropagationRule {
    pub operator: String,
    pub join_semantics: TaintLatticeJoin,
}

/// Lattice join semantics for taint composition.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TaintLatticeJoin {
    Meet,
    Join,
    LeastUpperBound,
}

// ─── Homomorphic Contributions ────────────────────────────────────────────────

/// An encrypted contribution share using a homomorphic encryption scheme.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EncryptedShare {
    pub ciphertext: Vec<u8>,
    pub scheme: String,
    pub public_key_hash: Vec<u8>,
}

/// A zero-knowledge proof of a valid contribution.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ContributionProof {
    pub proof: Vec<u8>,
    pub proof_type: String,
}

// ─── IP Specification ─────────────────────────────────────────────────────────

/// A formal, machine-readable specification derived from an invention's logic.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct IPSpecification {
    pub node_id: Uuid,
    pub preconditions: Vec<String>,
    pub postconditions: Vec<String>,
    pub invariants: Vec<String>,
    pub resource_bounds: Vec<String>,
    pub spec_language: String,
    pub spec_bytes: Vec<u8>,
}

// ─── State / Aggregates ───────────────────────────────────────────────────────

/// The full patent graph state — nodes and edges.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PatentGraphState {
    pub nodes: HashMap<Uuid, PatentNode>,
    pub edges: Vec<Edge>,
}

/// An event in the append-only lifecycle log.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LifecycleEvent {
    pub id: Uuid,
    pub event_type: LifecycleEventType,
    pub node_id: Uuid,
    pub timestamp_unix: i64,
    pub signature: Vec<u8>,
    pub parent_events: Vec<Uuid>, // for Merkle chaining
    pub merkle_root: Vec<u8>,
}

/// The type of lifecycle event.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum LifecycleEventType {
    NodeCreated,
    NodeModified,
    NodeBranched,
    NodeRemoved,
    EdgeAdded,
    EdgeRemoved,
}