# Smart-Matter — Development Roadmap & Action Tasks

> This file is the authoritative internal development roadmap and task sequencing plan.
> **Language:** `Rust`
> **Summary:** A multi-crate embedded Rust workspace (`firmware/core`, `firmware/bootloader`, `firmware/node`, `mesh`, `storage`, `broadcaster`, `desktop-tool`, `tools/network-simulator`) for autonomous, piezoelectric kinetic-energy-harvesting compute nodes embedded inside concrete and asphalt.

## Task Queue

- [ ] **TASK-01: Enforce flash-write energy margin and power-fail interrupt protection in energy.rs**
  - **Target Files:** `firmware/core/src/energy.rs`, `firmware/core/src/storage/dead_drop.rs`
  - **Context Files:** `firmware/core/src/storage/wear_level.rs`, `firmware/core/src/lib.rs`
  - **Verify:** `cargo test -p smartmatter-core --quiet`
  - **Spec:** Harden `firmware/core/src/energy.rs` so that before calling any flash write/erase routine in `dead_drop.rs` or `wear_level.rs`, the node queries the energy monitor.

1. If capacitor voltage $V_{\text{cap}} < V_{\text{flash\_safe}}$ (e.g. $2.70\text{ V}$), abort the write and buffer the shard in non-volatile low-voltage ferroelectric RAM (FRAM) or defer until the next kinetic energy pulse arrives.
2. Add unit tests in `firmware/core` verifying zero partial writes under simulated brownout voltage drops.

- [ ] **TASK-02: Implement Bloom-filter deduplication and TTL decay in ad-hoc mesh routing**
  - **Target Files:** `firmware/core/src/mesh.rs`, `mesh/src/lib.rs`
  - **Context Files:** `firmware/core/src/radio.rs`, `tools/network-simulator/src/sim.rs`
  - **Verify:** `cargo test -p smartmatter-mesh --quiet`
  - **Spec:** Enhance `firmware/core/src/mesh.rs` and `mesh/src/lib.rs` to prevent broadcast storms across dense concrete sensor matrices.

1. Maintain a rolling 64-bit Bloom filter of recently seen packet hashes. Drop any duplicate packet within the active epoch.
2. Enforce strict hop-limit TTL decrement (default `max_hops = 6`), dropping packets when TTL reaches zero.
3. Add unit and network-simulation tests in `mesh/tests/integration_test.rs` demonstrating epidemic propagation terminates deterministically across a 50-node simulated asphalt grid.

- [ ] **TASK-03: Implement cryptographic dead-drop epoch shredding and anti-tamper zeroization**
  - **Target Files:** `firmware/core/src/storage/dead_drop.rs`, `firmware/core/src/crypto.rs`
  - **Context Files:** `firmware/core/src/integrity.rs`
  - **Verify:** `cargo test -p smartmatter-core --quiet`
  - **Spec:** Extend `firmware/core/src/storage/dead_drop.rs` so stored encrypted shards are associated with an epoch key. Upon epoch expiration or tamper detection (piezoelectric shock exceeding fracture limits), overwrite local key slots with volatile zeroes, rendering all stored dead-drop shards permanently unrecoverable.
