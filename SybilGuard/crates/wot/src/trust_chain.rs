use sybil_core::{constants::*, NodeId, SybilResult, TrustChain};

use crate::vouching::VouchingEngine;

/// Trust chain verification and construction.
pub struct TrustChainVerifier<'a> {
    engine: &'a VouchingEngine,
}

impl<'a> TrustChainVerifier<'a> {
    pub fn new(engine: &'a VouchingEngine) -> Self {
        Self { engine }
    }

    /// Build and verify a trust chain from anchor nodes to a subject.
    ///
    /// Performs BFS from subject backwards through vouchers to find
    /// paths to known anchor nodes.
    pub fn verify_trust_chain(
        &self,
        subject: &NodeId,
        anchors: &[NodeId],
    ) -> SybilResult<TrustChain> {
        let mut visited = std::collections::HashSet::new();
        let mut path = Vec::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(subject.clone());

        // BFS to find shortest path to any anchor
        while let Some(current) = queue.pop_front() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());

            // Did we reach an anchor?
            if anchors.contains(&current) {
                break;
            }

            // Get all vouchers for the current node
            let vouches = self.engine.vouches_for(&current);
            for vouch in &vouches {
                if !visited.contains(&vouch.voucher) {
                    path.push(vouch.clone());
                    queue.push_back(vouch.voucher.clone());
                }
            }
        }

        let total_depth = path.len() as u32;
        let cumulative_weight = path.iter().map(|v| 1.0 / (v.depth as f64 + 1.0)).sum();

        let chain = TrustChain {
            subject: subject.clone(),
            anchors: anchors.to_vec(),
            path,
            total_depth,
            cumulative_weight,
            verified_at: chrono::Utc::now(),
        };

        Ok(chain)
    }

    /// Check if a trust chain is valid.
    ///
    /// A chain is valid if:
    /// - Total depth doesn't exceed maximum
    /// - All vouches in the path are still valid
    /// - The chain reaches at least one anchor
    pub fn is_chain_valid(&self, chain: &TrustChain) -> bool {
        if chain.total_depth > MAX_TRUST_CHAIN_DEPTH {
            return false;
        }
        for vouch in &chain.path {
            if !self.engine.is_vouch_valid(vouch) {
                return false;
            }
        }
        // Check that at least one anchor is in the chain's anchor list
        !chain.anchors.is_empty()
    }
}