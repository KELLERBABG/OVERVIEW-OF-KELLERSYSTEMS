# Smart Matter Firmware Guide

## Building

### Prerequisites
- Rust toolchain with `thumbv6m-none-eabi` target:
  ```bash
  rustup target add thumbv6m-none-eabi
  ```
- `probe-rs` or `openocd` for flashing via SWD

### Build Commands

```bash
# Build all workspace crates (host: tools, libraries, node simulation)
cargo build --workspace

# Run all tests (unit + integration)
cargo test --workspace

# Host simulation: run the real node firmware loop on the desktop
cargo run -p smart-matter-node -- --ticks 200

# Build the node firmware for the embedded target (STM32G031J6)
cargo build --release -p smart-matter-node \
    --target thumbv6m-none-eabi \
    --no-default-features --features embedded

# Build the bootloader
cargo build --release -p smart-matter-bootloader \
    --target thumbv6m-none-eabi \
    --no-default-features --features embedded

# Bootloader host simulation (OTA scenarios on the desktop)
cargo run -p smart-matter-bootloader
```

> **Note:** the node and bootloader crates are feature-split. The `host`
> feature (default) makes them run on the desktop for testing; the
> `embedded` feature (with `--no-default-features`) is the real firmware.
> Always use `--release` for embedded builds. Debug builds of a full
> protocol stack exceed the 32 KB app region.

## Firmware Architecture

The node firmware (`firmware/node`) follows an energy-aware main loop:

```
┌─────────────────────────────────────────────────────────┐
│ 1. WAKE (vibration interrupt or RTC)                    │
├─────────────────────────────────────────────────────────┤
│ 2. MEASURE (voltage, sensors, entropy)                   │
├─────────────────────────────────────────────────────────┤
│ 3. PROCESS (radio messages, routing, storage)            │
├─────────────────────────────────────────────────────────┤
│ 4. TRANSMIT (beacons, fragments, reports if budget OK)   │
├─────────────────────────────────────────────────────────┤
│ 5. SLEEP (WFI / deep sleep until next event)             │
└─────────────────────────────────────────────────────────┘
```

### Energy States

| State | Condition | Actions |
|-------|-----------|---------|
| Surplus | V_storage > 3.3 V | Full mesh routing, storage, broadcast |
| Nominal | 2.7–3.3 V | Normal ops, reduced beacon frequency |
| Low | 2.2–2.7 V | No mesh relay, only critical messages |
| Critical | < 2.2 V | Wake-on-vibration only, no radio |

## Flashing

### Via SWD (Development)

```bash
# Using probe-rs (release build!)
probe-rs run --chip STM32G031J6 target/thumbv6m-none-eabi/release/smart-matter-node

# Or use the flasher tool (validates the binary and prints the exact command)
cargo run -p smart-matter-flasher -- swd --firmware target/thumbv6m-none-eabi/release/smart-matter-node
```

### Over-the-Air (Production)

1. Build the firmware (`--release`, see above).
2. Prepare the update with the flasher. It validates the size (≤ 32 KB),
   computes the SHA-256, splits the image into FwUpdate slices, and writes a
   `.ota` staging file (bootloader metadata header + image):
   ```bash
   cargo run -p smart-matter-flasher -- ota --node-id 0x01 --firmware app.bin
   ```
3. The image is transmitted through the mesh as FwUpdate slices
   (energy-budgeted, 50 µJ per slice) and **staged in the external SPI
   flash** (AT25SF041, BOM U7). The 64 KB internal flash has no room for a
   second bank. The staging write also records the metadata record at
   `0x0800_F800` (`magic | image_len | sha256 | flags`).
4. On the node's next reset the bootloader reads the metadata, **SHA-256
   validates the staged image**, and only then erases the application region
   and copies the image in. Any validation failure (corrupt, truncated,
   oversized, tampered) clears the pending flag and boots the *old*
   firmware. Fail-closed: never a bricked node.
5. The bootloader then jumps to the new application vector table.

## Radio Configuration

The firmware supports two frequency bands via the `radio` trait in `smart_matter_core::radio`:

| Band | Radio | Range (free air) | Concrete penetration | Data rate |
|------|-------|-------------------|---------------------|-----------|
| 2.4 GHz | nRF24L01+ | ~100 m | ~0.5 m | Up to 2 Mbps |
| Sub-1 GHz (868/915 MHz) | CC1101 | ~500 m | ~5 m | Up to 250 kbps |

For nodes embedded in concrete or asphalt, **Sub-1 GHz is strongly recommended**.
2.4 GHz range can collapse to under 1 meter through structural materials due to
moisture content and high dielectric constant.

The radio is abstracted behind the `Radio` trait
(`smart_matter_core::radio` for the frequency-agnostic hardware trait,
`smart_matter_core::mesh::Radio` for the mesh transport). The same mesh
protocol runs over either band; a driver plugs into the trait in
`firmware/node/src/platform.rs` (bench bring-up point). The link layer
(`smart_matter_core::wire`) fragments frames to fit each radio's payload
limit (32 B for nRF24L01+, 64 B for CC1101), so the protocol is independent
of the radio's MTU.

```bash
# The firmware builds once for either band; swap the driver in platform.rs
cargo build --release -p smart-matter-node \
    --target thumbv6m-none-eabi --no-default-features --features embedded
```

## Flash Wear-Leveling

The STM32G031J6 has 64 KB of Flash with ~10,000 write cycles per sector.
The firmware uses two storage strategies:

| Store | Location | Volatile? | Purpose |
|-------|----------|-----------|---------|
| Dead-drop store | SRAM (32 fragments, ~6 KB) | Yes (lost on power loss) | Active fragment routing, ephemeral messages |
| Wear-level store | Flash log (12 KB, 3 sectors) | No (survives power loss) | Routing table checkpoints, config, integrity snapshots |

The wear-leveling store uses a log-structured design:
- Writes go to SRAM buffer first (up to 16 entries)
- Flash flush is rate-limited to ≤2 writes/second
- Sector rotation ensures even wear: 4 KB × 3 sectors × 10k cycles = 120 MB lifetime
- At 10 writes/hour: >10 years lifetime
- At 1 write/hour: >50 years lifetime

## Power Optimization Tips

- Reduce beacon frequency to 30 s in Low energy mode
- Use radio in ShockBurst™ mode for lowest TX energy
- Sample sensors at 10 Hz instead of 100 Hz in Low mode
- Store frequently-used routing entries in MCU retention RAM
- For concrete-embedded nodes: use Sub-1 GHz radio (CC1101)
