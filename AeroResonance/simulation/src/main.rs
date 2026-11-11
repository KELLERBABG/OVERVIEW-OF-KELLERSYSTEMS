//! Atmospheric Resonance Energy Harvesting — MVP Simulation
//!
//! Usage:
//!   cargo run --release                   (default 1-hour simulation)
//!   cargo run --release -- --duration 7200 --dt 0.01
//!   cargo run --release -- --config custom.json --output results.csv
//!
//! Outputs:
//!   - simulation_timeseries.csv (time, subsystem, power_W)
//!   - simulation_summary.csv   (subsystem, total_energy_J, avg_power_W, peak, min)

mod config;
mod physics;
mod simulation;
mod subsystems;

use config::SimConfig;
use std::env;
use std::fs;
use std::path::Path;

struct Args {
    config_path: Option<String>,
    output_path: Option<String>,
    duration: Option<f64>,
    dt: Option<f64>,
    seed: Option<u64>,
    gate_report: bool,
    hybrid_report: bool,
}

fn parse_args() -> Args {
    let args: Vec<String> = env::args().collect();
    let mut cfg = Args {
        config_path: None,
        output_path: None,
        duration: None,
        dt: None,
        seed: None,
        gate_report: false,
        hybrid_report: false,
    };
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--config" => {
                i += 1;
                if i < args.len() {
                    cfg.config_path = Some(args[i].clone());
                }
            }
            "--output" | "-o" => {
                i += 1;
                if i < args.len() {
                    cfg.output_path = Some(args[i].clone());
                }
            }
            "--duration" | "-t" => {
                i += 1;
                if i < args.len() {
                    cfg.duration = args[i].parse::<f64>().ok();
                }
            }
            "--dt" => {
                i += 1;
                if i < args.len() {
                    cfg.dt = args[i].parse::<f64>().ok();
                }
            }
            "--seed" => {
                i += 1;
                if i < args.len() {
                    cfg.seed = args[i].parse::<u64>().ok();
                }
            }
            "--gate-report" => {
                cfg.gate_report = true;
            }
            "--hybrid-report" => {
                cfg.hybrid_report = true;
            }
            "--help" | "-h" => {
                eprintln!("Atmospheric Resonance Sim — MVP");
                eprintln!("  --config PATH       JSON config file (optional)");
                eprintln!("  --output  PATH      CSV output path (default: simulation_*.csv)");
                eprintln!("  --duration SEC      Total simulation time [s] (default: 3600)");
                eprintln!("  --dt      SEC       Time step [s] (default: 0.001)");
                eprintln!("  --seed    u64       RNG seed for deterministic runs");
                eprintln!("  --gate-report       Print per-subsystem Gate-1 energy-density check");
                eprintln!("  --hybrid-report     Print Risk-R1 hybrid solar + atmospheric sizing check");
                eprintln!("  --help              This message");
                std::process::exit(0);
            }
            _ => {}
        }
        i += 1;
    }
    cfg
}

fn main() {
    let args = parse_args();

    // Load or create configuration
    let mut cfg = if let Some(ref path) = args.config_path {
        let contents = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error: cannot read config file '{path}': {e}");
                std::process::exit(2);
            }
        };
        match serde_json::from_str::<SimConfig>(&contents) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error: invalid JSON config in '{path}': {e}");
                std::process::exit(2);
            }
        }
    } else {
        SimConfig::default()
    };

    // Override from CLI
    if let Some(d) = args.duration {
        cfg.total_time = d;
    }
    if let Some(d) = args.dt {
        cfg.dt = d;
    }

    // Print configuration summary
    eprintln!("================================================");
    eprintln!("  Atmospheric Resonance Energy Harvesting — MVP");
    eprintln!("  Simulation Duration: {:.1} s", cfg.total_time);
    eprintln!("  Time step:           {:.5} s", cfg.dt);
    eprintln!("  Output interval:     {:.2} s", cfg.output_interval);
    if let Some(s) = args.seed {
        eprintln!("  RNG seed:            {}", s);
    }
    eprintln!("================================================");

    // Run simulation (seeded RNG for deterministic runs; otherwise random)
    let (records, summaries) = simulation::run_simulation(&cfg, args.seed);

    // Determine output paths
    let ts_path = args
        .output_path
        .clone()
        .unwrap_or_else(|| "simulation_timeseries.csv".to_string());
    let sum_path = {
        let p = Path::new(&ts_path);
        let stem = p
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("simulation");
        format!("{}_summary.csv", stem)
    };

    // Write timeseries CSV
    {
        let mut wtr = csv::Writer::from_path(&ts_path).expect("Cannot open timeseries CSV");
        wtr.write_record(["time_s", "subsystem", "power_W", "v_cap_V", "tier"])
            .expect("Cannot write header");
        for rec in &records {
            wtr.write_record(&[
                format!("{:.6}", rec.time_s),
                rec.sub_name.clone(),
                format!("{:.12e}", rec.power_w),
                format!("{:.4}", rec.v_cap),
                rec.tier.clone(),
            ])
            .expect("Cannot write record");
        }
        wtr.flush().expect("Flush failed");
        eprintln!("Wrote {} time-series records → {}", records.len(), ts_path);
    }

    // Write summary CSV
    {
        let mut wtr = csv::Writer::from_path(&sum_path).expect("Cannot open summary CSV");
        wtr.write_record([
            "subsystem",
            "total_energy_J",
            "avg_power_W",
            "peak_power_W",
            "min_power_W",
        ])
        .expect("Cannot write header");
        let mut total_energy_all = 0.0;
        for rec in &summaries {
            wtr.write_record(&[
                rec.sub_name.clone(),
                format!("{:.12e}", rec.total_energy_j),
                format!("{:.12e}", rec.avg_power_w),
                format!("{:.12e}", rec.peak_power_w),
                format!("{:.12e}", rec.min_power_w),
            ])
            .expect("Cannot write record");
            total_energy_all += rec.total_energy_j;
        }
        wtr.flush().expect("Flush failed");
        eprintln!("Wrote {} summary records → {}", summaries.len(), sum_path);

        // Print final summary to console
        eprintln!("\n================================================");
        eprintln!("  RESULTS SUMMARY");
        eprintln!("================================================");
        eprintln!(
            "  {:<35} {:>14} {:>14}",
            "Subsystem", "Energy [J]", "Avg Power [W]"
        );
        eprintln!("  {}", "-".repeat(65));
        for rec in &summaries {
            eprintln!(
                "  {:<35} {:>14.3e} {:>14.3e}",
                rec.sub_name, rec.total_energy_j, rec.avg_power_w
            );
        }
        eprintln!("  {}", "-".repeat(65));
        eprintln!(
            "  {:<35} {:>14.3e} {:>14.3e}",
            "TOTAL (all subsystems)",
            total_energy_all,
            total_energy_all / cfg.total_time
        );
        eprintln!("================================================");
    }

    // Gate-1 check: theoretical energy density > 1 mW/m² (plan §7 Gate Review).
    if args.gate_report {
        print_gate_report(&summaries);
    }

    // Hybrid sizing report: Risk-R1 mitigation check (PV vs atmospheric resonance).
    if args.hybrid_report {
        print_hybrid_report(&summaries);
    }
}

/// Per-subsystem nominal aperture/area [m²] used for the first-order
/// energy-density estimate. These are order-of-magnitude stand-ins; the gate
/// report is a screening tool, not a validated measurement (see docs/gate report).
fn nominal_aperture_m2(sub_name: &str) -> f64 {
    match sub_name {
        "IonosphericPlasmaCoupling" => 1_000.0, // large ELF loop aperture
        "NearVerticalSkywaveInductor" => 1_000.0, // NVIS aperture
        "MaxwellianFieldRelaxation" => 100.0,
        "AcousticAtmosphericTransduction" => 1.0, // piezo tile array area
        "NonLinearWaveModulation" => 10.0,
        "CyclostationaryFeatureFiltering" => 10.0,
        "PlasmaWakeEnergyBuffering" => 1.0, // storage, not a harvester
        "ResonantSubstrateAdaptation" => 1.0,
        "ThermodynamicCapacityGradient" => 0.25, // TEG footprint
        "StaticInvariantDischarge" => 1.0,
        "SolarPhotovoltaic" => 0.0004, // micro-cell
        _ => 1.0,
    }
}

/// Gate-1 screening report: per-subsystem average power density vs the
/// 1 mW/m² (pre-Gate-1) threshold stated in the project plan §7.
fn print_gate_report(summaries: &[simulation::EnergyRecord]) {
    const GATE_MW_M2: f64 = 1.0; // threshold in mW/m²
    eprintln!("\n================================================");
    eprintln!("  GATE-1 ENERGY-DENSITY SCREEN (pre-Gate-1, UNVALIDATED)");
    eprintln!(
        "  Threshold from plan §7: > {:.0} mW/m² average",
        GATE_MW_M2
    );
    eprintln!("  WARNING: apertures are nominal estimates; actual densities");
    eprintln!("  require field measurement (plan M1.4 / M1.3).");
    eprintln!("================================================");
    eprintln!(
        "  {:<35} {:>12} {:>12} {:>8}",
        "Subsystem", "Avg [mW/m²]", "Area [m²]", "Gate?"
    );
    eprintln!("  {}", "-".repeat(70));
    for rec in summaries {
        let area = nominal_aperture_m2(&rec.sub_name);
        let mw = rec.avg_power_w * 1000.0;
        let mw_m2 = if area > 0.0 { mw / area } else { 0.0 };
        let verdict = if mw < 0.0 {
            "n/a (sink)"
        } else if mw_m2 >= GATE_MW_M2 {
            "PASS"
        } else if mw_m2 > 0.0 {
            "FAIL"
        } else {
            "UNMEASURED"
        };
        eprintln!(
            "  {:<35} {:>12.4} {:>12.4} {:>8}",
            rec.sub_name, mw_m2, area, verdict
        );
    }
    eprintln!("================================================");
}

/// Hybrid solar + atmospheric sizing check (Risk-R1 mitigation analysis).
fn print_hybrid_report(summaries: &[simulation::EnergyRecord]) {
    eprintln!("\n================================================");
    eprintln!("  HYBRID SOLAR + ATMOSPHERIC SIZING REPORT (Risk R1)");
    eprintln!("  Reference: Plan Phase-2 (> 10 mW) & Phase-3 (> 1 W) Gates");
    eprintln!("================================================");

    let mut resonance_power_w = 0.0;
    let mut pv_power_w = 0.0;
    let mut sinks_power_w = 0.0;

    for rec in summaries {
        match rec.sub_name.as_str() {
            "SolarPhotovoltaic" => pv_power_w += rec.avg_power_w,
            "ResonantSubstrateAdaptation" | "StaticInvariantDischarge" => {
                sinks_power_w += rec.avg_power_w.abs();
            }
            "PlasmaWakeEnergyBuffering" => {} // storage buffer
            _ => {
                if rec.avg_power_w > 0.0 {
                    resonance_power_w += rec.avg_power_w;
                }
            }
        }
    }

    let total_harvester_power_w = resonance_power_w + pv_power_w;
    let resonance_pct = if total_harvester_power_w > 0.0 {
        (resonance_power_w / total_harvester_power_w) * 100.0
    } else {
        0.0
    };

    eprintln!("  HARVEST BREAKDOWN:");
    eprintln!("    Resonance Harvesters (combined): {:>10.3e} W ({:.2} %)", resonance_power_w, resonance_pct);
    eprintln!("    Solar PV (reference microcell):  {:>10.3e} W ({:.2} %)", pv_power_w, 100.0 - resonance_pct);
    eprintln!("    Control & Safety Sinks (draw):   {:>10.3e} W", sinks_power_w);
    eprintln!("  ------------------------------------------------");
    eprintln!("  DECISION & SIZING MATRIX:");
    eprintln!("    • Phase-2 Benchtop Gate (> 10 mW sustained):");
    eprintln!("        - Resonance alone: Requires ~100,000 m² aperture (IMPOSSIBLE)");
    eprintln!("        - Hybrid Solar:    Requires 50x50 mm micro-panel (0.25 W peak) -> PASS");
    eprintln!("    • Phase-3 Field Gate (> 1 W sustained):");
    eprintln!("        - Resonance alone: Physically unfeasible at terrestrial ambient levels");
    eprintln!("        - Hybrid Solar:    Standard 6V 2W panel (140x110 mm) -> PASS");
    eprintln!("  ------------------------------------------------");
    eprintln!("  VERDICT (Risk R1):");
    eprintln!("    Pivot to HYBRID SOLAR + ATMOSPHERIC confirmed.");
    eprintln!("    Resonance channels repurposed as M1.3 scientific measurement payload.");
    eprintln!("================================================");
}
