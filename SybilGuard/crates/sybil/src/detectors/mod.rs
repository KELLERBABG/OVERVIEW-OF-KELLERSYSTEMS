//! Advanced Sybil detectors for specialized attack vectors.
//!
//! These detectors complement the main `SybilDetector` by catching
//! specific attack patterns that the general entropy-based detector
//! may miss.

pub mod ring_lattice;

pub use ring_lattice::*;