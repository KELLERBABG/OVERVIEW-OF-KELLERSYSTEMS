# Energy Harvesting via Atmospheric Resonance — Simulation MVP

> **Status: concept-stage simulation MVP.** The physics is **NOT validated** and the
> project is **pre-Gate-1** (the Phase-1 energy-density gate > 1 mW/m² is not met by the
> resonance-based harvesters; see the [Gate-1 report](docs/GATE1_ENERGY_DENSITY_REPORT.md)).
> Do not infer deployable energy production from these numbers.

A Rust simulation of a concept to passively extract usable electrical energy from
natural atmospheric electromagnetic and thermodynamic phenomena via resonance-based
coupling. It models 10 documented subsystems plus a solar-PV microcell, with PMIC /
capacitor / task-tier scheduling, entropy extraction and (simulated) Ghost-Net
telemetry framing.

## Layout

```
simulation/
  Cargo.toml          workspace root (bin + sim core)
  src/lib.rs          library entry (config, physics, simulation, subsystems)
  src/main.rs         CLI wrapper
  src/config.rs       scenario configuration (serde, with defaults)
  src/physics.rs      physical models (Schumann, TEG, piezo, FDTD, PMIC)
  src/simulation.rs   time-stepping engine + Ghost-Net/entropy integration
  src/subsystems.rs   per-harvester models and energy-tier scheduling
  simulation_core/    no_std core (crypto: chacha20poly1305; entropy: blake3;
                      scheduler; cap_aging; physics)
  tests/              integration tests
  benches/            Criterion benchmark
  docs/               gate-1 energy-density report
  50x50mm_config.json / severe_weather.json / severe_weather_mitigated.json
```

## Build & test

```bash
cd simulation
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo bench --bench run_simulation
```

## Run

```bash
cargo run --release                       # default 1-hour simulation
cargo run -- --config severe_weather.json # use a bundled scenario
cargo run -- --seed 42                    # bit-deterministic run
cargo run -- --gate-report                # per-subsystem Gate-1 energy-density check
```

Outputs `simulation_timeseries.csv` and `<name>_summary.csv`.

## Gate-1 status (important)

Run `cargo run -- --gate-report` (short-run example below) — the resonance-based
harvesters produce sub-milliwatt densities and **fail** the plan's 1 mW/m² gate;
only a conventional solar cell passes. See the
[Gate-1 report](docs/GATE1_ENERGY_DENSITY_REPORT.md) and project-plan §7 / risk R1.

## License

Proprietary. See `LICENSE`. This is a private research/simulation artifact; no
deployment or field use is intended at this stage.
