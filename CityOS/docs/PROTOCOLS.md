# City OS — Protocol Reference

This document specifies the wire formats, cryptographic protocols, and
service agreements used by City OS. Every protocol below is implemented in
this workspace and enforced by tests.

## 1. Notation

- `‖` — byte concatenation.
- `MAC(k, a, b, c, body)` — **keyed SipHash-2-4** over a fixed field set,
  from `city_crypto::mac`: `MAC = SipHash-2-4(k, a.E ‖ b.E ‖ c.E ‖ body)`
  with a 128-bit secret key `k` and a **64-bit tag**. Deterministic per key;
  changing any byte, any field, or the key changes the tag. Replaces the
  old salted FNV-1a tag, whose structure was invertible (tag forgery
  without the key).
- `K256(x)` — the SipHash counter-mode PRF expansion of a 256-bit seed
  (`city_crypto::osrng::derive_entropy`), producing 32 bytes. Replaces the
  old 32-bit-seeded LCG, whose output space was brute-forceable.
- `G`, `n`, `p` — secp256k1 generator, group order, field prime (reused
  verbatim from KELLER `bigint.rs` / `ec.rs`).
- `x.E` — little-endian encoding of integer `x` (4 bytes unless noted);
  tags are little-endian `u64` (8 bytes).

## 2. Capability slots (city-caps)

| Slot | Name | Used by |
|---|---|---|
| 0..=6 | KELLER slots | kernel (identical) |
| 7 | `CAP_CITY_DAG` | ledger + governance |
| 8 | `CAP_CITY_BRP` | identity-less access |
| 9 | `CAP_CITY_DEADDROP` | encrypted async store |
| 10 | `CAP_CITY_SHARD` | geofenced shards |
| 11 | `CAP_CITY_ALLOC` | resource allocation |
| 12 | `CAP_CITY_VOTE` | governance ballots |
| 13 | `CAP_CITY_HEAL` | self-healing / failover |
| 14 | `CAP_CITY_INTEROP` | V2V / drones / logistics |
| 15 | `CAP_CITY_INCENTIVE` | hosting rewards |
| 16 | `CAP_CITY_ENTROPY` | environmental OTP sync |

Revocation is O(1): clearing the handle's valid bit takes effect at the next
capability check. Tag is wiped to prevent handle-guessing.

## 3. IPC frame (city-ipc)

**Channel** — one of 8 `.Channels[i]`, each with a **secret MAC key**
(host profile: `MacKey::derive(MAC_LABEL ^ (i * 0x9E3779B9))`; kernel
profile: vault-held per-channel keys) and independent send/recv sequence
counters.

**Send** (32-byte wire body):

```
payload_len ≤ 24 bytes are copied into body[0..payload_len]
MAC     = MAC(channel.key, sender, msg_type, next_send_seq, body[0..24])
body[24..32] = MAC tag (8 bytes, little-endian)
next_send_seq += 1
```

**Recv** — verifies `body[24..32] == MAC(key, sender, type, next_recv_seq,
body[0..24])`; on mismatch or seq mismatch the frame is dropped and the tail
advances (channel never wedges).

**Tests.** `roundtrip_verified`, `tampered_mac_is_dropped`,
`replay_seq_is_rejected`.

## 4. BRP — Blind Randomization Protocol (city-brp)

**Token issue** (no identity stored):

```
seed = K256(nonce)                      # nonce is 32 bytes (256 bits)
seed[0..4]  ^= service_id.E
seed[4..8]  ^= expiry_slot.E
token = K256(seed)                      # 32 bytes
```

- `nonce` is a **256-bit** value (host: `city_crypto::fill_os_random`;
  kernel: tick counter + sensor entropy). The old 32-bit nonce space was
  brute-forceable; the full-width nonce is not.
- Deterministic per `(service_id, expiry_slot, nonce)` — the same triple
  reproduces the same token, so a service validates with no per-holder state.

**Blind secret** (never transmitted):

```
x = parse_be256(token) mod n
V = x·G
```

**Proof of token knowledge** (Schnorr):

```
Prover:   k ← fresh scalar
          K = k·G
Verifier: c ← challenge (32 bytes, one-shot)
Prover:   r = (k + c·x) mod n
Verifier: accept iff r·G == K + c·V
```

The verifier learns only `(K, r)`. No identity is linked to the token.

**Tests.** `token_issue_and_verify`, `schnorr_proof_roundtrip`.

## 5. DAG ledger (city-dag)

**Vertex.** `{ id, parents[2], sender, body, seq, finalized_under }`.

**Transaction id** (32 bytes, one IPC payload) — four 64-bit keyed MAC
blocks (`t0..t3`), each covering `(sender, tx seq, block index)` plus one
input slice, so every byte of the vertex contributes to every block:

```
t0 = MAC(DAG_KEY, sender, seq, 0, body)
t1 = MAC(DAG_KEY, sender, seq, 1, parents)
t2 = MAC(DAG_KEY, sender, seq, 2, body)
t3 = MAC(DAG_KEY, sender, seq, 3, parents)
id = t0.E ‖ t1.E ‖ t2.E ‖ t3.E            # 32 bytes
```

**Insertion rules.** Parents must reference known vertices (`0xFF` = none) or
the insert is rejected; duplicate ids are rejected.

**Finality.** A vertex is finalized once ≥ `DAG_FINALIZE_DEPTH` later vertices
transitively reach it (`advance_finality()`).

**Tests.** `author_and_tips`, `unknown_parent_rejected_and_vertices_unique`,
`depth_and_finality`.

## 6. P2P gossip (city-p2p)

**Frame.** `{ seq, kind, from, body[32], mac: u64 }` with

```
mac = MAC(P2P_KEY, from, kind, seq, body)
```

`P2P_KEY` is the district's network key (host profile: label-derived; a
real node: vault-held).

`ingest` validates the MAC and rejects `(from, seq)` replays. `broadcast`
sends the body over the real `city-ipc` transport to each peer's channel.

**Tests.** `peer_and_gossip_roundtrip`, `invalid_mac_is_rejected`.

## 7. Shards (city-shard)

**Geo encoding.** `GeoTag = (lat_u16 << 16) | lon_u16`.

**Ownership.** A `GeoBox` owns a tag iff both coordinates are inside the box.

**Put.** `put(tag, data)` → store AES-XTS in the Keller vault; the actual
vault sector is recorded in the tag→sector map. `get(tag)` reads the recorded
sector and verifies the integrity tag.

**Tests.** `district_ownership_is_geofenced`, `shard_put_get_roundtrip`.

## 8. Dead-drops (city-deaddrop)

- `drop(owner, tag, message, ttl_ticks)` — message stored encrypted in the
  vault (≤ 32 bytes = one sector).
- `pickup(tag, out)` — fire-and-forget: decrypt + remove; tampered sectors are
  purged on read.
- `sweep()` — decrements remaining TTL; expiring drops wipe their vault
  sector.

**Tests.** `drop_pickup_roundtrip`, `ttl_expires_drops`.

## 9. Resource allocation (city-alloc)

- 4 demand windows `(demand, supply)` per district.
- `predict(w)` = integer moving average over the last `w` windows.
- `grant(node, window, share_pct)` yields a MAC-signed `AllocRecord`; under
  projected scarcity (`predict > last supply`) grants are denied
  (smart-contract enforcement).

```
AllocRecord.mac = MAC(ALLOC_KEY, node, 5, window, node‖window‖granted)   # u64
```

**Tests.** `prediction_and_granting`, `scarcity_denies_grants`.

## 10. Anonymous voting (city-vote)

**Ballot id** (stable, excludes proof bytes, 64-bit tag):

```
id = MAC(VOTE_KEY, proposal, 6, prev, proposal‖choice‖prev)
```

**Blind identity.** `V = x·G` with `x = K256(seed) mod n`, where `seed` is
a 256-bit value built from `id ‖ proposal ‖ choice ‖ prev` (all 256 bits
contribute — the old scheme derived from a 32-bit counter).

**Challenge.** `c = K256(seed) mod n` — stateless verifier.

**Known limitation.** The blind secret is derived *from the public ballot*,
so this demo scheme is tamper-evident and chain-anchored but **not sound
against a voter forging a fresh ballot** — the roadmap's zk-SNARK verifier
swap is required for sound anonymous voting.

**Proof.** `K = k·G`, `r = (k + c·x) mod n`; accept iff `r·G == K + c·V`.

**Chain.** `prev` must reference the previous ballot slot; ballots are
DAG-chained so the tally is tamper-evident.

**Tests.** `anonymous_ballot_roundtrip_and_tally`, `forged_ballot_rejected`.

## 11. Incentives (city-incentive)

```
UptimeClaim.mac = MAC(INCENT_KEY, node, 5, window, node‖window‖uptime_ticks)   # u64
```

- `submit` credits `uptime_ticks * INCENT_CREDITS_PER_UPTIME` to the node
  balance.
- Rejected when: MAC invalid, node unregistered, or `window ≤ last_window`
  (replay).

**Tests.** `uptime_claim_rewards_and_rejects_forgery`.

## 12. Interop (city-interop)

**Frame.** `V2vMsg { kind, from, seq, body[32], mac: u64 }` with kinds
V2V=1, DRONE=2, LOGISTICS=3; keyed MAC over `(kind, from, seq, body)`.

**Intersection.** `arrive(vehicle, lane)` enqueues; `grant_next()` grants up
to `capacity` oldest arrivals per tick (FIFO right-of-way);
`clear_passed()` removes granted vehicles.

**Tests.** `v2v_message_mac_and_tamper`, `intersection_gives_fifo_right_of_way`,
`intersection_queue_full_rejects`.

## 13. Entropy sync (city-entropy)

**Quantization.** Each reading is divided by `ENV_CELL_WIDTH` before
hashing:

```
fingerprint = 0x5EED7A7A, then folded with window and the quantized cells
seed32 = window.E ‖ fingerprint.E ‖ (seismic/W).E ‖ (em/W).E ‖ (temp/W).E
OTP key = K256(seed32)                   # 32 bytes
```

Two nodes in the same window with readings in the same quantization cells
derive the **identical** seed and identical 32-byte OTP key (the seed uses
only quantized cells, never raw readings). Readings more than a cell apart
do not sync. Key strength is bounded by the entropy of the quantized
environment readings — inherent to the protocol.

**Rotation.** `derive_otp(shared, counter)` = `K256(shared XOR counter)`
rotates the OTP per window tick from a full 256-bit seed.

**Tests.** `noisy_neighbors_sync_and_share_otp_key`,
`readings_beyond_one_cell_do_not_sync`, `beacon_confirms_and_relays_key`.

## 14. Kernel syscall ABI (Phase 3.6)

The kernel exposes City services to ring-3 tasks via opcodes 64–77. The
`city-syscall` crate mirrors these constants so the application layer stays
in lock-step with `sovereign-core/src/city.rs`.

**Service ids.**

```
CITY_DAG=0  CITY_BRP=1  CITY_DEADDROP=2  CITY_SHARD=3  CITY_ALLOC=4
CITY_VOTE=5 CITY_HEAL=6 CITY_INTEROP=7   CITY_INCENTIVE=8 CITY_ENTROPY=9
```

**Capability slots** (added to the kernel table, 7–16):

```
CAP_CITY_DAG=7  CAP_CITY_BRP=8  CAP_CITY_DEADDROP=9  CAP_CITY_SHARD=10
CAP_CITY_ALLOC=11 CAP_CITY_VOTE=12 CAP_CITY_HEAL=13 CAP_CITY_INTEROP=14
CAP_CITY_INCENTIVE=15 CAP_CITY_ENTROPY=16
```

**Syscall opcodes** (`int 0x80`; `eax=op`, `ebx=cap_handle`, `ecx/edx=args`):

```
SYS_CITY_BASE = 64
SYS_CITY_DAG       = 64  SYS_CITY_BRP       = 65
SYS_CITY_DEADDROP  = 66  SYS_CITY_SHARD     = 67
SYS_CITY_ALLOC     = 68  SYS_CITY_VOTE      = 69
SYS_CITY_HEAL      = 70  SYS_CITY_INTEROP   = 71
SYS_CITY_INCENTIVE = 72  SYS_CITY_ENTROPY   = 73
SYS_CITY_NET_TX    = 74  SYS_CITY_NET_RX    = 75   (shared transport)
SYS_CITY_SDR_TX    = 76  SYS_CITY_SDR_RX    = 77   (shared transport)
```

**Enforcement.** `syscall_dispatch` routes any opcode in
`SYS_CITY_BASE..SYS_CITY_BASE+31` to `city::city_syscall`, which requires the
caller's `cap_handle` to resolve to the opcode's service slot. The shared
`NET_*`/`SDR_*` transport opcodes accept any valid City capability.

**Returns.** `0` success; `−1` `ERR_CAP_REVOKED` (wrong/revoked slot); `−2`
bad opcode; `−3` bad payload.

**Tests.** `city-syscall::abi_mirrors_kernel` (host); kernel boot test
`city_services` (issue/revoke per service, dispatch each opcode,
cross-service denial, e1000 TX buffering, SDR arm/inject/collect).

## 15. Hardware transport bridge (city-p2p::hw)

`Nic` mirrors the kernel e1000 MMIO descriptor-ring bridge:

```
TX: TDBAL(0x3800) TDLEN(0x3808) TDH(0x3810) TDT(0x3818)
RX: RAL(0x5400)   RDLEN(0x2808) RDH(0x2810) RDT(0x2818)
```

Host builds run a safe no-op (the kernel programs real registers from the
PCI-detected BAR0 base). `Sdr` is the Phase-4 RF-seam: `arm()` gates I/O,
`tx()`/`rx()` buffer until real hardware binds; `Transport::{Nic,Sdr}`
dispatches both behind one frame API.

**Tests.** `nic_absent_by_default_on_host`, `sdr_arm_gates_io`,
`transport_dispatch_is_safe`.

## 16. District simulation (city-sim)

A full-mesh multi-node harness: `Simulation::new(nodes, rounds)` runs every
node's `city-dag` + `city-p2p` for `rounds` rounds. Each round every node
authors a transaction, records it locally, and fans a MAC-verified frame
out to all peers; peers verify (MAC + replay) before chaining the body into
their own ledger.

**Convergence property.** After `rounds` rounds every node's ledger holds
every authored body exactly once (`converged(nodes × rounds)`), and
finality advances. Replayed or tampered frames never enter any ledger.

**Scope.** Per-node ledgers replicate identical *contents*; there is no
shared global DAG identity or Byzantine consensus (kernel work, see
`docs/ROADMAP.md`).

**Tests.** `gossip_converges_across_nodes`, `replayed_and_tampered_frames_
never_enter_the_ledger`, `district_capacity_is_bounded`.
