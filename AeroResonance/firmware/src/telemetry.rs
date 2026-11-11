//! Ghost-Net telemetry framing: fixed-width binary frame, ChaCha20-Poly1305
//! authenticated encryption on the wire (same primitive as the simulator:
//! `eh_sim_core::crypto`).

use alloc::string::ToString;
use alloc::vec::Vec;

use esim_core::crypto::{decrypt_packet, encrypt_packet, PACKET_OVERHEAD};

use crate::energy::CycleReading;

/// Magic prefix so receivers can identify EHAR frames.
pub const FRAME_MAGIC: [u8; 4] = *b"EHAR";
/// Current frame version.
pub const FRAME_VERSION: u8 = 1;
/// Fixed plaintext payload length (all fields, pre-encryption).
pub const PAYLOAD_LEN: usize = 1 + 4 + 1 + 4 * 9 + 2; // version+seq+flags+9×f32+entropy bytes

/// One decoded telemetry frame.
#[derive(Debug, Clone, PartialEq)]
pub struct TelemetryFrame {
    pub seq: u32,
    pub reading: CycleReading,
    pub tier_name: alloc::string::String,
    pub entropy_bytes: u8,
}

/// Encode a reading into the fixed-width plaintext payload.
///
/// Layout (little-endian):
///   [0]   version
///   [1..5]  sequence number u32
///   [5]   tier code
///   [6..]  9 × f32: v_cap, temp_c, humidity_pct, pressure_hpa,
///          pv_voltage_v, elf_mv, piezo_mv, harvest_mw, (reserved)
///   [+2]  entropy bytes extracted this cycle
pub fn encode_payload(
    seq: u32,
    tier_code: u8,
    reading: &CycleReading,
    entropy_bytes: u8,
) -> Vec<u8> {
    let mut p = Vec::with_capacity(PAYLOAD_LEN);
    p.push(FRAME_VERSION);
    p.extend_from_slice(&seq.to_le_bytes());
    p.push(tier_code);
    for v in [
        reading.v_cap,
        reading.temp_c,
        reading.humidity_pct,
        reading.pressure_hpa,
        reading.pv_voltage_v,
        reading.elf_mv,
        reading.piezo_mv,
        reading.harvest_mw,
        0.0, // reserved
    ] {
        p.extend_from_slice(&(v as f32).to_le_bytes());
    }
    p.push(entropy_bytes);
    p.push(0); // reserved flags
    p
}

/// Decode a plaintext payload (inverse of [`encode_payload`]).
pub fn decode_payload(payload: &[u8]) -> Option<TelemetryFrame> {
    if payload.len() != PAYLOAD_LEN || payload[0] != FRAME_VERSION {
        return None;
    }
    let seq = u32::from_le_bytes(payload[1..5].try_into().ok()?);
    let tier_code = payload[5];
    let f = |i: usize| -> f64 {
        let mut b = [0u8; 4];
        b.copy_from_slice(&payload[6 + 4 * i..10 + 4 * i]);
        f32::from_le_bytes(b) as f64
    };
    let tier_name = match tier_code {
        0 => "T-1-DeepHibernate",
        1 => "T0-Critical",
        2 => "T1-High",
        3 => "T2-Medium",
        4 => "T3-Low",
        _ => "?",
    }
    .to_string();
    Some(TelemetryFrame {
        seq,
        reading: CycleReading {
            v_cap: f(0),
            temp_c: f(1),
            humidity_pct: f(2),
            pressure_hpa: f(3),
            pv_voltage_v: f(4),
            elf_mv: f(5),
            piezo_mv: f(6),
            harvest_mw: f(7),
            adc_entropy_bytes: payload[PAYLOAD_LEN - 2],
        },
        tier_name,
        entropy_bytes: payload[PAYLOAD_LEN - 2],
    })
}

/// Build a complete wire packet: magic + encrypted payload (nonce ‖ ct ‖ tag).
/// `counter` doubles as the ChaCha20-Poly1305 nonce input (sim parity).
pub fn build_packet(
    key: &[u8; 32],
    seq: u32,
    tier_code: u8,
    reading: &CycleReading,
    entropy_bytes: u8,
) -> Vec<u8> {
    let payload = encode_payload(seq, tier_code, reading, entropy_bytes);
    let ct = encrypt_packet(key, &payload, seq as u64);
    let mut packet = Vec::with_capacity(FRAME_MAGIC.len() + ct.len());
    packet.extend_from_slice(&FRAME_MAGIC);
    packet.extend_from_slice(&ct);
    packet
}

/// Parse a wire packet back into a frame (for the gateway / tests).
pub fn parse_packet(key: &[u8; 32], packet: &[u8]) -> Option<TelemetryFrame> {
    if packet.len() < FRAME_MAGIC.len() + PACKET_OVERHEAD || packet[..4] != FRAME_MAGIC {
        return None;
    }
    let ct = &packet[FRAME_MAGIC.len()..];
    // Recover the counter from the nonce prefix (crypto module encodes it).
    let payload = decrypt_packet(key, ct)?;
    decode_payload(&payload)
}

/// Expected on-wire packet length for a full frame.
pub fn packet_len() -> usize {
    FRAME_MAGIC.len() + PAYLOAD_LEN + PACKET_OVERHEAD
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::energy::CycleReading;

    fn sample() -> CycleReading {
        CycleReading {
            v_cap: 4.12,
            temp_c: 21.5,
            humidity_pct: 44.0,
            pressure_hpa: 1013.25,
            pv_voltage_v: 6.1,
            elf_mv: 0.12,
            piezo_mv: 1.8,
            adc_entropy_bytes: 8,
            harvest_mw: 25.0,
        }
    }

    #[test]
    fn payload_roundtrip() {
        let p = encode_payload(7, 4, &sample(), 8);
        assert_eq!(p.len(), PAYLOAD_LEN);
        let f = decode_payload(&p).expect("decode");
        assert_eq!(f.seq, 7);
        assert_eq!(f.tier_name, "T3-Low");
        assert!((f.reading.v_cap - 4.12).abs() < 1e-4);
        assert!((f.reading.pressure_hpa - 1013.25).abs() < 1e-2);
    }

    #[test]
    fn wire_roundtrip_with_encryption() {
        let key = [0x5Au8; 32];
        let reading = sample();
        let pkt = build_packet(&key, 42, 4, &reading, 8);
        assert_eq!(pkt.len(), packet_len());
        let f = parse_packet(&key, &pkt).expect("parse");
        assert_eq!(f.seq, 42);
        assert!((f.reading.elf_mv - 0.12).abs() < 1e-4);
    }

    #[test]
    fn tampered_packet_rejected() {
        let key = [0x5Au8; 32];
        let pkt = build_packet(&key, 1, 3, &sample(), 8);
        let mut bad = pkt.clone();
        let n = bad.len();
        bad[n - 1] ^= 0x01; // flip one tag/ciphertext byte
        assert!(parse_packet(&key, &bad).is_none());
    }

    #[test]
    fn wrong_key_rejected() {
        let pkt = build_packet(&[1u8; 32], 1, 3, &sample(), 8);
        assert!(parse_packet(&[2u8; 32], &pkt).is_none());
    }
}
