# CityOS — Development Roadmap & Action Tasks

> This file is the authoritative internal development roadmap and task sequencing plan.
> **Language:** `Rust`
> **Summary:** An 18-crate `#![no_std]` Rust urban operating system workspace (`city-dag`, `city-crypto`, `city-caps`, `city-brp`, `city-shard`, `city-deaddrop`, `city-alloc`, `city-heal`, `city-zkp`, `city-ata`, `city-ehar`, `city-smartmatter`, `city-node`, etc.) built to run as sovereign municipal infrastructure atop KELLER-OS.

## Task Queue

- [ ] **TASK-01: Implement equivocation detection and causal topological ordering in city-dag**
  - **Target Files:** `crates/city-dag/src/lib.rs`
  - **Context Files:** `crates/city-crypto/src/lib.rs`
  - **Verify:** `cargo test -p city-dag --quiet`
  - **Spec:** Enhance `crates/city-dag/src/lib.rs` with Byzantine equivocation detection and deterministic topological finality.

1. Detect when the same `author_id` submits two distinct vertices with identical sequence numbers or non-ancestral parent references, returning `DagError::EquivocationDetected { author, seq }`.
2. Implement deterministic causal sorting (Kahn's algorithm with hash tie-breaking) so all honest city nodes derive identical state transition sequences.
3. Add unit tests verifying fork detection and deterministic ordering across 3 simulated districts.

- [ ] **TASK-02: Enforce anonymous credential nullifier uniqueness and epoch bounds in city-zkp and city-caps**
  - **Target Files:** `crates/city-zkp/src/lib.rs`, `crates/city-caps/src/lib.rs`
  - **Context Files:** `crates/city-crypto/src/mac.rs`
  - **Verify:** `cargo test -p city-zkp -p city-caps --quiet`
  - **Spec:** Harden `city-zkp` and `city-caps` to guarantee identity-less municipal service access without replay or privilege escalation.

1. In `city-zkp/src/lib.rs`, maintain a bounded epoch nullifier set rejecting any repeated service-access nullifier within the same municipal billing epoch.
2. In `city-caps/src/lib.rs`, enforce monotonic capability attenuation (a delegated token can only restrict permissions/TTL, never widen them).
3. Add unit tests for double-spend nullifier rejection and capability attenuation.

- [ ] **TASK-03: Implement automated utility load-shedding and self-healing failover across city-alloc and city-heal**
  - **Target Files:** `crates/city-alloc/src/lib.rs`, `crates/city-heal/src/lib.rs`
  - **Context Files:** `crates/city-ata/src/lib.rs`, `crates/city-ehar/src/lib.rs`, `crates/city-smartmatter/src/lib.rs`
  - **Verify:** `cargo test -p city-alloc -p city-heal --quiet`
  - **Spec:** Wire `city-alloc` and `city-heal` to enforce strict priority-tier resource allocation (Tier 0: Emergency/Hospital/Water > Tier 1: Transit/Streetlight > Tier 2: Commercial/ATA).

1. Prove via unit tests that during simulated grid degradation, Tier 0 municipal services receive 100% of requested power/bandwidth before any Tier 2 allocation occurs, and dead edge nodes are isolated within 2 heartbeat ticks.
