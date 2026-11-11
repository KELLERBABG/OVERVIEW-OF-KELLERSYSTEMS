//! # Property-Based Tests for Decoupled Fluid IP Network
//!
//! This is the test binary entry point. Run with: `cargo test --test tests`
//! Verifies system invariants under random inputs using `proptest`.

use dfip_core::delta::AuditTrail;
use dfip_core::graph::PatentDag;
use dfip_core::models::*;
use dfip_core::royalties::{compute_royalty_shares, RoyaltyVerifier};
use dfip_core::taint::TaintValue;
use proptest::prelude::*;
use std::collections::{HashSet};
use uuid::Uuid;

// ─── Strategies ──────────────────────────────────────────────────────────────

fn arb_weight() -> impl Strategy<Value = f64> {
    (0u64..u64::MAX).prop_map(|x| x as f64 / u64::MAX as f64)
}

fn arb_node() -> impl Strategy<Value = PatentNode> {
    Just(()).prop_map(|_| PatentNode {
        id: Uuid::new_v4(),
        title: "prop-test".into(),
        patent_type: PatentType::Software,
        authors: vec!["Alice".into()],
        created_at_unix: 0,
        content_hash: vec![],
        invariant: None,
        tags: vec![],
        license: License::Open,
        meta_commitment: vec![],
    })
}

fn arb_nodes(count: usize) -> impl Strategy<Value = Vec<PatentNode>> {
    prop::collection::vec(arb_node(), count..=count)
}

// ─── Property 1: DAG remains acyclic ────────────────────────────────────────

proptest! {
    #[test]
    fn dag_remains_acyclic(nodes in arb_nodes(5)) {
        let mut dag = PatentDag::new();
        let ids: Vec<Uuid> = nodes.iter().map(|n| n.id).collect();
        for node in &nodes { prop_assert!(dag.add_node(node.clone()).is_ok()); }
        for i in 0..ids.len().saturating_sub(1) {
            let e = Edge {
                id: Uuid::new_v4(), parent_id: ids[i], child_id: ids[i+1],
                kind: EdgeKind::Derivative, delta: None,
                contribution_weight: 1.0, signature: vec![],
            };
            prop_assert!(dag.add_edge(&ids[i], &ids[i+1], e).is_ok());
        }
        if let Ok(order) = dag.topological_order_indices() {
            for i in 0..ids.len().saturating_sub(1) {
                let pi = order.iter().position(|&x| x == ids[i]).unwrap();
                let pj = order.iter().position(|&x| x == ids[i+1]).unwrap();
                prop_assert!(pi < pj, "order violation");
            }
        }
    }
}

// ─── Property 2: Cycle prevention ───────────────────────────────────────────

proptest! {
    #[test]
    fn cycle_prevention(nodes in arb_nodes(4)) {
        let mut dag = PatentDag::new();
        let ids: Vec<Uuid> = nodes.iter().map(|n| n.id).collect();
        for node in &nodes { dag.add_node(node.clone()).unwrap(); }
        for i in 0..ids.len().saturating_sub(1) {
            let e = Edge {
                id: Uuid::new_v4(), parent_id: ids[i], child_id: ids[i+1],
                kind: EdgeKind::Derivative, delta: None,
                contribution_weight: 1.0, signature: vec![],
            };
            dag.add_edge(&ids[i], &ids[i+1], e).unwrap();
        }
        let back = Edge {
            id: Uuid::new_v4(), parent_id: ids[ids.len()-1], child_id: ids[0],
            kind: EdgeKind::Derivative, delta: None,
            contribution_weight: 1.0, signature: vec![],
        };
        prop_assert!(dag.add_edge(&ids[ids.len()-1], &ids[0], back).is_err());
    }
}

// ─── Property 3: Royalty sum invariants ─────────────────────────────────────

proptest! {
    #[test]
    fn royalty_sum_invariant(w1 in arb_weight(), w2 in arb_weight()) {
        let total = w1 + w2;
        if total <= 0.0 { return Ok(()); }
        let wa = w1 / total; let wb = w2 / total;
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
            contribution_weight: wa, signature: vec![],
        }).unwrap();
        dag.add_edge(&b, &c, Edge {
            id: Uuid::new_v4(), parent_id: b, child_id: c,
            kind: EdgeKind::Derivative, delta: None,
            contribution_weight: wb, signature: vec![],
        }).unwrap();
        let v = RoyaltyVerifier::new(1e-6);
        prop_assert!(v.verify_all(&dag).is_ok());
        let shares = compute_royalty_shares(&dag, &c).unwrap();
        // shares includes c (1.0) plus ancestor shares; ancestor shares must sum to 1.0
        let ancestor_sum: f64 = shares.iter().filter(|(k, _)| **k != c).map(|(_, v)| v).sum();
        prop_assert!((ancestor_sum - 1.0).abs() < 1e-6, "ancestor shares sum to {}", ancestor_sum);
    }
}

// ─── Property 4: Taint lattice laws ─────────────────────────────────────────

fn to_taint(x: u64) -> TaintValue {
    match x {
        0 => TaintValue::Unlicensed,
        1 => TaintValue::Licensed(Uuid::from_u64_pair(1, 0)),
        2 => TaintValue::Licensed(Uuid::from_u64_pair(2, 0)),
        3 => TaintValue::Multiple(HashSet::from([Uuid::from_u64_pair(1, 0)])),
        _ => TaintValue::Multiple(HashSet::from([Uuid::from_u64_pair(1, 0), Uuid::from_u64_pair(2, 0)])),
    }
}

proptest! {
    #[test]
    fn taint_associative(a in 0..5u64, b in 0..5u64, c in 0..5u64) {
        let (ta, tb, tc) = (to_taint(a), to_taint(b), to_taint(c));
        prop_assert_eq!(ta.join(&tb).join(&tc), ta.join(&(tb.join(&tc))));
    }
}

proptest! {
    #[test]
    fn taint_commutative(a in 0..5u64, b in 0..5u64) {
        let (ta, tb) = (to_taint(a), to_taint(b));
        prop_assert_eq!(ta.join(&tb), tb.join(&ta));
    }
}

proptest! {
    #[test]
    fn taint_idempotent(a in 0..5u64) {
        let ta = to_taint(a);
        prop_assert_eq!(ta.join(&ta), ta);
    }
}

// ─── Property 5: Audit trail integrity ──────────────────────────────────────

proptest! {
    #[test]
    fn audit_trail_verifies(formulas in prop::collection::vec("[a-z]+", 1..5)) {
        let mut trail = AuditTrail::new();
        for expr in &formulas {
            let dc = dfip_core::DeltaConstraint {
                original_node_id: Uuid::new_v4(),
                derivative_node_id: Uuid::new_v4(),
                original_invariant_hash: vec![],
                formula_smt_lib2: expr.as_bytes().to_vec(),
                assumptions: vec![],
            };
            trail.push(dc);
        }
        prop_assert!(trail.verify(), "clean trail must verify");
    }
}

// ─── Property 6: Topological order ──────────────────────────────────────────

proptest! {
    #[test]
    fn topological_order(count in 2..6usize) {
        let mut dag = PatentDag::new();
        let ids: Vec<Uuid> = (0..count).map(|_| Uuid::new_v4()).collect();
        for &id in &ids {
            dag.add_node(PatentNode {
                id, title: "".into(), patent_type: PatentType::Software,
                authors: vec![], created_at_unix: 0, content_hash: vec![],
                invariant: None, tags: vec![], license: License::Open, meta_commitment: vec![],
            }).unwrap();
        }
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
                prop_assert!(pi < pj, "order violated");
            }
        }
    }
}