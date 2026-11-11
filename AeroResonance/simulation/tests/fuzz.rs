//! Fuzz-lite property tests (no external fuzz toolchain required — they run
//! in `cargo test` and CI).
//!
//! The real cargo-fuzz target needs libFuzzer/clang; until that toolchain is
//! available, these deterministic pseudo-fuzz loops give the same guarantee
//! at the parser boundary: **arbitrary input must never panic**, only
//! `Result::Err`. See PRODUCTION_READINESS_ACTION_PLAN.md (optional fuzz item).

use atmospheric_resonance_sim::config::SimConfig;

/// Deterministic PRNG so failures are reproducible (xorshift64*).
fn next_rand(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *state = x;
    x.wrapping_mul(0x2545F4914F6CDD1D)
}

fn rand_bytes(state: &mut u64, n: usize) -> Vec<u8> {
    (0..n).map(|_| next_rand(state) as u8).collect()
}

/// A valid-JSON blob with random values in known config fields, plus random
/// junk fields — exercises type coercion and serde defaulting. Integer-typed
/// fields get integers, float-typed fields get numbers, so a large share of
/// the blobs actually parse (that is the point: assert parseable configs are
/// usable, while malformed ones are merely rejected).
fn rand_config_json(state: &mut u64) -> String {
    let float_fields = [
        "dt",
        "total_time",
        "output_interval",
        "tower_height_m",
        "v_cap_tier0_min",
        "v_cap_tier1_min",
        "teg_min_dt_degc",
        "pv_overcast_probability",
    ];
    let int_fields = ["pv_cell_count", "nvis_antenna_count", "piezo_tile_count"];
    let mut s = String::from("{");
    let mut first = true;
    for f in float_fields {
        if !first {
            s.push(',');
        }
        first = false;
        let kind = next_rand(state) % 4; // 0..3 → numbers (strings are invalid here)
        let val = match kind {
            0 => format!("{}", (next_rand(state) % 1_000_000_000) as f64 / 1000.0),
            1 => format!("{}", (next_rand(state) as i64 as f64) / 1e6),
            2 => (next_rand(state) % 1_000_000).to_string(),
            _ => format!("-{}", (next_rand(state) % 1_000_000) as f64 / 100.0),
        };
        s.push_str(&format!("\"{f}\":{val}"));
    }
    for f in int_fields {
        s.push(',');
        // Sometimes an int-typed field gets a non-integer — must be rejected,
        // never panic.
        let val = if next_rand(state) % 5 == 0 {
            format!("{}", (next_rand(state) % 1000) as f64 / 3.0)
        } else {
            (next_rand(state) % 100_000).to_string()
        };
        s.push_str(&format!("\"{f}\":{val}"));
    }
    // Random junk fields the struct does not know (serde ignores unknown).
    s.push_str(&format!(",\"junk{}x\":{}", next_rand(state) % 50, next_rand(state)));
    s.push('}');
    s
}

#[test]
fn config_parser_never_panics_on_garbage_bytes() {
    let mut state = 0x5EED_CAFE_1234_5678;
    for _ in 0..10_000 {
        let n = (next_rand(&mut state) % 512) as usize;
        let bytes = rand_bytes(&mut state, n);
        let text = String::from_utf8_lossy(&bytes);
        // Must be a Result, never a panic.
        let _ = serde_json::from_str::<SimConfig>(&text);
    }
}

#[test]
fn config_parser_never_panics_on_random_valid_json() {
    let mut state = 0xABCD_EF01_2345_6789;
    let mut loads = 0;
    for _ in 0..10_000 {
        let json = rand_config_json(&mut state);
        match serde_json::from_str::<SimConfig>(&json) {
            Ok(cfg) => {
                // A config that parses must be *usable*: sane numeric fields.
                loads += 1;
                assert!(
                    cfg.dt.is_finite() && cfg.total_time.is_finite(),
                    "non-finite timings leaked through: {json}"
                );
            }
            Err(_) => {} // rejected — fine
        }
    }
    // Sanity: the generator must produce a good number of *parseable* blobs,
    // otherwise this test isn't exercising the happy path at all.
    assert!(loads > 100, "only {loads}/10000 random configs parsed");
}
