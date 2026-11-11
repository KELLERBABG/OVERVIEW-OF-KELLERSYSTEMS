//! Criterion benchmark for the simulation core.
//!
//! Run with: `cargo bench --bench run_simulation`.
//! (Baseline numbers are stored in `criterion/` after the first run.)

use atmospheric_resonance_sim::config::SimConfig;
use atmospheric_resonance_sim::simulation::run_simulation;
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_short_simulation(c: &mut Criterion) {
    let cfg = SimConfig {
        dt: 0.05,
        total_time: 100.0,
        ..SimConfig::default()
    }; // 2000 steps
    c.bench_function("run_simulation_short", |b| {
        b.iter(|| {
            let (_records, _summaries) = run_simulation(&cfg, Some(1));
        })
    });
}

criterion_group!(benches, bench_short_simulation);
criterion_main!(benches);
