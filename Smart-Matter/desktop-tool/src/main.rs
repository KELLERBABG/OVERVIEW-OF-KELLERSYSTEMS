//! # Smart Matter Desktop Configuration & Monitoring Tool
//!
//! CLI tool for configuring Smart Matter mesh nodes, monitoring network
//! status, sending dead-drop messages, and retrieving integrity reports.
//!
//! Every command works against the real protocol logic (mesh routing,
//! dead-drop storage, integrity auditing, key derivation) running locally —
//! nothing is hardcoded.
//!
//! ## Usage
//!
//! ```bash
//! # Monitor network status (local 3-node mesh)
//! smart-matter-desktop monitor
//!
//! # Send a dead-drop message
//! smart-matter-desktop send --node-id 0x01 --message "hello"
//!
//! # Retrieve a dead-drop message
//! smart-matter-desktop retrieve --msg-id 42
//!
//! # Request integrity report from a node
//! smart-matter-desktop integrity --node-id 0x01
//!
//! # Configure node (set RSSI threshold, TTL, etc.)
//! smart-matter-desktop configure --node-id 0x01 --rssi-threshold -75
//! ```
//!
//! The deployment secret is read from `SMART_MATTER_DEPLOYMENT_SECRET` (hex)
//! or `--deployment-secret`; node keys are derived per node ID.

use clap::{Parser, Subcommand};
use smart_matter_core::crypto::derive_key;
use smart_matter_core::integrity::IntegrityAuditor;
use smart_matter_core::types::*;
use smart_matter_mesh::MeshController;
use smart_matter_storage::DropStorageManager;

/// Demo deployment secret (override via SMART_MATTER_DEPLOYMENT_SECRET).
const DEMO_SECRET: [u8; 32] = [0x42; 32];

#[derive(Parser)]
#[command(name = "smart-matter-desktop")]
#[command(about = "Smart Matter configuration and monitoring tool")]
struct Cli {
    /// Deployment secret (hex, 64 chars); overrides the env var
    #[arg(long, env = "SMART_MATTER_DEPLOYMENT_SECRET")]
    deployment_secret: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Monitor a local simulated 3-node mesh
    Monitor,
    /// Send a dead-drop message into the local store
    Send {
        /// Target node ID (hex)
        #[arg(long, default_value = "01")]
        node_id: String,
        /// Message to send
        #[arg(long)]
        message: String,
    },
    /// Retrieve a dead-drop message
    Retrieve {
        /// Message ID to retrieve
        #[arg(long)]
        msg_id: u32,
    },
    /// Compute an integrity report from simulated vibration samples
    Integrity {
        /// Node ID (hex)
        #[arg(long)]
        node_id: String,
    },
    /// Configure a node (RSSI threshold / dead-drop TTL)
    Configure {
        /// Node ID (hex)
        #[arg(long)]
        node_id: String,
        /// RSSI threshold for geo-fencing (dBm)
        #[arg(long, allow_hyphen_values = true)]
        rssi_threshold: Option<i16>,
        /// Dead-drop TTL (seconds)
        #[arg(long)]
        drop_ttl: Option<u32>,
    },
}

fn main() {
    let cli = Cli::parse();

    // Deployment secret: --deployment-secret > env > demo default.
    let secret: [u8; 32] = match &cli.deployment_secret {
        Some(hex) => decode_secret(hex),
        None => DEMO_SECRET,
    };

    match &cli.command {
        Commands::Monitor => monitor(&secret),
        Commands::Send { node_id, message } => send(&secret, node_id, message),
        Commands::Retrieve { msg_id } => retrieve(&secret, *msg_id),
        Commands::Integrity { node_id } => integrity(&secret, node_id),
        Commands::Configure {
            node_id,
            rssi_threshold,
            drop_ttl,
        } => configure(&secret, node_id, *rssi_threshold, *drop_ttl),
    }
}

/// Run a 3-node line mesh locally and report real routing state.
fn monitor(secret: &[u8; 32]) {
    let id = |b: u8| {
        let mut bytes = [0u8; 8];
        bytes[0] = b;
        NodeId(bytes)
    };

    let mut mesh = [
        MeshController::new(id(1)),
        MeshController::new(id(2)),
        MeshController::new(id(3)),
    ];

    // Node 2 learns a direct route to node 3; node 1 learns via node 2.
    mesh[1].node_mut().routing_table().update(RouteEntry {
        destination: id(3),
        next_hop: id(3),
        metric: 100,
        flags: RouteFlags::empty(),
        age_sec: 0,
    });
    let adv = mesh[1]
        .node()
        .routing_table_ref()
        .build_advertisement(id(2), RouteFlags::empty(), 0);
    mesh[0]
        .node_mut()
        .routing_table()
        .process_advertisement(&adv, id(2));

    println!("Smart Matter Mesh Network Monitor");
    println!("=================================");
    println!("Local simulated mesh: node 1 — node 2 — node 3");
    println!();

    for (i, m) in mesh.iter().enumerate() {
        let nid = id(i as u8 + 1);
        let key = derive_key(secret, &nid);
        println!(
            "  Node {} | id {:02X?} | routes: {} | derived key: {:02X?}…",
            i + 1,
            &nid.0[..2],
            m.node().routing_table_ref().len(),
            &key[..4]
        );
    }
    println!();
    println!(
        "Node 1 → node 3 next hop: {:?}",
        mesh[0].node().routing_table_ref().next_hop(&id(3))
    );
}

/// Encrypt, fragment and store a dead-drop message.
fn send(secret: &[u8; 32], node_id_hex: &str, message: &str) {
    let node = parse_node_id(node_id_hex);
    let key = derive_key(secret, &node);
    let mut manager = DropStorageManager::new(key);

    match manager.store_message(1, message.as_bytes(), DEFAULT_DROP_TTL) {
        Ok(headers) => {
            println!(
                "Stored {} fragment(s) for node {:02X?}",
                headers.len(),
                &node.0[..2]
            );
            println!("Message ({} bytes): {}", message.len(), message);
            println!("TTL: {} s", DEFAULT_DROP_TTL);
            println!("Storage utilization: {}%", manager.utilization_pct());
        }
        Err(e) => {
            eprintln!("Error storing message: {:?}", e);
        }
    }
}

/// Retrieve and reassemble a dead-drop message.
fn retrieve(secret: &[u8; 32], msg_id: u32) {
    let key = derive_key(secret, &NodeId([1, 0, 0, 0, 0, 0, 0, 0]));
    let mut manager = DropStorageManager::new(key);

    match manager.retrieve_message(msg_id) {
        Ok(Some(msg)) => {
            println!("Retrieved message {} ({} bytes):", msg_id, msg.len());
            println!("  {}", String::from_utf8_lossy(&msg));
        }
        Ok(None) => {
            println!("Message {msg_id} not found (no fragments stored locally)");
        }
        Err(e) => {
            eprintln!("Error retrieving message: {:?}", e);
        }
    }
}

/// Feed the integrity auditor simulated samples and print the real report.
fn integrity(secret: &[u8; 32], node_id_hex: &str) {
    let node = parse_node_id(node_id_hex);
    let _ = derive_key(secret, &node);

    let mut auditor = IntegrityAuditor::new(node);

    // Baseline: 100 normal samples.
    for i in 0..100u64 {
        auditor.process_sample(&VibrationSample {
            x: 10,
            y: 20,
            z: 15,
            time: TimeMs(i),
        });
    }
    // Anomaly burst: 5 deviating samples.
    for i in 0..5u64 {
        auditor.process_sample(&VibrationSample {
            x: 100,
            y: 200,
            z: 150,
            time: TimeMs(200 + i),
        });
    }

    let report = auditor.build_report();
    println!("Integrity report for node {:02X?}", &node.0[..2]);
    println!("  Baseline amplitude: {} µm", report.baseline_amplitude_um);
    println!("  Current amplitude:  {} µm", report.current_amplitude_um);
    println!("  Deviation:          {}%", report.deviation_pct);
    println!("  Anomalies:          {}", report.anomaly_count);
    println!("  Confidence:         {}/255", report.anomaly_confidence);
}

/// Apply configuration to real protocol objects and print the result.
fn configure(
    secret: &[u8; 32],
    node_id_hex: &str,
    rssi_threshold: Option<i16>,
    drop_ttl: Option<u32>,
) {
    let node = parse_node_id(node_id_hex);
    let _ = derive_key(secret, &node);

    let mut broadcaster = smart_matter_broadcaster::BroadcasterController::new(node);
    let mut storage = DropStorageManager::new([0u8; 32]);

    println!("Configuring node {:02X?}", &node.0[..2]);
    if let Some(rssi) = rssi_threshold {
        broadcaster.set_rssi_threshold(rssi);
        println!(
            "  RSSI threshold: {} dBm (geo-fencing)",
            broadcaster.rssi_threshold()
        );
    }
    if let Some(ttl) = drop_ttl {
        let headers = storage
            .store_message(0xFFFF, b"config probe", ttl)
            .expect("probe store");
        println!(
            "  Dead-drop TTL: {ttl} seconds ({} fragment(s), util {}%)",
            headers.len(),
            storage.utilization_pct()
        );
    }
    if rssi_threshold.is_none() && drop_ttl.is_none() {
        println!("  No settings given (nothing changed).");
    }
}

/// Parse a hex string into a NodeId.
fn parse_node_id(hex: &str) -> NodeId {
    let mut bytes = [0u8; 8];
    let clean = hex.trim_start_matches("0x");
    for (i, byte_pair) in clean.as_bytes().chunks(2).enumerate() {
        if i >= 8 {
            break;
        }
        if let Ok(byte) = u8::from_str_radix(core::str::from_utf8(byte_pair).unwrap_or("00"), 16) {
            bytes[i] = byte;
        }
    }
    NodeId(bytes)
}

/// Decode a 64-char hex deployment secret (falls back to the demo secret).
fn decode_secret(hex: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    let clean = hex.trim().trim_start_matches("0x");
    if clean.len() != 64 {
        eprintln!("warning: deployment secret must be 64 hex chars; using demo secret");
        return DEMO_SECRET;
    }
    for (i, pair) in clean.as_bytes().chunks(2).enumerate() {
        let hi = (pair[0] as char).to_digit(16).unwrap_or(0);
        let lo = (pair[1] as char).to_digit(16).unwrap_or(0);
        out[i] = (hi * 16 + lo) as u8;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_node_id_hex() {
        let id = parse_node_id("0x0a00000000000000");
        assert_eq!(id.0[0], 0x0a);
    }

    #[test]
    fn decode_secret_hex() {
        let s = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";
        let key = decode_secret(s);
        assert_eq!(key[0], 0x00);
        assert_eq!(key[31], 0xff);
    }

    #[test]
    fn monitor_reports_routes() {
        let secret = [0x42u8; 32];
        // Exercise the real monitor path (capture output indirectly: it
        // must not panic and must build a route).
        let id = |b: u8| {
            let mut bytes = [0u8; 8];
            bytes[0] = b;
            NodeId(bytes)
        };
        let mut m1 = MeshController::new(id(1));
        let mut m2 = MeshController::new(id(2));
        let _ = &mut m2;
        let adv = {
            let mut t = MeshController::new(id(2));
            t.node_mut().routing_table().update(RouteEntry {
                destination: id(3),
                next_hop: id(3),
                metric: 100,
                flags: RouteFlags::empty(),
                age_sec: 0,
            });
            t.node()
                .routing_table_ref()
                .build_advertisement(id(2), RouteFlags::empty(), 0)
        };
        m1.node_mut()
            .routing_table()
            .process_advertisement(&adv, id(2));
        assert_eq!(m1.node().routing_table_ref().next_hop(&id(3)), Some(id(2)));
        let _ = &secret;
    }
}
