# City OS — A Sovereign, Capability-Based Operating System for Decentralized Urban Infrastructure

**Technical Whitepaper**

Version 1.0 · [City OS workspace](../README.md) · Foundation: KELLER OS
(`sovereign-core`)

---

## Abstract

City OS is a decentralized urban operating system that reuses the verified
security foundation of KELLER OS — a capability-based microkernel with
256-bit secp256k1 zero-knowledge identification, AES-XTS encrypted RAM
vaults, keyed-MAC IPC, and Panic-Zeroing — and layers the full City
application stack on top of it. The City OS workspace implements **16
service crates** covering every software component of the City OS vision:

- a DAG-based transaction ledger,
- identity-less service access via blind randomized tokens,
- geofenced localized shard storage,
- dead-drop infrastructure,
- autonomous resource allocation,
- anonymous P2P governance voting,
- self-healing / failover,
- V2V/drone/logistics interoperability with ad-hoc traffic orchestration,
- incentivized node hosting with verification,
- environmental-signature-synchronized OTP keys,
- and a MAC-authenticated gossip transport over real kernel IPC channels.

All code is `no_std`, allocation-free, and capability-gated, so the same
crates compile for a host demo (`city-node`, which prints the kernel-style
`[TEST] name: PASS` boot protocol) and, on the KELLER microkernel, as the
ring-3 services behind the `SYS_CITY_*` ABI (KELLER OS Phase 3.6) — and
eventually on ARM64 city edge nodes (Phase 4). The kernel-side integration
(`SYS_CITY_*` opcodes, `CAP_CITY_*` slots, service registry, e1000/SDR
bridges) is also compiled and self-tested in `sovereign-core`. The workspace
currently ships **52 passing tests** that enforce the security properties
claimed here, including a multi-node district simulation that proves
verified gossip converges to identical ledger contents on every node.

---

## 1. Introduction

### 1.1 Motivation

Contemporary "smart city" platforms are centralized: a vendor-operated cloud
ingests data from proprietary sensors, and citizens' access to urban services
is mediated by accounts, identity providers, and opaque governance. City OS
inverts this model. It treats the city as a **sovereign peer-to-peer network
of nodes**, where every node runs a hardened microkernel, every service is a
capability-gated application, and no central party is required for trust.

The vision spans thirteen core components (DAG ledgers, edge/fog computing,
autonomous resource allocation, P2P governance, identity-less access,
localized shards, resilience, V2V/drone interop, incentives, dead-drops,
environmental entropy, shadow-grid emergency comms). This whitepaper
describes the *software* implementation of those components that run on a
node, and why building them on KELLER OS gives them a security posture that a
cloud-hosted implementation cannot match.

### 1.2 Relationship to KELLER OS

> KELLER OS IS City OS Phase 1/2. The kernel, Keller Net, Keller Auth, and
> Keller Vault components ship unchanged to city edge nodes. City-specific
> services are applications that run ON TOP of this kernel. One codebase, two
> deployment profiles.

The City OS workspace is the **application layer** of that plan. It does not
reimplement cryptography or kernel primitives; it reuses them verbatim and
aligns every City service with the kernel's ABI:

| KELLER OS primitive | City OS reuse |
|---|---|
| `bigint.rs` / `ec.rs` — secp256k1 256-bit arithmetic | `city-crypto` (verbatim) |
| `aes.rs` — software AES-128 + XTS | `city-crypto` (verbatim) |
| `vault.rs` — AES-XTS secret store, Panic-Zeroing | `city-crypto` (verbatim) |
| `caps.rs` — capability table, O(1) runtime revocation | `city-caps` (slots 0–6 identical) |
| `ipc.rs` — salted-MAC channels, anti-replay | `city-ipc` (keyed SipHash-2-4 MAC + seq checking; KELLER's own tag is not keyed) |
| `auth.rs` — 256-bit Schnorr ZK identification | `city-brp` / `city-vote` (same equation) |
| `temporal.rs` — TSC slot budgets | `city-alloc` (4 fixed windows) |
| `selftest.rs` — `[TEST] name: PASS` protocol | `city-node` (host harness) |

## 2. Design Principles

1. **Security by inheritance.** The kernel is the trust root; services gain
   capability enforcement, authenticated IPC, and Panic-Zeroing for free.
2. **No allocation.** Every crate is `no_std` with fixed-capacity arrays. A
   service cannot be denial-of-serviced by heap exhaustion.
3. **No secrets on the wire.** BRP tokens and ballots prove knowledge of
   secrets with zero-knowledge proofs; raw identities and raw sensor readings
   are never transmitted.
4. **Minimum capability.** Each service is granted only the slot it needs and
   can be revoked in real time.
5. **One binary, two profiles.** The same code runs on a host for tests/demos
   or as a ring-3 service on the KELLER kernel.
6. **Determinism and auditability.** Protocols are defined byte-for-byte in
   `docs/PROTOCOLS.md` and enforced by tests, so behavior is reproducible and
   verifiable.

## 3. The Foundation (Layer 0)

### 3.1 city-types

Shared wire vocabulary sized to the kernel: `Bytes32` (exactly one KELLER IPC
payload), `CityMsg` (envelope + body), `GeoTag` (lat/lon quantization),
`NodeId`, `CapSlot`, and `SlotBudget`. All City protocols serialize into
these types.

### 3.2 city-crypto

Four modules are copied verbatim from KELLER, preserving its audit trail:

- **bigint** — 256-bit little-endian `[u32; 8]` arithmetic over the secp256k1
  field prime `p` and group order `n`, with provably-correct binary long
  division and Fermat-inverse.
- **ec** — affine secp256k1 point math (double/add/scalar-mul), the same
  group Keller Auth uses.
- **aes** — table-free software AES-128; XTS mode with per-unit tweaks for
  the vault.
- **vault** — AES-XTS encrypted sectors with integrity tags, `store` /
  `retrieve` / `forget`, and atomic Panic-Zeroing (wipes ciphertext, tags,
  and key schedules).

On top, City adds two primitives:

- **mac** — a **keyed SipHash-2-4** MAC over `(key, sender, type, seq,
  payload)` with a 128-bit secret key and a 64-bit tag. The earlier salted
  FNV-1a tag was dropped because its multiply/XOR steps are invertible — a
  single observed frame leaked the salt and enabled forgery. SipHash is a
  keyed PRF: a tag reveals nothing about the key, and any byte change or
  any sequence replay fails verification.
- **osrng** — deterministic 256-bit entropy expansion via a SipHash
  counter-mode PRF over **256-bit seeds** (the earlier 32-bit-seeded LCG
  was brute-forceable). Used for BRP tokens and ballot secrets.
  Deterministic so services can re-derive and validate without storing
  per-holder state. A host OS-entropy source (`fill_os_random`, `std`
  feature) feeds it with fresh seed material.

### 3.3 city-caps

The capability table is KELLER-compatible: handles are indices into a
64-slot table; `check(handle, slot)` verifies validity **and** the slot
binding; `revoke(handle)` clears validity in O(1) and wipes the tag. City
extends the table with application slots:

| Slot | Name | Service |
|---|---|---|
| 0–6 | `CAP_SERIAL` … `CAP_IO` | kernel (identical) |
| 7 | `CAP_CITY_DAG` | ledger + governance |
| 8 | `CAP_CITY_BRP` | identity-less access |
| 9 | `CAP_CITY_DEADDROP` | dead-drop store |
| 10 | `CAP_CITY_SHARD` | geofenced shards |
| 11 | `CAP_CITY_ALLOC` | resource allocation |
| 12–16 | `CAP_CITY_VOTE` … `CAP_CITY_ENTROPY` | governance, healing, interop, incentive, entropy |

Because revocation is O(1), a misbehaving service can be switched off in real
time — the same property the kernel uses for sandboxed drivers.

### 3.4 city-ipc

Eight channels, each a ring buffer of 32-byte messages with a per-channel
**secret key** and independent send/recv sequence counters. A frame is:

```
payload[24] ‖ SipHash-2-4(key, sender, type, seq, payload)
```

The receiver drops any frame with a bad MAC or an out-of-order sequence
counter. This is the universal transport for City services and the substrate
for the P2P gossip layer.

## 4. District Services (Layer 1)

### 4.1 city-brp — blind randomized access

BRP gives identity-less service access. A district service issues a 32-byte
token deterministically from `(service_id, expiry_slot, nonce)` where the
nonce is **256 bits** — the old 32-bit nonce space was brute-forceable, so
token secrets are now full-width. The holder
derives a local secret `x = parse(token) mod n`, public point `V = x·G`, and
proves knowledge with a Schnorr round: commitment `K = k·G`, challenge `c`,
response `r = k + c·x mod n`; the service checks `r·G == K + c·V`. The
service never sees `x` and never sees an identity — it only learns that
someone knew the token for that service/expiry. Capability revocation
(`CAP_CITY_BRP`) gives the district operator a runtime kill switch.

### 4.2 city-dag — the ledger

A fixed 64-vertex DAG. Every vertex has one or two parents (the "tips"), and
its 32-byte transaction id is four keyed SipHash-2-4 MAC blocks over
`(sender, seq, block, parents/body)`, so a transaction id fits exactly one
IPC payload and can be verified end-to-end by any node that holds the DAG
key. `advance_finality()` finalizes a vertex once at least
`DAG_FINALIZE_DEPTH` later vertices transitively reach it. Unknown parents
and duplicate ids are rejected at insert, preventing double-spend in the
DAG-consensus sense.

### 4.3 city-p2p — verified gossip

The P2P layer provides a fixed peer table and a verified frame buffer.
Frames carry a salted MAC; `ingest` rejects invalid and replayed frames;
`broadcast` sends verified frames over the real `city-ipc` channels — the
first step toward riding on KELLER's e1000 NIC in hardware.

### 4.4 city-shard — geofenced data

A `GeoBox` defines a district; a `GeoTag` encodes latitude/longitude. Shards
are only written when their tag's district owns them, and are stored
AES-XTS-encrypted via the Keller Vault. This enforces the "data stays within
district boundaries" guarantee at the storage layer.

### 4.5 city-deaddrop — async encrypted messaging

Dead-drops store encrypted 32-byte messages with a TTL in scheduler ticks. A
sweep wipes expired drops from the vault. Because storage uses the kernel
vault, a panic erases all drop contents via Panic-Zeroing.

### 4.6 city-alloc — autonomous resource allocation

Four fixed demand windows (aligned with KELLER's `SLOT_COUNT = 4`) feed a
fixed-point moving-average predictor. Grants are MAC-signed `AllocRecord`s;
under projected scarcity, grants are denied — a simple smart-contract
enforcement executed on-node.

## 5. Governance & Edge Services (Layer 2)

### 5.1 city-vote — anonymous governance

Ballots carry a zero-knowledge proof of ballot knowledge (the same Schnorr
equation as BRP) and are chained to the previous ballot (tamper-evident).
The ballot id is a keyed MAC over the *stable* fields — proposal, choice,
previous link — deliberately excluding the proof bytes, so the proof can be
computed over the id deterministically. The challenge and blind identity
derive from a 256-bit seed of the ballot. The verifier is stateless, so no
vote-tracking state exists to subpoena. (Honest scope: because the blind
secret is derived from the public ballot, this demo scheme is tamper-evident
rather than sound against a voter forging a fresh ballot; the roadmap's
zk-SNARK verifier is the planned fix.)

### 5.2 city-heal — self-healing

A heartbeat table tracks `offline_ticks` per node. When a node exceeds the
threshold, `failover(role)` promotes the first healthy standby. A crucial
design detail: standby nodes do **not** accrue offline time until promoted,
so a prolonged primary outage cannot accidentally kill the failover path.

### 5.3 city-interop — vehicles, drones, logistics

`V2vMsg` frames are MAC-verified and kind-validated. The
`IntersectionScheduler` grants FIFO right-of-way up to a per-tick capacity —
ad-hoc traffic orchestration with no central coordinator and a hard bound on
queue depth (the "cone-zone" limit).

### 5.4 city-incentive — hosting rewards

Nodes submit MAC-signed uptime claims; verified claims credit energy to the
node's account. Forgery, replay, and out-of-order windows are rejected.
Credits can be paid out through the DAG ledger — both are reachable by ring-3
tasks via the `SYS_CITY_*` ABI (`SYS_CITY_INCENTIVE`, `SYS_CITY_DAG`).

### 5.5 city-entropy — environmental sync

Neighboring nodes observe environmental noise (seismic amplitude, EM field
strength, temperature). Each reading is quantized into cells before hashing,
so two honest neighbors within one cell derive the *identical* 32-byte OTP
key — without ever transmitting raw readings. The key size matches one vault
sector, so it can be stored encrypted; it rotates per window.

## 6. The Boot Protocol

The `city-node` binary boots the full stack and prints the same
`[TEST] name: PASS` assertion protocol the kernel uses at boot:

```
[L0] caps_issue_revoke, ipc_salted_mac, vault_aes_xts
[L1] dag_ledger, brp_identityless, p2p_gossip, sim_convergence,
     shard_geofence, deaddrop_ttl, alloc_predict_grant
[L2] vote_anonymous, heal_failover, interop_v2v_traffic,
     incentive_uptime, entropy_sync_otp
[L3] syscall_abi, hw_e1000_bridge, hw_sdr_seam
[HARNESS] CITY OS BOOT: PASS
```

This gives a single, greppable, machine-readable health gate for the
application layer, mirroring KELLER's `run-tests.ps1` for the kernel.

## 7. Security Model

The full threat model is in `docs/SECURITY.md`. Key claims:

- **Authenticity / freshness.** Every IPC message, P2P frame, V2V frame,
  allocation record, and uptime claim is keyed-MAC authenticated
  (SipHash-2-4, 64-bit tag) with anti-replay sequencing. Tests demonstrate
  tamper and replay rejection.
- **Confidentiality.** Shards and dead-drops are AES-XTS encrypted in the
  vault; tampered ciphertext fails the integrity tag.
- **Unlinkability.** BRP tokens and governance ballots are zero-knowledge;
  services never see holder identity or raw inputs.
- **Revocation.** Capabilities are re-checked at every dispatch; revocation
  is O(1).
- **Resilience.** Standby promotion, geo-ownership enforcement, and bounded
  queues prevent data loss and unbounded resource use.
- **No persistent secrets.** Vault contents and key schedules are wiped on
  panic (KELLER Panic-Zeroing).

## 8. Verification & Evidence

- **52/52 unit tests pass** (`cargo test --workspace --release`), each tied
  to a printable security claim in `docs/SECURITY.md`, including the
  SipHash-2-4 reference vectors and the multi-node convergence sim.
- KELLER's own verification (Schnorr-256 protocol checks, AES-XTS tamper
  detection, TLA+ capability-safety spec, QEMU self-test harness) carries
  over to `city-crypto` because the code is copied verbatim.
- The boot protocol is machine-readable, enabling the same harness style
  CI used for KELLER.

## 9. Roadmap

`docs/ROADMAP.md` separates what is implemented from what needs hardware.

**Implemented (beyond layer 0–2 software).**

- *Kernel syscall integration (KELLER Phase 3.6):* the kernel now exposes
  `SYS_CITY_*` opcodes (64–77) for all ten City services plus the e1000/SDR
  transport, backs them with `CAP_CITY_*` capability slots (7–16), registers
  each service in `SERVICES[10]` with its own handle and reserved ring-3 task
  slot, and runs a `city_services` boot selftest that proves per-service
  dispatch, cross-service denial, e1000 TX buffering, and the SDR seam.
  `city-syscall` mirrors the ABI so host services and ring-3 tasks stay in
  lock-step.
- *Hardware interoperability:* `city-p2p::hw` provides the NIC bridge
  (`Nic`, programming the e1000 TDBAL/TDLEN/TDH/TDT and RAL/RDLEN/RDH/RDT
  descriptor registers) and the SDR Shadow-Grid seam (`Sdr`: arm/submit/
  collect), unified behind one `Transport` frame API.

**Remaining (requires physical hardware).** ARM64 / Raspberry Pi bring-up
(KELLER Phase 4.1), binding the kernel e1000 bridge to packets on the real
NIC in QEMU/hardware, binding the SDR seam to an RF device (HackRF One,
Phase 4.4), and the municipal pilot.

## 10. Conclusion

City OS demonstrates that the full decentralized-urban plan — ledger,
identity-less access, sharding, dead-drops, resource allocation, governance,
healing, interop, incentives, and entropy — can be implemented as a
capability-gated, allocation-free application layer on top of a verified
microkernel, with the same security primitives the kernel itself depends on.
The kernel ABI, service registry, and e1000/SDR bridges are implemented and
tested; the remaining work is hardware deployment (spawning the real ring-3
tasks, ARM64, RF binding) specified in the roadmap.

---

*Appendix A (protocol byte specifications) and Appendix B (test-to-claim
mapping) live in `docs/PROTOCOLS.md` and `docs/SECURITY.md` respectively.*