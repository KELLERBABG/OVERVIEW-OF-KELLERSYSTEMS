# City OS — Security Model

City OS inherits its security guarantees from KELLER OS and layers City
services on top of them. This document is the threat-model and security
reference for the City workspace.

## 1. Trust root: KELLER OS

All City crates are `no_std`, allocation-free, and capability-gated because
the kernel enforces:

| KELLER OS mechanism | What it gives City OS |
|---|---|
| Capability table with O(1) runtime revocation | A City service can be disabled in real time by revoking its cap handle |
| Keyed-MAC IPC channels with anti-replay seqs | No forged, replayed, or out-of-order City messages can be delivered (SipHash-2-4, 64-bit tag, per-channel 128-bit key) |
| AES-XTS vault with integrity tags + Panic-Zeroing | Shards, dead-drop payloads, and OTP keys at rest are ciphertext; a panic wipes them atomically |
| 256-bit secp256k1 Schnorr ZK identification | The same ZK equation backs BRP and anonymous voting proofs |
| Ring-3 CR3 isolation + IOMMU detection | City service code cannot touch kernel memory or other services' page tables |
| Temporal isolation (TSC slot budgets) | Scheduler timing is budgeted, so a resource-allocator's ticks are deterministic |

**Assumption.** The kernel is trusted; City services are untrusted
applications that can only act through capabilities and verified IPC.

## 2. Threats and mitigations

| # | Threat | Mitigation |
|---|---|---|
| T1 | A forged/replayed IPC message | Every frame carries a per-channel-key SipHash-2-4 MAC over `(sender, type, seq, payload)` (64-bit tag, 128-bit key); `recv` drops on any mismatch (tests: `tampered_mac_is_dropped`, `replay_seq_is_rejected`, `wrong_key_fails_but_does_not_leak`) |
| T2 | Revoked service keeps acting | Capabilities are re-checked at every dispatch; revocation is O(1) (test: `issue_check_revoke_default`) |
| T3 | Stolen vault / dead-drop / shard contents | AES-XTS encryption at rest + integrity tag; tampered sector → `retrieve` fails (test: `keller_vault_xts_roundtrip_and_tamper`) |
| T4 | A node's identity is linkable to its service use | BRP issues blinded 32-byte tokens from **256-bit nonces**; service only sees a Schnorr commitment/response, never the secret. The nonce space is full-width, so token secrets are not brute-forceable (tests: `token_issue_and_verify`, `schnorr_proof_roundtrip`) |
| T5 | Tampered governance ballots | Ballot id is a keyed MAC over `(proposal, choice, prev)`; `cast` verifies the Schnorr proof and chaining, so tampering a ballot after proof fails. **Known limitation:** the blind secret is derived from the public ballot id, so a voter can forge a *fresh* ballot for their own proposal — sound anonymous voting needs the planned zk-SNARK verifier (test: `forged_ballot_rejected`) |
| T6 | Double-spend / duplicate transactions | DAG rejects duplicate tx ids; every vertex's id includes a per-insert sequence (test: `unknown_parent_rejected_and_vertices_unique`) |
| T7 | Geofence violation (data leaves district) | `city-shard` only writes a tag whose district `GeoBox` contains it (test: `district_ownership_is_geofenced`) |
| T8 | Fake uptime / forged incentive claims | `UptimeClaim` carries a keyed SipHash-2-4 MAC; ledger rejects forged/replayed/out-of-order windows (test: `uptime_claim_rewards_and_rejects_forgery`) |
| T9 | Forged V2V/interop frames | `V2vMsg.signature_ok()` validates kind + MAC (test: `v2v_message_mac_and_tamper`) |
| T10 | Node health spoofing | `city-heal` counts `offline_ticks` from verified heartbeats; standbys don't decay until promoted (tests: `failover_promotes_standby`, `heartbeat_revives_node`) |
| T11 | OTP key disagreement between honest nodes | `city-entropy` quantizes sensor readings so in-cell neighbors derive identical keys; out-of-cell readings don't sync (tests: `noisy_neighbors_sync_and_share_otp_key`, `readings_beyond_one_cell_do_not_sync`) |
| T12 | Secret persistence after a panic | Every vault-backed store (shards, dead-drops) uses KELLER's vault, whose Panic-Zeroing wipes all sectors + key schedules |
| T13 | Timing leaks in payment/governance | Resource allocator uses fixed 4-window slots matching KELLER's temporal isolation; no heap, no variable-length allocation anywhere |
| T14 | A City ring-3 task calls another service's opcode | `city::city_syscall` validates the caller's capability slot per opcode; cross-service calls are denied (kernel test: `city_services` step 4) |
| T15 | A revoked City service keeps acting | `city::revoke(service)` clears the handle → next dispatch returns `ERR_CAP_REVOKED`; per-service quarantine/restore |
| T16 | Fabricated e1000/SDR transport frames | The transport ops (`SYS_CITY_NET_*`, `SYS_CITY_SDR_*`) are only reachable with a valid City capability; on-wire framing still carries the P2P MAC |
| T17 | Unmapped-MMIO fault when probing hardware on host | `city-p2p::hw::Nic::wr` is a host-side programming mirror (no volatile write); real MMIO writes happen only inside the kernel from a PCI-detected BAR0 base |

## 3. Threat model boundaries

- **In scope.** Application-layer forgery, replay, tamper, linkability,
  geo-escape, and revocation-evasion of *City service traffic* running on a
  trusted KELLER kernel.
- **Out of scope in this workspace.** Physical attacks on the node (should
  rely on KELLER's cold-boot defense + IOMMU work), RF-layer attacks on the
  SDR Shadow Grid (Phase 4 hardware), social/municipal attacks, and
  compromised kernel (the trust root).

## 4. What the test suite proves

Every security claim above is enforced by a unit test in the workspace.
Current status: **52/52 passing** (`cargo test --workspace --release`).

| Crate | Security tests |
|---|---|
| `city-crypto` | SipHash-2-4 reference vectors; keyed MAC field/key sensitivity (wrong key fails); oversized-payload byte sensitivity; 256-bit PRF expansion; vault XTS tamper detection; bigint/EC selftests; Schnorr response mod-n |
| `city-sim` | multi-node gossip convergence; replay/tamper rejection across a district; bounded capacity |
| `city-caps` | issue/check/revoke; wipe |
| `city-ipc` | verified roundtrip; tampered MAC dropped; replayed seq rejected |
| `city-brp` | token verify/revoke; Schnorr proof roundtrip + attacker rejection |
| `city-dag` | unknown parent rejected; vertex uniqueness |
| `city-p2p` | invalid MAC rejected; replay rejected |
| `city-p2p::hw` | NIC absent-safe on host; MMIO no-op mirror; SDR arm-gates-I/O; transport dispatch safe |
| `city-shard` | geofence ownership; encrypted roundtrip |
| `city-deaddrop` | TTL expiry wipes vault sectors |
| `city-vote` | forged/tampered ballot rejection |
| `city-incentive` | forged claim rejection |
| `city-interop` | V2V MAC tamper rejection |
| `city-heal` | failover promotion correctness |
| `city-syscall` | ABI mirrors kernel (opcode range 64–77, slots 7–16) |

## 5. Known limitations (honest scope)

- **Key management.** The host profile derives MAC keys from public domain
  labels (`MacKey::derive`) so tests/demos are deterministic. That is *not*
  secret-key material: a production node must hand services vault-held
  random keys at boot (the API is shaped for exactly this).
- **Voting soundness.** As noted in T5, the demo ballot scheme is
  tamper-evident but not resistant to a voter forging fresh ballots; the
  roadmap's zk-SNARK verifier swap is the fix.
- **Entropy.** The host OS-entropy source (`fill_os_random`) uses std's
  OS-seeded hashers — adequate for a demo; the kernel profile must seed
  from the tick counter plus sensor entropy. Environment-derived OTP keys
  are bounded by the entropy of the quantized readings.
- **No Byzantine consensus.** The district simulation proves verified
  gossip converges to identical ledger *contents*; a shared global DAG
  with adversarial consensus is kernel work, not in this workspace.
- **Vault integrity tag.** The Keller Vault's per-sector integrity tag
  (copied verbatim from KELLER) is an FNV-1a checksum, not a keyed MAC.
  At rest this is acceptable because the AES-XTS ciphertext dominates and
  random tampering breaks the XTS decrypt; a deliberate RAM attacker who
  can rewrite ciphertext *and* recompute the tag is out of this layer's
  threat model (KELLER's cold-boot defense covers the stolen-RAM case).

## 6. Recommended deployment checklist

1. Boot KELLER with the included self-tests green (`run-tests.ps1`).
2. Verify NO allocation: run KELLER's `check_no_alloc.ps1` over `crates/`.
3. Grant each City service the **minimum** capability slot it needs.
4. Monitor cap table `live_count()` + `city-heal` heartbeats on the node
   dashboard.
5. Rotate OTP keys each temporal window (`city-entropy::derive_otp`).
6. Configure dead-drop TTLs so drops expire before they are ever read at
   rest for longer than policy allows.