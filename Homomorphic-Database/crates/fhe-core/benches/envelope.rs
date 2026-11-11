//! Criterion benchmarks for the FHE-DB primitives.
//!
//! FHE work is expensive, so the default criterion settings are tuned for
//! it (small sample sizes, short measurement windows) and CI runs with
//! `--quick --noplot`. The homomorphic benchmarks require the server key,
//! which is installed once per benchmark function (criterion runs benches
//! single-threaded by default).
//!
//! Regression guard: on a **fixed machine**, commit a baseline once with
//! `cargo bench --bench envelope -- --save-baseline saved` and compare
//! later with `cargo bench --bench envelope -- --baseline saved
//! --threshold 25%` (see `scripts/check-bench-regression.sh`). Timings are
//! not comparable across different hardware.

use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use fhe_core::{Ciphertext, CiphertextAlgebra, ClientKey, Encryptor, ScalarWidth, ServerKeyGuard};

/// Tunings that keep a full bench run under a couple of minutes even in
/// debug-adjacent conditions. FHE ops are millisecond-scale, so 10 samples
/// with a 3 s measurement window is statistically meaningful.
fn fhe_group<'a>(
    c: &'a mut Criterion,
    name: &str,
) -> criterion::BenchmarkGroup<'a, criterion::measurement::WallTime> {
    let mut group = c.benchmark_group(name);
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(3));
    group
}

/// Serialize + deserialize one ciphertext envelope per width.
fn envelope_roundtrip(c: &mut Criterion) {
    let (client_key, _server_key) = ClientKey::new();
    let encryptor = Encryptor::new(&client_key);
    let mut group = fhe_group(c, "envelope_roundtrip");
    for width in [
        ScalarWidth::U8,
        ScalarWidth::U16,
        ScalarWidth::U32,
        ScalarWidth::U64,
    ] {
        let ct = encryptor.encrypt_u64(42, width).expect("encrypt");
        let bytes = ct.to_envelope_bytes().expect("serialize");
        group.bench_function(format!("{width:?}"), |b| {
            b.iter(|| Ciphertext::from_envelope_bytes(&bytes).expect("deserialize"));
        });
    }
    group.finish();
}

/// Client-side encryption throughput (U32 — the schema default width).
fn encryption_throughput(c: &mut Criterion) {
    let (client_key, _server_key) = ClientKey::new();
    let encryptor = Encryptor::new(&client_key);
    let mut group = fhe_group(c, "encryption_throughput");
    group.bench_function("u32", |b| {
        b.iter(|| {
            encryptor
                .encrypt_u64(42, ScalarWidth::U32)
                .expect("encrypt")
        });
    });
    group.finish();
}

/// Homomorphic SUM over 32 U8 ciphertexts (server-key installed).
fn homomorphic_sum(c: &mut Criterion) {
    let (client_key, server_key) = ClientKey::new();
    let _guard = ServerKeyGuard::new(&server_key);
    let encryptor = Encryptor::new(&client_key);
    let algebra = CiphertextAlgebra;
    let values: Vec<Ciphertext> = (0u64..32)
        .map(|i| encryptor.encrypt_u64(i, ScalarWidth::U8).expect("encrypt"))
        .collect();
    let mut group = fhe_group(c, "homomorphic_sum");
    for n in [8usize, 32] {
        let slice = &values[..n];
        group.bench_function(BenchmarkId::new("u8", n), |b| {
            b.iter(|| algebra.sum(slice).expect("homomorphic sum"));
        });
    }
    group.finish();
}

/// Rejection-path throughput: malformed envelopes must be rejected without
/// any FHE work (this is the per-request cost of the DoS guard).
fn reject_garbage(c: &mut Criterion) {
    let garbage: &[u8] = b"this is definitely not a ciphertext envelope";
    let mut group = fhe_group(c, "envelope_reject_garbage");
    group.bench_function("malformed", |b| {
        b.iter(|| {
            assert!(Ciphertext::from_envelope_bytes(garbage).is_err());
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    envelope_roundtrip,
    encryption_throughput,
    homomorphic_sum,
    reject_garbage
);
criterion_main!(benches);
