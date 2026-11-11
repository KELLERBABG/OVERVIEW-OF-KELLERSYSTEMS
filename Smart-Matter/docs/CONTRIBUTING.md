# Contributing to Smart Matter

Thanks for helping! This project is small and solo-maintained, so the bar is
simple: **keep everything green and honest.**

## Getting started

```bash
rustup target add thumbv6m-none-eabi   # (or rely on rust-toolchain.toml)
cargo test --workspace
```

## Before you submit a change

1. **Format.** Run `cargo fmt --all`.
2. **Lint.** Run `cargo clippy --workspace -- -D warnings` (host) and the same
   for the embedded crates:
   ```bash
   cargo clippy -p smart-matter-node -p smart-matter-bootloader \
     --no-default-features --features embedded --target thumbv6m-none-eabi -- -D warnings
   ```
3. **Test.** Run `cargo test --workspace`.
4. **Embedded builds still link:**
   ```bash
   cargo build --release -p smart-matter-node -p smart-matter-bootloader \
     --target thumbv6m-none-eabi --no-default-features --features embedded
   ```

CI runs all of these and fails on any warning.

## Conventions

- The core crate is **`no_std`**: no allocation, no `std` imports. Keep it
  that way; the same code runs on the chip and in the simulator.
- No hardcoded keys or secrets. Keys are derived (see the security notes in
  `README.md`).
- Honesty over polish: if a driver is a bench bring-up stub, say so in a
  `TODO(bring-up)` comment and in `PROGRESS.md`. Don't fake it.
- Update `HOW_IT_WORKS.md` and `CHANGELOG.md` when behavior changes.

## Areas that need work

See "Known limitations" in [PROGRESS.md](PROGRESS.md), and read
[README.md](README.md) for the project overview. The physical peripheral
drivers (LIS3DH, nRF24L01+/CC1101, DS18B20, external SPI staging flash) and
PCB design are the most valuable next contributions.
