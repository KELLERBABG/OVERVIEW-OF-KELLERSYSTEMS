//! Plane OS — demo binary.
//!
//! Runs a short demo flight scenario through the mock simulator:
//!
//! 1. Normal cruise — every command is verified and approved.
//! 2. A high-speed disturbance — the trustless execution gate blocks the
//!    unsafe command and the aircraft freezes.
//! 3. The hash-chained flight-recorder spec is validated and exported.
//!
//! **Research prototype only.** Not a certified avionics system.

use plane_os::controller::PilotInput;
use plane_os::dynamics::{AircraftConfig, FlightState, TerrainModel, Vec3};
use plane_os::simulator::PlaneSimulator;
use plane_os::spec::export_spec;

fn main() {
    println!("=== Plane OS: Formally Verified Flight Control Prototype ===");
    println!("(Research / open-source tooling only — NOT certified avionics)\n");

    // 1) Build the mock transport-category aircraft at cruise.
    let initial = FlightState::level(Vec3::new(0.0, 0.0, 3_000.0), 150.0, 0.0);
    let mut sim = PlaneSimulator::new(
        initial,
        AircraftConfig::default(),
        TerrainModel::sea_level(),
    );

    println!("--- Phase 1: Normal cruise (all commands should be approved) ---");
    for i in 1..=10 {
        let report = sim.step(PilotInput::default());
        let (_approved, blocked) = sim.stats();
        println!(
            "cycle {:>2} | t={:6.2}s | approved={} | blocks_total={} | check={}us",
            i,
            sim.state.t,
            report.approved,
            blocked,
            report.check_time_us
        );
        assert!(
            report.approved,
            "normal cruise must not be blocked (cycle {})",
            i
        );
    }
    println!(
        "Cruise result: IAS {:.0} kt, ALT {:.0} m, spec-chain valid: {}\n",
        sim.state.ias_knots(),
        sim.state.altitude(),
        sim.spec_valid()
    );

    // 2) Trigger a dangerous condition: over-speed.
    //
    // NOTE on altitudes: IAS is density-compensated, so 182 m/s at
    // 3000 m yields only ~301 kt IAS — below V_NE = 340 kt. To make the
    // hazard real we dive to sea level first, where 182 m/s ≈ 354 kt IAS.
    println!("--- Phase 2: Overspeed disturbance (gate must block) ---");
    sim.state.pos = Vec3::new(0.0, 0.0, 0.0); // dive to sea level
    sim.state.vel_body = Vec3::new(182.0, 0.0, 0.0); // ~354 kt IAS
    let report = sim.step(PilotInput::default());
    println!(
        "cycle {:>2} | approved={} | blocked_reason={:?}",
        sim.cycle,
        report.approved,
        report.blocked_reason
    );
    assert!(report.blocked(), "overspeed must be blocked");
    println!("  Verdicts:");
    for case in &report.cases {
        println!("    [{}] {} — {}", case.verdict, case.name, case.formula);
    }
    println!(
        "  Aircraft frozen: IAS {:.0} kt, ALT {:.0} m\n",
        sim.state.ias_knots(),
        sim.state.altitude()
    );

    // 3) The flight recorder spec.
    println!("--- Phase 3: Flight-recorder formal spec (hash-chained) ---");
    let (approved, blocked) = sim.stats();
    println!("Gate totals: approved={} blocked={}", approved, blocked);
    println!("Spec chain valid: {}\n", sim.spec_valid());
    println!("--- Exported formal spec (excerpt) ---");
    let spec = export_spec(&sim.spec);
    // Print a short excerpt.
    let lines = spec.lines().collect::<Vec<_>>();
    let take = 12.min(lines.len());
    for line in &lines[..take] {
        println!("{}", line);
    }
    println!("  ... ({} lines total)", lines.len());
    println!("\nDone. Simulation safe, verification chain intact.");
}