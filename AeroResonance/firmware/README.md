# EHAR Firmware

Rust (`no_std`) firmware for the EHAR Field Node MVP — see
[`../hardware/`](../hardware/README.md) for the build.

## Crates

| Crate | Target | Purpose |
|-------|--------|---------|
| `ehar-firmware-core` (this dir) | host (`cargo test`) **and** bare metal | all device logic: energy model, tier mapping, duty planning, entropy, Ghost-Net telemetry framing. Reuses `eh-sim-core` (same scheduler/entropy/crypto/cap model as the simulator) |
| `esp32c3/` | `riscv32imc-unknown-none-elf` (ESP32-C3) | thin HAL glue: ADC sampling, loop, USB-serial telemetry |

## Test the core on your machine (no hardware needed)

```bash
cd firmware
cargo test          # 12 tests: energy model, tiers, duty plan, telemetry round-trip
cargo clippy --all-targets -- -D warnings
cargo build --target riscv32imc-unknown-none-elf   # prove the core builds for the MCU
```

## Build & flash the ESP32-C3 binary

```bash
cd firmware/esp32c3
rustup target add riscv32imc-unknown-none-elf
cargo build --release --target riscv32imc-unknown-none-elf
cargo install espflash            # one-time
cargo run --release --target riscv32imc-unknown-none-elf   # build + flash + monitor
```

Then open the serial monitor at 115200 baud — the C3's native USB-serial
needs no extra driver on Windows 10+/Linux/macOS.

## Telemetry format (v1)

Per cycle the node prints:

```
[seq=42] tier=T2-Medium vcap=4100mV pv=6100mV elf=312 piezo=9 E=210J interval=120s autonomy=5400s
  GHOST [0x45, 0x48, 0x41, 0x52, ...]   <- "EHAR" magic + ChaCha20-Poly1305 frame (hex)
```

- The header is human-readable debug; the **GHOST line is the real payload**:
  `EHAR` magic ‖ nonce(12) ‖ ciphertext ‖ tag(16) — see
  `ehar-firmware-core/src/telemetry.rs` for the layout.
- The frame decrypts with the dev key (`GHOST_KEY` in `esp32c3/src/main.rs`)
  via `ehar_firmware_core::parse_packet` — that function is unit-tested
  (round-trip, tamper rejection, wrong-key rejection).

## Firmware ↔ simulation mapping (the digital twin)

| Device | Simulator counterpart |
|--------|-----------------------|
| V_cap (supercap voltage) | `subsystems::PlasmaWakeEnergyBuffering` / PMIC |
| Tier scheduler | `eh_sim_core::scheduler` (identical code) |
| ADC-noise TRNG | `eh_sim_core::entropy::EntropyPipeline` |
| Ghost-Net frame | `eh_sim_core::crypto::encrypt_packet` |
| Duty plan / autonomy | `simulation::simulation::hybrid_sizing` energy math |
| Supercap ageing | `eh_sim_core::cap_aging::CapState` (v2) |

## v2 roadmap

- BME280 + DS18B20 over I2C (T/H/P + ground ΔT)
- ESP-NOW radio via the `esp-radio` crate (no frame-format change)
- Light-sleep / deep-sleep duty cycle (20 mA → ~1 mA → ~5 µA)
- NVS-provisioned Ghost-Net key (replaces the dev key)
