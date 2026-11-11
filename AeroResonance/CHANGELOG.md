# Changelog — EHAR (simulation + hardware + firmware)

The `simulation/CHANGELOG.md` covers the simulator. This file covers the
repo-wide milestones.

## [0.2.0-unreleased] — 2026-08-13 — hardware MVP (risk-R1 pivot, decision D6)

### Added
- `hardware/` — buildable MVP: architecture README, BOM (~120 €), power
  budget with the viability math, assembly guide (bench → breadboard →
  perfboard), safety rules, bring-up + acceptance checklist.
- `firmware/` — Rust firmware:
  - `ehar-firmware-core`: no_std device logic reusing `eh-sim-core`
    (scheduler, entropy, crypto, cap model). 12 host tests; builds for
    `riscv32imc-unknown-none-elf`.
  - `esp32c3/`: ESP32-C3 binary — ADC sampling (V_cap, PV, ELF, piezo),
    entropy feed, tier/duty planning, encrypted Ghost-Net frames over
    USB-serial.
- Root `README.md` (three-layer overview) and root `LICENSE` (Proprietary,
  consistent with `simulation/LICENSE`).

### Changed
- `eh-sim-core`: `chacha20poly1305`/`blake3` now use `default-features =
  false` (+`alloc` for the AEAD) so the crate compiles for bare-metal
  targets — the simulator build is unaffected.
- `simulation/Cargo.toml`: removed redundant direct `blake3` /
  `chacha20poly1305` deps (unused since WP1; they live in `eh-sim-core`).
- `PRODUCTION_READINESS_ACTION_PLAN.md`: decision **D6 recorded** — proceed
  with the hybrid solar + atmospheric MVP (owner, 2026-08-13).

### Notes
- Physics gate-1 remains **not met** for resonance-based harvesters (no
  change in the simulation verdict). The MVP's viability comes from the PV +
  supercap power train; the resonance channels are M1.3 instrumentation.
- The ESP32-C3 binary is compile-verified for the target; hardware bring-up
  (BRINGUP.md) is pending parts purchase.
