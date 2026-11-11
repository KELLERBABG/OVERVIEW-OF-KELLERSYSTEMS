# Smart Matter

**Infrastructure-embedded intelligent materials.** Self-powered compute
nodes embedded directly in concrete and asphalt that harvest kinetic energy,
form ad-hoc wireless mesh networks, store encrypted dead-drop messages,
self-monitor structural integrity, and enforce geographic data locality.

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](../LICENSE)
![Rust](https://img.shields.io/badge/rust-stable-blue)
[![no_std](https://img.shields.io/badge/no__std-%E2%9C%93-brightgreen)](../firmware/core)

---

## What it is

Each node is a small (~$14) energy-autonomous computer:

- **Piezoelectric harvesting.** A PZT-5A disc converts ambient vibration
  (footsteps, traffic) into power stored in a 1 F supercapacitor.
- **STM32G031J6 MCU** (Cortex-M0+, 64 KB flash / 8 KB RAM) running bare-metal
  Rust (`no_std`).
- **Wireless mesh** over nRF24L01+ (2.4 GHz) or CC1101 (Sub-1 GHz for
  concrete), with gradient-based routing that needs no infrastructure.
- **Dead-drop storage.** Encrypted, fragmented messages left on nodes with a
  TTL, retrievable by anyone who knows the message ID.
- **Structural integrity monitoring.** The accelerometer watches the
  vibration signature; cracks and delamination change it and trigger
  geo-fenced alerts.
- **Sovereign locality.** Broadcasts are RSSI-geo-fenced: data can only
  spread within the physical radius where the signal is strong enough.

The same Rust code runs on a desktop simulator and on the chip. The core
crate is fully `no_std` and the firmware builds for `thumbv6m-none-eabi`.

## Quick start

```bash
# Build the whole workspace and run all 92 tests
cargo build
cargo test

# Watch one node live (the real firmware loop, simulated sensors)
cargo run -p smart-matter-node -- --ticks 200

# Simulate a 10-node mesh and measure delivery / latency / energy
cargo run --bin smart-matter-network-simulator -- --nodes 10 --duration 40

# Desktop tool: monitor, send/retrieve dead-drops, integrity reports
cargo run --bin smart-matter-desktop -- monitor

# Bootloader OTA scenarios (host simulation)
cargo run -p smart-matter-bootloader
```

## Read the docs

| Doc | What's in it |
|-----|--------------|
| [HOW_IT_WORKS.md](HOW_IT_WORKS.md) | **Start here.** A complete, human-language walkthrough of every subsystem, with end-to-end stories. |
| [whitepaper.md](whitepaper.md) | The system design, energy model, protocol, security. |
| [protocol/protocol-spec.md](protocol/protocol-spec.md) | The wire format: slices, CRC framing, link-layer, routing, security. |
| [firmware/firmware-guide.md](firmware/firmware-guide.md) | Building and flashing the firmware; OTA flow. |
| [hardware/build-guide.md](hardware/build-guide.md) | Soldering, first power-up, enclosure. |
| [PROGRESS.md](PROGRESS.md) | Status, known limitations, verification log. |
| [DEVELOPMENT_PLAN.md](DEVELOPMENT_PLAN.md) | The multi-phase roadmap. |
| [ideas/](ideas/) | Original ideation canvas. |

## Project layout

```
├── firmware/
│   ├── core/          # no_std protocol logic (mesh, storage, crypto, energy, ...)
│   ├── node/          # node firmware (host simulation + STM32G031J6 target)
│   └── bootloader/    # fail-closed OTA bootloader
├── mesh/              # mesh networking crate + integration tests
├── storage/           # dead-drop storage crate
├── broadcaster/       # RSSI geo-fenced broadcast crate
├── desktop-tool/      # operator CLI (monitor / send / retrieve / configure)
├── tools/
│   ├── network-simulator/  # protocol-driven mesh simulation
│   └── flasher/            # SWD / OTA update preparation
└── docs/              # everything in this folder (start with README.md)
```

## Security notes

- Authenticated encryption: **ChaCha20-Poly1305 (RFC 8439)**, RustCrypto.
- Keys are **derived, never hardcoded**:
  `node_key = HMAC-SHA256(deployment_secret, "smart-matter-v1" + node_id)`.
  Set `SMART_MATTER_DEPLOYMENT_SECRET` when provisioning.
- OTA updates are SHA-256-validated by the bootloader before being applied.
  A corrupt or tampered image is never booted (fail-closed).

## Status

The software is feature-complete and fully tested (92 tests, clippy-clean,
both firmware images build for the embedded target). The remaining work is
physical: peripheral drivers (accelerometer, radio, staging flash) are
documented bench bring-up points, and no PCB has been fabricated yet. See
[PROGRESS.md](PROGRESS.md) for the honest details.

## License

MIT. See [LICENSE](../LICENSE).
