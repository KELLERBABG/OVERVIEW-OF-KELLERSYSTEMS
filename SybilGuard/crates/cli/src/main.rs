//! Sybil-Resistentes CLI — Command-line interface for the Proof-of-Personhood system.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "sybil", version, about = "Sybil-Resistentes Proof-of-Personhood CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a synthetic social graph for testing
    GenerateGraph {
        /// Number of nodes
        #[arg(long, default_value = "100")]
        nodes: usize,
        /// Number of Sybil clusters to inject
        #[arg(long, default_value = "3")]
        sybil_clusters: usize,
        /// Size of each Sybil cluster
        #[arg(long, default_value = "20")]
        cluster_size: usize,
    },
    /// Analyze graph topology
    AnalyzeGraph {
        /// Path to graph JSON file
        #[arg(long)]
        input: String,
    },
    /// Start the API server
    Serve {
        /// Port to bind
        #[arg(long, default_value = "8080")]
        port: u16,
    },
    /// Create a new DID
    CreateDid {
        /// Node ID label
        #[arg(long)]
        node: String,
    },
    /// Run validation tests
    Validate {
        /// Graph size for synthetic test
        #[arg(long, default_value = "200")]
        graph_size: usize,
    },
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::GenerateGraph {
            nodes,
            sybil_clusters,
            cluster_size,
        } => {
            let config = sybil_core::SyntheticGraphConfig {
                num_nodes: nodes,
                num_sybil_clusters: sybil_clusters,
                sybil_cluster_size: cluster_size,
                ..Default::default()
            };
            let generator = sybil_graph::synthetic::SyntheticGraphGenerator::new(config);
            match generator.generate() {
                Ok((graph, ground_truth)) => {
                    println!("Generated graph: {} nodes, {} edges", graph.node_count(), graph.edge_count());
                    let sybil_count = ground_truth.iter().filter(|(_, s)| *s).count();
                    println!("Sybil nodes: {} / {}", sybil_count, ground_truth.len());
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Commands::AnalyzeGraph { input } => {
            println!("Analyzing graph from {}... (not yet implemented)", input);
        }
        Commands::Serve { port } => {
            println!("Starting API server on port {}...", port);
            sybil_api::server::start_server(port).await;
        }
        Commands::CreateDid { node } => {
            println!("Creating DID for node {}... (not yet implemented)", node);
        }
        Commands::Validate { graph_size } => {
            println!("Running validation with graph size {}...", graph_size);
            let config = sybil_core::SyntheticGraphConfig {
                num_nodes: graph_size,
                ..Default::default()
            };
            let generator = sybil_graph::synthetic::SyntheticGraphGenerator::new(config);
            match generator.generate() {
                Ok((graph, ground_truth)) => {
                    let sybil_detector = sybil_sybil::detector::SybilDetector::new(
                        &graph,
                        sybil_core::SybilDetectionConfig::default(),
                    );
                    let results = sybil_detector.detect().unwrap();
                    println!("Detection found {} suspicious regions", results.len());
                    for result in &results {
                        println!(
                            "  Region {}: {} nodes, entropy={:.3}, anomaly={:.3}, confidence={:.3}",
                            result.region_id, result.node_count, result.entropy_score, result.anomaly_score, result.confidence
                        );
                    }
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }
    }
}