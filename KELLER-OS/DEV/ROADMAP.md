# KELLER-OS — Development Roadmap & Action Tasks

> This file is the authoritative internal development roadmap and task sequencing plan.
> **Language:** `Rust`
> **Summary:** A bare-metal `#![no_std]` sovereign x86_64/i686 microkernel (`src/main.rs`, `src/proc.rs`, `src/mm.rs`, `src/ipc.rs`, `src/crypto.rs`, `src/net.rs`, `src/storage.rs`, `src/vault.rs`, `src/zk.rs`, `src/panic.rs`) with Python QEMU verification harnesses (`dev-tools/qemu_check.py`, `wire_check.py`, `smp_check.py`, `disk_check.py`).

## Task Queue

- [ ] **TASK-01: Enforce OCap capability attenuation and monotonic revocation counters in ipc.rs and proc.rs**
  - **Target Files:** `src/ipc.rs`, `src/proc.rs`
  - **Context Files:** `src/main.rs`, `SPECIFICATION.md`
  - **Verify:** `python dev-tools/wire_check.py`
  - **Spec:** Ensure `src/ipc.rs` and `src/proc.rs` enforce the Take-Grant capability invariant from `KELLER OS.canvas`.

1. A process $A$ transferring capability $c$ to process $B$ via IPC can only attenuate rights (`new_rights = c.rights & mask`), never amplify them.
2. Validate capability generation counters against the resource table on every IPC send/recv so revoked capabilities are rejected immediately.
3. Verify clean compilation with `cargo check --quiet` and `python dev-tools/wire_check.py`.

- [ ] **TASK-02: Verify Base-Offset-Shifting anti-replay sliding window in session.rs**
  - **Target Files:** `src/session.rs`, `src/net.rs`
  - **Context Files:** `src/crypto.rs`
  - **Verify:** `python dev-tools/wire_check.py`
  - **Spec:** Ensure `src/session.rs` implements the 64-packet sliding-window replay filter (`Base Offset Shifting` from `Life Work.canvas`): packets older than `max_seq - 64` are dropped, packets within `[max_seq - 63, max_seq]` check/set their bit in a `u64` bitmask, and packets with `seq > max_seq` shift the window forward.

- [ ] **TASK-03: Enforce volatile RAM key scrubbing in panic.rs and vault.rs on tamper NMI**
  - **Target Files:** `src/panic.rs`, `src/vault.rs`
  - **Context Files:** `src/crypto.rs`
  - **Verify:** `python dev-tools/wire_check.py`
  - **Spec:** Ensure `src/panic.rs` invokes a volatile zeroization routine in `src/vault.rs` using `core::ptr::write_volatile` + `core::sync::atomic::compiler_fence(Ordering::SeqCst)` so the compiler cannot dead-store-eliminate key wiping prior to CPU halt.
