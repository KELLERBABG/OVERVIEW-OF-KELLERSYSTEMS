# Changelog

All notable changes to Smart Matter are documented here.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- **RFC 8439 ChaCha20-Poly1305** authenticated encryption (RustCrypto
  `chacha20poly1305`), replacing the non-standard hand-rolled construction.
- **Deployment-scoped key derivation** (`derive_key(deployment_secret,
  node_id)`, HMAC-SHA256). No hardcoded key material anywhere.
- **Wire format module** (`smart_matter_core::wire`): CRC-16/CCITT framing
  for protocol slices and link-layer fragmentation for 32-byte (nRF24L01+)
  and 64-byte (CC1101) radio payloads.
- **Route advertisement serialization.** Mesh entries now travel over the
  wire (postcard, ≤5 entries/slice, rotated), enabling multi-hop route
  learning.
- **Shared flash memory map** (`smart_matter_core::memmap`) used by the
  bootloader, node, and flasher.
- **Buildable node firmware** with host simulation mode
  (`cargo run -p smart-matter-node`) and embedded Cortex-M0+ target
  (STM32G031J6, `--target thumbv6m-none-eabi`, links at 0x08004000, ~20 KB
  of the 32 KB app region). Real ADC voltage sensing + status LED on the
  embedded platform; LIS3DH / radio / DS18B20 drivers are documented
  bench bring-up points.
- **Real bootloader** with fail-closed OTA: SHA-256 validation, erase-and-
  copy, vector-table jump; host-testable via `Flash`/`StagedImage` traits
  (6 unit tests).
- **Real tools**: network simulator (protocol-driven metrics), desktop tool
  (mesh/storage/integrity/key commands), flasher (SHA-256, slice plans,
  `.ota` staging files).
- **CI** with blocking embedded builds (no more `|| echo` swallowing),
  clippy `-D warnings`, fmt check, docs job.
- MIT `LICENSE`, this changelog, `.gitignore`.

### Changed
- Supercapacitor model corrected to **1 F** (was 100 mF) to match the BOM
  and whitepaper; session slice cap remains 255.
- Dead-drop store capacity set to **32 fragments** (~6 KB SRAM) so it fits
  the 8 KB SRAM of the STM32G031J6.
- Flash wear-leveling log set to **3 sectors (12 KB)**, consistent with the
  whitepaper memory map.
- Embedded clock stored as `u32` ms (thumbv6m has no 64-bit atomics).
- `no_std` float math via `libm` (f32 `sqrt`/`exp`/`powf` are std-only on
  current Rust); `[T]::sort_by*` moved to `alloc`, so the thermal scheduler
  uses a linear priority scan.
- Integration tests moved from the workspace root into `mesh/tests/` so they
  are actually executed.

### Fixed
- Multi-hop forwarding bug: `dst` is no longer rewritten to the next hop,
  which silently dropped messages at the penultimate node.
- `firmware/node` and `firmware/bootloader` no longer fail the host
  workspace build (feature-split host/embedded).
- "No global memory allocator" in the embedded build (removed the bare
  `extern crate alloc;`).

## [0.1.0] - prototype baseline (pre-changelog)

- Protocol logic in `no_std` Rust: gradient mesh routing, dead-drop
  store/fragment/assemble/gc, RSSI geo-fenced broadcast, thermal/integrity/
  energy models, entropy pool, wear-leveling model, protocol slice types.
- Docs: whitepaper, protocol spec, firmware guide, hardware build guide,
  BOM, energy-harvester schematic.
