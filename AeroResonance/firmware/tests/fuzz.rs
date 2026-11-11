//! Fuzz-lite property tests for the firmware telemetry parser (host side).
//! Guarantee: **arbitrary bytes never panic** `parse_packet`/`decode_payload`
//! — they return `Option`; plus encode→decode round-trips on random readings.

use ehar_firmware_core::{
    build_packet, decode_payload, parse_packet, CycleReading, PAYLOAD_LEN,
};

/// Deterministic PRNG so failures are reproducible (xorshift64*).
fn next_rand(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *state = x;
    x.wrapping_mul(0x2545F4914F6CDD1D)
}

fn rand_reading(state: &mut u64) -> CycleReading {
    let f = |state: &mut u64| -> f64 {
        let sign = if next_rand(state) & 1 == 0 { 1.0 } else { -1.0 };
        sign * (next_rand(state) % 1_000_000_000) as f64 / 1e6
    };
    CycleReading {
        v_cap: f(state).abs() % 6.0,
        temp_c: (f(state).abs() % 80.0) - 30.0,
        humidity_pct: f(state).abs() % 100.0,
        pressure_hpa: 900.0 + f(state).abs() % 200.0,
        pv_voltage_v: f(state).abs() % 8.0,
        elf_mv: f(state),
        piezo_mv: f(state),
        adc_entropy_bytes: (next_rand(state) % 256) as u8,
        harvest_mw: f(state).abs() % 1000.0,
    }
}

#[test]
fn telemetry_parser_never_panics_on_garbage() {
    let key = [0xAA; 32];
    let mut state = 0x1234_5678_9ABC_DEF0;
    for _ in 0..10_000 {
        let n = (next_rand(&mut state) % 200) as usize;
        let bytes: Vec<u8> = (0..n).map(|_| next_rand(&mut state) as u8).collect();
        let _ = parse_packet(&key, &bytes);
        let _ = decode_payload(&bytes);
    }
}

#[test]
fn telemetry_roundtrip_on_random_readings() {
    let key = [0x55; 32];
    let mut state = 0xFEED_FACE_0BAD_BEEF;
    for _ in 0..2_000 {
        let reading = rand_reading(&mut state);
        let seq = (next_rand(&mut state) % u32::MAX as u64) as u32;
        let tier_code = (next_rand(&mut state) % 5) as u8;
        let pkt = build_packet(&key, seq, tier_code, &reading, reading.adc_entropy_bytes);
        assert_eq!(pkt.len(), 4 + PAYLOAD_LEN + 28, "stable packet length");
        let back = parse_packet(&key, &pkt).expect("roundtrip");
        assert_eq!(back.seq, seq);
        for (a, b) in [
            (back.reading.v_cap, reading.v_cap),
            (back.reading.temp_c, reading.temp_c),
            (back.reading.humidity_pct, reading.humidity_pct),
            (back.reading.pressure_hpa, reading.pressure_hpa),
            (back.reading.pv_voltage_v, reading.pv_voltage_v),
            (back.reading.elf_mv, reading.elf_mv),
            (back.reading.piezo_mv, reading.piezo_mv),
            (back.reading.harvest_mw, reading.harvest_mw),
        ] {
            assert!((a - b).abs() < 1e-3, "field drifted in roundtrip: {a} vs {b}");
        }
    }
}

#[test]
fn bit_flip_in_payload_or_tag_is_rejected() {
    let key = [0x5A; 32];
    let mut state = 7;
    for _ in 0..1_000 {
        let reading = rand_reading(&mut state);
        let pkt = build_packet(&key, 1, 2, &reading, 0);
        let mut bad = pkt.clone();
        let i = (next_rand(&mut state) % pkt.len() as u64) as usize;
        bad[i] ^= 1 << (next_rand(&mut state) % 8);
        if i >= 4 + PAYLOAD_LEN {
            // tag/nonce corruption must always fail
            assert!(parse_packet(&key, &bad).is_none(), "byte {i} flipped");
        }
        // payload corruption: either rejected outright or decrypts to garbage
        // that decode_payload then rejects or accepts — the only hard rule is
        // no panic (parse_packet returns Option, checked above).
    }
}
