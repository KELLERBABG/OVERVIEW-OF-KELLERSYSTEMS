use anyhow::Result;
use clap::{Parser, Subcommand};
use rand::thread_rng;
use smpc_engine::add::secure_add;
use smpc_engine::types::SecretShare;
use sss::Share;
use sss::field::random_scalar;
use sss::sss::{reconstruct, split_secret};
use std::time::Instant;
use threshold_crypto::bls::{generate_keypair, sign, verify};

#[derive(Parser)]
#[command(name = "smpc-cli", about = "SMPC Dark Pool CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Test SSS split and reconstruct
    Sss {
        #[arg(long, default_value = "5")]
        n: u64,
        #[arg(long, default_value = "3")]
        t: u64,
        #[arg(long)]
        bench: bool,
    },
    /// Test SMPC secure addition
    SmpcAdd {
        #[arg(long, default_value = "10")]
        iterations: u64,
    },
    /// Test BLS threshold signatures
    BlsSign {
        #[arg(long, default_value = "5")]
        nodes: u64,
        #[arg(long, default_value = "3")]
        threshold: u64,
    },
    /// Test settlement engine
    Settle {
        #[arg(long, default_value = "3")]
        orders: u64,
    },
    /// Test BFT consensus
    Bft {
        #[arg(long, default_value = "4")]
        nodes: u64,
    },
    /// Test node reputation
    Reputation,
    /// Run all diagnostics
    Diag,
}

fn cmd_sss(n: u64, t: u64, bench: bool) -> Result<()> {
    let mut rng = thread_rng();
    let secret = random_scalar(&mut rng);
    println!("SSS Test: n={n}, t={t}");
    println!("  Secret: {secret}");
    let shares = split_secret(secret, n, t)?;
    println!("  Generated {} shares", shares.len());
    let recovered = reconstruct(&shares[..t as usize])?;
    println!("  Reconstructed (t shares): {recovered}");
    assert_eq!(recovered, secret);
    println!("  ✓ Match!");
    if bench {
        let start = Instant::now();
        let iterations = 1000;
        for _ in 0..iterations {
            let s = split_secret(secret, n, t)?;
            let _ = reconstruct(&s[..t as usize])?;
        }
        let elapsed = start.elapsed();
        println!(
            "  Benchmark: {iterations} ops in {elapsed:?} ({:?} per op)",
            elapsed / iterations
        );
    }
    Ok(())
}

fn cmd_smpc_add(iterations: u64) -> Result<()> {
    let mut rng = thread_rng();
    println!("SMPC Add Test: {iterations} iterations");
    for i in 0..iterations {
        let a = random_scalar(&mut rng);
        let b = random_scalar(&mut rng);
        let a_shares = split_secret(a, 5, 3)?;
        let b_shares = split_secret(b, 5, 3)?;
        let sum_shares: Vec<Share> = a_shares
            .into_iter()
            .zip(b_shares)
            .map(|(a, b)| secure_add(&SecretShare::new(a), &SecretShare::new(b)).share)
            .collect();
        let recovered = reconstruct(&sum_shares[..3])?;
        assert_eq!(recovered, a + b);
        if i % 100 == 0 && i > 0 {
            println!("  {}/{} passed", i, iterations);
        }
    }
    println!("  ✓ All passed!");
    Ok(())
}

fn cmd_bls_sign(nodes: u64, threshold: u64) -> Result<()> {
    println!("BLS Threshold Sign: n={nodes}, t={threshold}");
    let (sk, pk) = generate_keypair();
    println!("  Keypair generated");
    let msg = b"trade_42";
    let sig = sign(&sk, msg);
    println!("  Signature generated");
    let valid = verify(&pk, msg, &sig);
    println!("  Verification: {valid}");
    assert!(valid);
    println!("  ✓ BLS works!");
    Ok(())
}

fn cmd_settle(order_count: u64) -> Result<()> {
    use settlement::engine::SettlementEngine;
    use settlement::types::SettlementProof;
    println!("Settlement Test: {order_count} orders");
    let mut engine = SettlementEngine::new();
    for i in 0..order_count {
        let proof = SettlementProof::new(
            i,
            &[0u8; 32],
            &[1u8; 64],
            i,
            i + 1,
            &100u64.to_le_bytes(),
            &10u64.to_le_bytes(),
        );
        let record = engine.submit_proof(proof)?;
        println!("  Order {} submitted: {:?}", i, record.status);
        let settled = engine.settle(i)?;
        println!("  Order {} settled: {:?}", i, settled.status);
    }
    println!("  Total settled: {}", engine.total_settled());
    Ok(())
}

fn cmd_bft(node_count: u64) -> Result<()> {
    use bft::consensus::Consensus;
    use uuid::Uuid;
    println!("BFT Consensus Test: {node_count} nodes");
    let ids: Vec<Uuid> = (0..node_count).map(|_| Uuid::new_v4()).collect();
    let mut nodes: Vec<Consensus> = ids
        .iter()
        .map(|id| Consensus::new(*id, ids.clone(), "test_sk".into()))
        .collect();
    let proposal = nodes[0].propose(b"trade_42")?;
    println!("  Leader proposed: seq={}", proposal.sequence);
    // Replicas prepare the proposal and broadcast their prepare messages.
    let prepares: Vec<_> = (1..node_count as usize)
        .map(|i| nodes[i].receive_pre_prepare(&proposal).unwrap())
        .collect();
    // Every node (including the leader) collects prepares, emitting a commit
    // message when the prepare quorum is reached.
    let mut commits = Vec::new();
    for prepare in &prepares {
        for node in nodes.iter_mut() {
            if let Some(commit) = node.receive_prepare(prepare).unwrap() {
                commits.push(commit);
            }
        }
    }
    // Commit messages are broadcast to all nodes to finalize the consensus.
    for commit in &commits {
        for node in nodes.iter_mut() {
            let _ = node.receive_commit(commit);
        }
    }
    for (i, node) in nodes.iter().enumerate() {
        println!("  Node {} committed: {}", i, node.is_committed());
        assert!(node.is_committed(), "Node {} should be committed", i);
    }
    println!("  ✓ BFT consensus reached!");
    Ok(())
}

fn cmd_reputation() -> Result<()> {
    use node::reputation::ReputationSystem;
    use uuid::Uuid;
    println!("Reputation System Test");
    let mut system = ReputationSystem::default();
    let good = Uuid::new_v4();
    let bad = Uuid::new_v4();
    for _ in 0..10 {
        system.record_success(good);
    }
    // Give the "bad" node a recorded interaction before its offenses so the
    // offense penalty actually applies (a brand-new node scores the neutral 0.5).
    system.record_failure(bad);
    system.record_offense(bad);
    system.record_offense(bad);
    println!("  Good node score: {:.2}", system.score(&good));
    println!("  Bad node score: {:.2}", system.score(&bad));
    assert!(system.score(&good) > 0.9);
    assert!(system.score(&bad) < 0.5);
    println!("  ✓ Reputation system works!");
    Ok(())
}

fn cmd_diag() -> Result<()> {
    println!("=== SMPC Dark Pool Diagnostics ===\n");
    cmd_sss(5, 3, true)?;
    println!();
    cmd_smpc_add(5)?;
    println!();
    cmd_bls_sign(5, 3)?;
    println!();
    cmd_settle(3)?;
    println!();
    cmd_bft(4)?;
    println!();
    cmd_reputation()?;
    println!();
    println!("=== All diagnostics passed ===");
    Ok(())
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();
    match cli.command {
        Commands::Sss { n, t, bench } => cmd_sss(n, t, bench),
        Commands::SmpcAdd { iterations } => cmd_smpc_add(iterations),
        Commands::BlsSign { nodes, threshold } => cmd_bls_sign(nodes, threshold),
        Commands::Settle { orders } => cmd_settle(orders),
        Commands::Bft { nodes } => cmd_bft(nodes),
        Commands::Reputation => cmd_reputation(),
        Commands::Diag => cmd_diag(),
    }
}
