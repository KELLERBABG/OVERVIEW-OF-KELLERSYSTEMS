# City OS

A decentralized urban operating system: DAG-based ledger, edge computing, identity-less service access, localized data sharding, dead-drop infrastructure, and autonomous resource allocation — built **on top of [KELLER OS](../sovereign-core/README.md)**.

## Relationship to KELLER OS

> KELLER OS IS City OS Phase 1/2. City-specific services are applications that run ON TOP of that kernel. One codebase, two deployment profiles.

This workspace (`City OS/`) is the **City OS application layer**. It reuses KELLER OS's verified foundation directly:

| KELLER OS (sovereign-core) | City OS reuse |
|---|---|
| `bigint.rs` / `ec.rs` (secp256k1 256-bit) | `city-crypto` verbatim |
| `aes.rs` (AES-128 + XTS) | `city-crypto` verbatim |
| `vault.rs` (AES-XTS RAM secret store, Panic-Zeroing) | `city-crypto` verbatim |
| `caps.rs` (capability table, O(1) runtime revocation) | `city-caps` (slots 0–6 identical, 7–16 City services) |
| `ipc.rs` (salted-MAC channels, anti-replay) | `city-ipc` (keyed SipHash-2-4 MAC + seq checking) |
| `auth.rs` (Schnorr ZK identification) | `city-brp` (same equation, applied to blind tokens) |
| `temporal.rs` (TSC slot budgets) | `city-alloc` (4 fixed slots, demand windows) |
| `selftest.rs` (`[TEST] name: PASS` protocol) | `city-node` (host demo prints the same protocol) |

## Workspace layout

```
Cargo.toml                 # workspace root
crates/
  city-types/              # shared wire types (Bytes32, CityMsg, GeoTag)
  city-crypto/             # KELLER bigint/ec/aes/vault + City MAC/osrng
  city-caps/               # capability table (KELLER-aligned + City slots)
  city-ipc/                # keyed-MAC IPC channels w/ anti-replay
  city-brp/                # identity-less service access (Schnorr on tokens)
  city-dag/                # DAG ledger (no-alloc, finality)
  city-p2p/                # fixed-slot gossip, verified frames
  city-shard/              # geofenced localized shard distribution
  city-deaddrop/           # TTL encrypted async message store
  city-alloc/              # autonomous resource allocation (energy/water)
  city-vote/               # anonymous P2P governance voting (ZK ballots)
  city-heal/               # self-healing / failover
  city-interop/            # V2V/drone/logistics + traffic orchestration
  city-incentive/          # proof-of-uptime / energy credits
  city-entropy/            # environmental signature sync → OTP
  city-syscall/            # SYS_CITY_* / CAP_CITY_* kernel ABI mirror
  city-sim/                # multi-node district simulation (gossip convergence)
  city-node/               # host demo binary: full service stack boot
```

## Build & test

```bash
cargo build --workspace
cargo test  --workspace        # all service unit tests
cargo run   -p city-node        # host demo: [HARNESS] CITY OS BOOT: PASS
```

## Documentation

| Document | Contents |
|---|---|
| [`WHITEPAPER.md`](WHITEPAPER.md) | System design, security model, verification evidence, roadmap |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | Layered architecture + district day data flow |
| [`docs/SECURITY.md`](docs/SECURITY.md) | Threat model, mitigations, test-to-claim mapping |
| [`docs/PROTOCOLS.md`](docs/PROTOCOLS.md) | Wire formats + cryptographic protocols (byte-level) |
| [`docs/ROADMAP.md`](docs/ROADMAP.md) | What is done, what is next (KELLER ring-3 wiring), deferred hardware |

## Service contracts (all no_std, no-alloc)

- **city-caps** — `CAP_SERIAL..=CAP_IO` match KELLER exactly; City adds
  slots 7–16 (`CAP_CITY_DAG`/`BRP`/`DEADDROP`/`SHARD`/`ALLOC`/`VOTE`/
  `HEAL`/`INTEROP`/`INCENTIVE`/`ENTROPY`).
- **city-syscall** — the `SYS_CITY_*` ring-3 ABI (opcodes 64–77) mirroring
  the KELLER kernel's `city.rs`, so host services call the same opcodes the
  kernel dispatches.
- **city-ipc** — 8 channels, per-channel 128-bit keys, keyed SipHash-2-4
  MAC (64-bit tag) + seq anti-replay; a 32-byte frame = 24-byte payload +
  8-byte tag, fitting one KELLER message.
- **city-brp** — `issue(service_id, expiry_slot, nonce: &[u8; 32]) -> 32-byte
  token` (256-bit nonce → full-width secret space); prover shows knowledge of
  the token secret via `r = k + c·x mod n` and the verifier checks
  `r·G == K + c·V` (identical to Keller Auth).
- **city-dag** — fixed 64-vertex ledger, 2 parents, tx ids built from four
  keyed-MAC blocks (32 bytes, one IPC payload), `advance_finality()`
  finalizes vertices covered by `DAG_FINALIZE_DEPTH`.
- **city-p2p** — peer table (8) + frame buffer (32), keyed-MAC-verified
  ingest, replay rejection, `broadcast()` sends over the real `city-ipc`
  channels (KELLER 8-channel ABI; e1000 NIC swap planned in the roadmap).
- **city-sim** — full-mesh multi-node harness proving gossip convergence:
  after `rounds` rounds every node's ledger holds every authored body
  exactly once, and replayed/tampered frames never enter any ledger.
- **city-shard** — `GeoBox` geofenced districts; `put(tag, data)` stores an
  AES-XTS vault sector only for tags owned by a district.
- **city-deaddrop** — vault-encrypted messages, TTL in ticks, `sweep()`
  wipes expired drops.
- **city-alloc** — 4-window demand history, moving-average prediction,
  scarcity denial (smart-contract enforcement), keyed-MAC-signed grant
  records.
- **city-vote** — anonymous governance: ZK-validated, DAG-chained ballots;
  unlinkable to any node, tamper-evident, majority winner.
- **city-heal** — self-healing: heartbeat table, offline detection, standby
  promotion (failover) for district roles.
- **city-interop** — verified V2V/drone/logistics frames + FIFO intersection
  right-of-way scheduling (ad-hoc traffic orchestration).
- **city-incentive** — proof-of-uptime, MAC-signed claims, energy credit
  ledger with replay/forgery rejection.
- **city-entropy** — environmental signature sync: noisy neighboring nodes
  derive a shared 32-byte OTP key without transmitting raw readings.

## Kernel integration status

The KELLER kernel side of Phase 3.6 is implemented: `SYS_CITY_*` opcodes
(64–77) route to `city::city_syscall`, every City service gets a `CAP_CITY_*`
slot (7–16) and a registry entry in `SERVICES[10]`, and the boot selftest
(`city_services`) proves per-service dispatch, cross-service denial, e1000 TX
buffering, and the SDR seam. `city-p2p::hw` mirrors the e1000 NIC MMIO
descriptor ring and provides the SDR Shadow-Grid HAL seam (`Transport`).

## Next steps

1. Spawn the ten City services as real ring-3 tasks via
   `scheduler::start_task` (slots 11–20 are reserved in the registry), then
   verify both layers on one QEMU `[TEST]` log.
2. On hardware: bind the kernel e1000 ring to packets on the NIC and bind the
   SDR seam to an RF device (HackRF One, Phase 4.4).
3. Deploy district demo on 4× Raspberry Pi running KELLER OS (Phase 4.6).
