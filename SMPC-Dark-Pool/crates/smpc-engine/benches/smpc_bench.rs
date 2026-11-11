use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rand::thread_rng;
use smpc_engine::add::secure_add;
use smpc_engine::types::SecretShare;
use sss::field::random_scalar;
use sss::sss::split_secret;

fn bench_secure_add(c: &mut Criterion) {
    let mut rng = thread_rng();
    let a = random_scalar(&mut rng);
    let b = random_scalar(&mut rng);
    let a_shares = split_secret(a, 5, 3).unwrap();
    let b_shares = split_secret(b, 5, 3).unwrap();
    let sa = SecretShare::new(a_shares[0].clone());
    let sb = SecretShare::new(b_shares[0].clone());
    c.bench_function("smpc_secure_add", |b| {
        b.iter(|| secure_add(black_box(&sa), black_box(&sb)))
    });
}

criterion_group!(benches, bench_secure_add);
criterion_main!(benches);
