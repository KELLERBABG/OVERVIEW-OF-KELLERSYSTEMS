// ---------------------------------------------------------------------------
// Test 1: SSS split + reconstruct (existing)
// ---------------------------------------------------------------------------
use rand::thread_rng;
use smpc_engine::add::secure_add;
use smpc_engine::types::SecretShare;
use sss::Share;
use sss::field::random_scalar;
use sss::sss::{reconstruct, split_secret};

#[test]
fn test_end_to_end_sss_add_reconstruct() {
    let mut rng = thread_rng();
    let a = random_scalar(&mut rng);
    let b = random_scalar(&mut rng);
    let a_shares = split_secret(a, 5, 3).unwrap();
    let b_shares = split_secret(b, 5, 3).unwrap();
    let sum_shares: Vec<Share> = a_shares
        .into_iter()
        .zip(b_shares)
        .map(|(a, b)| secure_add(&SecretShare::new(a), &SecretShare::new(b)).share)
        .collect();
    let recovered = reconstruct(&sum_shares[..3]).unwrap();
    assert_eq!(recovered, a + b);
}

// ---------------------------------------------------------------------------
// Test 2: Full order lifecycle
// ---------------------------------------------------------------------------
use ark_bn254::Fr as BnFr;
use matching_engine::engine::MatchingEngine;
use matching_engine::types::{Order, OrderSide};
use settlement::engine::SettlementEngine;

fn make_order(id: u64, side: OrderSide, price: u64, qty: u64, ts: u64) -> Order {
    Order::new(
        id,
        id,
        side,
        Share::new(1, BnFr::from(price)),
        Share::new(1, BnFr::from(qty)),
        Share::new(1, BnFr::from(ts)),
    )
}

#[test]
fn test_full_order_lifecycle() {
    let mut engine = MatchingEngine::new();
    let mut settlement = SettlementEngine::new();

    let bid = make_order(1, OrderSide::Bid, 100, 10, 1);
    let ask = make_order(2, OrderSide::Ask, 90, 10, 2);

    let _ = engine.submit_order(bid.clone()).unwrap();
    let matches = engine.submit_order(ask.clone()).unwrap();

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].buy_order_id, 1);
    assert_eq!(matches[0].sell_order_id, 2);

    for m in &matches {
        let proof = settlement::types::SettlementProof::new(
            m.buy_order_id,
            &[0u8; 32],
            &[1u8; 64],
            m.buy_order_id,
            m.sell_order_id,
            &m.price.value.to_string().into_bytes(),
            &m.quantity.value.to_string().into_bytes(),
        );
        let record = settlement.submit_proof(proof).unwrap();
        assert_eq!(record.status, settlement::types::SettlementStatus::Pending);
    }

    let settled = settlement.settle(1).unwrap();
    assert_eq!(settled.status, settlement::types::SettlementStatus::Settled);
    assert!(settled.settled_at.is_some());

    let status = settlement.get_status(1).unwrap();
    assert_eq!(status, settlement::types::SettlementStatus::Settled);
}

// ---------------------------------------------------------------------------
// Test 3: BFT consensus
// ---------------------------------------------------------------------------
use bft::consensus::Consensus;
use bft::types::{ConsensusPhase, MessageType, NodeId};
use uuid::Uuid;

fn setup_consensus(count: usize) -> Vec<Consensus> {
    let ids: Vec<NodeId> = (0..count).map(|_| Uuid::new_v4()).collect();
    ids.iter()
        .map(|id| Consensus::new(*id, ids.clone(), "test_sk".to_string()))
        .collect()
}

#[test]
fn test_bft_consensus() {
    let mut nodes = setup_consensus(4);

    let proposal = nodes[0].propose(b"trade_42").unwrap();
    assert_eq!(proposal.msg_type, MessageType::PrePrepare);

    let non_leader = nodes[1].propose(b"trade_99");
    assert!(non_leader.is_err());

    let prepares: Vec<_> = (1..4)
        .map(|i| nodes[i].receive_pre_prepare(&proposal).unwrap())
        .collect();

    let mut commits = Vec::new();
    for prepare in &prepares {
        for node in nodes.iter_mut() {
            if let Some(commit) = node.receive_prepare(prepare).unwrap() {
                commits.push(commit);
            }
        }
    }

    for commit in &commits {
        for node in nodes.iter_mut() {
            let _ = node.receive_commit(commit);
        }
    }

    for (i, node) in nodes.iter().enumerate() {
        assert!(node.is_committed(), "node {} should be committed", i);
    }
    for (i, node) in nodes.iter().enumerate() {
        assert_eq!(
            node.state.phase,
            ConsensusPhase::Committed,
            "node {} phase mismatch",
            i
        );
    }
}

// ---------------------------------------------------------------------------
// Test 4: Commit-reveal + front-running protection
// ---------------------------------------------------------------------------
use front_running::commit_reveal::CommitReveal;
use front_running::types::OrderStatus;

#[test]
fn test_commit_reveal_integrity() {
    let mut cr = CommitReveal::new();

    let payload1 = b"limit_order_100_eth_10@2000";
    let payload2 = b"limit_order_200_btc_1@30000";

    let (order1, opening1) = cr.commit(payload1, 1, 1000);
    let (order2, opening2) = cr.commit(payload2, 2, 1001);

    assert_eq!(
        cr.status(&order1.commitment_hash),
        Some(OrderStatus::Committed)
    );
    assert_eq!(
        cr.status(&order2.commitment_hash),
        Some(OrderStatus::Committed)
    );

    let result = cr.reveal(&order1.commitment_hash, payload1, &opening1);
    assert!(result.is_ok());
    assert_eq!(
        cr.status(&order1.commitment_hash),
        Some(OrderStatus::Revealed)
    );

    let result = cr.reveal(&order2.commitment_hash, b"wrong_payload", &opening2);
    assert!(result.is_err());

    let result = cr.reveal(&order1.commitment_hash, payload1, &opening1);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Test 5: Reputation system
// ---------------------------------------------------------------------------
use node::reputation::ReputationSystem;

#[test]
fn test_reputation_scoring() {
    let mut system = ReputationSystem::new(0.95);
    let node_a = Uuid::new_v4();
    let node_b = Uuid::new_v4();

    assert!((system.score(&node_a) - 0.5).abs() < 1e-9);

    for _ in 0..10 {
        system.record_success(node_a);
    }
    assert!(system.score(&node_a) > 0.9);
    assert!(system.is_trusted(&node_a, 0.8));

    system.record_failure(node_b);
    system.record_offense(node_b);
    system.record_offense(node_b);
    let score_b = system.score(&node_b);
    assert!(score_b < 0.5);

    assert_eq!(system.active_trusted_count(0.8), 1);
}

// ---------------------------------------------------------------------------
// Test 6: Node config
// ---------------------------------------------------------------------------
use node::config::{NodeConfig, PeerInfo};

#[test]
fn test_node_config_creation() {
    let peer = PeerInfo::new(Uuid::new_v4(), "127.0.0.1:9001".into(), "abc123".into());
    let config = NodeConfig::new("127.0.0.1:9000".into(), "sk_test".into(), vec![peer], 3, 5);
    assert_eq!(config.listen_addr, "127.0.0.1:9000");
    assert_eq!(config.peers.len(), 1);
    assert_eq!(config.consensus_threshold, 3);
    assert_eq!(config.consensus_total, 5);
}

// ---------------------------------------------------------------------------
// Test 7: Monitoring / alerting stubs
// ---------------------------------------------------------------------------
use node::monitoring::{AlertConfig, AlertManager};

#[test]
fn test_alert_manager_stub() {
    let config = AlertConfig {
        pagerduty_key: None,
        slack_webhook: None,
    };
    let manager = AlertManager::new(config);
    manager.send_alert("critical", "test alert");
    let default_mgr = AlertManager::default();
    default_mgr.send_alert("info", "default config works");
}
