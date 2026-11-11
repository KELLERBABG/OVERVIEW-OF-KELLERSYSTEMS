use ark_bn254::{Fr, G1Projective, G2Projective};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct ThresholdConfig {
    pub threshold: usize,
    pub total_nodes: usize,
}

impl ThresholdConfig {
    pub fn new(threshold: usize, total_nodes: usize) -> Self {
        assert!(threshold <= total_nodes, "threshold must be <= total_nodes");
        Self {
            threshold,
            total_nodes,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PublicKey {
    pub point: G2Projective,
}

#[derive(Debug, Clone)]
pub struct SecretKey {
    pub scalar: Fr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Signature {
    pub point: G1Projective,
}

#[derive(Debug, Clone)]
pub struct PartialSignature {
    pub share_index: u64,
    pub signature: Signature,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AggregatedSignature {
    pub signature: Signature,
}

#[derive(Debug, Clone)]
pub struct KeyShare {
    pub index: u64,
    pub secret: SecretKey,
    pub public: PublicKey,
}

#[derive(Debug, Error)]
pub enum ThresholdError {
    #[error("insufficient partial signatures: need {need} got {got}")]
    InsufficientSignatures { need: usize, got: usize },
    #[error("invalid signature")]
    InvalidSignature,
}
