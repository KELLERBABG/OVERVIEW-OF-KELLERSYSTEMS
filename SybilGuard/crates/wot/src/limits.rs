/// Recursive vouching limits engine.
///
/// Mathematical constraints that prevent unbounded trust propagation:
/// - Maximum depth from any anchor
/// - Maximum vouching power per node (reputation-dependent)
/// - Propagation caps (a node's vouching power diminishes with depth)

use sybil_core::constants::*;

/// Calculate the maximum vouching power a node should have based on its reputation.
pub fn max_vouching_power_from_reputation(reputation_score: f64) -> u32 {
    let power = (reputation_score * 2.0).round() as u32;
    power.min(MAX_VOUCHES_PER_NODE)
}

/// Calculate the effective depth of a node in the trust graph.
/// This is the longest chain from any anchor to the node minus 1.
pub fn effective_embedding_depth(total_chain_length: u32) -> u32 {
    if total_chain_length == 0 {
        0
    } else {
        total_chain_length - 1
    }
}

/// Determine if a node at a given depth can still issue valid vouches.
///
/// Vouches issued beyond `MAX_TRUST_CHAIN_DEPTH` are invalid.
pub fn can_vouch_at_depth(current_depth: u32) -> bool {
    current_depth < MAX_TRUST_CHAIN_DEPTH
}

/// Calculate the weight of a vouch based on the voucher's depth.
///
/// Weight diminishes with depth: deeper nodes have less vouching power.
pub fn vouch_weight(voucher_depth: u32) -> f64 {
    (1.0 / (voucher_depth as f64 + 1.0)).clamp(0.1, 1.0)
}

/// Calculate the propagated vouching power for a node issuing a vouch.
///
/// Propagation cap: a node's vouching power is divided among the vouches it issues,
/// and the value of each vouch decreases exponentially with depth.
pub fn propagated_vouching_power(voucher_depth: u32, remaining_power: u32) -> f64 {
    let base = vouch_weight(voucher_depth);
    let dilution = remaining_power as f64;
    base / dilution
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_power_from_reputation() {
        assert_eq!(max_vouching_power_from_reputation(5.0), 10);
        assert_eq!(max_vouching_power_from_reputation(0.0), 0);
    }

    #[test]
    fn test_effective_depth() {
        assert_eq!(effective_embedding_depth(3), 2);
        assert_eq!(effective_embedding_depth(0), 0);
    }

    #[test]
    fn test_can_vouch_at_depth() {
        assert!(can_vouch_at_depth(0));
        assert!(can_vouch_at_depth(MAX_TRUST_CHAIN_DEPTH - 1));
        assert!(!can_vouch_at_depth(MAX_TRUST_CHAIN_DEPTH));
    }

    #[test]
    fn test_vouch_weight_decreases_with_depth() {
        let w0 = vouch_weight(0);
        let w5 = vouch_weight(5);
        assert!(w0 > w5);
    }

    #[test]
    fn test_propagated_power() {
        let p = propagated_vouching_power(0, 10);
        assert!(p > 0.0 && p <= 1.0);
    }
}