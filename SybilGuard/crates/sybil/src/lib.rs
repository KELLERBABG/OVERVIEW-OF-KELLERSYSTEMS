//! # Sybil-Resistance by Graph Entropy
//!
//! Detects artificially created identity clusters through:
//! - Graph entropy analysis comparing observed vs expected small-world properties
//! - Anomaly detection based on clustering coefficient distributions
//! - Component isolation analysis
//! - Confidence-weighted classification
//! - Advanced degeneracy detectors (ring lattice, star topology)
//! - Differential privacy for graph queries (Phase 1 mitigation)

pub mod entropy;
pub mod detector;
pub mod metrics;
pub mod detectors;
pub mod privacy;

pub use entropy::*;
pub use detector::*;
pub use metrics::*;
pub use detectors::*;
pub use privacy::*;