use mesh_credit::constraints::{ConstraintEngine, Exposure};
use mesh_credit::loan::calculate_interest_rate;
use mesh_graph_core::algorithms;
use mesh_graph_core::entropy;
use mesh_graph_core::graph::SocialGraph;
use mesh_graph_core::types::*;
use mesh_settlement::contract::SettlementEngine;
use mesh_settlement::pool::CreditPool;
use mesh_smpc::solvency::{prepare_solvency_shares, verify_solvency};
use mesh_wot::identity::Identity;
use mesh_wot::limits::VouchingLimits;
use mesh_wot::nullifier::NullifierRegistry;
use mesh_zkp::proof::ClusterMembershipProof;

fn build_chain_directed(size: usize) -> SocialGraph {
    let mut graph = SocialGraph::new();
    for i in 0..size {
        graph.add_node(NodeData {
            id: format!("u{}", i),
            display_name: format!("User {}", i),
            created_at: i as i64,
        });
    }
    for i in 1..size {
        let _ = graph.add_edge(
            &format!("u{}", i - 1),
            &format!("u{}", i),
            EdgeData {
                relationship: RelationshipType::Friend,
                trust_weight: 0.8,
                created_at: i as i64,
                expires_at: None,
            },
        );
    }
    graph
}

fn build_well_connected_graph() -> SocialGraph {
    // A graph where all 5 nodes vouch for each other with reciprocity — low sybil score
    let mut graph = SocialGraph::new();
    for i in 0..5 {
        let id = format!("v{}", i);
        graph.add_node(NodeData {
            id: id.clone(),
            display_name: format!("Node {}", i),
            created_at: i as i64,
        });
    }
    // Fully bidirectional connections
    for i in 0..5 {
        for j in 0..5 {
            if i != j {
                let _ = graph.add_edge(
                    &format!("v{}", i),
                    &format!("v{}", j),
                    EdgeData {
                        relationship: RelationshipType::Friend,
                        trust_weight: 0.7,
                        created_at: i as i64,
                        expires_at: None,
                    },
                );
            }
        }
    }
    graph
}

fn build_rich_graph() -> SocialGraph {
    // Build a dense legitimate community with high reciprocity
    let mut graph = SocialGraph::new();
    for i in 0..20 {
        let id = format!("n{}", i);
        graph.add_node(NodeData {
            id: id.clone(),
            display_name: format!("Node {}", i),
            created_at: i as i64,
        });
    }
    // Dense core: n0-n9 are all connected bidirectionally (high cluster density, high reciprocity)
    for i in 0..10 {
        for j in 0..10 {
            if i != j {
                let _ = graph.add_edge(
                    &format!("n{}", i),
                    &format!("n{}", j),
                    EdgeData {
                        relationship: RelationshipType::Friend,
                        trust_weight: 0.7,
                        created_at: 0,
                        expires_at: None,
                    },
                );
            }
        }
    }
    // n10-n19 are periphery with bidirectional links to core
    for i in 10..20 {
        for j in 0..5 {
            let _ = graph.add_edge(
                &format!("n{}", i),
                &format!("n{}", j),
                EdgeData {
                    relationship: RelationshipType::Colleague,
                    trust_weight: 0.5,
                    created_at: 0,
                    expires_at: None,
                },
            );
            let _ = graph.add_edge(
                &format!("n{}", j),
                &format!("n{}", i),
                EdgeData {
                    relationship: RelationshipType::Colleague,
                    trust_weight: 0.5,
                    created_at: 0,
                    expires_at: None,
                },
            );
        }
    }
    graph
}

#[test]
fn test_sybil_attack_detection() {
    let mut graph = build_rich_graph();

    // Chain-based Sybil cluster (low cluster density, one-directional)
    let sybil_ids: Vec<String> = (0..15).map(|i| format!("sybil_{}", i)).collect();
    for id in &sybil_ids {
        graph.add_node(NodeData {
            id: id.clone(),
            display_name: format!("Sybil {}", id),
            created_at: 0,
        });
    }
    // Chain structure — no triangles, low reciprocity = high sybil signal
    for i in 1..15 {
        let _ = graph.add_edge(
            &sybil_ids[i - 1],
            &sybil_ids[i],
            EdgeData {
                relationship: RelationshipType::Friend,
                trust_weight: 0.9,
                created_at: 0,
                expires_at: None,
            },
        );
    }
    // Single weak link to legitimate graph
    let _ = graph.add_edge(
        "sybil_0",
        "n0",
        EdgeData {
            relationship: RelationshipType::Friend,
            trust_weight: 0.3,
            created_at: 0,
            expires_at: None,
        },
    );

    let clusters = entropy::detect_sybil_clusters(&graph, 0.3);
    assert!(
        !clusters.is_empty(),
        "should detect at least one Sybil cluster"
    );

    let sybil_scores: Vec<f64> = sybil_ids
        .iter()
        .filter_map(|id| entropy::sybil_score(&graph, id))
        .collect();
    assert!(!sybil_scores.is_empty());
    let avg_sybil = sybil_scores.iter().sum::<f64>() / sybil_scores.len() as f64;

    let legit_ids: Vec<String> = (0..5).map(|i| format!("n{}", i)).collect();
    let legit_scores: Vec<f64> = legit_ids
        .iter()
        .filter_map(|id| entropy::sybil_score(&graph, id))
        .collect();
    let avg_legit = if legit_scores.is_empty() {
        0.0
    } else {
        legit_scores.iter().sum::<f64>() / legit_scores.len() as f64
    };

    eprintln!("avg_sybil={:.4}, avg_legit={:.4}", avg_sybil, avg_legit);
    assert!(
        avg_sybil > avg_legit,
        "Sybil scores ({:.4}) should exceed legitimate scores ({:.4})",
        avg_sybil,
        avg_legit
    );
}

#[test]
fn test_double_borrow_prevention() {
    let identity = Identity::generate("alice".to_string(), "Alice".to_string());
    let mut registry = NullifierRegistry::new();

    let nonce1 = b"loan_1";
    let result1 = registry.register_nullifier("alice", &identity.public_key_bytes, nonce1);
    assert!(result1.is_ok(), "first registration should succeed");

    let result2 = registry.register_nullifier("alice", &identity.public_key_bytes, nonce1);
    assert!(
        result2.is_err(),
        "second registration with same nonce should fail"
    );

    let nonce2 = b"loan_2";
    let result3 = registry.check_nullifier("alice", &identity.public_key_bytes, nonce2);
    assert!(result3.is_ok(), "different nonce should be allowed");

    assert_eq!(
        registry.used_count(),
        1,
        "exactly one nullifier should be registered"
    );
}

#[test]
fn test_vouching_limits_enforcement() {
    let mut graph = SocialGraph::new();
    let limits = VouchingLimits {
        max_depth: 3,
        max_breadth: 2,
        min_weight: 0.1,
        decay_factor: 0.5,
        max_chain_trust: 0.95,
    };

    for i in 0..5 {
        graph.add_node(NodeData {
            id: format!("u{}", i),
            display_name: format!("User {}", i),
            created_at: i as i64,
        });
    }

    assert!(
        limits.can_vouch(&graph, "u0").is_ok(),
        "initial vouch should be allowed"
    );

    let _ = graph.add_edge(
        "u0",
        "u1",
        EdgeData {
            relationship: RelationshipType::Friend,
            trust_weight: 0.8,
            created_at: 0,
            expires_at: None,
        },
    );
    let _ = graph.add_edge(
        "u0",
        "u2",
        EdgeData {
            relationship: RelationshipType::Friend,
            trust_weight: 0.8,
            created_at: 0,
            expires_at: None,
        },
    );

    assert!(
        limits.can_vouch(&graph, "u0").is_err(),
        "should reject when max_breadth=2 is reached"
    );

    let trust_map = limits.propagate_trust(&graph, "u0");
    for (_trust, path) in trust_map.values() {
        assert!(
            path.len() - 1 <= limits.max_depth,
            "path length {} exceeds max_depth {}",
            path.len() - 1,
            limits.max_depth
        );
    }
}

#[test]
fn test_smpc_solvency_honest_majority() {
    let (parties, stmt) = prepare_solvency_shares("bob", 5000, 2000, 5, 3).unwrap();
    assert_eq!(stmt.num_parties, 5);
    assert_eq!(stmt.threshold, 3);

    let result = verify_solvency(&stmt.entity_id, &parties[..3], 3).unwrap();
    assert!(result.solvent);
    assert_eq!(result.net_position, 3000);
}

#[test]
fn test_smpc_insolvent_rejected() {
    let result = prepare_solvency_shares("carol", 1000, 5000, 5, 3);
    assert!(result.is_err(), "insolvent entity should be rejected");
}

#[test]
fn test_zkp_cluster_membership() {
    let identities: Vec<Vec<u8>> = (0..10)
        .map(|i| format!("user_{}", i).into_bytes())
        .collect();
    let scores: Vec<f64> = (0..10).map(|i| 0.1 + i as f64 * 0.05).collect();

    let proof =
        ClusterMembershipProof::prove("user_3", &identities[3], &identities, &scores, 0.1).unwrap();
    assert_eq!(proof.cluster_size, 10);
    assert!((proof.avg_reputation - 0.325).abs() < 0.001);

    let valid = proof.verify(5, 0.1).unwrap();
    assert!(
        valid,
        "proof should verify for min_size=5, min_rep=0.1 (stored=0.1)"
    );

    let too_strict = proof.verify(5, 0.2).unwrap();
    assert!(
        !too_strict,
        "proof should fail for min_rep=0.2 > stored=0.1"
    );

    let invalid = proof.verify(15, 0.1).unwrap();
    assert!(!invalid, "proof should fail for min_size=15 > actual=10");
}

#[test]
fn test_credit_constraint_evaluation() {
    let graph = build_well_connected_graph();
    let exposures = vec![Exposure {
        borrower: "v0".to_string(),
        outstanding: 100.0,
        active: true,
    }];

    let (allowed, constraints) =
        ConstraintEngine::check_borrower(&graph, "v0", 500.0, &exposures).unwrap();

    let failed_constraints: Vec<String> = constraints
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.evaluate(&graph, &exposures).unwrap_or(false))
        .map(|(i, _)| format!("constraint #{}", i + 1))
        .collect();
    eprintln!("Failed: {:?}", failed_constraints);

    assert!(
        allowed,
        "u0 should meet constraints for 500: {:?}",
        failed_constraints
    );
}

#[test]
fn test_credit_pool_operations() {
    let mut pool = CreditPool::new("pool_1".to_string(), "Test Pool".to_string(), 0.05);
    pool.deposit("lender_1", 10000.0).unwrap();
    pool.deposit("lender_2", 5000.0).unwrap();
    assert!((pool.total_liquidity - 15000.0).abs() < 0.001);

    pool.allocate(8000.0).unwrap();
    assert!((pool.available_liquidity() - 7000.0).abs() < 0.001);
    assert!((pool.utilization_rate() - 8000.0 / 15000.0).abs() < 0.001);

    pool.repay_allocation(3000.0);
    assert!((pool.allocated - 5000.0).abs() < 0.001);

    let withdrawn = pool.withdraw("lender_1", 2000.0).unwrap();
    assert!((withdrawn - 2000.0).abs() < 0.001);
    assert_eq!(pool.provider_count(), 2);
}

#[test]
fn test_interest_rate_model() {
    let mut graph = build_well_connected_graph();
    // Add truly isolated nodes
    for i in 0..5 {
        graph.add_node(NodeData {
            id: format!("isolated_{}", i),
            display_name: format!("Isolated {}", i),
            created_at: i as i64,
        });
    }

    let trusted_rate = calculate_interest_rate(&graph, "v0", 0.05);
    let isolated_rate = calculate_interest_rate(&graph, "isolated_0", 0.05);
    assert!(
        trusted_rate < isolated_rate,
        "trusted node should get better rate: {:.4} vs {:.4}",
        trusted_rate,
        isolated_rate
    );
}

#[test]
fn test_taint_tracker_flow() {
    let mut tracker = mesh_reputation::taint::TaintTracker::new();
    tracker.record_repayment("alice", "bob", 500.0);
    assert!((tracker.net_flow("alice") - (-500.0)).abs() < 0.001);
    assert!((tracker.net_flow("bob") - 500.0).abs() < 0.001);

    tracker.record_repayment("alice", "bob", 300.0);
    assert!((tracker.net_flow("alice") - (-800.0)).abs() < 0.001);

    tracker.record_repayment("bob", "charlie", 200.0);
    assert!((tracker.net_flow("bob") - 600.0).abs() < 0.001);
}

#[test]
fn test_settlement_engine_end_to_end() {
    let mut pool = CreditPool::new("pool_1".to_string(), "Growth Pool".to_string(), 0.06);
    pool.deposit("lender_1", 50000.0).unwrap();

    let graph = build_well_connected_graph();
    let exposures = vec![];

    let result = SettlementEngine::auto_issue_from_pool(
        &mut pool,
        &graph,
        "v0",
        10000.0,
        "business expansion",
        &exposures,
    );
    assert!(
        result.approved,
        "settlement engine should approve: {}",
        result.reason
    );
    assert!((result.principal - 10000.0).abs() < 0.001);
    assert!((pool.allocated - 10000.0).abs() < 0.001);
}

#[test]
fn test_pagerank_trust_propagation() {
    let graph = build_chain_directed(10);
    let pr = algorithms::compute_pagerank(&graph, 0.85, 100, 1e-6);

    assert_eq!(pr.len(), 10);

    let u0_score = pr.get("u0").copied().unwrap_or(0.0);
    let u9_score = pr.get("u9").copied().unwrap_or(0.0);

    assert!(
        u9_score > u0_score,
        "leaf u9 should have higher PageRank (more incoming links) than root u0: {:.6} vs {:.6}",
        u9_score,
        u0_score
    );

    let sum: f64 = pr.values().sum();
    assert!(
        (sum - 1.0).abs() < 0.01,
        "PageRank should sum to ~1.0, got {:.4}",
        sum
    );
}

#[test]
fn test_graph_clustering_coefficient() {
    let mut graph = build_chain_directed(5);
    for i in 0..4 {
        let _ = graph.add_edge(
            &format!("u{}", i),
            &format!("u{}", i + 1),
            EdgeData {
                relationship: RelationshipType::Friend,
                trust_weight: 0.7,
                created_at: 0,
                expires_at: None,
            },
        );
    }
    let _ = graph.add_edge(
        "u0",
        "u4",
        EdgeData {
            relationship: RelationshipType::Friend,
            trust_weight: 0.5,
            created_at: 0,
            expires_at: None,
        },
    );

    let avg_cc = algorithms::average_clustering(&graph);
    assert!(
        avg_cc >= 0.0,
        "clustering coefficient should be non-negative"
    );
}
