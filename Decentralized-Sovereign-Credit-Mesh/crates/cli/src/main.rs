use std::collections::HashMap;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

use mesh_credit::constraints::{ConstraintEngine, Exposure};
use mesh_credit::loan::{calculate_interest_rate, Loan, LoanStatus};
use mesh_graph_core::algorithms;
use mesh_graph_core::entropy;
use mesh_graph_core::graph::SocialGraph;
use mesh_graph_core::types::*;
use mesh_reputation::scoring;
use mesh_reputation::taint::TaintTracker;
use mesh_settlement::contract::SettlementEngine;
use mesh_settlement::pool::CreditPool;
use mesh_smpc::solvency::{prepare_solvency_shares, verify_solvency, PartyShare};
use mesh_wot::identity::Identity;
use mesh_wot::keyring::Keyring;
use mesh_wot::limits::VouchingLimits;
use mesh_wot::nullifier::NullifierRegistry;
use mesh_wot::vouching::VouchStatement;
use mesh_zkp::commitment::commit_value;
use mesh_zkp::proof::ClusterMembershipProof;

const STATE_FILE: &str = "mesh_state.json";

#[derive(Debug, Serialize, Deserialize)]
struct SolvencyShareFile {
    entity_id: String,
    threshold: u8,
    parties: Vec<PartyShare>,
}

fn keyring_path_for(state_path: &std::path::Path) -> PathBuf {
    // Secrets never live next to the state file in plaintext-adjacent naming;
    // use the gitignored mesh-keys/ directory.
    let stem = state_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "mesh".to_string());
    PathBuf::from("mesh-keys").join(format!("{stem}.keyring.json"))
}

fn atomic_write(path: &std::path::Path, content: &[u8]) -> Result<(), String> {
    let tmp = PathBuf::from(format!("{}.tmp.{}", path.display(), std::process::id()));
    std::fs::write(&tmp, content).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct AppState {
    local_id: Option<NodeId>,
    identities: HashMap<NodeId, Identity>,
    graph: GraphSnapshot,
    nullifiers: NullifierRegistry,
    loans: Vec<Loan>,
    taint_tracker: TaintTracker,
    vouching_limits: VouchingLimits,
    credit_pools: Vec<CreditPool>,
}

impl AppState {
    fn new() -> Self {
        AppState {
            local_id: None,
            identities: HashMap::new(),
            graph: GraphSnapshot {
                nodes: vec![],
                edges: vec![],
            },
            nullifiers: NullifierRegistry::new(),
            loans: Vec::new(),
            taint_tracker: TaintTracker::new(),
            vouching_limits: VouchingLimits::default(),
            credit_pools: Vec::new(),
        }
    }

    fn load(path: &PathBuf) -> Self {
        if path.exists() {
            let content = std::fs::read_to_string(path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_else(|_| AppState::new())
        } else {
            AppState::new()
        }
    }

    fn save(&self, path: &std::path::Path) -> Result<(), String> {
        let content = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        atomic_write(path, content.as_bytes())?;
        Ok(())
    }

    fn to_graph(&self) -> SocialGraph {
        SocialGraph::from_snapshot(self.graph.clone())
    }

    fn apply_graph(&mut self, graph: &SocialGraph) {
        self.graph = graph.to_snapshot();
    }
}

#[derive(Parser)]
#[command(
    name = "credit-mesh",
    version = "0.1.0",
    about = "Decentralized Sovereign Credit Mesh"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[arg(short, long, default_value = STATE_FILE)]
    state_file: String,
}

#[derive(Subcommand)]
enum Commands {
    Init {
        name: String,
    },
    AddUser {
        name: String,
        #[arg(short, long)]
        id: Option<String>,
    },
    Vouch {
        target: String,
        #[arg(short, long)]
        as_user: Option<String>,
        #[arg(short, long, default_value = "0.5")]
        weight: f64,
        #[arg(short, long, default_value = "friend")]
        relationship: String,
    },
    TrustScore {
        user: String,
    },
    TrustPath {
        from: String,
        to: String,
    },
    Users,
    Stats,
    Export {
        output: String,
    },
    Import {
        input: String,
    },
    Whoami,

    // --- New ---
    /// Show or configure vouching limits
    VouchingLimits {
        #[arg(long)]
        max_depth: Option<usize>,
        #[arg(long)]
        max_breadth: Option<usize>,
        #[arg(long)]
        min_weight: Option<f64>,
        #[arg(long)]
        decay_factor: Option<f64>,
    },
    /// Compute sybil risk score for a user
    SybilScore {
        user: String,
    },
    /// Detect potential Sybil clusters in the graph
    SybilClusters {
        #[arg(short, long, default_value = "0.6")]
        threshold: f64,
    },
    /// Check if a nullifier is already used (double-borrow protection)
    CheckNullifier {
        identity_id: String,
    },
    /// Register a nullifier for an identity
    RegisterNullifier {
        identity_id: String,
        nonce: String,
    },
    /// Check credit constraints for a borrower
    Constraints {
        borrower: String,
        amount: f64,
    },
    /// Create a new loan
    CreateLoan {
        borrower: String,
        lender: String,
        principal: f64,
        purpose: String,
    },
    /// Repay an existing loan
    RepayLoan {
        loan_id: String,
        amount: f64,
    },
    /// List all loans
    Loans,
    /// Record a repayment flow (for taint tracking)
    Repayment {
        from: String,
        to: String,
        amount: f64,
    },
    // --- SMPC ---
    /// Prepare SMPC solvency shares for a user
    SmpcPrepare {
        user: String,
        assets: u64,
        liabilities: u64,
        #[arg(long, default_value = "5")]
        parties: u8,
        #[arg(long, default_value = "3")]
        threshold: u8,
    },
    /// Verify solvency from party shares
    SmpcVerify {
        shares: String,
    },
    // --- ZKP ---
    /// Create a hash-based commitment
    ZkpCommit {
        value: String,
        #[arg(long)]
        label: Option<String>,
    },
    /// Generate a cluster membership proof
    ZkpClusterProof {
        user: String,
    },
    /// Verify a cluster membership proof from a file
    ZkpVerifyProof {
        proof_file: String,
        min_size: usize,
        min_reputation: f64,
    },
    // --- Settlement ---
    /// Create a new credit pool
    PoolCreate {
        name: String,
        #[arg(long, default_value = "0.05")]
        apr: f64,
    },
    /// Deposit into a credit pool
    PoolDeposit {
        pool_id: String,
        provider: String,
        amount: f64,
    },
    /// Withdraw from a credit pool
    PoolWithdraw {
        pool_id: String,
        provider: String,
        amount: f64,
    },
    /// Issue an automated loan from a pool
    PoolLoan {
        pool_id: String,
        borrower: String,
        amount: f64,
        #[arg(default_value = "pool loan")]
        purpose: String,
    },
    /// Show pool details
    PoolInfo {
        pool_id: String,
    },
}

fn parse_relationship(s: &str) -> Result<RelationshipType, String> {
    match s.to_lowercase().as_str() {
        "friend" => Ok(RelationshipType::Friend),
        "family" => Ok(RelationshipType::Family),
        "colleague" => Ok(RelationshipType::Colleague),
        "guarantor" => Ok(RelationshipType::Guarantor),
        _ => Ok(RelationshipType::Custom(s.to_string())),
    }
}

fn main() {
    let cli = Cli::parse();
    let state_path = PathBuf::from(&cli.state_file);
    let mut state = AppState::load(&state_path);

    // Re-attach private signing keys from the keyring (they are never stored in state).
    let keyring_path = keyring_path_for(&state_path);
    let mut keyring = match Keyring::load(&keyring_path) {
        Ok(k) => k,
        Err(e) => {
            // Never silently overwrite a corrupt keyring with an empty one —
            // preserve it as a timestamped backup (finding #7).
            if keyring_path.exists() {
                let backup = keyring_path
                    .with_extension(format!("corrupt.{}.json", chrono::Utc::now().timestamp()));
                let _ = std::fs::rename(&keyring_path, &backup);
                eprintln!(
                    "Warning: {} — backed up to '{}' and starting with an empty keyring.",
                    e,
                    backup.display()
                );
            } else {
                eprintln!("Warning: {}", e);
            }
            Keyring::new()
        }
    };
    for identity in state.identities.values_mut() {
        if identity.signing_key_bytes.is_none() {
            identity.signing_key_bytes = keyring.get(&identity.id).map(|k| k.to_vec());
        }
    }

    let result = match &cli.command {
        Commands::Init { name } => cmd_init(&mut state, name),
        Commands::AddUser { name, id } => cmd_add_user(&mut state, name, id.as_deref()),
        Commands::Vouch {
            target,
            as_user,
            weight,
            relationship,
        } => cmd_vouch(
            &mut state,
            target,
            as_user.as_deref(),
            *weight,
            relationship,
        ),
        Commands::TrustScore { user } => cmd_trust_score(&state, user),
        Commands::TrustPath { from, to } => cmd_trust_path(&state, from, to),
        Commands::Users => cmd_users(&state),
        Commands::Stats => cmd_stats(&state),
        Commands::Export { output } => cmd_export(&state, output),
        Commands::Import { input } => cmd_import(&mut state, input),
        Commands::Whoami => cmd_whoami(&state),
        Commands::VouchingLimits {
            max_depth,
            max_breadth,
            min_weight,
            decay_factor,
        } => cmd_vouching_limits(
            &mut state,
            *max_depth,
            *max_breadth,
            *min_weight,
            *decay_factor,
        ),
        Commands::SybilScore { user } => cmd_sybil_score(&state, user),
        Commands::SybilClusters { threshold } => cmd_sybil_clusters(&state, *threshold),
        Commands::CheckNullifier { identity_id } => cmd_check_nullifier(&state, identity_id),
        Commands::RegisterNullifier { identity_id, nonce } => {
            cmd_register_nullifier(&mut state, identity_id, nonce)
        }
        Commands::Constraints { borrower, amount } => cmd_constraints(&state, borrower, *amount),
        Commands::CreateLoan {
            borrower,
            lender,
            principal,
            purpose,
        } => cmd_create_loan(&mut state, borrower, lender, *principal, purpose),
        Commands::RepayLoan { loan_id, amount } => cmd_repay_loan(&mut state, loan_id, *amount),
        Commands::Loans => cmd_loans(&state),
        Commands::Repayment { from, to, amount } => cmd_repayment(&mut state, from, to, *amount),
        Commands::SmpcPrepare {
            user,
            assets,
            liabilities,
            parties,
            threshold,
        } => cmd_smpc_prepare(user, *assets, *liabilities, *parties, *threshold),
        Commands::SmpcVerify { shares } => cmd_smpc_verify(shares),
        Commands::ZkpCommit { value, label } => cmd_zkp_commit(value, label.as_deref()),
        Commands::ZkpClusterProof { user } => cmd_zkp_cluster_proof(&state, user),
        Commands::ZkpVerifyProof {
            proof_file,
            min_size,
            min_reputation,
        } => cmd_zkp_verify_proof(proof_file, *min_size, *min_reputation),
        Commands::PoolCreate { name, apr } => cmd_pool_create(&mut state, name, *apr),
        Commands::PoolDeposit {
            pool_id,
            provider,
            amount,
        } => cmd_pool_deposit(&mut state, pool_id, provider, *amount),
        Commands::PoolWithdraw {
            pool_id,
            provider,
            amount,
        } => cmd_pool_withdraw(&mut state, pool_id, provider, *amount),
        Commands::PoolLoan {
            pool_id,
            borrower,
            amount,
            purpose,
        } => cmd_pool_loan(&mut state, pool_id, borrower, *amount, purpose),
        Commands::PoolInfo { pool_id } => cmd_pool_info(&state, pool_id),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    if let Err(e) = state.save(&state_path) {
        eprintln!("Warning: failed to save state: {}", e);
    }

    // Persist any signing keys created this run to the keyring store.
    for identity in state.identities.values() {
        if let Some(key) = identity.signing_key_bytes.as_ref() {
            keyring.insert(identity.id.clone(), key.clone());
        }
    }
    if let Err(e) = keyring.save(&keyring_path) {
        eprintln!("Warning: failed to save keyring: {}", e);
    }
}

// --- Existing commands ---

fn cmd_init(state: &mut AppState, name: &str) -> Result<(), String> {
    let id = format!(
        "user_{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
    );
    let identity = Identity::generate(id.clone(), name.to_string());
    let node = NodeData {
        id: id.clone(),
        display_name: name.to_string(),
        created_at: chrono::Utc::now().timestamp(),
    };
    let mut graph = state.to_graph();
    graph.add_node(node);
    state.apply_graph(&graph);
    state.identities.insert(id.clone(), identity);
    state.local_id = Some(id.clone());
    println!("✅ Initialized identity: {} ({})", id, name);
    Ok(())
}

fn cmd_add_user(state: &mut AppState, name: &str, custom_id: Option<&str>) -> Result<(), String> {
    let id = custom_id.map(|s| s.to_string()).unwrap_or_else(|| {
        format!(
            "user_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        )
    });
    // Reject duplicate ids so an identity's keypair cannot be silently rotated,
    // orphaning its vouches and nullifiers.
    if state.identities.contains_key(&id) {
        return Err(format!("user '{}' already exists", id));
    }
    let identity = Identity::generate(id.clone(), name.to_string());
    let node = NodeData {
        id: id.clone(),
        display_name: name.to_string(),
        created_at: chrono::Utc::now().timestamp(),
    };
    let mut graph = state.to_graph();
    graph.add_node(node);
    state.apply_graph(&graph);
    state.identities.insert(id.clone(), identity);
    println!("✅ Added user: {} ({})", name, id);
    Ok(())
}

fn cmd_vouch(
    state: &mut AppState,
    target: &str,
    as_user: Option<&str>,
    weight: f64,
    relationship: &str,
) -> Result<(), String> {
    let voucher_id = match as_user {
        Some(id) => id.to_string(),
        None => state
            .local_id
            .as_ref()
            .ok_or_else(|| "not initialized. Run 'credit-mesh init <name>' first.".to_string())?
            .clone(),
    };
    let graph = state.to_graph();
    state.vouching_limits.can_vouch(&graph, &voucher_id)?;
    let voucher = state
        .identities
        .get(&voucher_id)
        .ok_or_else(|| format!("voucher '{}' not found", voucher_id))?;
    if !graph.contains_node(target) {
        return Err(format!("target '{}' not found. Add them first.", target));
    }
    let rel = parse_relationship(relationship)?;
    let vouch = VouchStatement::create(voucher, target, rel.clone(), weight)?;
    let mut graph = state.to_graph();
    graph.add_edge(&voucher.id, target, vouch.to_edge_data())?;
    state.apply_graph(&graph);
    let trust_map = state.vouching_limits.propagate_trust(&graph, &voucher_id);
    let reachable = trust_map.len();
    println!(
        "✅ Vouched for {} (weight: {}, reachable via decay: {})",
        target, weight, reachable
    );
    println!(
        "   Signature: {}...{}",
        hex_encode(&vouch.signature_bytes[..4]),
        hex_encode(&vouch.signature_bytes[60..])
    );
    Ok(())
}

fn cmd_trust_score(state: &AppState, user: &str) -> Result<(), String> {
    let graph = state.to_graph();
    if !graph.contains_node(user) {
        return Err(format!("user '{}' not found", user));
    }
    let mut result =
        scoring::compute_reputation(&graph, user).ok_or_else(|| "no data".to_string())?;
    let adjusted = state.taint_tracker.adjusted_reputation(&graph, user);
    if let Some(adj) = adjusted {
        result = adj;
    }
    let name = graph
        .get_node(user)
        .map(|n| n.display_name.as_str())
        .unwrap_or(user);
    let sybil = entropy::sybil_score(&graph, user).unwrap_or(0.0);
    let net_flow = state.taint_tracker.net_flow(user);
    println!("📊 {}", name);
    println!("   Composite:  {:.6}", result.composite_score);
    println!("   PageRank:   {:.6}", result.pagerank_score);
    println!("   Centrality: {:.6}", result.degree_centrality);
    println!("   Clustering: {:.6}", result.clustering_coefficient);
    println!("   Sybil Risk: {:.6}", sybil);
    println!("   Net Flow:   {:.2}", net_flow);
    Ok(())
}

fn cmd_trust_path(state: &AppState, from: &str, to: &str) -> Result<(), String> {
    let graph = state.to_graph();
    if !graph.contains_node(from) {
        return Err(format!("source '{}' not found", from));
    }
    if !graph.contains_node(to) {
        return Err(format!("target '{}' not found", to));
    }
    match graph.shortest_trust_path(from, to) {
        Some(path) => {
            println!("🔗 Trust Path ({} hops):", path.len() - 1);
            for (i, node_id) in path.iter().enumerate() {
                let name = graph
                    .get_node(node_id)
                    .map(|n| n.display_name.as_str())
                    .unwrap_or(node_id);
                if i < path.len() - 1 {
                    if let Some((_, edge)) = graph
                        .neighbors(node_id)
                        .iter()
                        .find(|(id, _)| id == &path[i + 1])
                    {
                        let next_name = graph
                            .get_node(&path[i + 1])
                            .map(|n| n.display_name.as_str())
                            .unwrap_or(&path[i + 1]);
                        println!(
                            "   {}. {} ──({}, w={})──▶ {}",
                            i + 1,
                            name,
                            edge.relationship,
                            edge.trust_weight,
                            next_name
                        );
                    }
                } else {
                    println!("   {}. {}", i + 1, name);
                }
            }
        }
        None => println!("No trust path found"),
    }
    Ok(())
}

fn cmd_users(state: &AppState) -> Result<(), String> {
    let graph = state.to_graph();
    let nodes = graph.all_nodes();
    if nodes.is_empty() {
        println!("No users.");
        return Ok(());
    }
    println!("👥 Users ({}):", nodes.len());
    println!(
        "{:<22} {:<24} {:<6} {:<6} {:<8}",
        "ID", "Name", "Out", "In", "Sybil"
    );
    println!("{}", "-".repeat(72));
    for n in graph.all_nodes() {
        let out = graph.neighbors(&n.id).len();
        let inn = graph.predecessors(&n.id).len();
        let syb = entropy::sybil_score(&graph, &n.id).unwrap_or(0.0);
        let marker = state.local_id.as_ref().is_some_and(|id| *id == n.id);
        println!(
            "{:<22} {:<24} {:<6} {:<6} {:<8.4}{}",
            n.id,
            n.display_name,
            out,
            inn,
            syb,
            if marker { " *" } else { "" }
        );
    }
    Ok(())
}

fn cmd_stats(state: &AppState) -> Result<(), String> {
    let graph = state.to_graph();
    let n = graph.node_count();
    let e = graph.edge_count();
    let density = if n > 1 {
        2.0 * e as f64 / (n * (n - 1)) as f64
    } else {
        0.0
    };
    println!("📈 Graph Stats");
    println!("   Nodes:     {}", n);
    println!("   Edges:     {}", e);
    println!("   Density:   {:.6}", density);
    if n > 0 {
        println!(
            "   Avg Clustering: {:.6}",
            algorithms::average_clustering(&graph)
        );
        println!("   Graph Entropy:  {:.6}", entropy::graph_entropy(&graph));
        let pr = algorithms::compute_pagerank(&graph, 0.85, 100, 1e-6);
        let max_pr = pr.values().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min_pr = pr.values().cloned().fold(f64::INFINITY, f64::min);
        println!("   PageRank:       [{:.6}, {:.6}]", min_pr, max_pr);
        let top = scoring::compute_all_reputations(&graph);
        println!("\n🏆 Top 5:");
        for r in top.iter().take(5) {
            let name = graph
                .get_node(&r.node_id)
                .map(|n| n.display_name.as_str())
                .unwrap_or(&r.node_id);
            println!("   {:.6}  {}", r.composite_score, name);
        }
        println!(
            "\n📊 Loans: {} total, {} active",
            state.loans.len(),
            state
                .loans
                .iter()
                .filter(|l| l.status == LoanStatus::Active)
                .count()
        );
        println!("   Nullifiers used: {}", state.nullifiers.used_count());
        println!(
            "   Repayments tracked: {}",
            state.taint_tracker.repayment_count()
        );
    }
    Ok(())
}

fn cmd_export(state: &AppState, output: &str) -> Result<(), String> {
    let graph = state.to_graph();
    let json = serde_json::to_string_pretty(&graph.to_snapshot()).map_err(|e| e.to_string())?;
    std::fs::write(output, &json).map_err(|e| e.to_string())?;
    println!("✅ Exported to {}", output);
    Ok(())
}

fn cmd_import(state: &mut AppState, input: &str) -> Result<(), String> {
    let content = std::fs::read_to_string(input).map_err(|e| e.to_string())?;
    let snapshot: GraphSnapshot = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    let imported = SocialGraph::from_snapshot(snapshot);
    let mut graph = state.to_graph();
    for node in imported.all_nodes() {
        graph.add_node(node.clone());
    }
    for (src, tgt, edge) in imported.all_edges() {
        let _ = graph.add_edge(&src, &tgt, edge.clone());
    }
    state.apply_graph(&graph);
    println!(
        "✅ Imported ({} nodes, {} edges)",
        graph.node_count(),
        graph.edge_count()
    );
    Ok(())
}

fn cmd_whoami(state: &AppState) -> Result<(), String> {
    match &state.local_id {
        Some(id) => {
            let identity = state
                .identities
                .get(id)
                .ok_or_else(|| "not found".to_string())?;
            println!("👤 {}", identity.display_name);
            println!("   ID:         {}", identity.id);
            println!(
                "   Public Key: {}...{}",
                hex_encode(&identity.public_key_bytes[..4]),
                hex_encode(&identity.public_key_bytes[28..])
            );
        }
        None => println!("No local identity. Run 'credit-mesh init <name>'."),
    }
    Ok(())
}

// --- New commands ---

fn cmd_vouching_limits(
    state: &mut AppState,
    max_depth: Option<usize>,
    max_breadth: Option<usize>,
    min_weight: Option<f64>,
    decay_factor: Option<f64>,
) -> Result<(), String> {
    if let Some(v) = max_depth {
        state.vouching_limits.max_depth = v;
    }
    if let Some(v) = max_breadth {
        state.vouching_limits.max_breadth = v;
    }
    if let Some(v) = min_weight {
        state.vouching_limits.min_weight = v;
    }
    if let Some(v) = decay_factor {
        state.vouching_limits.decay_factor = v;
    }
    println!("⚙️ Vouching Limits:");
    println!("   Max Depth:     {}", state.vouching_limits.max_depth);
    println!("   Max Breadth:   {}", state.vouching_limits.max_breadth);
    println!("   Min Weight:    {:.2}", state.vouching_limits.min_weight);
    println!(
        "   Decay Factor:  {:.2}",
        state.vouching_limits.decay_factor
    );
    if let Some(local_id) = state.local_id.as_ref() {
        let graph = state.to_graph();
        let trust_map = state.vouching_limits.propagate_trust(&graph, local_id);
        println!("   Reachable users via decay: {}", trust_map.len());
    }
    Ok(())
}

fn cmd_sybil_score(state: &AppState, user: &str) -> Result<(), String> {
    let graph = state.to_graph();
    if !graph.contains_node(user) {
        return Err(format!("user '{}' not found", user));
    }
    let score = entropy::sybil_score(&graph, user).unwrap_or(0.0);
    let local_ent = entropy::local_entropy(&graph, user, 2);
    let global_ent = entropy::graph_entropy(&graph);
    println!("🔍 Sybil Analysis for {}", user);
    println!(
        "   Sybil Score:   {:.4} ({})",
        score,
        if score > 0.7 {
            "HIGH RISK"
        } else if score > 0.4 {
            "MEDIUM"
        } else {
            "LOW"
        }
    );
    println!("   Local Entropy: {:.4}", local_ent);
    println!("   Global Entropy: {:.4}", global_ent);
    Ok(())
}

fn cmd_sybil_clusters(state: &AppState, threshold: f64) -> Result<(), String> {
    let graph = state.to_graph();
    let clusters = entropy::detect_sybil_clusters(&graph, threshold);
    if clusters.is_empty() {
        println!("✅ No Sybil clusters detected (threshold: {})", threshold);
    } else {
        println!(
            "⚠️  {} potential Sybil cluster(s) detected:",
            clusters.len()
        );
        for (i, cluster) in clusters.iter().enumerate() {
            println!("   Cluster {}: {} users", i + 1, cluster.len());
            for node_id in cluster {
                let name = graph
                    .get_node(node_id)
                    .map(|n| n.display_name.as_str())
                    .unwrap_or(node_id);
                let score = entropy::sybil_score(&graph, node_id).unwrap_or(0.0);
                println!("      {} ({}) — score: {:.4}", name, node_id, score);
            }
        }
    }
    Ok(())
}

fn cmd_check_nullifier(state: &AppState, identity_id: &str) -> Result<(), String> {
    let identity = state
        .identities
        .get(identity_id)
        .ok_or_else(|| format!("identity '{}' not found", identity_id))?;
    let nonce = b"default_check";
    let nullifier =
        NullifierRegistry::compute_nullifier(identity_id, &identity.public_key_bytes, nonce);
    let used = state.nullifiers.is_used(&nullifier);
    println!("🔑 Nullifier check for {}:", identity_id);
    println!("   Computed: {}", &nullifier[..16]);
    println!(
        "   Status:   {}",
        if used {
            "USED — double-borrow risk"
        } else {
            "CLEAR"
        }
    );
    Ok(())
}

fn cmd_register_nullifier(
    state: &mut AppState,
    identity_id: &str,
    nonce: &str,
) -> Result<(), String> {
    let identity = state
        .identities
        .get(identity_id)
        .ok_or_else(|| format!("identity '{}' not found", identity_id))?;
    let nullifier = state.nullifiers.register_nullifier(
        identity_id,
        &identity.public_key_bytes,
        nonce.as_bytes(),
    )?;
    println!("🔒 Nullifier registered: {}...", &nullifier[..16]);
    println!("   Identity {} now has an active credit line.", identity_id);
    Ok(())
}

fn cmd_constraints(state: &AppState, borrower: &str, amount: f64) -> Result<(), String> {
    let graph = state.to_graph();
    if !graph.contains_node(borrower) {
        return Err(format!("borrower '{}' not found", borrower));
    }

    let exposures: Vec<Exposure> = state
        .loans
        .iter()
        .map(|l| Exposure {
            borrower: l.borrower.clone(),
            outstanding: l.outstanding,
            active: l.status == LoanStatus::Active,
        })
        .collect();

    let constraints = ConstraintEngine::generate_credit_constraints(&graph, borrower, amount);
    println!(
        "🔒 Credit Constraints for {} (request: {:.2}):",
        borrower, amount
    );
    for (i, constraint) in constraints.iter().enumerate() {
        let passed = constraint.evaluate(&graph, &exposures).unwrap_or(false);
        println!(
            "   {}. {} — {}",
            i + 1,
            constraint_name(constraint),
            if passed { "✅ PASS" } else { "❌ FAIL" }
        );
    }
    let (allowed, _) = ConstraintEngine::check_borrower(&graph, borrower, amount, &exposures)?;
    println!(
        "\n{}",
        if allowed {
            "✅ Borrower meets all constraints"
        } else {
            "❌ Borrower does not meet constraints"
        }
    );
    Ok(())
}

fn cmd_create_loan(
    state: &mut AppState,
    borrower: &str,
    lender: &str,
    principal: f64,
    purpose: &str,
) -> Result<(), String> {
    let graph = state.to_graph();
    if !graph.contains_node(borrower) {
        return Err(format!("borrower '{}' not found", borrower));
    }
    if !graph.contains_node(lender) {
        return Err(format!("lender '{}' not found", lender));
    }

    let exposures: Vec<Exposure> = state
        .loans
        .iter()
        .map(|l| Exposure {
            borrower: l.borrower.clone(),
            outstanding: l.outstanding,
            active: l.status == LoanStatus::Active,
        })
        .collect();

    let (allowed, _) = ConstraintEngine::check_borrower(&graph, borrower, principal, &exposures)?;
    if !allowed {
        return Err("borrower does not meet credit constraints".to_string());
    }

    let identity = state
        .identities
        .get(borrower)
        .ok_or_else(|| format!("identity '{}' not found", borrower))?;
    // Identity-stable nonce: a borrower may hold exactly one active credit line
    // (double-borrow protection). A timestamp nonce would make this a no-op.
    const CREDIT_NONCE: &[u8] = b"credit-line:v1";
    state
        .nullifiers
        .check_nullifier(borrower, &identity.public_key_bytes, CREDIT_NONCE)?;

    let rate = calculate_interest_rate(&graph, borrower, 0.05);
    let loan_id = format!(
        "loan_{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
    );
    let mut loan = Loan::new(
        loan_id.clone(),
        borrower.to_string(),
        lender.to_string(),
        principal,
        rate,
        purpose.to_string(),
    )?;
    loan.drawdown()?;

    // Only after the loan is valid do we mark the identity's credit line used.
    state
        .nullifiers
        .register_nullifier(borrower, &identity.public_key_bytes, CREDIT_NONCE)?;

    state.loans.push(loan);
    println!("✅ Loan created:");
    println!("   ID:       {}", loan_id);
    println!("   Borrower: {}", borrower);
    println!("   Lender:   {}", lender);
    println!("   Amount:   {:.2}", principal);
    println!("   Rate:     {:.2}%", rate * 100.0);
    Ok(())
}

fn cmd_repay_loan(state: &mut AppState, loan_id: &str, amount: f64) -> Result<(), String> {
    let loan = state
        .loans
        .iter_mut()
        .find(|l| l.id == loan_id)
        .ok_or_else(|| format!("loan '{}' not found", loan_id))?;
    let repaid = loan.repay(amount)?;
    let fully_repaid = loan.status == LoanStatus::Repaid;
    let borrower = loan.borrower.clone();
    state
        .taint_tracker
        .record_repayment(&loan.borrower, &loan.lender, repaid);
    println!("💰 Repayment of {:.2} on loan {}", repaid, loan_id);
    println!("   Outstanding: {:.2}", loan.outstanding);
    println!("   Status:      {:?}", loan.status);
    if fully_repaid {
        release_credit_line(state, &borrower)?;
        println!("   Credit line released — {} can borrow again.", borrower);
    }
    Ok(())
}

/// Release the identity's credit-line nullifier after full settlement.
fn release_credit_line(state: &mut AppState, borrower: &str) -> Result<(), String> {
    let identity = state
        .identities
        .get(borrower)
        .ok_or_else(|| format!("identity '{}' not found", borrower))?;
    state
        .nullifiers
        .release_nullifier(borrower, &identity.public_key_bytes, b"credit-line:v1");
    Ok(())
}

fn cmd_loans(state: &AppState) -> Result<(), String> {
    if state.loans.is_empty() {
        println!("No loans.");
        return Ok(());
    }
    println!("📋 Loans ({}):", state.loans.len());
    println!(
        "{:<22} {:<18} {:<18} {:<10} {:<10} {:<10}",
        "ID", "Borrower", "Lender", "Principal", "Rate", "Status"
    );
    println!("{}", "-".repeat(94));
    for l in &state.loans {
        println!(
            "{:<22} {:<18} {:<18} {:<10.2} {:<10.2} {:<10?}",
            l.id,
            l.borrower,
            l.lender,
            l.principal,
            l.interest_rate * 100.0,
            l.status
        );
    }
    Ok(())
}

fn cmd_repayment(state: &mut AppState, from: &str, to: &str, amount: f64) -> Result<(), String> {
    state.taint_tracker.record_repayment(from, to, amount);
    println!(
        "📊 Repayment flow recorded: {} → {} ({:.2})",
        from, to, amount
    );
    println!(
        "   Net flow for {}: {:.2}",
        from,
        state.taint_tracker.net_flow(from)
    );
    println!(
        "   Net flow for {}: {:.2}",
        to,
        state.taint_tracker.net_flow(to)
    );
    Ok(())
}

fn constraint_name(c: &mesh_credit::constraints::Constraint) -> String {
    match c {
        mesh_credit::constraints::Constraint::TrustPathExists { .. } => "Trust Path Exists",
        mesh_credit::constraints::Constraint::MinReputation { .. } => "Minimum Reputation",
        mesh_credit::constraints::Constraint::MaxExposure { .. } => "Maximum Exposure",
        mesh_credit::constraints::Constraint::VouchingLimit { .. } => "Vouching Propagation Limit",
        mesh_credit::constraints::Constraint::SybilScoreBelow { .. } => "Sybil Score Threshold",
        mesh_credit::constraints::Constraint::And(_) => "All Of",
        mesh_credit::constraints::Constraint::Or(_) => "Any Of",
        mesh_credit::constraints::Constraint::Not(_) => "Not",
    }
    .to_string()
}

// --- SMPC ---
fn cmd_smpc_prepare(
    user: &str,
    assets: u64,
    liabilities: u64,
    parties: u8,
    threshold: u8,
) -> Result<(), String> {
    let (party_shares, stmt) =
        prepare_solvency_shares(user, assets, liabilities, parties, threshold)?;
    println!(
        "🔐 SMPC Solvency Shares for {} (assets={}, liabilities={})",
        user, assets, liabilities
    );
    println!("   Commitment: {}...", &stmt.asset_commitment[..16]);
    println!("   Threshold:  {}/{}", stmt.threshold, stmt.num_parties);
    println!("\n   Party Shares:");
    for ps in &party_shares {
        println!(
            "     Party {}: asset_share={}, liability_share={}",
            ps.party_index, ps.asset_share, ps.liability_share
        );
    }
    let file = SolvencyShareFile {
        entity_id: user.to_string(),
        threshold,
        parties: party_shares,
    };
    let json = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
    let filename = format!("solvency_{}.json", user);
    std::fs::write(&filename, &json).map_err(|e| e.to_string())?;
    println!("\n   Saved to {}", filename);
    Ok(())
}

fn cmd_smpc_verify(shares: &str) -> Result<(), String> {
    let content =
        std::fs::read_to_string(shares).map_err(|e| format!("can't read '{}': {}", shares, e))?;
    let file: SolvencyShareFile =
        serde_json::from_str(&content).map_err(|e| format!("invalid share file: {}", e))?;
    let result = verify_solvency(&file.entity_id, &file.parties, file.threshold)?;
    println!("🔐 Solvency Verification Result:");
    println!("   Entity:     {}", result.entity_id);
    println!(
        "   Status:     {}",
        if result.solvent {
            "✅ SOLVENT"
        } else {
            "❌ INSOLVENT"
        }
    );
    println!("   Net Position: {}", result.net_position);
    println!("   Parties Used: {}", result.parties_used);
    Ok(())
}

// --- ZKP ---
fn cmd_zkp_commit(_value: &str, label: Option<&str>) -> Result<(), String> {
    let label = label.unwrap_or("default");
    let (commitment, opening) = commit_value(0, label);
    println!("🔏 Commitment:");
    println!("   Value:      {}", commitment.value);
    println!(
        "   Opening:    secret={}, blinding={}",
        opening.secret, opening.blinding
    );
    Ok(())
}

fn cmd_zkp_cluster_proof(state: &AppState, user: &str) -> Result<(), String> {
    let graph = state.to_graph();
    let nodes = graph.all_nodes();
    if !graph.contains_node(user) {
        return Err(format!("user '{}' not found", user));
    }

    let member_data: Vec<Vec<u8>> = nodes.iter().map(|n| n.id.as_bytes().to_vec()).collect();
    let scores: Vec<f64> = nodes
        .iter()
        .map(|n| {
            mesh_reputation::scoring::compute_reputation(&graph, &n.id)
                .map(|r| r.composite_score)
                .unwrap_or(0.0)
        })
        .collect();

    let user_secret = user.as_bytes().to_vec();
    let proof = ClusterMembershipProof::prove(user, &user_secret, &member_data, &scores, 0.05)
        .ok_or_else(|| "failed to generate proof — user not in cluster".to_string())?;

    println!("🔏 Cluster Membership Proof for {}", user);
    println!("   Proof:      {}", proof.summary());
    println!("   Merkle Root: {}...", &proof.merkle_proof.root[..16]);
    println!("   Path Length: {}", proof.merkle_proof.path.len());

    let filename = format!("zkp_{}.json", user);
    let json = serde_json::to_string_pretty(&proof).map_err(|e| e.to_string())?;
    std::fs::write(&filename, &json).map_err(|e| e.to_string())?;
    println!("   Saved to {}", filename);
    Ok(())
}

fn cmd_zkp_verify_proof(
    proof_file: &str,
    min_size: usize,
    min_reputation: f64,
) -> Result<(), String> {
    let content = std::fs::read_to_string(proof_file)
        .map_err(|e| format!("can't read '{}': {}", proof_file, e))?;
    let proof: ClusterMembershipProof =
        serde_json::from_str(&content).map_err(|e| format!("invalid proof file: {}", e))?;
    let valid = proof.verify(min_size, min_reputation)?;
    println!("🔏 ZK Proof Verification:");
    println!("   Proof:      {}", proof.summary());
    println!(
        "   Required:   size>={}, rep>={:.2}",
        min_size, min_reputation
    );
    println!(
        "   Result:     {}",
        if valid { "✅ VALID" } else { "❌ INVALID" }
    );
    Ok(())
}

// --- Settlement ---
fn cmd_pool_create(state: &mut AppState, name: &str, apr: f64) -> Result<(), String> {
    let id = format!(
        "pool_{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
    );
    let pool = CreditPool::new(id.clone(), name.to_string(), apr);
    state.credit_pools.push(pool);
    println!("🏦 Credit Pool Created:");
    println!("   ID:   {}", id);
    println!("   Name: {}", name);
    println!("   APR:  {:.2}%", apr * 100.0);
    Ok(())
}

fn cmd_pool_deposit(
    state: &mut AppState,
    pool_id: &str,
    provider: &str,
    amount: f64,
) -> Result<(), String> {
    let pool = state
        .credit_pools
        .iter_mut()
        .find(|p| p.id == pool_id)
        .ok_or_else(|| format!("pool '{}' not found", pool_id))?;
    let shares = pool.deposit(provider, amount)?;
    println!("💰 Deposited {:.2} into pool '{}'", amount, pool.name);
    println!("   Shares:   {:.4}", shares);
    println!("   Liquidity: {:.2}", pool.total_liquidity);
    Ok(())
}

fn cmd_pool_withdraw(
    state: &mut AppState,
    pool_id: &str,
    provider: &str,
    amount: f64,
) -> Result<(), String> {
    let pool = state
        .credit_pools
        .iter_mut()
        .find(|p| p.id == pool_id)
        .ok_or_else(|| format!("pool '{}' not found", pool_id))?;
    let withdrawn = pool.withdraw(provider, amount)?;
    println!("💰 Withdrew {:.2} from pool '{}'", withdrawn, pool.name);
    println!("   Remaining liquidity: {:.2}", pool.total_liquidity);
    Ok(())
}

fn cmd_pool_loan(
    state: &mut AppState,
    pool_id: &str,
    borrower: &str,
    amount: f64,
    purpose: &str,
) -> Result<(), String> {
    let graph = state.to_graph();
    if !graph.contains_node(borrower) {
        return Err(format!("borrower '{}' not found", borrower));
    }

    let exposures: Vec<Exposure> = state
        .loans
        .iter()
        .map(|l| Exposure {
            borrower: l.borrower.clone(),
            outstanding: l.outstanding,
            active: l.status == LoanStatus::Active,
        })
        .collect();

    // Nullifier check BEFORE the pool allocates liquidity, so a double-borrow
    // attempt cannot reserve funds that then stay locked (review fix).
    let identity = state
        .identities
        .get(borrower)
        .ok_or_else(|| format!("identity '{}' not found", borrower))?;
    const CREDIT_NONCE: &[u8] = b"credit-line:v1";
    state
        .nullifiers
        .check_nullifier(borrower, &identity.public_key_bytes, CREDIT_NONCE)?;

    let pool = state
        .credit_pools
        .iter_mut()
        .find(|p| p.id == pool_id)
        .ok_or_else(|| format!("pool '{}' not found", pool_id))?;

    let result =
        SettlementEngine::auto_issue_from_pool(pool, &graph, borrower, amount, purpose, &exposures);
    if result.approved {
        let rate = result.interest_rate;
        let loan_id = format!(
            "loan_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        );
        let mut loan = mesh_credit::loan::Loan::new(
            loan_id.clone(),
            borrower.to_string(),
            pool_id.to_string(),
            amount,
            rate,
            purpose.to_string(),
        )?;
        loan.drawdown()?;
        state
            .nullifiers
            .register_nullifier(borrower, &identity.public_key_bytes, CREDIT_NONCE)?;
        state.loans.push(loan);
        println!("✅ Pool Loan Approved:");
        println!("   ID:     {}", loan_id);
        println!("   Amount: {:.2}", amount);
        println!("   Rate:   {:.2}%", rate * 100.0);
    } else {
        println!("❌ Pool Loan Denied: {}", result.reason);
    }
    Ok(())
}

fn cmd_pool_info(state: &AppState, pool_id: &str) -> Result<(), String> {
    let pool = state
        .credit_pools
        .iter()
        .find(|p| p.id == pool_id)
        .ok_or_else(|| format!("pool '{}' not found", pool_id))?;
    println!("🏦 Pool: {}", pool.name);
    println!("   ID:     {}", pool.id);
    println!("   Target APR: {:.2}%", pool.target_apr * 100.0);
    println!("   Effective APR: {:.2}%", pool.effective_apr() * 100.0);
    println!("   Liquidity:  {:.2}", pool.total_liquidity);
    println!("   Allocated:  {:.2}", pool.allocated);
    println!("   Available:  {:.2}", pool.available_liquidity());
    println!("   Utilization: {:.1}%", pool.utilization_rate() * 100.0);
    println!("   Providers:  {}", pool.provider_count());
    Ok(())
}

fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        write!(out, "{:02x}", b).expect("writing to a String cannot fail");
    }
    out
}
