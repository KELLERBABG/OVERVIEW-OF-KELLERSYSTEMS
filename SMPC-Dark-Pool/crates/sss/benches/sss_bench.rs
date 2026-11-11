use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rand::thread_rng;
use sss::field::random_scalar;
use sss::sss::{reconstruct, split_secret};

fn bench_split_secret(c: &mut Criterion) {
    let mut rng = thread_rng();
    let secret = random_scalar(&mut rng);
    c.bench_function("sss_split_5_3", |b| {
        b.iter(|| split_secret(black_box(secret), 5, 3))
    });
}

fn bench_reconstruct(c: &mut Criterion) {
    let mut rng = thread_rng();
    let secret = random_scalar(&mut rng);
    let shares = split_secret(secret, 5, 3).unwrap();
    c.bench_function("sss_reconstruct_3_of_5", |b| {
        b.iter(|| reconstruct(black_box(&shares[..3])))
    });
}

criterion_group!(benches, bench_split_secret, bench_reconstruct);
criterion_main!(benches);
