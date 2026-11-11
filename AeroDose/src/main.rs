use clap::Parser;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use smokeshift::{
    AlertTier, AuditLog, ComplianceEngine, ExposureLimits, ShiftDoseTracker, Sps30Calibrator, Sps30Reading,
};
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(name = "smokeshift-cli", author = "DeepMind", version = "0.1.0", about = "SmokeShift Occupational PM2.5 Dose Tracker Engine")]
struct Cli {
    #[arg(short, long, default_value_t = 8.0)]
    hours: f64,

    #[arg(short, long, default_value_t = 65.0)]
    smoke_pm25: f64,

    #[arg(long, default_value_t = 55.0)]
    rh_pct: f64,
}

fn main() {
    let args = Cli::parse();
    println!("=== SmokeShift Outdoor Worker Dose Tracker ===");
    println!("Shift Duration: {:.1} hours | Smoke Level: {:.1} µg/m³ | Ambient RH: {:.1}%\n", args.hours, args.smoke_pm25, args.rh_pct);

    let worker_id = Uuid::new_v4();
    let mut tracker = ShiftDoseTracker::new(ExposureLimits::default());
    let calibrator = Sps30Calibrator::new();
    
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();
    let mut audit_log = AuditLog::new(signing_key);

    let raw = Sps30Reading {
        pm1_0_ug_m3: args.smoke_pm25 * 0.6,
        pm2_5_ug_m3: args.smoke_pm25,
        pm4_0_ug_m3: args.smoke_pm25 * 1.2,
        pm10_0_ug_m3: args.smoke_pm25 * 1.5,
        typical_particle_size_um: 0.9,
    };

    let dt_step = 600.0; // 10 min steps
    let steps = ((args.hours * 3600.0) / dt_step) as usize;

    for _ in 0..steps {
        let calibrated_pm25 = calibrator.calibrate(&raw, args.rh_pct);
        let tier = tracker.record_sample(calibrated_pm25, dt_step);
        audit_log.append_entry(worker_id, calibrated_pm25, tracker.cumulative_dose_ug_h_m3, &format!("{tier:?}"));
    }

    println!("--- Shift Summary ---");
    println!("Time-Weighted Average (TWA): {:.2} µg/m³", tracker.twa_pm25_ug_m3());
    println!("Cumulative Dose: {:.2} µg·h/m³", tracker.cumulative_dose_ug_h_m3);
    println!("Current Alert Tier: {:?}", tracker.current_tier);
    println!("Audit Log Status: {} entries cryptographically verified ({})", audit_log.entries().len(), audit_log.verify_chain(&verifying_key).is_ok());

    let report = ComplianceEngine::generate_report(worker_id, &tracker, audit_log.entries());
    println!("\n--- OSHA Compliance Report ---");
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
