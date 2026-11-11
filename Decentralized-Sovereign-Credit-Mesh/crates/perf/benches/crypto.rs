use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mesh_smpc::secret_sharing::{recover_secret, split_secret};
use mesh_smpc::solvency::{prepare_solvency_shares, verify_solvency};
use mesh_zkp::merkle::{build_merkle_tree, prove_membership};

fn bench_shamir_split(c: &mut Criterion) {
    c.bench_function("shamir_split_5_3", |b| {
        b.iter(|| split_secret(black_box(12345), 5, 3))
    });
}

fn bench_shamir_recover(c: &mut Criterion) {
    let shares = split_secret(12345, 5, 3).unwrap();
    c.bench_function("shamir_recover_3", |b| {
        b.iter(|| recover_secret(black_box(&shares[..3]), 3))
    });
}

fn bench_solvency_prepare(c: &mut Criterion) {
    c.bench_function("solvency_prepare_10_5", |b| {
        b.iter(|| prepare_solvency_shares(black_box("test"), 10000, 3000, 10, 5))
    });
}

fn bench_solvency_verify(c: &mut Criterion) {
    let (parties, stmt) = prepare_solvency_shares("test", 10000, 3000, 10, 5).unwrap();
    let subset: Vec<_> = parties[..5].to_vec();
    c.bench_function("solvency_verify_5", |b| {
        b.iter(|| verify_solvency(black_box(&stmt.entity_id), black_box(&subset), 5))
    });
}

fn bench_merkle_build(c: &mut Criterion) {
    let values: Vec<Vec<u8>> = (0..100)
        .map(|i| format!("value_{}", i).into_bytes())
        .collect();
    c.bench_function("merkle_build_100", |b| {
        b.iter(|| build_merkle_tree(black_box(&values)))
    });
}

fn bench_merkle_prove(c: &mut Criterion) {
    let values: Vec<Vec<u8>> = (0..100)
        .map(|i| format!("value_{}", i).into_bytes())
        .collect();
    let (tree, _root) = build_merkle_tree(&values);
    let target = values[42].clone();
    c.bench_function("merkle_prove", |b| {
        b.iter(|| prove_membership(black_box(&target), black_box(&tree)))
    });
}

criterion_group!(
    crypto_benches,
    bench_shamir_split,
    bench_shamir_recover,
    bench_solvency_prepare,
    bench_solvency_verify,
    bench_merkle_build,
    bench_merkle_prove
);
criterion_main!(crypto_benches);
