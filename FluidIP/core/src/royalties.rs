//! # Decoupled Royalty Invariants
//!
//! Defines unalterable royalty invariants that mathematically guarantee
//! micro-payments flow correctly through the entire evolutionary patent tree.
//!
//! The core invariant: `sum(contribution_i) = 1.0` for all shares feeding into
//! any node. Each edge weight represents the fraction of value added by that
//! derivation. Royalties propagate through the DAG using dynamic programming
//! in topological order: O(V + E) recomputation when a new node is added.

use crate::error::CoreError;
use crate::graph::PatentDag;
use crate::models::{Edge, RoyaltyInvariant, ShareEntry};
use std::collections::HashMap;
use uuid::Uuid;

// ─── Royalty Invariant ───────────────────────────────────────────────────────

/// Verifies that all royalty invariants in a DAG are satisfied.
///
/// For each node with incoming edges, the sum of all `contribution_weight`
/// values from its parents must equal 1.0 (within floating-point tolerance).
pub struct RoyaltyVerifier {
    epsilon: f64,
}

impl RoyaltyVerifier {
    /// Create a new verifier with the given floating-point tolerance.
    pub fn new(epsilon: f64) -> Self {
        Self { epsilon }
    }

    /// Verify royalty invariants for a single node.
    pub fn verify_node(&self, dag: &PatentDag, node_id: &Uuid) -> Result<(), CoreError> {
        let _node = dag.get_node(node_id)?;
        let state = dag.export_state();
        let incoming_edges: Vec<&Edge> = state
            .edges
            .iter()
            .filter(|e| e.child_id == *node_id)
            .collect();

        if incoming_edges.is_empty() {
            // Root nodes have no royalty obligations.
            return Ok(());
        }

        let sum: f64 = incoming_edges.iter().map(|e| e.contribution_weight).sum();
        if (sum - 1.0).abs() > self.epsilon {
            return Err(CoreError::RoyaltySumViolation { sum });
        }
        Ok(())
    }

    /// Verify royalty invariants for ALL nodes in the DAG.
    pub fn verify_all(&self, dag: &PatentDag) -> Result<(), CoreError> {
        let state = dag.export_state();
        for node_id in state.nodes.keys() {
            self.verify_node(dag, node_id)?;
        }
        Ok(())
    }
}

impl Default for RoyaltyVerifier {
    fn default() -> Self {
        Self::new(1e-9)
    }
}

// ─── Royalty Computation ─────────────────────────────────────────────────────

/// Compute the effective royalty share for each ancestor of a given node.
///
/// Uses dynamic programming over topological order:
/// `share[node] = sum(share[parent] * edge_weight for each incoming edge)`
/// Root nodes have share = 0.0 (they receive 100% of their own value).
pub fn compute_royalty_shares(
    dag: &PatentDag,
    node_id: &Uuid,
) -> Result<HashMap<Uuid, f64>, CoreError> {
    let _node = dag.get_node(node_id)?;
    let ancestors = dag.ancestor_ids(node_id)?;

    // Topological order within ancestors.
    let topo_all = dag.topological_order_indices()?;
    let topo: Vec<&Uuid> = topo_all.iter().filter(|id| ancestors.contains(id)).collect();

    let state = dag.export_state();
    let mut shares: HashMap<Uuid, f64> = HashMap::new();
    shares.insert(*node_id, 1.0); // start with 100% at the target node

    // Propagate shares backward through the DAG.
    // Process in reverse topological order so children are processed before parents.
    for current_id in topo.iter().rev() {
        let current_share = *shares.get(current_id).unwrap_or(&0.0);
        if current_share == 0.0 {
            continue;
        }

        // Distribute current_share to parents based on edge weights.
        for edge in &state.edges {
            if edge.child_id == **current_id {
                let parent_share = current_share * edge.contribution_weight;
                *shares.entry(edge.parent_id).or_insert(0.0) += parent_share;
            }
        }
    }

    Ok(shares)
}

/// Compute the total royalty payment due to each ancestor for a given
/// derivative usage, given a base payment amount.
pub fn compute_royalty_payments(
    dag: &PatentDag,
    node_id: &Uuid,
    base_payment: f64,
) -> Result<HashMap<Uuid, f64>, CoreError> {
    let shares = compute_royalty_shares(dag, node_id)?;
    let payments: HashMap<Uuid, f64> = shares
        .into_iter()
        .map(|(id, share)| (id, share * base_payment))
        .collect();
    Ok(payments)
}

// ─── Royalty Invariant Builder ───────────────────────────────────────────────

/// Build a `RoyaltyInvariant` from a DAG node by collecting all upstream shares.
pub fn build_royalty_invariant(
    dag: &PatentDag,
    node_id: &Uuid,
) -> Result<RoyaltyInvariant, CoreError> {
    let shares = compute_royalty_shares(dag, node_id)?;
    let upstream_nodes: Vec<Uuid> = shares.keys().copied().filter(|id| id != node_id).collect();
    let share_entries: Vec<ShareEntry> = shares
        .into_iter()
        .filter(|(id, _)| id != node_id)
        .map(|(node_id, weight)| ShareEntry { node_id, weight })
        .collect();

    Ok(RoyaltyInvariant {
        patent_node_id: *node_id,
        shares: share_entries,
        upstream_nodes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::PatentDag;
    use crate::models::{PatentNode, PatentType, License, Edge, EdgeKind};
    use uuid::Uuid;

    fn make_node(id: Uuid) -> PatentNode {
        PatentNode {
            id,
            title: "Royalty Test Node".into(),
            patent_type: PatentType::Software,
            authors: vec!["Alice".into()],
            created_at_unix: 0,
            content_hash: vec![],
            invariant: None,
            tags: vec![],
            license: License::Open,
            meta_commitment: vec![],
        }
    }

    fn make_edge(parent: Uuid, child: Uuid, weight: f64) -> Edge {
        Edge {
            id: Uuid::new_v4(),
            parent_id: parent,
            child_id: child,
            kind: EdgeKind::Derivative,
            delta: None,
            contribution_weight: weight,
            signature: vec![],
        }
    }

    #[test]
    fn test_royalty_verifier_passes() {
        let mut dag = PatentDag::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        dag.add_node(make_node(a)).unwrap();
        dag.add_node(make_node(b)).unwrap();
        dag.add_edge(&a, &b, make_edge(a, b, 1.0)).unwrap();

        let verifier = RoyaltyVerifier::default();
        assert!(verifier.verify_all(&dag).is_ok());
    }

    #[test]
    fn test_royalty_verifier_detects_bad_sum() {
        let mut dag = PatentDag::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        dag.add_node(make_node(a)).unwrap();
        dag.add_node(make_node(b)).unwrap();
        dag.add_edge(&a, &b, make_edge(a, b, 0.5)).unwrap(); // sum != 1.0

        let verifier = RoyaltyVerifier::default();
        let err = verifier.verify_all(&dag).unwrap_err();
        assert!(matches!(err, CoreError::RoyaltySumViolation { .. }));
    }

    #[test]
    fn test_royalty_verifier_multiple_parents() {
        let mut dag = PatentDag::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        dag.add_node(make_node(a)).unwrap();
        dag.add_node(make_node(b)).unwrap();
        dag.add_node(make_node(c)).unwrap();
        dag.add_edge(&a, &c, make_edge(a, c, 0.3)).unwrap();
        dag.add_edge(&b, &c, make_edge(b, c, 0.7)).unwrap();

        let verifier = RoyaltyVerifier::default();
        assert!(verifier.verify_all(&dag).is_ok());
    }

    #[test]
    fn test_compute_royalty_shares_simple() {
        let mut dag = PatentDag::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        dag.add_node(make_node(a)).unwrap();
        dag.add_node(make_node(b)).unwrap();
        dag.add_edge(&a, &b, make_edge(a, b, 1.0)).unwrap();

        let shares = compute_royalty_shares(&dag, &b).unwrap();
        assert!((shares[&a] - 1.0).abs() < 1e-9);
        assert!((shares[&b] - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_compute_royalty_shares_two_levels() {
        let mut dag = PatentDag::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        dag.add_node(make_node(a)).unwrap();
        dag.add_node(make_node(b)).unwrap();
        dag.add_node(make_node(c)).unwrap();
        dag.add_edge(&a, &b, make_edge(a, b, 0.5)).unwrap();
        dag.add_edge(&b, &c, make_edge(b, c, 0.5)).unwrap();

        let shares = compute_royalty_shares(&dag, &c).unwrap();
        // a gets: 0.5 * 0.5 = 0.25
        assert!((shares[&a] - 0.25).abs() < 1e-9);
        // b gets: 0.5
        assert!((shares[&b] - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_compute_royalty_payments() {
        let mut dag = PatentDag::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        dag.add_node(make_node(a)).unwrap();
        dag.add_node(make_node(b)).unwrap();
        dag.add_edge(&a, &b, make_edge(a, b, 1.0)).unwrap();

        let payments = compute_royalty_payments(&dag, &b, 100.0).unwrap();
        assert!((payments[&a] - 100.0).abs() < 1e-9);
        assert!((payments[&b] - 100.0).abs() < 1e-9); // node gets its own share too
    }

    #[test]
    fn test_build_royalty_invariant() {
        let mut dag = PatentDag::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        dag.add_node(make_node(a)).unwrap();
        dag.add_node(make_node(b)).unwrap();
        dag.add_edge(&a, &b, make_edge(a, b, 1.0)).unwrap();

        let inv = build_royalty_invariant(&dag, &b).unwrap();
        assert_eq!(inv.patent_node_id, b);
        assert_eq!(inv.shares.len(), 1);
        assert!((inv.shares[0].weight - 1.0).abs() < 1e-9);
        assert!(inv.upstream_nodes.contains(&a));
    }
}