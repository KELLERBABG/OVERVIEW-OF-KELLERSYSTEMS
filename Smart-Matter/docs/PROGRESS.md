# Smart Matter Progress & Work Log

**Last updated:** 2026-08-13

This file records what has been done and what remains, so any session can
resume without re-discovery. The guiding checklist is
`PRODUCTION_READINESS_REPORT.md` §3.16 ("Smart Matter: What's TO BE DONE")
and the goal of the current task is: *"read the docs, make this a finished
product."*

---

## 1. Status

| # | Task | Status |
|---|------|--------|
| 1 | Core fixes (crypto, wire codec, mesh, capacities) | ✅ done, 62 tests |
| 2 | Buildable node firmware (host + embedded) | ✅ done, ~20 KB / 32 KB |
| 3 | Real bootloader + OTA logic | ✅ done, 6 tests |
| 4 | Real tools (simulator / desktop / flasher) | ✅ done |
| 5 | CI + LICENSE + CHANGELOG + .gitignore + git init | ✅ done |
| 6 | Docs (HOW_IT_WORKS, spec, whitepaper, guides) | ✅ done |
| 7 | Full verification | ✅ done (see §4) |

**Workspace:** `cargo build --workspace` ✅ · `cargo test --workspace` ✅
**92 tests** · `cargo clippy --workspace -- -D warnings` ✅ 0 warnings ·
`cargo fmt --check` ✅ · embedded node + bootloader release builds ✅ ·
git initialized with a full commit.

**Remaining known gaps (honest, documented):** the physical peripheral
drivers (LIS3DH I2C, nRF24L01+/CC1101 SPI, DS18B20 1-Wire, external SPI
staging flash) are bench bring-up points marked `TODO(bring-up)` in
`firmware/node/src/platform.rs` and `firmware/bootloader/src/platform.rs`.
The protocol stack is fully exercised by the host simulation and tests.
No PCB has been fabricated. No field validation.

---

## 2. What was done

### smart-matter-core (all green: std + no_std, 62 tests, clippy 0)
- **Crypto** (`src/crypto.rs`): RFC 8439 ChaCha20-Poly1305 (audited
  `chacha20poly1305` crate) with optional AAD; `derive_key(deployment_secret,
  node_id)` = HMAC-SHA256. No hardcoded keys anywhere.
- **Wire format** (new `src/wire.rs`): slice codec with CRC-16/CCITT framing,
  EnergyLevel wire encoding, and link-layer fragmentation for 32 B
  (nRF24L01+) / 64 B (CC1101) radio MTUs.
- **Mesh** (`src/mesh.rs`): RouteAdv entries postcard-serialized over the
  wire (≤5 entries/slice, rotated); multi-hop forwarding `dst` bug fixed;
  `node_id()`/`next_seqno()` accessors.
- **memmap** (new `src/memmap.rs`): single source of truth for the 64 KB
  flash layout, used by node, bootloader, and flasher.
- **Capacities**: 1 F supercap, 32 SRAM fragments (~6 KB), 3 flash sectors
  (12 KB), all consistent with the whitepaper now.
- **no_std realities**: `libm` for f32 math; removed bare `extern crate
  alloc` (fixes "no global memory allocator" on thumbv6m); embedded clock as
  `AtomicU32`; thermal scheduler uses a linear priority scan (slice sorting
  moved to `alloc` on rustc 1.97).

### firmware/node (host + embedded)
- `node.rs` implements the `Node` struct with the full energy-aware tick loop
  (sample → process → transmit → recharge); uses storage, broadcaster,
  crypto/key for real.
- `host.rs` is the desktop simulation with mock sensors (traffic vibration +
  anomaly bursts), loopback radio, `--ticks` arg, `SMART_MATTER_DEPLOYMENT_SECRET`.
- `platform.rs` is the STM32G031J6 HAL: HSI clock, PA5 status LED, real ADC
  supercap voltage sense (PA0, 100k/100k divider); sensor/radio drivers are
  documented bring-up points.
- `memory.x` (app at 0x08004000, 32 KB) + `build.rs` (`-Tlink.x` only;
  link.x INCLUDEs memory.x).
- Release profile: `opt-level="z"` + thin LTO → ~19.8 KB of the 32 KB app
  region.
- 3 unit tests (full loop, dead-drop store+retrieve via DataFragment/DataAck).

### firmware/bootloader
- `boot.rs` defines the `Flash` + `StagedImage` traits, the OTA metadata
  record at 0x0800_F800 (magic | len | sha256 | flags), fail-closed SHA-256
  validation, erase-and-copy, pending-flag clearing; host-testable (6 tests)
  + demo.
- `platform.rs` provides HAL-backed `InternalFlash` (2 KB pages), the SPI
  staging bring-up point, and the vector-table jump via `msr msp` + branch.
- `memory.x` (bootloader at 0x08000000, 16 KB) + `build.rs`; ~1 KB image.

### Tools
- **network-simulator** (`sim.rs`): line-topology mesh running the real
  protocol; computed PDR / avg hops / energy / routing coverage; 3 tests.
- **desktop-tool**: monitor / send / retrieve / integrity / configure driven
  by real subsystems + derived keys; 3 tests.
- **flasher**: `swd` validates + prints probe-rs commands; `ota` computes
  real SHA-256, slice plan, and writes a bootloader-compatible `.ota`
  staging file; 2 tests.

### CI & release artifacts
- `.github/workflows/ci.yml`: blocking embedded jobs (node + bootloader,
  `--release --target thumbv6m-none-eabi --no-default-features --features
  embedded`), clippy `-D warnings` (host + embedded), fmt, tests, docs.
  No more `|| echo "non-blocking"`.
- `LICENSE` (MIT), `CHANGELOG.md`, `.gitignore`, git repo initialized with
  a full commit.

### Docs
- **`HOW_IT_WORKS.md`** (new): complete human-language walkthrough of the
  energy chain, protocol, mesh, dead-drops, geo-fencing, crypto, thermal,
  integrity, flash, bootloader/OTA, tools, end-to-end stories, code map.
- `whitepaper.md`: dead-drop capacity corrected (32 fragments).
- `protocol/protocol-spec.md`: CRC16 in frame layout, link-layer
  fragmentation, RouteAdv 5-entry cap + no-dst-rewrite, concrete key
  derivation scheme.
- `firmware/firmware-guide.md`: corrected build commands (host +
  `--no-default-features --features embedded --release`), real OTA flow,
  radio abstraction, storage table.
- `README.md`: quick-start verified, links to the other docs.

---

## 3. Known limitations (by design / honest)

1. **Peripheral drivers not yet wired.** LIS3DH (I2C1), nRF24L01+/CC1101
   (SPI1), DS18B20 (1-Wire), and the external SPI staging flash are
   `TODO(bring-up)` in the platform modules. Everything else is exercised by
   the host simulation + 92 tests.
2. **No hardware.** PCB/KiCad files don't exist; only BOM, schematic
   description, and build guide.
3. **Piezo energy assumptions.** Harvest model assumes ~1 mJ per event; real
   deployments need the whitepaper's cantilever/tuning mitigations.
4. **The desktop tool's store is per-process.** Send/retrieve don't persist
   across invocations (the CLI reports honestly when a message isn't found
   locally). The simulator covers multi-node flows.

---

## 4. Verification log (2026-08-13)

- `cargo build --workspace` ✅
- `cargo test --workspace` ✅ (92 tests, 0 failed)
- `cargo clippy --workspace -- -D warnings` ✅ (0 warnings)
- `cargo clippy -p smart-matter-node -p smart-matter-bootloader
  --no-default-features --features embedded --target thumbv6m-none-eabi` ✅ (0 warnings)
- `cargo build -p smart-matter-core --no-default-features` ✅
- `cargo build --release -p smart-matter-node --target thumbv6m-none-eabi
  --no-default-features --features embedded` ✅ (~19.8 KB / 32 KB)
- `cargo build --release -p smart-matter-bootloader --target
  thumbv6m-none-eabi --no-default-features --features embedded` ✅ (~1 KB / 16 KB)
- `cargo fmt --all -- --check` ✅
- `cargo run -p smart-matter-node -- --ticks 200` ✅ (anomaly alerts, energy, tx)
- `cargo run -p smart-matter-network-simulator -- --nodes 10 --duration 40 --messages 15`
  ✅ (13 sent / 13 delivered, 100% PDR, 3.9 avg hops)
- `cargo run -p smart-matter-desktop -- monitor` ✅
- `cargo run -p smart-matter-flasher -- ota --firmware …` ✅
- `cargo run -p smart-matter-bootloader` ✅ (all OTA scenarios)

---

## 5. Resume cheat-sheet

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo run -p smart-matter-node -- --ticks 200
cargo run -p smart-matter-network-simulator -- --nodes 10 --duration 40
cargo run -p smart-matter-bootloader
export SMART_MATTER_DEPLOYMENT_SECRET=$(openssl rand -hex 32)
cargo build --release -p smart-matter-node --target thumbv6m-none-eabi \
    --no-default-features --features embedded
```
