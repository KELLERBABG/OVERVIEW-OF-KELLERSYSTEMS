# Changelog

All notable changes to this project are documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to a pre-1.0 versioning scheme.

## [Unreleased]

### Added (2026-08-13 production-readiness pass)
- `--seed <u64>` flag → bit-deterministic simulation runs.
- `--gate-report` flag → per-subsystem Gate-1 energy-density screen.
- Integration test suite (`tests/`) and a Criterion benchmark (`benches/`).
- `src/lib.rs` exposing the simulation modules for tests/benches.
- `docs/GATE1_ENERGY_DENSITY_REPORT.md` — honest Gate-1 verdict.
- README, LICENSE (Proprietary), CHANGELOG, CI workflow, `.gitignore`.
- `teg_min_dt_degc` config field (replaces a hardcoded, UNVALIDATED `dt.max(2.5)`).

### Changed
- Repaired the build: added a proper cargo `[workspace]` and fixed the
  `simulation_core` lib `crate-type` to `["rlib"]` (resolved the fingerprint
  filename-collision that broke every `cargo` invocation).
- Crypto: replaced the hand-rolled ChaCha20-Poly1305 + fake "BLAKE3" sponge with
  the conformant `chacha20poly1305` (RFC 8439) and `blake3` crates; derived the
  Ghost-Net key from the entropy pipeline (no hardcoded `[0x42; 32]` key).
- Config: `SimConfig` is now `#[serde(default)]` (missing fields fall back to
  defaults); the CLI reports malformed config with a clean `exit(2)` instead of
  panicking. Repaired `severe_weather.json` (was missing 11 fields and panicked).
- Dead code removed: unused physics constants/functions (KB/QE/ME, plasma
  models, capacitor helpers, mppt piezo), unused `EnvState` fields, unused
  `reset` machinery, and unused config fields.
- Wired the 1D FDTD Maxwell kernel into the Maxwellian subsystem and the
  safety `safety_*` fields into `StaticInvariantDischarge`.
- Removed the unused `chrono` dependency.

### Fixed
- Two failing crypto unit tests (u128 overflow in the hand-rolled Poly1305) are
  now green and conformant.

## [0.1.0] — 2026-07-27 (initial)

- Runnable MVP simulation of 10+1 subsystems; bundled scenarios; deterministic
  CSV output; no_std core crate.
