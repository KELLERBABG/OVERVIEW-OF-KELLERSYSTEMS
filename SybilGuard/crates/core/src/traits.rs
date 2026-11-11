use crate::types::*;
use crate::SybilResult;

/// Any component that can vouch for a node's personhood.
#[async_trait::async_trait]
pub trait Voucher: Send + Sync {
    /// Issue a vouch from `voucher` for `subject`.
    async fn vouch(&self, voucher: NodeId, subject: NodeId) -> SybilResult<VouchRecord>;

    /// Revoke a previously issued vouch.
    async fn revoke_vouch(&self, vouch_id: &VouchRecord) -> SybilResult<()>;

    /// Verify a trust chain from an anchor to a subject.
    async fn verify_trust_chain(&self, subject: &NodeId) -> SybilResult<TrustChain>;

    /// Get the remaining vouching power for a node.
    async fn vouching_power(&self, node: &NodeId) -> SybilResult<u32>;
}

/// A reputation scoring engine.
#[async_trait::async_trait]
pub trait ReputationEngine: Send + Sync {
    /// Calculate the reputation score for a node.
    async fn calculate_score(&self, node: &NodeId) -> SybilResult<ReputationScore>;

    /// Apply decay to a node's score based on inactivity.
    async fn apply_decay(&self, node: &NodeId) -> SybilResult<ReputationScore>;

    /// Apply a penalty (e.g. for Sybil suspicion).
    async fn apply_penalty(&self, node: &NodeId, penalty: f64) -> SybilResult<ReputationScore>;

    /// Get the current reputation config.
    fn config(&self) -> ReputationConfig;
}

/// A social graph topology analyzer.
#[async_trait::async_trait]
pub trait GraphAnalyzer: Send + Sync {
    /// Compute the clustering coefficient for a node.
    fn clustering_coefficient(&self, node: &NodeId) -> SybilResult<Option<f64>>;

    /// Compute betweenness centrality for a node.
    fn betweenness_centrality(&self, node: &NodeId) -> SybilResult<Option<f64>>;

    /// Compute eigenvector centrality for a node.
    fn eigenvector_centrality(&self, node: &NodeId) -> SybilResult<Option<f64>>;

    /// Detect communities using the Louvain algorithm.
    fn detect_communities_louvain(&self) -> SybilResult<Vec<Vec<NodeId>>>;

    /// Detect communities using the Leiden algorithm.
    fn detect_communities_leiden(&self) -> SybilResult<Vec<Vec<NodeId>>>;

    /// Check if the graph has small-world properties.
    fn is_small_world(&self) -> SybilResult<bool>;

    /// Compute the average shortest path length.
    fn average_path_length(&self) -> SybilResult<f64>;
}

/// A Sybil detection engine that analyzes graph entropy.
#[async_trait::async_trait]
pub trait SybilDetector: Send + Sync {
    /// Analyze a cluster/region for Sybil characteristics.
    async fn analyze_region(&self, nodes: &[NodeId]) -> SybilResult<SybilAnalysisResult>;

    /// Run full-graph Sybil detection, returning all suspicious regions.
    async fn detect_sybil_clusters(&self) -> SybilResult<Vec<SybilAnalysisResult>>;

    /// Compute the graph entropy score for the entire graph.
    fn graph_entropy(&self) -> SybilResult<f64>;

    /// Get the detection configuration.
    fn config(&self) -> SybilDetectionConfig;
}

/// A decentralized identity (DID) manager.
#[async_trait::async_trait]
pub trait DidManager: Send + Sync {
    /// Create a new DID for a node.
    async fn create_did(&self, node: &NodeId) -> SybilResult<DidDocument>;

    /// Resolve a DID string to its document.
    async fn resolve_did(&self, did: &str) -> SybilResult<DidDocument>;

    /// Update a DID document.
    async fn update_did(&self, did: &str, document: DidDocument) -> SybilResult<DidDocument>;

    /// Deactivate a DID.
    async fn deactivate_did(&self, did: &str) -> SybilResult<()>;
}

/// A nullifier manager preventing double-credential creation.
#[async_trait::async_trait]
pub trait NullifierManager: Send + Sync {
    /// Generate a nullifier from an identity seed.
    async fn generate_nullifier(&self, seed: &[u8]) -> SybilResult<IdentityNullifier>;

    /// Check if a nullifier already exists (has been used).
    async fn nullifier_exists(&self, nullifier: &IdentityNullifier) -> SybilResult<bool>;

    /// Register a nullifier as used.
    async fn register_nullifier(&self, nullifier: &IdentityNullifier) -> SybilResult<()>;

    /// Verify that a nullifier is valid and unused, then mark it used.
    async fn verify_and_consume(&self, nullifier: &IdentityNullifier) -> SybilResult<bool>;
}

/// A zk proof generator and verifier (Phase 2+).
#[async_trait::async_trait]
pub trait ZkProver: Send + Sync {
    /// Generate a cluster membership proof.
    async fn prove_cluster_membership(
        &self,
        node: &NodeId,
        cluster: &[NodeId],
        min_size: u32,
    ) -> SybilResult<ClusterMembershipProof>;

    /// Verify a cluster membership proof.
    async fn verify_cluster_membership(
        &self,
        proof: &ClusterMembershipProof,
    ) -> SybilResult<bool>;
}

/// A persistent store for graph data, DIDs, nullifiers, etc.
#[async_trait::async_trait]
pub trait StorageBackend: Send + Sync {
    /// Store a node's metadata.
    async fn store_node(&self, metadata: &NodeMetadata) -> SybilResult<()>;

    /// Retrieve a node by ID.
    async fn get_node(&self, node_id: &NodeId) -> SybilResult<NodeMetadata>;

    /// Store an edge.
    async fn store_edge(&self, edge: &Edge) -> SybilResult<()>;

    /// Retrieve all edges for a node.
    async fn get_edges(&self, node_id: &NodeId) -> SybilResult<Vec<Edge>>;

    /// Store a vouch record.
    async fn store_vouch(&self, vouch: &VouchRecord) -> SybilResult<()>;

    /// Store a DID document.
    async fn store_did(&self, document: &DidDocument) -> SybilResult<()>;

    /// Check if a nullifier exists.
    async fn has_nullifier(&self, nullifier: &IdentityNullifier) -> SybilResult<bool>;

    /// Persist a nullifier.
    async fn persist_nullifier(&self, nullifier: &IdentityNullifier) -> SybilResult<()>;
}