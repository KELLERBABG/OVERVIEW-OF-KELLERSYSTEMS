//! # Host Simulation Platform
//!
//! Runs the real node firmware loop as a desktop binary with simulated
//! sensors and a loopback radio. Used for demos and manual inspection of the
//! full wake→sample→process→transmit cycle.
//!
//! ```bash
//! cargo run -p smart-matter-node -- --ticks 120
//! ```

use std::time::{Duration, Instant};

use heapless::Vec;
use smart_matter_core::mesh::{Radio, RadioError};
use smart_matter_core::sensor::{Accelerometer, AdcNoiseSource, SensorError, TemperatureSensor};
use smart_matter_core::types::{NodeId, ProtocolSlice, TimeMs, VibrationSample};

use crate::node::Node;

/// Default number of ticks to run before exiting (1 tick ≈ 1 ms in sim).
const DEFAULT_TICKS: u64 = 200;

/// Simulated accelerometer: pedestrian/bridge vibration with occasional
/// structural-anomaly bursts (large deviations from baseline).
struct MockAccelerometer {
    tick: u64,
    anomaly_until: u64,
}

impl MockAccelerometer {
    fn new() -> Self {
        Self {
            tick: 0,
            anomaly_until: 0,
        }
    }
}

impl Accelerometer for MockAccelerometer {
    fn read_sample(&mut self) -> Result<VibrationSample, SensorError> {
        self.tick += 1;

        // Baseline forms after 100 samples; then every ~40 ticks a 6-tick
        // anomaly burst (e.g. a crack event).
        if self.tick >= 100 && self.tick.is_multiple_of(40) {
            self.anomaly_until = self.tick + 6;
        }
        let anomalous = self.tick < self.anomaly_until;

        let (base, amp) = if anomalous {
            // Structural event: high amplitude, off-axis.
            (0.0f32, 120.0f32)
        } else {
            // Ambient traffic vibration ~50 milli-g with ±6 oscillation
            // (≈12% deviation — below the 20% anomaly threshold).
            (50.0f32, 6.0f32)
        };
        let phase = (self.tick % 10) as f32 / 10.0 * core::f32::consts::TAU;
        let wobble = base + amp * phase.sin();

        Ok(VibrationSample {
            x: wobble as i16,
            y: (wobble * 0.6) as i16,
            z: (wobble * 0.3) as i16,
            time: TimeMs::now(),
        })
    }

    fn set_rate(&mut self, _hz: u32) -> Result<(), SensorError> {
        Ok(())
    }

    fn noise_floor(&self) -> f32 {
        220.0 // μg/√Hz (LIS3DH)
    }
}

/// Simulated ADC noise source (dithering around a nominal reading).
struct MockAdcNoise {
    tick: u32,
}

impl AdcNoiseSource for MockAdcNoise {
    fn read_raw(&mut self) -> Result<u32, SensorError> {
        self.tick = self.tick.wrapping_add(1);
        // Pseudo-random dither from a LCG — good enough for entropy demo.
        let noise = self.tick.wrapping_mul(1664525).wrapping_add(1013904223) % 4096;
        Ok(2048 + noise)
    }
}

/// Simulated temperature sensor.
struct MockTemperature;

impl TemperatureSensor for MockTemperature {
    fn read_millicelsius(&mut self) -> Result<i32, SensorError> {
        Ok(25_000) // 25.0 °C
    }
}

/// Loopback radio: outgoing slices are counted and dropped.
struct LoopbackRadio {
    tx_count: u32,
}

impl Radio for LoopbackRadio {
    fn send(&mut self, _slice: &ProtocolSlice) -> Result<(), RadioError> {
        self.tx_count += 1;
        Ok(())
    }

    fn receive(&mut self, _timeout_ms: u32) -> Result<Option<ProtocolSlice>, RadioError> {
        Ok(None)
    }

    fn last_rssi(&self) -> i16 {
        -50
    }
}

/// Run the host simulation. `--ticks N` controls the run length; the node
/// deployment secret is read from `SMART_MATTER_DEPLOYMENT_SECRET` (hex) or
/// a fixed demo default.
pub fn run(args: std::vec::Vec<String>) {
    let mut ticks = DEFAULT_TICKS;
    let mut secret = [0x42u8; 32];
    if let Ok(env_secret) = std::env::var("SMART_MATTER_DEPLOYMENT_SECRET") {
        if let Some(bytes) = decode_hex(&env_secret) {
            secret.copy_from_slice(&bytes);
        } else {
            eprintln!(
                "warning: SMART_MATTER_DEPLOYMENT_SECRET is not valid hex; using demo default"
            );
        }
    }

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--ticks" => {
                i += 1;
                if let Some(v) = args.get(i).and_then(|s| s.parse().ok()) {
                    ticks = v;
                }
            }
            other => {
                eprintln!("unknown argument: {other}");
            }
        }
        i += 1;
    }

    let mut node_id = [0u8; 8];
    node_id[7] = 0x01;
    let node_id = NodeId(node_id);

    let mut node = Node::new(
        node_id,
        &secret,
        LoopbackRadio { tx_count: 0 },
        MockAccelerometer::new(),
        MockAdcNoise { tick: 0 },
        MockTemperature,
    );

    println!(
        "Smart Matter node (host simulation) — node {:02X?}",
        &node.id().0[..]
    );
    println!("====================================================");
    println!(
        "Ticks: {ticks} | deployment secret: {} bytes | derived key: {:02X?}…",
        secret.len(),
        &node.key()[..4]
    );
    println!();

    let started = Instant::now();
    let mut last_status = 0;
    let mut total_tx = 0u32;
    let mut total_retrieved = 0u32;

    for _ in 0..ticks {
        let report = node.tick();
        total_tx += report.tx_count;
        total_retrieved += report.retrieved_count;

        if report.tick - last_status >= 10 {
            last_status = report.tick;
            let deviation = report.integrity.map(|r| r.deviation_pct).unwrap_or(0);
            println!(
                "tick {:>4} | energy {:<8} ({:>6} µJ, {} slices) | routes {} | storage {}% | anomalies {} | deviation {}% | tx {}",
                report.tick,
                format!("{:?}", report.energy_level),
                report.stored_uj,
                report.available_slices,
                report.routing_entries,
                report.storage_util_pct,
                report.anomaly_count,
                deviation,
                report.tx_count,
            );
        }

        std::thread::sleep(Duration::from_millis(5)); // sim: 1 tick ≈ 5 ms
    }

    let elapsed = started.elapsed();
    println!();
    println!(
        "Simulation complete after {} ticks ({elapsed:?}).",
        node.tick_count()
    );
    println!("Total slices transmitted: {total_tx}");
    println!("Total dead-drop messages retrieved: {total_retrieved}");
    println!("Final energy level: {:?}", node.energy_level());
    println!("Fragments still stored: {}", node.storage().len());
}

/// Decode a hex string into bytes (returns `None` on odd length / bad hex).
fn decode_hex(s: &str) -> Option<Vec<u8, 64>> {
    let s = s.trim().trim_start_matches("0x");
    if !s.len().is_multiple_of(2) {
        return None;
    }
    let mut out = Vec::new();
    for chunk in s.as_bytes().chunks(2) {
        let hi = (chunk[0] as char).to_digit(16)?;
        let lo = (chunk[1] as char).to_digit(16)?;
        out.push((hi * 16 + lo) as u8).ok()?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_hex() {
        assert_eq!(decode_hex("0x0a0b0c").unwrap()[..], [0x0a, 0x0b, 0x0c]);
        assert_eq!(decode_hex("00ff").unwrap()[..], [0x00, 0xff]);
        assert!(decode_hex("abc").is_none()); // odd length
        assert!(decode_hex("zz").is_none()); // invalid hex
    }
}
