//! End-to-end check of the decode tool path: build a frame the way the node
//! does, format it exactly as the node prints it (`{:02x?}`), then decode it
//! the way `examples/decode.rs` does (strip brackets/commas, hex-parse).

use ehar_firmware_core::{build_packet, parse_packet, CycleReading};

const DEV_KEY: [u8; 32] = [0x42; 32];

fn decode_like_tool(hex_line: &str) -> Vec<u8> {
    let cleaned: String = hex_line
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ',' && *c != '[' && *c != ']')
        .collect();
    let cleaned = cleaned.trim_start_matches("0x");
    (0..cleaned.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&cleaned[i..i + 2], 16).expect("valid hex"))
        .collect()
}

#[test]
fn node_print_format_decodes_back() {
    let reading = CycleReading {
        v_cap: 4.12,
        temp_c: 21.5,
        humidity_pct: 44.0,
        pressure_hpa: 1013.25,
        pv_voltage_v: 6.1,
        elf_mv: 0.12,
        piezo_mv: 1.8,
        adc_entropy_bytes: 8,
        harvest_mw: 25.0,
    };
    // What the ESP32-C3 prints after "GHOST" (same format string).
    let pkt = build_packet(&DEV_KEY, 99, 4, &reading, 8);
    let printed = format!("{:02x?}", pkt);
    assert!(printed.starts_with("[45, 48, 41, 52")); // "EHAR" magic

    let bytes = decode_like_tool(&printed);
    let f = parse_packet(&DEV_KEY, &bytes).expect("frame must decode");
    assert_eq!(f.seq, 99);
    assert_eq!(f.tier_name, "T3-Low");
    assert!((f.reading.v_cap - 4.12).abs() < 1e-4);
}

#[test]
fn decoder_rejects_tampered_line() {
    let reading = CycleReading {
        v_cap: 3.3,
        temp_c: 20.0,
        humidity_pct: 50.0,
        pressure_hpa: 1000.0,
        pv_voltage_v: 5.0,
        elf_mv: 0.0,
        piezo_mv: 0.0,
        adc_entropy_bytes: 0,
        harvest_mw: 0.0,
    };
    let pkt = build_packet(&DEV_KEY, 1, 2, &reading, 0);
    let printed = format!("{:02x?}", pkt);
    let mut bytes = decode_like_tool(&printed);
    let n = bytes.len();
    bytes[n - 1] ^= 0x01; // corrupt the tag
    let tampered = format!("{:02x?}", bytes);
    assert!(parse_packet(&DEV_KEY, &decode_like_tool(&tampered)).is_none());
}
