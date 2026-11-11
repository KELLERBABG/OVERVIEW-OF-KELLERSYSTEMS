# City OS — Roadmap

City OS is implemented in phases aligned with the KELLER OS plan (KELLER OS IS
City OS Phase 1/2). This roadmap tracks what exists today, what is next, and
what requires hardware.

## Phase status

| Phase | Scope | Status |
|---|---|---|
| **L0 foundation** | types, crypto (KELLER reuse), caps, IPC | ✅ complete |
| **L1 district services** | BRP, DAG, P2P, shards, dead-drops, allocation | ✅ complete |
| **L2 governance & edge** | voting, healing, interop, incentives, entropy | ✅ complete |
| **Demo/harness** | `city-node` host boot + 52 tests (incl. multi-node convergence sim) | ✅ complete |
| **Crypto hardening** | keyed SipHash-2-4 MAC (replaces forgeable FNV-1a), 256-bit-seed PRF entropy (replaces 32-bit LCG), 256-bit BRP nonces | ✅ complete |
| **Phase 3.6 kernel wiring** | `SYS_CITY_*` opcodes, `CAP_CITY_*` slots, ring-3 registry, kernel boot selftest (`city_services`) | ✅ complete |
| **Hardware bridge** | e1000 NIC TX/RX ring bridge + SDR HAL seam (kernel + host mirror) | ✅ implemented (builds; RF/PCI binding deferred) |
| **Docs** | architecture, security, protocols, roadmap, whitepaper | ✅ complete |
| Phase 4 hardware | ARM64 Pi bring-up, real RF SDR binding | 🔜 next (hardware required) |

## Completed (this workspace — 16 library crates + `city-node`)

All software components of the City OS canvas that run on the node are
implemented, tested, and exercised by the boot demo:

1. Foundation — `city-types`, `city-crypto`, `city-caps`, `city-ipc`
2. District — `city-brp`, `city-dag`, `city-p2p`, `city-shard`,
   `city-deaddrop`, `city-alloc`
3. Edge/governance — `city-vote`, `city-heal`, `city-interop`,
   `city-incentive`, `city-entropy`
4. Kernel ABI — `city-syscall` (mirrors `SYS_CITY_*` / `CAP_CITY_*`)
5. Boot + hardware bridge — `city-node` (L0–L4 boot), `city-p2p::hw`
6. District simulation — `city-sim` (multi-node gossip/ledger convergence,
   wired into the boot demo as `sim_convergence`)

**Security hardening completed in this round:**

- `city-crypto::mac` — the salted FNV-1a tag was **forgeable** (invertible
  structure → salt recovery from one frame). Replaced with keyed
  SipHash-2-4: 128-bit secret key, 64-bit tag, reference-vector-tested.
- `city-crypto::osrng` — the 32-bit-seeded LCG was brute-forceable.
  Replaced with a SipHash counter-mode PRF over **256-bit seeds** plus a
  host OS-entropy source (`fill_os_random`).
- `city-brp` — token nonce widened to 256 bits; `city-vote` challenge and
  blind identity derive from a 256-bit ballot seed.

**Kernel-side (sovereign-core) completed in this round:**

- `caps.rs` — ten `CAP_CITY_*` slots (7–16) issued at boot.
- `city.rs` — ring-3 City service registry (`SERVICES[10]`), per-service
  capability handles, the `SYS_CITY_*` dispatch gate (opcodes 64–77), the
  e1000 MMIO TX/RX descriptor-ring bridge, and the SDR HAL seam.
- `syscall.rs` — routes any opcode in `SYS_CITY_BASE..SYS_CITY_BASE+31`
  into `city::city_syscall`, matching the API the driver sandbox uses.
- `selftest.rs` — new `city_services` boot test proves capability
  issue/revoke per service, per-service opcode dispatch, cross-service
  denial, e1000 TX buffering, and the SDR arm/inject/collect round trip.
- `main.rs` — `city::init()` during stage 2.
- `cargo check` on `sovereign-core` passes (previous warnings unchanged).

## Next: spawn the ring-3 service tasks (Phase 3.6 remainder)

The ABI, registration, and dispatch are in place; the remaining wiring is to
spawn each City service as an actual ring-3 task:

1. **Spawn each City service via `scheduler::start_task`** into slots 11–20,
   exactly like the existing `serial_driver_task`, handing each task its
   registry capability handle. The `SERVICES[i].task_slot` fields are already
   reserved (11–20) for this.
2. **Bridge the application layer onto the kernel IPC.** `city-p2p`'s
   `broadcast()` already calls `city_ipc::send`; replace the host `CHANNELS`
   backing table with the kernel's real `ipc::send/recv` channels.
3. **Carry the boot `[TEST]` protocol.** The kernel's `city_services` test and
   the host demo's L0–L4 checks use the same `[TEST] name: PASS` format, so a
   QEMU `run-tests.ps1` run shows both layers on one log.
4. **Reproduce.** `repro-build.ps1` pins the toolchain; extend
   `check_no_alloc.ps1` over `crates/` to keep the no-alloc promise.

## Deferred (requires hardware)

| Item | Blocking item |
|---|---|
| ARM64 / Raspberry Pi 4× district | KELLER Phase 4.1 cross-compile + bring-up |
| e1000 NIC integration for real P2P | KELLER `e1000.rs` link state is detected; driver traffic to be wired |
| SDR Shadow Grid overlay | RF hardware (HackRF One) + IOMMU isolation work, KELLER Phase 4.4 |
| Municipal pilot | permits, insurance, integration with existing traffic/power systems |
| Proof-of-vote zk-SNARK (vs Schnorr) | the demo ballot scheme is tamper-evident but derives its blind secret from the public ballot, so it is not sound against a voter forging a fresh ballot; swap in a SNARK verifier crate when the circuit is needed |

## Ongoing

- Keep every crate `no_std` / no-allocation.
- Keep `city-crypto` identical to KELLER (it is copied verbatim) so security
  reviews of KELLER carry over.
- Extend the `[TEST] name: PASS` protocol to the new services as they move
  into the kernel.