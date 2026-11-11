use std::path::PathBuf;

use chrono::Utc;
use clap::{Parser, Subcommand};
use pod_core::event::{EventMetadata, IncidentWindow, TelemetryEvent, Verdict};
use pod_core::policy::DamagePolicy;
use pod_core::sensor::{SensorReading, SensorType};
use uuid::Uuid;

use incident_audit::{build_trace, generate_report, IncidentExplorer};
use physics_logic::conditions::{evaluate_combination, evaluate_policy};
use sensor_integrity::taint::audit_signed_readings;
use telemetry_graph::ingest;
use telemetry_graph::TelemetryGraph;

#[derive(Parser)]
#[command(name = "pod", version, about = "Autonomous Proof-of-Damage CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Ingest sensor data from CSV into a telemetry graph
    Ingest {
        /// Path to CSV file
        file: PathBuf,
        #[arg(long, default_value = "00000000-0000-0000-0000-000000000001")]
        sensor_id: String,
        #[arg(long, default_value = "accelerometer")]
        sensor_type: String,
        #[arg(long, default_value = "timestamp_ms")]
        timestamp_col: String,
        #[arg(long, default_value = "value")]
        value_col: String,
        #[arg(long, default_value = "g")]
        unit: String,
    },
    /// Verify a damage policy against sensor data (event JSON)
    Verify {
        /// Path to policy JSON
        policy: PathBuf,
        /// Path to event JSON
        event: PathBuf,
    },
    /// Simulate a scenario and write test data files
    Simulate {
        #[arg(value_enum)]
        scenario: Scenario,
        /// Output directory for generated files
        #[arg(short, long, default_value = "test-data")]
        output: PathBuf,
        /// Also output CSV of raw sensor values
        #[arg(long)]
        csv: bool,
    },
    /// Query a sensor's time window from an ingested graph
    Query {
        /// Path to ingested event JSON
        event: PathBuf,
        /// Sensor ID to query
        #[arg(long)]
        sensor: Option<String>,
        /// Window start timestamp (ms)
        #[arg(long)]
        start: Option<i64>,
        /// Window end timestamp (ms)
        #[arg(long)]
        end: Option<i64>,
    },
    /// Build and display an audit trail (constraint trace) from event + policy
    Trail {
        /// Path to policy JSON
        policy: PathBuf,
        /// Path to event JSON
        event: PathBuf,
    },
    /// Classify an incident as acute, gradual, anomaly, etc.
    Classify {
        /// Path to policy JSON
        policy: PathBuf,
        /// Path to event JSON
        event: PathBuf,
    },
    /// Generate a machine-readable damage report
    Report {
        /// Path to policy JSON
        policy: PathBuf,
        /// Path to event JSON
        event: PathBuf,
        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Compact JSON (one line)
        #[arg(long)]
        compact: bool,
    },
    /// Run taint analysis on a signed sensor stream
    Taint {
        /// Path to event JSON with signed readings
        event: PathBuf,
    },
    /// Check integrity of a signed stream
    Audit {
        /// Path to signed stream JSON
        stream: PathBuf,
    },
}

#[derive(clap::ValueEnum, Clone)]
enum Scenario {
    NormalDriving,
    FrontalCollision,
    RearEndCollision,
    FraudSpike,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Ingest { file, sensor_id, sensor_type, timestamp_col, value_col, unit } => {
            cmd_ingest(file, &sensor_id, &sensor_type, &timestamp_col, &value_col, &unit)
        }
        Commands::Verify { policy, event } => cmd_verify(&policy, &event),
        Commands::Simulate { scenario, output, csv } => cmd_simulate(scenario, &output, csv),
        Commands::Query { event, sensor, start, end } => cmd_query(&event, sensor, start, end),
        Commands::Trail { policy, event } => cmd_trail(&policy, &event),
        Commands::Classify { policy, event } => cmd_classify(&policy, &event),
        Commands::Report { policy, event, output, compact } => cmd_report(&policy, &event, output, compact),
        Commands::Taint { event } => cmd_taint(&event),
        Commands::Audit { stream } => cmd_audit(&stream),
    }
}

fn cmd_ingest(
    file: PathBuf,
    sensor_id: &str,
    sensor_type: &str,
    timestamp_col: &str,
    value_col: &str,
    unit: &str,
) {
    let sid = Uuid::parse_str(sensor_id).expect("Invalid sensor ID");
    let st = parse_sensor_type(sensor_type);

    let config = ingest::CsvIngestionConfig {
        sensor_id: sid,
        sensor_type: st,
        timestamp_col: timestamp_col.to_string(),
        value_col: value_col.to_string(),
        unit: unit.to_string(),
    };

    let mut graph = TelemetryGraph::new();
    match ingest::ingest_csv(&file, config, &mut graph) {
        Ok(count) => {
            let all_ids: Vec<String> = graph.all_sensor_ids().iter().map(|id| id.to_string()).collect();
            println!("Ingested {count} readings");
            println!("Graph: {} sensor nodes", graph.node_count());
            println!("Sensors: {:?}", all_ids);
        }
        Err(e) => {
            eprintln!("Ingestion failed: {e}");
            std::process::exit(1);
        }
    }
}

fn cmd_verify(policy_path: &PathBuf, event_path: &PathBuf) {
    let policy_json = std::fs::read_to_string(policy_path).expect("Cannot read policy file");
    let policy: DamagePolicy = serde_json::from_str(&policy_json).expect("Invalid policy JSON");

    let event_json = std::fs::read_to_string(event_path).expect("Cannot read event file");
    let event: TelemetryEvent = serde_json::from_str(&event_json).expect("Invalid event JSON");

    let readings: Vec<SensorReading> = event.readings.into_iter().map(|sr| sr.reading).collect();

    let conditions = evaluate_policy(&policy, &readings);
    let approved = evaluate_combination(&conditions, &policy.combination);
    let verdict = if approved { Verdict::Approved } else { Verdict::Rejected };

    println!("Policy:     {}", policy.name);
    println!("Verdict:    {:?}", verdict);
    println!("Conditions:");
    for c in &conditions {
        let status = if c.satisfied { "PASS" } else { "FAIL" };
        println!("  [{status}] {:<30} ({})", c.name, c.expression);
    }

    std::process::exit(if approved { 0 } else { 1 });
}

fn cmd_simulate(scenario: Scenario, output_dir: &PathBuf, also_csv: bool) {
    std::fs::create_dir_all(output_dir).expect("Cannot create output directory");

    let now = Utc::now().timestamp_millis();
    let sensor_id = Uuid::nil();

    let (readings, label) = match scenario {
        Scenario::NormalDriving => {
            let r: Vec<_> = (0..100)
                .map(|i| SensorReading {
                    sensor_id,
                    sensor_type: SensorType::Accelerometer,
                    timestamp: now + i as i64 * 100,
                    value: 0.3 + (i as f64 * 0.01).sin() * 0.2,
                    unit: "g".into(),
                })
                .collect();
            (r, "normal-driving")
        }
        Scenario::FrontalCollision => {
            let mut r = Vec::new();
            for i in 0..100 {
                let val = if i < 70 {
                    0.3
                } else if i < 75 {
                    2.0 + (i as f64 - 70.0) * 2.5
                } else if i < 80 {
                    12.0 + (i as f64 - 75.0) * 1.5
                } else {
                    (14.0 - (i as f64 - 80.0) * 2.0).max(0.3)
                };
                r.push(SensorReading {
                    sensor_id,
                    sensor_type: SensorType::Accelerometer,
                    timestamp: now + i as i64 * 100,
                    value: val,
                    unit: "g".into(),
                });
            }
            (r, "frontal-collision")
        }
        Scenario::RearEndCollision => {
            let mut r = Vec::new();
            for i in 0..100 {
                let val = if i < 65 {
                    0.3
                } else if i < 75 {
                    -5.0 - (i as f64 - 65.0) * 1.5
                } else if i < 85 {
                    -8.0 + (i as f64 - 75.0) * 1.0
                } else {
                    (-3.0 + (i as f64 - 85.0) * 0.3).max(0.3)
                };
                r.push(SensorReading {
                    sensor_id,
                    sensor_type: SensorType::Accelerometer,
                    timestamp: now + i as i64 * 100,
                    value: val,
                    unit: "g".into(),
                });
            }
            (r, "rear-end-collision")
        }
        Scenario::FraudSpike => {
            let r: Vec<_> = (0..100)
                .enumerate()
                .map(|(i, _)| SensorReading {
                    sensor_id,
                    sensor_type: SensorType::Accelerometer,
                    timestamp: now + i as i64 * 100,
                    value: if i == 50 { 99.9 } else { 0.3 },
                    unit: "g".into(),
                })
                .collect();
            (r, "fraud-spike")
        }
    };

    let event = TelemetryEvent {
        event_id: Uuid::new_v4(),
        incident_window: IncidentWindow {
            start_timestamp: now,
            end_timestamp: now + 100 * 100,
        },
        readings: readings
            .iter()
            .cloned()
            .map(|r| pod_core::sensor::SignedReading {
                reading: r,
                signature: vec![],
                signer_public_key: vec![],
            })
            .collect(),
        metadata: EventMetadata {
            source: "simulation".into(),
            vehicle_id: Some("SIM-001".into()),
            location: None,
            description: Some(label.to_string()),
        },
    };

    let event_path = output_dir.join(format!("{label}.json"));
    let event_json = serde_json::to_string_pretty(&event).expect("Serialization failed");
    std::fs::write(&event_path, &event_json).expect("Cannot write event file");
    println!("Wrote event: {}", event_path.display());

    if also_csv {
        let csv_path = output_dir.join(format!("{label}.csv"));
        let mut wtr = csv::Writer::from_path(&csv_path).expect("Cannot create CSV");
        wtr.write_record(&["timestamp_ms", "value_g"]).ok();
        for r in &readings {
            wtr.write_record(&[r.timestamp.to_string(), r.value.to_string()]).ok();
        }
        wtr.flush().ok();
        println!("Wrote CSV:  {}", csv_path.display());
    }
}

fn cmd_query(event_path: &PathBuf, sensor: Option<String>, start: Option<i64>, end: Option<i64>) {
    let event_json = std::fs::read_to_string(event_path).expect("Cannot read event file");
    let event: TelemetryEvent = serde_json::from_str(&event_json).expect("Invalid event JSON");

    let readings: Vec<SensorReading> = event.readings.into_iter().map(|sr| sr.reading).collect();

    let sid = sensor
        .as_ref()
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or(Uuid::nil());

    let start_ts = start.unwrap_or(event.incident_window.start_timestamp);
    let end_ts = end.unwrap_or(event.incident_window.end_timestamp);

    let in_window: Vec<&SensorReading> = readings
        .iter()
        .filter(|r| r.sensor_id == sid && r.timestamp >= start_ts && r.timestamp <= end_ts)
        .collect();

    if in_window.is_empty() {
        println!("No readings in window [{start_ts}, {end_ts}] for sensor {sid}");
        return;
    }

    let values: Vec<f64> = in_window.iter().map(|r| r.value).collect();
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let sum: f64 = values.iter().sum();
    let mean = sum / values.len() as f64;

    println!("Query: {count} readings in window [{start_ts}, {end_ts}]", count = in_window.len());
    println!("  min:   {min:.3}");
    println!("  max:   {max:.3}");
    println!("  mean:  {mean:.3}");
    println!("  count: {}", in_window.len());
}

fn cmd_trail(policy_path: &PathBuf, event_path: &PathBuf) {
    let policy_json = std::fs::read_to_string(policy_path).expect("Cannot read policy file");
    let policy: DamagePolicy = serde_json::from_str(&policy_json).expect("Invalid policy JSON");

    let event_json = std::fs::read_to_string(event_path).expect("Cannot read event file");
    let event: TelemetryEvent = serde_json::from_str(&event_json).expect("Invalid event JSON");

    let readings: Vec<SensorReading> = event.readings.into_iter().map(|sr| sr.reading).collect();

    let trace = build_trace(&policy, &readings);

    println!("Incident ID:     {}", trace.incident_id);
    println!("Policy:          {}", trace.policy_name);
    println!("Boolean Expr:    {}", trace.boolean_expression);
    println!("Verdict:         {}", if trace.overall_verdict { "APPROVED" } else { "REJECTED" });
    println!("Events:          {}", trace.event_count());
    if let Some((start, end)) = trace.window_of_interest() {
        println!("Window of interest: [{start}, {end}] ({} ms)", end - start);
    }
    println!();
    println!("{:<12} {:<20} {:<10} {}", "Timestamp", "Threshold", "Value", "Status");
    println!("{}", "-".repeat(60));
    for e in &trace.events {
        let mark = if e.satisfied { "✓" } else { "·" };
        println!("{:<12} {:<20} {:<10.3} {}", e.timestamp, e.threshold_name, e.reading_value, mark);
    }
}

fn cmd_classify(policy_path: &PathBuf, event_path: &PathBuf) {
    let policy_json = std::fs::read_to_string(policy_path).expect("Cannot read policy file");
    let policy: DamagePolicy = serde_json::from_str(&policy_json).expect("Invalid policy JSON");

    let event_json = std::fs::read_to_string(event_path).expect("Cannot read event file");
    let event: TelemetryEvent = serde_json::from_str(&event_json).expect("Invalid event JSON");

    let readings: Vec<SensorReading> = event.readings.into_iter().map(|sr| sr.reading).collect();

    let trace = build_trace(&policy, &readings);
    let class = IncidentExplorer::classify(&trace);
    let confidence = IncidentExplorer::confidence(&trace);

    let verdict = if trace.overall_verdict { "APPROVED" } else { "REJECTED" };

    println!("Incident ID:  {}", trace.incident_id);
    println!("Classification: {}", class.label());
    println!("Confidence:     {:.1}%", confidence * 100.0);
    println!("Verdict:        {}", verdict);
}

fn cmd_report(policy_path: &PathBuf, event_path: &PathBuf, output: Option<PathBuf>, compact: bool) {
    let policy_json = std::fs::read_to_string(policy_path).expect("Cannot read policy file");
    let policy: DamagePolicy = serde_json::from_str(&policy_json).expect("Invalid policy JSON");

    let event_json = std::fs::read_to_string(event_path).expect("Cannot read event file");
    let event: TelemetryEvent = serde_json::from_str(&event_json).expect("Invalid event JSON");

    let report = generate_report(&event, &policy);

    let json = if compact {
        report.to_json_compact()
    } else {
        report.to_json_pretty()
    };

    match output {
        Some(path) => {
            std::fs::write(&path, &json).expect("Cannot write report file");
            println!("Wrote report: {}", path.display());
        }
        None => {
            println!("{}", json);
        }
    }
}

fn cmd_taint(event_path: &PathBuf) {
    let event_json = std::fs::read_to_string(event_path).expect("Cannot read event file");
    let event: TelemetryEvent = serde_json::from_str(&event_json).expect("Invalid event JSON");

    let report = audit_signed_readings(&event.readings);

    println!("=== Taint Analysis ===");
    println!("Total readings:  {}", report.total_readings);
    println!("Replay attacks:  {}", report.replay_count);
    println!("Injections:      {}", report.injection_count);
    println!("Temporal anomalies: {}", report.temporal_anomaly_count);
    println!("Trust score:     {:.1}%", report.trust_score * 100.0);
    println!("Verdict:         {:?}", report.verdict);

    if !report.replay_details.is_empty() {
        println!("\n[Replay Events]");
        for r in &report.replay_details {
            println!("  ts={} value={} occurrences={}", r.timestamp, r.value, r.occurrences);
        }
    }
    if !report.injection_details.is_empty() {
        println!("\n[Injection Events]");
        for i in &report.injection_details {
            println!("  index={} ts={} detail={}", i.index, i.timestamp, i.detail);
        }
    }
    if !report.temporal_details.is_empty() {
        println!("\n[Temporal Anomalies]");
        for t in &report.temporal_details {
            println!("  index={} ts={} prev={} detail={}", t.index, t.timestamp, t.prev_timestamp, t.detail);
        }
    }
}

fn cmd_audit(stream_path: &PathBuf) {
    let json = std::fs::read_to_string(stream_path).expect("Cannot read stream file");

    if let Ok(event) = serde_json::from_str::<TelemetryEvent>(&json) {
        let report = audit_signed_readings(&event.readings);
        println!("=== Stream Audit ===");
        println!("Verdict:     {:?}", report.verdict);
        println!("Trust score: {:.1}%", report.trust_score * 100.0);
        println!("Readings:    {}", report.total_readings);
        println!("Replays:     {}", report.replay_count);
        println!("Injections:  {}", report.injection_count);
        println!("Anomalies:   {}", report.temporal_anomaly_count);
        return;
    }

    if let Ok(chain) = serde_json::from_str::<Vec<sensor_integrity::chain::ChainLink>>(&json) {
        println!("Stream has {} chain links. Use 'taint' for detailed analysis.", chain.len());
        return;
    }

    eprintln!("Cannot parse as TelemetryEvent or ChainLink array");
    std::process::exit(1);
}

fn parse_sensor_type(s: &str) -> SensorType {
    match s {
        "accelerometer" => SensorType::Accelerometer,
        "gyroscope" => SensorType::Gyroscope,
        "speedometer" => SensorType::Speedometer,
        "gps" => SensorType::Gps,
        "temperature" => SensorType::Temperature,
        "strain" => SensorType::Strain,
        other => SensorType::Custom(other.to_string()),
    }
}
