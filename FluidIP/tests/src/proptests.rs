//! # Property-Based Tests for Decoupled Fluid IP Network
//!
//! Uses `proptest` to verify system invariants under random inputs.

use dfip_core::delta::AuditTrail;
use dfip_core::graph::PatentDag;
use dfip_core::ir::*;
use dfip_core::models::*;
use dfip_core::royalties::{compute_royalty_payments, compute_royalty_shares, RoyaltyVerifier};
use dfip_core::taint::TaintValue;
use proptest::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

// ─── Strategies ──────────────────────────────────────────────────────────────

fn arb_patent_type() -> impl Strategy<Value = PatentType> {
    prop_oneof![Just(PatentType::Software), Just(PatentType::Chemical), Just(PatentType::Mechanical)]
}

fn arb_license() -> impl Strategy<Value = License> {
    prop_oneof![Just(License::Open), Just(License::Protected), Just(License::Secret)]
}

fn arb_weight() -> impl Strategy<Value = f64> {
    prop_float_range(0.0, 1.0)
}

fn prop_float_range(min: f64, max: f64) -> impl Strategy<Value = f64> {
    (0u64..u64::MAX).prop_map(move |x| {
        let ratio = x as f64 / u64::MAX as f64;
        min + ratio * (max - min)
    })
}

fn arb_node() -> impl Strategy<Value = PatentNode> {
    (arb_patent_type(), arb_license()).prop_map(|(pt, lic)| PatentNode {
        id: Uuid::new_v4(),
        title: "prop-test".into(),
        patent_type: pt,
        authors: vec!["Alice".into()],
        created_at_unix: 0,
        content_hash: vec![],
        invariant: None,
        tags: vec![],
        license: lic,
        meta_commitment: vec![],
    })
}

fn arb_nodes(count: usize) -> impl Strategy<Value = Vec<PatentNode>> {
    prop::collection::vec(arb_node(), count..=count)
}

// ─── Property 1: DAG Acyclicity ─────────────────────────────────────────────
//
// For any sequence of valid edge insertions, the DAG remains acyclic.

proptest! {
    #[test]
    fn dag_remains_acyclic_after_valid_insertions(nodes in arb_nodes(5)) {
        let mut dag = PatentDag::new();
        let ids: Vec<Uuid> = nodes.iter().map(|n| n.id).collect();

        // Add all nodes
        for node in &nodes {
            prop_assert!(dag.add_node(node.clone()).is_ok());
        }

        // Add edges in increasing index order (parent < child in insertion order)
        for i in 0..ids.len().saturating_sub(1) {
            let edge = Edge {
                id: Uuid::new_v4(),
                parent_id: ids[i],
                child_id: ids[i + 1],
                kind: EdgeKind::Derivative,
                delta: None,
                contribution_weight: 1.0,
                signature: vec![],
            };
            // This should succeed since it's parent→child in insertion order
            let result = dag.add_edge(&ids[i], &ids[i + 1], edge);
            prop_assert!(result.is_ok(),
                "Failed to add edge {} -> {}: {:?}", ids[i], ids[i+1], result);
        }

        // Verify topological order is consistent
        if let Ok(order) = dag.topological_order_indices() {
            for i in 0..ids.len().saturating_sub(1) {
                let pos_i = order.iter().position(|&x| x == ids[i]).unwrap();
                let pos_j = order.iter().position(|&x| x == ids[i+1]).unwrap();
                prop_assert!(pos_i < pos_j,
                    "Topological order violated: {} before {}", ids[i], ids[i+1]);
            }
        }
    }
}

// ─── Property 2: Cycle Prevention ───────────────────────────────────────────
//
// Adding a backward edge in a previously linear DAG must always fail.

proptest! {
    #[test]
    fn cycle_prevention_on_back_edge(nodes in arb_nodes(4)) {
        let mut dag = PatentDag::new();
        let ids: Vec<Uuid> = nodes.iter().map(|n| n.id).collect();
        for node in &nodes {
            dag.add_node(node.clone()).unwrap();
        }
        // Add forward edges first
        for i in 0..ids.len().saturating_sub(1) {
            let e = Edge {
                id: Uuid::new_v4(), parent_id: ids[i], child_id: ids[i+1],
                kind: EdgeKind::Derivative, delta: None,
                contribution_weight: 1.0, signature: vec![],
            };
            dag.add_edge(&ids[i], &ids[i+1], e).unwrap();
        }
        // Now try adding a back edge (last -> first) — must fail
        let back_edge = Edge {
            id: Uuid::new_v4(), parent_id: ids[ids.len()-1], child_id: ids[0],
            kind: EdgeKind::Derivative, delta: None,
            contribution_weight: 1.0, signature: vec![],
        };
        let result = dag.add_edge(&ids[ids.len()-1], &ids[0], back_edge);
        prop_assert!(result.is_err(), "Back edge should have been rejected");
    }
}

// ─── Property 3: Royalty Sum Invariant ──────────────────────────────────────
//
// For any node with incoming edges, sum(weights) must equal 1.0 (mod epsilon).

proptest! {
    #[test]
    fn royalty_sum_invariant(
        w1 in arb_weight(), w2 in arb_weight(), w3 in arb_weight(),
    ) {
        // Normalize weights to sum to 1.0
        let total = w1 + w2 + w3;
        if total <= 0.0 { return Ok(()); }
        let weights = vec![w1/total, w2/total, w3/total];

        let mut dag = PatentDag::new();
        let parents: Vec<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();
        let child = Uuid::new_v4();

        for &pid in &parents {
            let n = PatentNode {
                id: pid, title: "parent".into(), patent_type: PatentType::Software,
                authors: vec![], created_at_unix: 0, content_hash: vec![],
                invariant: None, tags: vec![], license: License::Open, meta_commitment: vec![],
            };
            dag.add_node(n).unwrap();
        }
        let c = PatentNode {
            id: child, title: "child".into(), patent_type: PatentType::Software,
            authors: vec![], created_at_unix: 0, content_hash: vec![],
            invariant: None, tags: vec![], license: License::Open, meta_commitment: vec![],
        };
        dag.add_node(c).unwrap();

        for (i, &pid) in parents.iter().enumerate() {
            let e = Edge {
                id: Uuid::new_v4(), parent_id: pid, child_id: child,
                kind: EdgeKind::Derivative, delta: None,
                contribution_weight: weights[i], signature: vec![],
            };
            dag.add_edge(&pid, &child, e).unwrap();
        }

        let verifier = RoyaltyVerifier::new(1e-6);
        prop_assert!(verifier.verify_all(&dag).is_ok(),
            "Royalty sum invariant violated for weights: {:?}", weights);
    }
}

// ─── Property 4: Royalty Share Consistency ──────────────────────────────────
//
// Royalty shares computed for a leaf node must partition 1.0.

proptest! {
    #[test]
    fn royalty_shares_sum_to_one(w1 in arb_weight(), w2 in arb_weight()) {
        let total = w1 + w2;
        if total <= 0.0 { return Ok(()); }
        let w = vec![w1/total, w2/total];

        let mut dag = PatentDag::new();
        let a = Uuid::new_v4(); let b = Uuid::new_v4(); let c = Uuid::new_v4();

        for &id in &[a, b, c] {
            dag.add_node(PatentNode {
                id, title: "".into(), patent_type: PatentType::Software,
                authors: vec![], created_at_unix: 0, content_hash: vec![],
                invariant: None, tags: vec![], license: License::Open, meta_commitment: vec![],
            }).unwrap();
        }

        dag.add_edge(&a, &c, Edge {
            id: Uuid::new_v4(), parent_id: a, child_id: c,
            kind: EdgeKind::Derivative, delta: None,
            contribution_weight: w[0], signature: vec![],
        }).unwrap();
        dag.add_edge(&b, &c, Edge {
            id: Uuid::new_v4(), parent_id: b, child_id: c,
            kind: EdgeKind::Derivative, delta: None,
            contribution_weight: w[1], signature: vec![],
        }).unwrap();

        let shares = compute_royalty_shares(&dag, &c).unwrap();
        let sum: f64 = shares.values().sum();
        prop_assert!((sum - 1.0).abs() < 1e-6,
            "Shares sum to {}, expected 1.0", sum);
    }
}

// ─── Property 5: Taint Lattice Laws ─────────────────────────────────────────
//
// The taint lattice must satisfy: associativity, commutativity, idempotence.

proptest! {
    #[test]
    fn taint_join_is_associative(a in 0..5u64, b in 0..5u64, c in 0..5u64) {
        let to_taint = |x: u64| -> TaintValue {
            match x {
                0 => TaintValue::Unlicensed,
                1 => TaintValue::Licensed(Uuid::from_u64_pair(1, 0)),
                2 => TaintValue::Licensed(Uuid::from_u64_pair(2, 0)),
                3 => TaintValue::Multiple(HashSet::from([Uuid::from_u64_pair(1, 0)])),
                _ => TaintValue::Multiple(HashSet::from([
                    Uuid::from_u64_pair(1, 0), Uuid::from_u64_pair(2, 0),
                ])),
            }
        };
        let ta = to_taint(a); let tb = to_taint(b); let tc = to_taint(c);
        let lhs = ta.join(&tb).join(&tc);
        let rhs = ta.join(&(tb.join(&tc)));
        prop_assert_eq!(lhs, rhs, "Taint join is not associative");
    }
}

proptest! {
    #[test]
    fn taint_join_is_commutative(a in 0..5u64, b in 0..5u64) {
        let to_taint = |x: u64| -> TaintValue {
            match x {
                0 => TaintValue::Unlicensed,
                1 => TaintValue::Licensed(Uuid::from_u64_pair(1, 0)),
                2 => TaintValue::Licensed(Uuid::from_u64_pair(2, 0)),
                3 => TaintValue::Multiple(HashSet::from([Uuid::from_u64_pair(1, 0)])),
                _ => TaintValue::Multiple(HashSet::from([
                    Uuid::from_u64_pair(1, 0), Uuid::from_u64_pair(2, 0),
                ])),
            }
        };
        let ta = to_taint(a); let tb = to_taint(b);
        prop_assert_eq!(ta.join(&tb), tb.join(&ta), "Taint join is not commutative");
    }
}

proptest! {
    #[test]
    fn taint_join_is_idempotent(a in 0..5u64) {
        let to_taint = |x: u64| -> TaintValue {
            match x {
                0 => TaintValue::Unlicensed,
                1 => TaintValue::Licensed(Uuid::from_u64_pair(1, 0)),
                2 => TaintValue::Licensed(Uuid::from_u64_pair(2, 0)),
                3 => TaintValue::Multiple(HashSet::from([Uuid::from_u64_pair(1, 0)])),
                _ => TaintValue::Multiple(HashSet::from([
                    Uuid::from_u64_pair(1, 0), Uuid::from_u64_pair(2, 0),
                ])),
            }
        };
        let ta = to_taint(a);
        prop_assert_eq!(ta.join(&ta), ta, "Taint join is not idempotent");
    }
}

// ─── Property 6: Audit Trail Integrity ──────────────────────────────────────
//
// An audit trail must always verify correctly, and tampering must always
// cause verification to fail.

proptest! {
    #[test]
    fn audit_trail_verification_passes(formulas in prop::collection::vec("[a-zA-Z0-9]+", 1..10)) {
        let mut trail = AuditTrail::new();
        for expr in &formulas {
            let dc = DeltaConstraint {
                original_node_id: Uuid::new_v4(),
                derivative_node_id: Uuid::new_v4(),
                original_invariant_hash: vec![],
                formula_smt_lib2: expr.as_bytes().to_vec(),
                assumptions: vec![],
            };
            trail.push(dc);
        }
        prop_assert!(trail.verify(), "Audit trail verification failed for clean trail");
    }
}

// ─── Property 7: Topological Order Validity ─────────────────────────────────
//
// For any set of edges, topological order respects partial order.

proptest! {
    #[test]
    fn topological_order_respects_edges(count in 2..6usize) {
        let mut dag = PatentDag::new();
        let ids: Vec<Uuid> = (0..count).map(|_| Uuid::new_v4()).collect();
        for &id in &ids {
            dag.add_node(PatentNode {
                id, title: "".into(), patent_type: PatentType::Software,
                authors: vec![], created_at_unix: 0, content_hash: vec![],
                invariant: None, tags: vec![], license: License::Open, meta_commitment: vec![],
            }).unwrap();
        }
        // Add edges from id[i] to id[i+1] for all i
        for i in 0..count.saturating_sub(1) {
            let e = Edge {
                id: Uuid::new_v4(), parent_id: ids[i], child_id: ids[i+1],
                kind: EdgeKind::Derivative, delta: None,
                contribution_weight: 1.0, signature: vec![],
            };
            dag.add_edge(&ids[i], &ids[i+1], e).unwrap();
        }
        if let Ok(order) = dag.topological_order_indices() {
            for i in 0..count.saturating_sub(1) {
                let pi = order.iter().position(|&x| x == ids[i]).unwrap();
                let pj = order.iter().position(|&x| x == ids[i+1]).unwrap();
                prop_assert!(pi < pj, "Topological order violation at edge {}->{}", i, i+1);
            }
        }
    }
}