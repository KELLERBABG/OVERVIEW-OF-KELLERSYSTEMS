//! Performance regression guards (hardware-robust ceilings).
//!
//! These are deliberately **generous absolute ceilings** — an order of
//! magnitude above observed timings — so they never flake on slow CI
//! machines, but they do catch algorithmic regressions (e.g. an accidental
//! O(n²) deserialization, an extra allocation per envelope, a length-prefix
//! bomb that is no longer rejected up front). For precise tracking on a
//! fixed machine, see `scripts/check-bench-regression.sh` and the criterion
//! suite in `benches/envelope.rs`.

use std::time::{Duration, Instant};

use fhe_core::{Ciphertext, ClientKey, Encryptor, ScalarWidth};

/// Rejecting a malformed envelope is the per-request cost of the DoS guard,
/// so it must stay sub-millisecond. 10,000 rejections of a tiny input must
/// complete in well under 100 ms (observed: < 5 ms); the 100 ms ceiling
/// leaves a 20x headroom for slow machines while catching anything
/// pathological.
#[test]
fn envelope_rejection_throughput_guard() {
    let garbage: &[u8] = b"this is definitely not a ciphertext envelope";
    let start = Instant::now();
    for _ in 0..10_000 {
        assert!(Ciphertext::from_envelope_bytes(garbage).is_err());
    }
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_millis(100),
        "10k envelope rejections took {elapsed:?} — the DoS guard regressed"
    );
}

/// Decoding real envelopes must stay linear in the input size. Decoding the
/// same 100 KiB envelope 200 times must complete in well under 5 s
/// (observed: < 200 ms in release); the ceiling catches accidental
/// super-linear decode paths without flaking on slow CI.
#[test]
fn envelope_decode_throughput_guard() {
    let (client_key, _server_key) = ClientKey::new();
    let encryptor = Encryptor::new(&client_key);
    let ct = encryptor.encrypt_u64(42, ScalarWidth::U32).unwrap();
    let bytes = ct.to_envelope_bytes().unwrap();
    assert!(bytes.len() > 100_000, "expected a large U32 envelope");

    let start = Instant::now();
    for _ in 0..200 {
        let decoded = Ciphertext::from_envelope_bytes(&bytes).unwrap();
        assert_eq!(decoded.decrypt(&client_key).as_u64(), 42);
    }
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(5),
        "200 decodes of a {}-byte envelope took {elapsed:?} — decode path regressed",
        bytes.len()
    );
}
