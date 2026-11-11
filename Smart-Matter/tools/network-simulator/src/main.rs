//! # Smart Matter Network Simulator
//!
//! Desktop tool for simulating a Smart Matter mesh network with configurable
//! node counts and message traffic. Every reported metric is computed from a
//! real run of the mesh protocol (see `sim::run_simulation`).
//!
//! ## Usage
//!
//! ```bash
//! # Run a simulation with 50 nodes for 60 ticks and 100 messages
//! smart-matter-network-simulator --nodes 50 --duration 60 --messages 100
//! ```

mod sim;

use clap::Parser;

#[derive(Parser)]
#[command(name = "smart-matter-network-simulator")]
#[command(about = "Simulate Smart Matter mesh network")]
struct Cli {
    /// Number of nodes in the simulation (line topology)
    #[arg(long, default_value = "10")]
    nodes: u32,
    /// Duration of simulation in ticks (1 tick ≈ 1 beacon period / 5)
    #[arg(long, default_value = "60")]
    duration: u64,
    /// Number of dead-drop messages to route after warm-up
    #[arg(long, default_value = "20")]
    messages: u32,
}

fn main() {
    let cli = Cli::parse();

    let cfg = sim::SimConfig {
        nodes: cli.nodes,
        duration_ticks: cli.duration,
        messages: cli.messages,
    };

    println!("Smart Matter Network Simulator");
    println!("==============================");
    println!(
        "Topology: {nodes} nodes on a line, {spacing:.1} m spacing, {range:.1} m radio range",
        nodes = cfg.nodes,
        spacing = 1.0,
        range = 1.5
    );
    println!(
        "Duration: {} ticks | messages: {}",
        cfg.duration_ticks, cfg.messages
    );
    println!();
    println!("Running simulation...");

    let stats = sim::run_simulation(&cfg);

    println!();
    println!("Results (computed from the protocol run):");
    println!("  Fragments sent:      {}", stats.fragments_sent);
    println!("  Fragments delivered: {}", stats.fragments_delivered);
    let pdr = if stats.fragments_sent > 0 {
        stats.fragments_delivered as f32 * 100.0 / stats.fragments_sent as f32
    } else {
        0.0
    };
    println!("  Packet delivery:     {pdr:.1}%");
    println!("  Average latency:     {:.1} hops", stats.avg_hops);
    println!("  Messages routed:     {}", stats.routed_messages);
    println!("  Energy consumed:     {} µJ", stats.energy_consumed_uj);
    println!(
        "  Routing coverage:    {}/{} nodes know a route to the far end",
        stats.routing_coverage, cfg.nodes
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sim_cli_defaults_parse() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
