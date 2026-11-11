/// System-wide constants for the Proof-of-Personhood protocol.

// ── Graph constants ────────────────────────────────────────────────

/// Minimum number of nodes required for meaningful Sybil detection.
pub const MIN_GRAPH_SIZE_FOR_DETECTION: usize = 50;

/// Expected average degree in a small-world social graph.
pub const EXPECTED_SMALL_WORLD_DEGREE: f64 = 6.0;

/// Expected clustering coefficient range for human social networks.
pub const EXPECTED_HUMAN_CLUSTERING_MIN: f64 = 0.3;
pub const EXPECTED_HUMAN_CLUSTERING_MAX: f64 = 0.8;

/// Watts-Strogatz rewiring probability for synthetic graphs.
pub const DEFAULT_REWIRING_PROBABILITY: f64 = 0.1;

// ── Web of Trust constants ─────────────────────────────────────────

/// Maximum depth of a trust chain before it's considered unreliable.
pub const MAX_TRUST_CHAIN_DEPTH: u32 = 5;

/// Maximum number of vouches a single node can issue.
pub const MAX_VOUCHES_PER_NODE: u32 = 10;

/// Default vouch expiry (90 days).
pub const VOUCH_EXPIRY_DAYS: i64 = 90;

/// Minimum number of anchor vouches required for a new node.
pub const MIN_ANCHOR_VOUCHES: u32 = 3;

// ── Reputation constants ───────────────────────────────────────────

/// Initial reputation score for a new node.
pub const INITIAL_REPUTATION_SCORE: f64 = 1.0;

/// Daily reputation decay rate (0.1% per day).
pub const REPUTATION_DECAY_RATE_PER_DAY: f64 = 0.001;

/// Bonus multiplier per embedding depth level.
pub const DEPTH_BONUS_MULTIPLIER: f64 = 0.1;

/// Bonus multiplier for being in a validated cluster.
pub const CLUSTER_BONUS_MULTIPLIER: f64 = 0.05;

/// Penalty applied when a node is flagged as potential Sybil.
pub const SYBIL_PENALTY_SCORE: f64 = -10.0;

// ── Sybil detection constants ──────────────────────────────────────

/// Entropy threshold below which a cluster is suspicious.
pub const SYBIL_ENTROPY_THRESHOLD: f64 = 0.7;

/// Anomaly score threshold (standard deviations from mean).
pub const SYBIL_ANOMALY_THRESHOLD: f64 = 2.5;

/// Minimum cluster size to analyze for Sybil characteristics.
pub const MIN_CLUSTER_SIZE_FOR_ANALYSIS: usize = 10;

/// Confidence threshold for flagging a Sybil cluster.
pub const SYBIL_CONFIDENCE_THRESHOLD: f64 = 0.8;

// ── Attack mitigation constants ────────────────────────────────────

/// Stake requirement (in hypothetical tokens) to initiate vouching.
pub const VOUCHING_STAKE_REQUIREMENT: u64 = 100;

/// Time lock (in days) before a new node can start vouching.
pub const NEW_NODE_VOUCHING_LOCK_DAYS: i64 = 30;

/// Time lock (in days) between changing a vouch.
pub const VOUCH_CHANGE_COOLDOWN_DAYS: i64 = 7;

// ── Verification event constants ───────────────────────────────────

/// Minimum participants required for a verification event.
pub const MIN_VERIFICATION_PARTICIPANTS: u32 = 10;

/// Default verification event frequency (every 30 days).
pub const VERIFICATION_EVENT_INTERVAL_DAYS: i64 = 30;

/// Maximum time between verification events before reputation decays.
pub const MAX_INACTIVITY_DAYS: i64 = 90;

// ── DID constants ──────────────────────────────────────────────────

/// The DID method name for this system.
pub const DID_METHOD: &str = "sybil";

/// DID context URL for W3C compliance.
pub const DID_CONTEXT_URL: &str = "https://www.w3.org/ns/did/v1";

// ── API constants ─────────────────────────────────────────────────

/// Default API pagination page size.
pub const DEFAULT_PAGE_SIZE: u32 = 50;

/// Maximum API pagination page size.
pub const MAX_PAGE_SIZE: u32 = 500;

/// Default REST API port.
pub const DEFAULT_API_PORT: u16 = 8080;

/// Default gRPC API port.
pub const DEFAULT_GRPC_PORT: u16 = 9090;

/// API version string.
pub const API_VERSION: &str = "v1";

// ── zk proof constants ────────────────────────────────────────────

/// Minimum cluster size for zk cluster membership proof.
pub const MIN_CLUSTER_SIZE_FOR_ZK_PROOF: u32 = 5;

/// Circuit identifier for the cluster membership circuit.
pub const CLUSTER_MEMBERSHIP_CIRCUIT_ID: &str = "sybil-cluster-membership-v1";

// ── Testing / synthetic graph constants ────────────────────────────

/// Default number of nodes in synthetic test graphs.
pub const DEFAULT_SYNTHETIC_NODES: usize = 1000;

/// Default Sybil cluster size in synthetic test graphs.
pub const DEFAULT_SYNTHETIC_SYBIL_SIZE: usize = 20;

/// Default number of Sybil clusters in synthetic test graphs.
pub const DEFAULT_SYNTHETIC_SYBIL_CLUSTERS: usize = 3;