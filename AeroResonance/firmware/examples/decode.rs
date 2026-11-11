//! Decode a captured GHOST-Net frame from the field node (host tool).
//!
//! The node prints each telemetry frame as a hex line:
//!
//! ```text
//!   GHOST [45, 48, 41, 52, 1a, 2b, ...]
//! ```
//!
//! Copy that byte list (with or without brackets/commas/`0x` prefixes) and:
//!
//! ```bash
//! cargo run --example decode -- "45, 48, 41, 52, 1a, ..."
//! cargo run --example decode -- 454841521a...
//! ```
//!
//! Uses the **v1 dev key** (`GHOST_KEY` in `esp32c3/src/main.rs`). A frame
//! that fails authentication prints an error and exits non-zero — that is the
//! tamper/wrong-key detection working.

use ehar_firmware_core::{parse_packet, TelemetryFrame};

/// v1 dev key — must match `GHOST_KEY` in `firmware/esp32c3/src/main.rs`.
const DEV_KEY: [u8; 32] = [0x42; 32];

fn parse_hex(s: &str) -> Result<Vec<u8>, String> {
    let cleaned: String = s
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ',' && *c != '[' && *c != ']')
        .collect();
    let cleaned = cleaned.trim_start_matches("0x");
    if !cleaned.len().is_multiple_of(2) {
        return Err(format!("odd number of hex digits: {cleaned}"));
    }
    let mut out = Vec::with_capacity(cleaned.len() / 2);
    for i in (0..cleaned.len()).step_by(2) {
        let byte = u8::from_str_radix(&cleaned[i..i + 2], 16)
            .map_err(|e| format!("invalid hex at '{:?}': {e}", &cleaned[i..i + 2]))?;
        out.push(byte);
    }
    Ok(out)
}

fn print_frame(f: &TelemetryFrame) {
    println!("decoded frame:");
    println!("  seq       = {}", f.seq);
    println!("  tier      = {}", f.tier_name);
    println!("  v_cap     = {:.3} V", f.reading.v_cap);
    println!("  pv_voltage= {:.3} V", f.reading.pv_voltage_v);
    println!("  elf       = {:.1} mV", f.reading.elf_mv);
    println!("  piezo     = {:.1} mV", f.reading.piezo_mv);
    println!("  entropy   = {} B", f.entropy_bytes);
    if f.reading.temp_c != 0.0 {
        println!(
            "  bme280    = {:.1} °C, {:.0} %RH, {:.0} hPa",
            f.reading.temp_c, f.reading.humidity_pct, f.reading.pressure_hpa
        );
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: cargo run --example decode -- \"<hex bytes of the GHOST line>\"");
        std::process::exit(2);
    }
    let bytes = match parse_hex(&args[1]) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    };
    if bytes.len() < 4 + 28 {
        eprintln!(
            "error: too short to be a frame ({}, need ≥ 32 bytes)",
            bytes.len()
        );
        std::process::exit(2);
    }
    match parse_packet(&DEV_KEY, &bytes) {
        Some(frame) => print_frame(&frame),
        None => {
            eprintln!("authentication failed: wrong key, corrupted, or not an EHAR frame");
            std::process::exit(1);
        }
    }
}
