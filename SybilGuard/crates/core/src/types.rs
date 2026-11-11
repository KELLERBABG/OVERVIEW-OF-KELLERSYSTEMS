use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Identity types ───────────────────────────────────────────────

/// A unique identifier for a node (person/identity) in the social graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub Uuid);

impl NodeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ─── Graph types ──────────────────────────────────────────────────

/// Direction of a relationship in the social graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeDirection {
    Directed,
    Undirected,
}

/// A single edge (relationship/vouch) between two nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub source: NodeId,
    pub target: NodeId,
    pub weight: f64,
    pub metadata: EdgeMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeMetadata {
    pub created_at: DateTime<Utc>,
    pub edge_type: EdgeType,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EdgeType {
    /// A direct vouch from source → target
    Vouch,
    /// A known relationship (social connection)
    Social,
    /// A verified interaction
    VerifiedInteraction,
    /// Custom edge type
    Custom(String),
}

/// Full social graph representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialGraph {
    pub nodes: Vec<NodeMetadata>,
    pub edges: Vec<Edge>,
    pub directed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Metadata associated with a graph node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub node_id: NodeId,
    pub did: Option<String>,
    pub reputation_score: f64,
    pub clustering_coefficient: Option<f64>,
    pub centrality_score: Option<f64>,
    pub embedding_depth: u32,
    pub joined_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub is_sybil: Option<bool>,
    pub metadata: serde_json::Value,
}

// ─── Web of Trust types ───────────────────────────────────────────

/// A single vouch record: node A vouches for node B.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VouchRecord {
    pub voucher: NodeId,
    pub subject: NodeId,
    pub signature: String,
    pub depth: u32,
    pub power_consumed: u32,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// The trust chain from a root anchor to a subject.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustChain {
    pub subject: NodeId,
    pub anchors: Vec<NodeId>,
    pub path: Vec<VouchRecord>,
    pub total_depth: u32,
    pub cumulative_weight: f64,
    pub verified_at: DateTime<Utc>,
}

/// A verification event where the community validates liveness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationEvent {
    pub event_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub participants: Vec<NodeId>,
    pub threshold: u32,
    pub completed: bool,
    pub proof: Option<String>,
}

// ─── Reputation types ─────────────────────────────────────────────

/// Reputation score snapshot for a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationScore {
    pub node_id: NodeId,
    pub base_score: f64,
    pub embedding_bonus: f64,
    pub vouching_power: u32,
    pub decay_factor: f64,
    pub last_updated: DateTime<Utc>,
}

/// Configuration for reputation calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationConfig {
    pub initial_score: f64,
    pub max_vouching_power: u32,
    pub decay_rate_per_day: f64,
    pub depth_bonus_multiplier: f64,
    pub cluster_bonus_multiplier: f64,
    pub sybil_penalty: f64,
}

impl Default for ReputationConfig {
    fn default() -> Self {
        Self {
            initial_score: 1.0,
            max_vouching_power: 10,
            decay_rate_per_day: 0.001,
            depth_bonus_multiplier: 0.1,
            cluster_bonus_multiplier: 0.05,
            sybil_penalty: -10.0,
        }
    }
}

// ─── Sybil detection types ─────────────────────────────────────────

/// Result of a Sybil analysis on a graph region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SybilAnalysisResult {
    pub region_id: String,
    pub node_count: usize,
    pub entropy_score: f64,
    pub anomaly_score: f64,
    pub is_suspected_sybil: bool,
    pub confidence: f64,
    pub details: SybilAnalysisDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SybilAnalysisDetails {
    pub mean_clustering_coefficient: f64,
    pub expected_small_world_cc: f64,
    pub internal_edge_density: f64,
    pub external_edge_density: f64,
    pub isolated_component: bool,
    pub components: Vec<String>,
}

/// Configuration for Sybil detection algorithms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SybilDetectionConfig {
    pub min_graph_size: usize,
    pub entropy_threshold: f64,
    pub anomaly_threshold: f64,
    pub confidence_threshold: f64,
    pub min_cluster_size_for_analysis: usize,
}

impl Default for SybilDetectionConfig {
    fn default() -> Self {
        Self {
            min_graph_size: 50,
            entropy_threshold: 0.7,
            anomaly_threshold: 2.5,
            confidence_threshold: 0.8,
            min_cluster_size_for_analysis: 10,
        }
    }
}

// ─── zk Proof types ───────────────────────────────────────────────

/// A zero-knowledge proof that a user is member of a legitimate
/// trust cluster of minimum size N without revealing which cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMembershipProof {
    pub proof_bytes: Vec<u8>,
    pub public_inputs: Vec<u8>,
    pub min_cluster_size: u32,
    pub circuit_id: String,
    pub created_at: DateTime<Utc>,
    pub nullifier: String,
}

/// A nullifier uniquely derived from a person's identity seed,
/// preventing double-credential creation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IdentityNullifier(pub String);

// ─── DID types ────────────────────────────────────────────────────

/// A W3C Decentralized Identifier document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DidDocument {
    pub did: String,
    pub verification_methods: Vec<VerificationMethod>,
    pub authentication: Vec<String>,
    pub assertion_method: Vec<String>,
    pub service_endpoints: Vec<ServiceEndpoint>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationMethod {
    pub id: String,
    pub controller: String,
    pub key_type: String,
    pub public_key_multibase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    pub id: String,
    pub service_type: String,
    pub endpoint_url: String,
}

// ─── API types ────────────────────────────────────────────────────

/// Generic paginated response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub page: u32,
    pub page_size: u32,
    pub total: u64,
    pub has_more: bool,
}

/// Health status of a service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub service: String,
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub last_checked: DateTime<Utc>,
}

// ─── Synthetic graph generation types ─────────────────────────────

/// Configuration for generating synthetic social graphs for testing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntheticGraphConfig {
    pub num_nodes: usize,
    pub avg_degree: f64,
    pub rewiring_probability: f64,
    pub sybil_cluster_size: usize,
    pub num_sybil_clusters: usize,
    pub sybil_internal_density: f64,
    pub sybil_external_connectivity: f64,
}

impl Default for SyntheticGraphConfig {
    fn default() -> Self {
        Self {
            num_nodes: 1000,
            avg_degree: 6.0,
            rewiring_probability: 0.1,
            sybil_cluster_size: 20,
            num_sybil_clusters: 3,
            sybil_internal_density: 0.8,
            sybil_external_connectivity: 0.01,
        }
    }
}

// ─── Metrics types ────────────────────────────────────────────────

/// Validation metrics for Sybil detection accuracy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationMetrics {
    pub true_positives: u64,
    pub false_positives: u64,
    pub true_negatives: u64,
    pub false_negatives: u64,
    pub accuracy: f64,
    pub precision: f64,
    pub recall: f64,
    pub f1_score: f64,
    pub detection_latency_ms: f64,
}