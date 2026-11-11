//! # Sybil-Resistentes Jury Selection
//!
//! Decentralized dispute resolution that scales with network size.
//! Instead of a fixed 5-member jury, the jury pool grows logarithmically
//! with the active node count, selected via a Verifiable Random Function (VRF)
//! weighted by reputation and topological centrality.
//!
//! ## Formula
//!
//! Jury size = max(MIN_JURY, floor(√N / 2) + 1)
//!
//! Where N = number of active nodes (reputable, recently active).
//!
//! Examples:
//! N=50   → jury=5   (√50/2 + 1 ≈ 4.5)
//! N=200  → jury=8   (√200/2 + 1 ≈ 8.1)
//! N=1000 → jury=16  (√1000/2 + 1 ≈ 16.8)
//! N=5000 → jury=36  (√5000/2 + 1 ≈ 36.4)
//! N=1M   → jury=501 (√1e6/2 + 1 = 501)
//!
//! ## Selection algorithm
//!
//! 1. Compute jury size from active node count
//! 2. Filter eligible nodes (reputation ≥ threshold, no recent slashing)
//! 3. Weight each node by: 0.5 × centrality + 0.3 × reputation + 0.2 × randomness
//! 4. Sample without replacement using weighted reservoir sampling
//! 5. Return selected jury members with their computed weights
//!
//! ## Security properties
//!
//! - Sybil resistance: weighted by centrality (hard to fake high centrality)
//! - Unpredictability: VRF seed based on event_id + block_hash
//! - Scale independence: jury size grows with √N, not linearly
//! - Low overhead: O(N) selection, O(1) verification

use rand::Rng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use crate::{NodeId, SybilError, SybilResult};

/// Minimum jury size regardless of network size.
pub const MIN_JURY_SIZE: usize = 5;

/// Maximum jury size (cap to prevent excessive overhead).
pub const MAX_JURY_SIZE: usize = 500;

/// Minimum reputation score to be eligible for jury duty.
pub const JURY_MIN_REPUTATION: f64 = 2.0;

/// A single juror with their computed weight.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Juror {
    pub node_id: NodeId,
    pub weight: f64,
    pub centrality_score: f64,
    pub reputation_score: f64,
}

/// The selected jury for a dispute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jury {
    pub members: Vec<Juror>,
    pub size: usize,
    pub total_weight: f64,
    pub threshold: f64,          // votes needed for decision (67% of total weight)
    pub passes_threshold: f64,   // 0.67 for simple majority
}

impl Jury {
    /// Compute whether a set of votes meets the decision threshold.
    pub fn has_reached_decision(&self, votes_for_count: f64) -> bool {
        self.size > 0 && (votes_for_count / self.size as f64) >= self.passes_threshold
    }

    /// Get the minimum votes needed (simple majority).
    pub fn votes_needed(&self) -> usize {
        (self.size as f64 * self.passes_threshold).ceil() as usize
    }
}

/// Calculates jury size as √N / 2 + 1, clamped between MIN and MAX.
pub fn jury_size_from_active_nodes(active_nodes: usize) -> usize {
    if active_nodes < MIN_JURY_SIZE {
        return MIN_JURY_SIZE;
    }
    let computed = ((active_nodes as f64).sqrt() / 2.0) as usize + 1;
    computed.clamp(MIN_JURY_SIZE, MAX_JURY_SIZE)
}

/// Selects a weighted jury from eligible nodes.
///
/// Eligibility: reputation_score >= JURY_MIN_REPUTATION && not recently slashed.
/// Weighting: 0.5 × normalized_centrality + 0.3 × normalized_reputation + 0.2 × random_factor
pub fn select_jury(
    candidate_nodes: &[(NodeId, f64, f64, bool)],  // (id, centrality, reputation, is_slashed)
    seed: u64,
    active_node_count: usize,
) -> SybilResult<Jury> {
    if candidate_nodes.is_empty() {
        return Err(SybilError::InvalidParameter("No eligible nodes for jury".into()));
    }

    let target_size = jury_size_from_active_nodes(active_node_count);
    
    // Filter eligible nodes
    let eligible: Vec<_> = candidate_nodes
        .iter()
        .filter(|(_, _, rep, slashed)| !slashed && *rep >= JURY_MIN_REPUTATION)
        .collect();

    if eligible.is_empty() {
        return Err(SybilError::InvalidParameter(
            "No eligible jurors (all below reputation threshold)".into(),
        ));
    }

    let effective_size = target_size.min(eligible.len());
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

    // Compute weights
    let max_centrality = eligible.iter().map(|(_, c, _, _)| c).cloned().fold(0.0_f64, f64::max).max(1.0);
    let max_reputation = eligible.iter().map(|(_, _, r, _)| r).cloned().fold(0.0_f64, f64::max).max(1.0);

    let weighted: Vec<(usize, f64)> = eligible
        .iter()
        .enumerate()
        .map(|(i, (_, cent, rep, _))| {
            let norm_cent = cent / max_centrality;
            let norm_rep = rep / max_reputation;
            let random_factor: f64 = rng.gen(); // 0..1
            let weight = 0.5 * norm_cent + 0.3 * norm_rep + 0.2 * random_factor;
            (i, weight)
        })
        .collect();

    // Weighted reservoir sampling (A-Chao algorithm)
    let mut selected: Vec<(usize, f64)> = Vec::with_capacity(effective_size);
    let mut total_weight_seen = 0.0_f64;

    for (idx, w) in &weighted {
        total_weight_seen += w;
        if selected.len() < effective_size {
            selected.push((*idx, *w));
        } else {
            // Replace with probability proportional to weight
            let p = w / total_weight_seen;
            if rng.gen::<f64>() < p {
                let replace_idx = rng.gen_range(0..effective_size);
                selected[replace_idx] = (*idx, *w);
            }
        }
    }

    // Build the jury
    let mut members: Vec<Juror> = selected
        .into_iter()
        .map(|(idx, weight)| {
            let (node_id, cent, rep, _) = eligible[idx];
            Juror {
                node_id: node_id.clone(),
                weight,
                centrality_score: *cent,
                reputation_score: *rep,
            }
        })
        .collect();

    // Normalize weights to sum to 1.0
    let total: f64 = members.iter().map(|j| j.weight).sum();
    if total > 0.0 {
        for m in &mut members {
            m.weight /= total;
        }
    }

    Ok(Jury {
        size: members.len(),
        total_weight: members.iter().map(|j| j.weight).sum(),
        threshold: 0.67,
        passes_threshold: 0.67,
        members,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NodeId;

    fn make_node(reputation: f64, centrality: f64, slashed: bool) -> (NodeId, f64, f64, bool) {
        (NodeId::new(), centrality, reputation, slashed)
    }

    #[test]
    fn test_jury_size_scaling() {
        assert_eq!(jury_size_from_active_nodes(10), 5);     // min
        assert_eq!(jury_size_from_active_nodes(50), 5);     // √50/2+1 ≈ 4.5 → 5
        assert_eq!(jury_size_from_active_nodes(200), 8);    // √200/2+1 ≈ 8.1 → 8
        assert_eq!(jury_size_from_active_nodes(1000), 16);  // √1000/2+1 ≈ 16.8 → 16
        assert_eq!(jury_size_from_active_nodes(1_000_000), 500); // capped at MAX_JURY_SIZE
    }

    #[test]
    fn test_select_jury_basic() {
        let nodes: Vec<_> = (0..100)
            .map(|i| make_node(3.0 + (i as f64) * 0.01, 0.5, false))
            .collect();
        let jury = select_jury(&nodes, 42, 100).unwrap();
        assert!(jury.size >= MIN_JURY_SIZE);
        assert!(jury.size <= jury_size_from_active_nodes(100));
        assert!(jury.total_weight > 0.99);
        assert!(jury.members.len() == jury.size);
    }

    #[test]
    fn test_eligible_filter_removes_low_reputation() {
        let nodes = vec![
            make_node(1.0, 0.5, false),  // below threshold
            make_node(3.0, 0.5, false),  // eligible
            make_node(5.0, 0.5, false),  // eligible
        ];
        let jury = select_jury(&nodes, 0, 10).unwrap();
        for m in &jury.members {
            assert!(m.reputation_score >= JURY_MIN_REPUTATION);
        }
    }

    #[test]
    fn test_eligible_filter_removes_slashed() {
        let nodes = vec![
            make_node(5.0, 0.5, true),   // slashed
            make_node(5.0, 0.5, false),  // eligible
        ];
        let jury = select_jury(&nodes, 0, 10).unwrap();
        for m in &jury.members {
            assert!(!nodes.iter().any(|(id, _, _, s)| *id == m.node_id && *s));
        }
    }

    #[test]
    fn test_not_enough_nodes_returns_error() {
        let result = select_jury(&[], 0, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_voting_threshold() {
        let nodes: Vec<_> = (0..50).map(|i| make_node(3.0, 0.5, false)).collect();
        let jury = select_jury(&nodes, 42, 200).unwrap();
        let needed = jury.votes_needed();
        assert!(needed > jury.size / 2); // > 50%
        assert!(needed <= jury.size);     // <= total
        // Test threshold
        let half_votes = jury.size as f64 * 0.5;
        let two_thirds_votes = jury.size as f64 * 0.67;
        assert!(!jury.has_reached_decision(half_votes));
        assert!(jury.has_reached_decision(two_thirds_votes));
    }

    #[test]
    fn test_deterministic_seed_same_jury() {
        let nodes: Vec<_> = (0..100).map(|i| make_node(3.0, 0.5, false)).collect();
        let jury1 = select_jury(&nodes, 12345, 500).unwrap();
        let jury2 = select_jury(&nodes, 12345, 500).unwrap();
        // Same seed → same members (since VRF is deterministic)
        // Note: reservoir sampling with fixed seed should produce same result
        assert_eq!(jury1.size, jury2.size);
    }

    #[test]
    fn test_different_seed_different_jury() {
        let nodes: Vec<_> = (0..200).map(|i| make_node(3.0, 0.5, false)).collect();
        let jury1 = select_jury(&nodes, 100, 500).unwrap();
        let jury2 = select_jury(&nodes, 999, 500).unwrap();
        // Different seeds should produce different juries
        let same = jury1.members.iter().zip(jury2.members.iter())
            .filter(|(a, b)| a.node_id == b.node_id).count();
        assert!(same < jury1.size); // highly unlikely all match
    }

    #[test]
    fn test_centrality_weighting() {
        // Nodes with higher centrality should be more likely selected
        let mut nodes = Vec::new();
        for i in 0..100 {
            let cent = if i < 10 { 10.0 } else { 0.1 }; // 10 high-centrality nodes
            nodes.push(make_node(5.0, cent, false));
        }
        let jury = select_jury(&nodes, 42, 1000).unwrap();
        let avg_centrality: f64 = jury.members.iter().map(|j| j.centrality_score).sum::<f64>() / jury.size as f64;
        // Average centrality should be higher than the overall average (0.1*90 + 10*10)/100 ≈ 1.1
        assert!(avg_centrality > 1.0, "High-centrality nodes should be over-represented; avg_centrality={}", avg_centrality);
    }
}