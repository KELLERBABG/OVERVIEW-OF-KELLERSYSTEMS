# City OS — Architecture

City OS is a decentralized urban operating system implemented as a set of
`no_std`, no-allocation Rust crates that run **on top of KELLER OS**. This
document describes the system's layered architecture, data flows, and how the
KELLER OS security guarantees carry through every City service.

## 1. Layering

```
┌──────────────────────────────────────────────────────────────┐
│  City OS application layer (this workspace)                  │
│                                                              │
│  L2  Governance & edge services                              │
│     city-vote · city-heal · city-interop ·                   │
│     city-incentive · city-entropy                            │
│                                                              │
│  L1  District services                                       │
│     city-brp · city-dag · city-p2p · city-shard ·            │
│     city-deaddrop · city-alloc · city-sim (district harness) │
│                                                              │
│  L0  Shared foundation (KELLER-aligned)                      │
│     city-types · city-crypto · city-caps · city-ipc          │
│                                                              │
│  L-A Kernel ABI + hardware bridge                            │
│     city-syscall (SYS_CITY_* / CAP_CITY_*) ·                 │
│     city-p2p::hw (e1000 NIC + SDR seam)                      │
├──────────────────────────────────────────────────────────────┤
│  KELLER OS microkernel (sovereign-core)                      │
│     city.rs: service registry · SYS_CITY_* dispatch ·        │
│     e1000 TX/RX bridge · SDR HAL seam ·                      │
│     capability tables · keyed-MAC IPC · AES-XTS vault ·      │
│     secp256k1 Schnorr auth · fixed-slot scheduler ·          │
│     ring-3 CR3 isolation · Panic-Zeroing                     │
└──────────────────────────────────────────────────────────────┘
```

**One codebase, two deployment profiles.** The same crates compile:

- **Host profile** — `city-node` binary runs the full stack on a PC for tests
  and demos, mirroring the kernel's `[TEST] name: PASS` boot protocol.
- **Edge profile** — on real city nodes (Raspberry Pi / bare-metal, Phase 4 of
  the KELLER plan), the same crates run as ring-3 services on the KELLER
  microkernel, using KELLER IPC channels and the kernel vault.

## 2. Foundation layer (L0)

| Crate | Role | KELLER alignment |
|---|---|---|
| `city-types` | Shared wire types: `Bytes32` (32-byte payload), `CityMsg`, `GeoTag`, `NodeId`, `CapSlot` | Sized to KELLER's 32-byte IPC payload and 8-channel model |
| `city-crypto` | bigint, EC, AES-XTS vault copied **verbatim** from KELLER; plus `mac` (keyed SipHash-2-4, 64-bit tag) and `osrng` (SipHash counter-mode PRF over 256-bit seeds, plus a host OS-entropy source) | Same secp256k1 field/group, same XTS + Panic-Zeroing |
| `city-caps` | Capability table: slots 0–6 identical to KELLER, 7–16 are City services | O(1) runtime revocation, tag wipe on revoke |
| `city-ipc` | 8 channels, ring buffers of 32-byte messages, per-channel 128-bit key, keyed SipHash-2-4 MAC + anti-replay seq | Keyed-MAC discipline aligned with KELLER IPC (KELLER's own tag is not keyed) |

### Capability slots

| Slot | Name | Owner |
|---|---|---|
| 0–6 | `CAP_SERIAL` … `CAP_IO` | KELLER kernel (identical) |
| 7 | `CAP_CITY_DAG` | DAG ledger + governance ballots |
| 8 | `CAP_CITY_BRP` | identity-less access |
| 9 | `CAP_CITY_DEADDROP` | dead-drop storage |
| 10 | `CAP_CITY_SHARD` | geofenced shard store |
| 11 | `CAP_CITY_ALLOC` | resource allocation |
| 12 | `CAP_CITY_VOTE` | anonymous governance |
| 13 | `CAP_CITY_HEAL` | self-healing / failover |
| 14 | `CAP_CITY_INTEROP` | V2V / drones / logistics |
| 15 | `CAP_CITY_INCENTIVE` | hosting rewards |
| 16 | `CAP_CITY_ENTROPY` | environmental OTP sync |

### IPC frame format

A City message on the wire is a fixed 32-byte body:

```
bytes 0..24   payload (zero-padded)
bytes 24..32  SipHash-2-4 tag (little-endian u64)
```

The tag is `SipHash-2-4(key, sender ‖ type ‖ seq ‖ payload)` with a secret
per-channel 128-bit key — changing any byte, any field, the key, or
replaying any sequence number fails verification. (The older salted
FNV-1a tag was dropped: its structure was invertible, so a single observed
frame leaked the salt and enabled forgery.) Because `city-ipc` and
`city-p2p` share this framing, a P2P gossip frame can be sent verbatim
onto a KELLER-style channel (`P2pNet::broadcast` → `city_ipc::send`).

## 3. District services (L1)

### city-brp — identity-less service access (BRP)

Blind Randomization Protocol:

1. `issue(service_id, expiry_slot, nonce: &[u8; 32]) → 32-byte token`.
   Deterministic per input triple (the nonce is **256 bits**, so the token
   secret space is not brute-forceable), so a service validates without
   storing per-holder state.
2. Holder derives a local blind secret `x = H(token) mod n` (never sent).
3. Schnorr proof: commitment `K = k·G`, challenge `c`, response
   `r = k + c·x mod n`; service checks `r·G == K + c·V`.
4. No identity, no persistent linkage; capability-gated by `CAP_CITY_BRP`.

This is the same zero-knowledge equation Keller Auth uses to prove identity
to the kernel — reused here to prove *token knowledge* to a service.

### city-dag — DAG ledger

- Fixed 64-vertex array, each vertex has ≤ 2 parents (tips).
- Transaction id = four keyed SipHash-2-4 MAC blocks over `(sender, seq,
  block, parents/body)` → 32 bytes, one IPC payload.
- `advance_finality()` finalizes a vertex once ≥ `DAG_FINALIZE_DEPTH` later
  vertices transitively reach it.
- Duplicate ids and unknown parents are rejected at insert.

### city-p2p — gossip transport

- Peer table (8) + verified frame buffer (32).
- `ingest()` verifies MAC and rejects replays.
- `broadcast()` sends frames over the real `city-ipc` channels (not a stub).

### city-shard — localized, geofenced sharding

- A `GeoBox` defines a district cell; a `GeoTag` encodes lat/lon.
- `put(tag, data)` only stores when the tag's district owns it; data is
  AES-XTS encrypted via the Keller Vault; the real vault sector is recorded so
  `get(tag)` reads the right sector.

### city-deaddrop — encrypted async store with TTL

- Payloads stored AES-XTS encrypted in the vault; a sweep wipes expired drops.
- Because storage uses the kernel vault, a panic zeroes drop contents via
  the same Panic-Zeroing path as the kernel.

### city-alloc — autonomous resource allocation

- 4 demand windows (matching KELLER's `SLOT_COUNT = 4`).
- Fixed-point moving-average prediction; scarcity denial acts as
  smart-contract enforcement; grant records are MAC-signed.

## 4. Governance & edge services (L2)

### city-vote — anonymous governance

- Ballot id = MAC over stable fields `(proposal, choice, prev)` — deliberately
  excluding the proof bytes so the proof can be computed *over* the id.
- Ballot proof: Schnorr `r·G == K + c·V` against a public point derived from
  the ballot id. The verifier needs no stored state.
- `VoteTally::cast` rejects forged, tampered, out-of-range, or non-chained
  ballots; `winner()` computes the majority.

### city-heal — self-healing / failover

- Heartbeat table + `offline_ticks` counters.
- Standby nodes do **not** accrue offline time until promoted (otherwise
  failover would be impossible after a prolonged primary outage).
- `failover(role)` promotes the first healthy standby that doesn't own the
  role.

### city-interop — V2V/drone/logistics + traffic orchestration

- `V2vMsg` frames are MAC-verified and kind-validated (V2V / drone /
  logistics).
- `IntersectionScheduler` grants FIFO right-of-way up to a per-tick capacity;
  the queue has a fixed depth (cone-zone limit) — ad-hoc traffic
  orchestration without a central coordinator.

### city-incentive — node hosting rewards

- MAC-signed `UptimeClaim`s; credits = uptime × rate.
- Rejects forged MACs, replays (window ≤ last), and unregistered nodes.

### city-entropy — environmental signature sync

- Sensor readings are **quantized into cells** before hashing, so neighboring
  nodes within one cell derive the *identical* seed → identical 32-byte OTP
  key, without transmitting raw readings.
- The 32-byte key is vault-friendly (one IPC payload / one sector).

## 5. Kernel ABI + hardware bridge (L-A)

### 5.1 city-syscall — the ring-3 ABI

`city-syscall` is the exact constant set the KELLER kernel dispatches. It
mirrors `sovereign-core/src/city.rs` / `caps.rs`:

- Service ids: `CITY_DAG=0` … `CITY_ENTROPY=9` (10 services).
- Capability slots: `CAP_CITY_DAG=7` … `CAP_CITY_ENTROPY=16`.
- Syscall opcodes: `SYS_CITY_BASE=64` … `SYS_CITY_SDR_RX=77` (10 service
  ops + 4 transport ops: NET_TX, NET_RX, SDR_TX, SDR_RX).

The kernel's `syscall_dispatch` routes any opcode in
`SYS_CITY_BASE..SYS_CITY_BASE+31` to `city::city_syscall`, which enforces
per-service capability slots — a ring-3 task can only call its own service
(plus the shared NET/SDR transport), exactly like the driver sandbox.

### 5.2 Kernel-side registry (sovereign-core `city.rs`)

- `SERVICES[10]` — one entry per City service, each with a capability handle,
  a reserved task slot (11–20), a running flag, and a last-opcode audit.
- `city::init()` issues every service its capability at boot.
- `city::revoke(service)` / `city::restore(service)` quarantine / re-issue a
  service in real time (runtime revocation).
- `city::selftest()` — boot test proving issue/revoke per service, per-service
  opcode dispatch, cross-service denial (`SYS_CITY_BRP` with a DAG cap is
  rejected), e1000 TX buffering, and SDR arm/inject/collect.

### 5.3 e1000 NIC bridge

`city-p2p::hw::Nic` is the application-side mirror of the kernel's e1000
MMIO TX/RX descriptor-ring bridge:

- TX: `TDBAL/TDLEN/TDH/TDT` — program a descriptor, kick tail.
- RX: `RAL/RDLEN/RDH/RDT` — arm a receive descriptor.

On the host build the MMIO writes are a safe no-op (a fabricated `0xDEAD0000`
base would fault an unmapped host page); inside the kernel the same registers
are programmed from the PCI-detected BAR0 base. `Transport::Nic` lets a
gossip/V2V frame ride the NIC when present.

### 5.4 SDR HAL seam

`city-p2p::hw::Sdr` is the Phase-4 Shadow-Grid seam: `arm()` gates I/O,
`tx()`/`rx()` are buffer-only until real RF hardware (HackRF One etc.)
binds, and the syscall opcodes `SYS_CITY_SDR_TX/RX` route into the kernel's
`city::sdr_tx/sdr_rx`/`sdr_inject` so the seam is exercisable end-to-end.

## 6. Data flow: a district day

1. **Boot** — KELLER starts, `city-node` wires caps, initializes IPC, and
   seeds the vault.
2. **Ledger** — energy transactions are authored into the DAG and gossiped via
   `city-p2p` over IPC channels.
3. **Access** — a pedestrian device obtains a BRP token; the streetlight node
   validates the Schnorr proof to grant the service (identity-less).
4. **Storage** — district data is sharded to nodes inside the owning `GeoBox`
   and stored encrypted in the local vault; dead-drops carry async messages
   with TTL.
5. **Resources** — demand windows feed `city-alloc`; grants are MAC-signed and
   enforced on scarcity.
6. **Governance** — anonymous ballots chain into the DAG and tally a majority.
7. **Resilience** — heartbeats feed `city-heal`; a silent node's role is
   failed over to a promoted standby.
8. **Interop** — vehicles negotiate intersections through `city-interop`;
   freight messages ride verified V2V/MAC frames.
9. **Incentives** — uptime claims credit energy to hosting nodes; forgeries
   are rejected.
10. **Entropy** — neighboring nodes quantize environmental noise into a shared
    OTP key for link encryption.