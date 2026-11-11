# FHE-DB Internal Audit Package

**Status:** internal review complete **2026-11-10**. This is the stage 1.1–1.2
deliverable that gates the external cryptographic review. It is **not** an
independent audit, and it does not make the project production-ready.

**Repository state:** commit `3408d55` plus the documentation/claim
corrections, the F1/F2/F11 fixes, and the B4/C5/C6 observability and
audit-readiness work in this worktree. Commands below were run on this
checkout; results are recorded in [`PROJECT_STATUS.md`](../../PROJECT_STATUS.md)
§1.

## What this package contains

| Document | Question it answers |
|---|---|
| [`THREAT_MODEL.md`](THREAT_MODEL.md) | What exactly can the server, disk, network, and local attackers observe, and what remains secret? |
| [`PARAMETERS.md`](PARAMETERS.md) | What do the pinned TFHE parameters actually guarantee, and which security claims are vendor claims rather than proofs? |
| [`KEY_LIFECYCLE.md`](KEY_LIFECYCLE.md) | How are keys generated, stored, backed up, restored, rotated, and destroyed — and where does the implementation diverge from the documented guarantees? |
| [`CLAIMS.md`](CLAIMS.md) | Which README/SECURITY/PROJECT_STATUS claims are supported, qualified, or unsupported, with evidence? |
| [`EVIDENCE.md`](EVIDENCE.md) | What must an independent reviewer be able to verify, with which commands, hashes, and file references? |
| [`ENGAGEMENT.md`](ENGAGEMENT.md) | What exactly is the audit scope, what is out of scope, and what do we ask bidders? |
| [`FORMAT_FREEZE.md`](FORMAT_FREEZE.md) | Which formats and parameters are frozen during the review, and how is a deliberate change made? |
| [`PRODUCT_MODEL.md`](PRODUCT_MODEL.md) | Which product model is supported, what does F9 acceptance mean, and what would multi-tenant use require? |

`SECURITY.md` is the short public posture; it now defers to this package for
the boundary and residual risks.

## Method

1. Trace the actual code paths in `crates/fhe-core`, `crates/fhe-db`,
   `crates/fhe-server`, `crates/fhe-client`, and `apps/fhe-phone`.
2. Prefer executable evidence over prose: run the workspace quality gate and
   probe the real server for the claims that could be tested (see
   [Executed evidence](#executed-evidence)).
3. Treat every README/SECURITY/PROJECT_STATUS claim as unproven until the code,
   a test, or an observed run supports it.
4. Record limitations explicitly. Code reading is not an external audit;
   absence of a finding is not proof of security.

## Audit readiness (C5/C6)

- **Engagement package:** [`ENGAGEMENT.md`](ENGAGEMENT.md) defines the scope of
  work, explicit out-of-scope items, required deliverables, a request-for-quote
  template, a suggested outreach set, and the parallel-work rules.
- **Evidence pack:** [`EVIDENCE.md`](EVIDENCE.md) records the artifact hashes,
  the re-runnable verification that the vendored TFHE-rs 1.7.0 source is
  unmodified, parameter provenance, wire/key formats, reproduction commands, and
  a claim→evidence index.
- **Freeze:** [`FORMAT_FREEZE.md`](FORMAT_FREEZE.md) freezes the envelope v1
  frame, the WAL v1 header/record frame, the golden corpus, and the parameter
  set, enforced by `crates/fhe-core/tests/format_freeze.rs` and
  `crates/fhe-db/tests/format_freeze.rs`.
- **Product model:** [`PRODUCT_MODEL.md`](PRODUCT_MODEL.md) records the
  single-user local-vault decision, the conditions under which F9 is accepted,
  and the requirements for any future multi-tenant deployment.
- **Not done:** the package has **not been sent**, no reviewer has been engaged,
  and no quote has been received. That is the owner's action; the external audit
  remains **Blocked**.

## Findings

Severity is for release planning, not a formal CVSS score. **F1, F2, F4, F8,
and F11 are fixed** in this worktree and pinned by regression tests plus live
probes. **F9 is accepted** under the documented single-user local-vault model
([`PRODUCT_MODEL.md`](PRODUCT_MODEL.md)); it reopens as a blocker for any
remote or multi-user deployment. F3, F5, F6, F7, and F10 remain **open**. These
fixes do not replace the external audit.

| ID | Finding | Status | Severity | Primary evidence |
|---|---|---|---|---|
| **F1** | The rate limiter was keyed by peer `SocketAddr` (IP **and** ephemeral port), not by IP. New TCP connections reset the bucket, so the documented per-IP quota was bypassable. | **Fixed** | High (availability) | `crates/fhe-server/src/routes.rs:239-317`; regression `rate_limiter_shares_one_bucket_per_ip_across_connections`; live probe: 125 separate connections → 120 handled, 5 `429`. |
| **F2** | WAL records were appended **before** database validation. A rejected mutation (duplicate table, unknown column, width mismatch) left a record that made replay fail on the next boot. | **Fixed** | High (availability / integrity) | `crates/fhe-server/src/routes.rs:662-809`; `crates/fhe-db/src/wal.rs:83-136`, `:595-655`; regression `rejected_mutations_do_not_poison_the_wal`; live restart probe recovered the committed table after rejected mutations. |
| **F3** | The XNOR model's bias is a plaintext `i64` array persisted and used by the server; weights are encrypted, but the model is not fully hidden. | Open | Medium (confidentiality) | `crates/fhe-server/src/routes.rs:940-980`, `:996-1030`; `crates/fhe-core/src/nn/layers.rs:8-10`. |
| **F4** | The phone PIN keystore permitted 4-character PINs, used unpinned `Argon2::default()`, wrote `pin.json`/`keys.json` non-atomically, had no attempt limiter or zeroization, and had no wired destruction action. | **Fixed** 2026-11-11 | High for the phone product | `apps/fhe-phone/src/keystore.rs`: single-file vault v2, `KdfParams::PINNED`, `check_pin_at` lockout, `Zeroizing` buffers, `destroy`; tests `kdf_parameters_are_pinned_and_recorded`, `failures_are_persisted_and_lock_out_then_recover`, `destroy_removes_local_key_material`. Residuals: offline guessing and no TFHE-rs key zeroization (see `KEY_LIFECYCLE.md`). |
| **F5** | The TFHE 2M128 parameter set is pinned and tested for value drift, but its 128-bit IND-CPA^D label is a TFHE-rs/Lattice-Estimator claim. This repository does not reproduce the estimate or hold an independent audit. | Open | High (release blocker) | `crates/fhe-core/src/keys.rs:148-161`, `:289-345`; `vendor/tfhe/README.md:198-208`; `vendor/tfhe/src/shortint/parameters/v1_4/classic/tuniform/p_fail_2_minus_128/ks_pbs.rs:29-43`. |
| **F6** | The server observes schema, row counts, batch sizes, access patterns, ciphertext widths/sizes, timing, peer address, and status codes. Values stay encrypted, but workload metadata does not. Per-request tracing is installed but the CLI's default filter emits no access log. | Open | Medium (confidentiality) | `crates/fhe-server/src/routes.rs:405-431`, `:685-687`, `:798-806`, `:1055-1075`; `crates/fhe-server/src/main.rs:78-82`; `crates/fhe-server/src/lib.rs:32-40`. |
| **F7** | TFHE-rs documents that side-channel mitigations are not implemented; this build adds no constant-time key handling, memory locking, or zeroization. | Open | High (release blocker) | `vendor/tfhe/README.md:215-219`; no project-level zeroization use (only transitive in `Cargo.lock`). |
| **F8** | Keygen persists the key pair before uploading the server key, and a PIN change wrote the new PIN hash before re-encrypting keys. A failure between steps left a recoverable-but-confusing state (keygen) or an unrecoverable lockout (PIN change). | **Fixed** 2026-11-11 | Medium (availability) | `KeyStore::change_pin` verifies and re-encrypts in one atomic vault write; `import_backup` and `save_keys` use the same commit; `pin_change_is_atomic_and_preserves_keys` injects a failed commit and proves the old vault still decrypts. Keygen order retained deliberately (see the F4/F8 section). |
| **F9** | There is no authorization/tenancy layer; auth is one shared bearer token, disabled by default, and `app()` enables permissive CORS. Loopback is not a boundary between local users/processes. | **Accepted** 2026-11-11 — single-user local vault only | Medium (deployment) | [`PRODUCT_MODEL.md`](PRODUCT_MODEL.md) records the decision, deployment conditions, and reopen triggers; `crates/fhe-server/src/routes.rs` `bearer_auth`, `app()`; `SECURITY.md` §4. Not fixed; out of scope for the local product. |
| **F10** | The phone's Monitor wire tap records only matched-route method/path/status, is lossy under load, and records no bodies or sizes; it is not proof of what the server handled. | Open | Low (claim accuracy) | `apps/fhe-phone/src/server.rs:58-61`, `:120-133`; `apps/fhe-phone/src/wire.rs:20-28`. |
| **F11** | The periodic checkpoint task acquired `wal.write` and then `db.read`, while every mutation handler acquired `db.write` and then `wal.write`. The inverted order allowed a concurrent checkpoint and mutation to deadlock the durability path. | **Fixed** | High (availability) | `crates/fhe-server/src/routes.rs` `AppState::checkpoint_wal`; `crates/fhe-server/src/main.rs` periodic/shutdown callers; regression `checkpoint_acquires_database_before_wal`; live 2,100-mutation stress probe at 1s checkpoints. |

### Finding F1 — per-IP rate limiting (fixed)

The original limiter stored buckets in `HashMap<SocketAddr, RateBucket>` and
read `ConnectInfo<SocketAddr>`. Because a `SocketAddr` includes the source
port, each new TCP connection was a new principal. A probe on the real binary
(no server key, so the expensive handler returned `409 NO_MODEL` before doing
work) showed 130 new connections were all admitted, while a reused connection
hit `429` after the 120-request quota.

The fix keys `RateLimitState` on `IpAddr` (`routes.rs:239-317`) and the
middleware takes `ConnectInfo<SocketAddr>::ip()`. The regression
`rate_limiter_shares_one_bucket_per_ip_across_connections` drives three
requests from one IP on different ports. A fresh live probe against the release
binary sent 125 requests on 125 separate connections: **120 returned `409`, the
remaining 5 returned `429`**, matching the documented 120/min expensive quota.

Residual: `ConnectInfo` reports the direct peer. Behind a reverse proxy every
client shares the proxy IP, and NAT-ed clients share a bucket. A trusted
forwarded-for policy is deployment work, not part of this fix.

### Finding F2 — WAL poisoning by rejected mutations (fixed)

The original handlers appended the WAL record first, then called
`db.create_table`, `db.add_column`, or `db.insert_ciphertexts`. A validation
error after the append left the record in the log; replay mapped the later
apply error to `CorruptedDatabase`, so the next boot refused to start.

Reproduced before the fix with the real server and the committed golden
envelope: `400 WIDTH_MISMATCH`, WAL growth from 78 bytes to 263,568 bytes, then
a forced restart exiting 1 at `log replay failed at record 1`.

The fix is to validate each record against the locked database **before**
`Wal::append`: `WalRecord::validate` (`crates/fhe-db/src/wal.rs:83-136`) mirrors
the table/column/width checks, and all three server mutations plus the
`WalBackedDatabase` wrappers use it (`routes.rs:662-809`). The regression
`rejected_mutations_do_not_poison_the_wal` commits a table and a `U16` row,
then rejects a duplicate table, duplicate column, and `U32` insert while
asserting the WAL size never grows; a restart recovers the table and decrypts
the committed value `7`.

Live post-fix probes: duplicate table and duplicate column both returned their
existing `409` codes (`TABLE_EXISTS`, `COLUMN_EXISTS`), and the golden `U32`
envelope into a `U16` column returned `400 WIDTH_MISMATCH`. In both probes the
WAL stayed at 78 bytes, and a force-killed restart served the table with exit
success.

Corruption handling is unchanged: a complete record with a bad CRC or
undecodable payload is still a fatal `CorruptedDatabase`, and a short torn tail
is still healed by truncation, pinned by `corrupt_complete_record_is_rejected`
and `torn_tail_is_truncated`.

Upgrade limitation: a log already poisoned by a **pre-fix** build still fails
replay. The fix prevents new rejected records but deliberately does not skip
logical failures during replay, because a skipped record could hide genuine
corruption or committed data. Recovering such an old log is an operator
runbook task (restore the last good copy or remove the bad tail deliberately);
the procedure is in [`docs/operations/WAL_RECOVERY.md`](../operations/WAL_RECOVERY.md).

### Finding F11 — checkpoint lock-order inversion (fixed)

The periodic checkpoint task took `wal.write` and then `db.read` in
`main.rs`, while `create_table`, `add_column`, and `insert_rows` took
`db.write` and then `wal.write`. If a mutation acquired the database lock
while the checkpoint held the WAL lock, each waited for the other: the server
stopped making progress on the durability path.

The fix centralizes checkpointing in `AppState::checkpoint_wal`, which acquires
`db.read` **before** `wal.write`. Both the periodic task and graceful shutdown
call that helper, so there is only one lock order to audit: **database before
WAL, never WAL while waiting for the database**. The same order is documented
on the helper, on the `wal` field, and in the periodic-task comment.

`checkpoint_acquires_database_before_wal` holds `db.write`, starts a checkpoint,
and asserts the WAL is still free while the checkpoint waits; the old order
would hold `wal.write` and fail that assertion. The live probe drove **2,100
successful mutations over three rounds**, twelve clients in parallel, with a
**one-second** checkpoint interval. Health returned `200` after every round,
all 2,100 mutations returned `200`, and the final WAL contained a 169,464-byte
snapshot with **zero record bytes**, confirming the checkpoints ran and
compacted the log.

### Finding F3 — XNOR bias is plaintext

`InstallXnorRequest.bias` is `Vec<i64>`; `install_xnor_model` stores
`bias: req.bias.clone()`; `infer_xnor` passes it to `DenseLayer`. The library
documentation is honest about this (`layers.rs:8-10`), but the server handler doc
and the old "server never sees the model" phrasing were not. The claims are now
corrected. Weight values remain encrypted.

### Finding F4/F8 — phone key lifecycle (fixed 2026-11-11)

The original store had real primitives (AES-256-GCM with a fresh salt and
nonce; Argon2id PIN hash) but weak lifecycle guarantees: no KDF pinning, a
4-character minimum, no attempt limiter, two non-atomic files, and no wired
destruction. PIN change committed the new PIN hash before re-encrypting the
keys, so a crash between those writes was an unrecoverable lockout.

The store is now a **single atomic `vault.json` (format v2)** containing the
pinned KDF parameters, the PHC PIN hash, the encrypted key pair, and the
unlock-failure state. Every commit writes `vault.json.tmp`, `sync_all`s it, and
renames it over `vault.json`. `change_pin` verifies the old PIN and writes the
new hash plus re-encrypted keys **in one commit**; `save_keys` and
`import_backup` use the same path. `pin_change_is_atomic_and_preserves_keys`
forces the commit to fail and proves the old vault still decrypts.

Other F4 changes:

- **Pinned KDF.** `KdfParams::PINNED` (Argon2id, 19 MiB, t=2, p=1, 32-byte
  output, v0x13) is recorded in the vault and used for every new derivation.
  `kdf_parameters_are_pinned_and_recorded` pins the on-disk values; a second
  test proves the recorded v1 parameters reproduce `Argon2::default()` so the
  one-time migration stays decryptable.
- **PIN policy matches the keypad.** 6–12 ASCII digits, enforced in the
  keystore and reflected in the Settings placeholders. The lock screen still
  accepts a legacy 4-digit PIN to unlock, but new/changed PINs must meet the
  floor.
- **Persisted lockout.** Five free attempts, then 30 s doubling to a 1 h cap,
  stored in the vault and enforced before the KDF runs. This bounds **online**
  guessing at the lock screen; it does not stop offline guessing from a copied
  vault.
- **Zeroization.** Derived AES keys, plaintext key buffers, and the in-memory
  PIN use `Zeroizing`. TFHE-rs key types do not implement zeroization, so the
  decrypted `ClientKey` is dropped but not explicitly wiped; that vendor limit
  is documented rather than overclaimed.
- **Destruction is wired.** `vault.destroy {pin}` (Settings → Security →
  Destroy keys) authenticates, erases the vault atomically, and clears the
  in-memory key and PIN. Existing ciphertexts become undecryptable; exported
  backups and OS copies are explicitly not deleted.
- **Legacy migration.** A v1 `pin.json` + `keys.json` pair is combined into the
  v2 vault and removed on first open. The v1 JSON payload layout is recorded as
  `payload_format: json_v1`, so old ciphertexts stay decryptable; v1 backups
  are also accepted by `import_backup`.
- **Keygen no longer overwrites a stored key.** `keygen` refuses when the vault
  already holds keys (for example while locked) instead of generating and
  saving a new pair over the existing one. The original ordering — save, then
  upload the public server key — is retained deliberately: a failed upload
  leaves the secret key safely stored, and the next unlock re-uploads the
  server key. That is recoverable and involves no key loss.

Residual risks (not fixed by this change, documented in `SECURITY.md`): an
offline attacker with a copy of the vault can still guess the PIN, limited by
Argon2id cost and the 6-digit floor; no memory locking; and TFHE-rs does not
expose key zeroization. The GUI was not launched in this pass, though the IPC
paths are covered by the headless engine flow.

## Executed evidence

All commands below were run in this checkout; this is a record, not a claim
that every possible surface was exercised.

| Probe | Result |
|---|---|
| `cargo build --release --workspace` | PASS (exit 0; 1m27s incremental) |
| `cargo test --release --workspace` | PASS: 119 passed, 0 failed, 1 ignored (includes the format-freeze gates, the WAL observability tests, and the phone keystore matrix) |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS (exit 0, no warnings) |
| `cargo fmt --all -- --check` | PASS (exit 0, no diff) |
| WAL poisoning probe, pre-fix (real server, golden U32 envelope into U16 column) | `400 WIDTH_MISMATCH`; WAL 263,568 bytes; forced restart exited 1 with `log replay failed at record 1` |
| WAL poisoning probe, post-fix (real server) | Duplicate table/column: `409 TABLE_EXISTS` / `409 COLUMN_EXISTS`; golden U32 envelope into U16 column: `400 WIDTH_MISMATCH`; WAL stayed at 78 bytes; forced restart served the committed table, exit success |
| Rate-limit probe, pre-fix (real server, port 18084) | 130 new connections: 130 handled, 0 `429`; 130 requests on one connection: 120 handled, 10 `429` |
| Rate-limit probe, post-fix (real server, 125 separate connections) | 120 handled (`409 NO_MODEL`), 5 `429`; quota binds across connections |
| Checkpoint lock-order stress probe (real server, `--save-interval-secs 1`) | 3 rounds × 700 mutations, 12 clients: 2,100/2,100 `200`; health `200` after every round; final WAL 169,464-byte snapshot with 0 record bytes; no checkpoint errors |
| Request-tracing probe (real server, default env, 18095) | Requests returned 200/200/404, but the log contained only two startup `INFO` lines and no per-request event |
| Format-freeze gates (`cargo test --release -p fhe-core -p fhe-db --test format_freeze`) | PASS: envelope version, golden-corpus SHA-256/lengths, parameter values, WAL magic/version/header, and a byte-level fresh-header check |
| WAL recovery/checkpoint observability (`cargo test --release --workspace`) | PASS: report counts in `fhe-db`, `fhe_wal_*` counters in `/metrics`, and structured `wal.recover` / `wal.checkpoint` lines in the real binary's output |
| Phone keystore + engine flow (`cargo test -p fhe-phone --release`) | PASS: 13 tests (12 unit + 1 headless engine flow). Covers vault v2, pinned/recorded KDF, v1 migration and v1 backup import, persisted lockout, injected-commit-failure atomicity, authenticated destroy, and the IPC change-PIN/destroy path. |
| Dependency boundary (`cargo tree -i tfhe --workspace --edges normal`) | `tfhe`'s only workspace parent is `fhe-core` (baseline) |

The final documentation edits were re-checked against the workspace quality
gate: release build, 119 tests, clippy, and fmt all passed on this checkout. The
full **debug** test run was attempted but exceeded the 10-minute tool limit
(individual TFHE unit tests take minutes in debug); release tests are the
documented baseline. TLS, fuzz targets, Criterion benchmarks, the phone GUI,
and an actual external audit were **not** run here and are not claimed.

## What this package does not establish

- That the TFHE parameters are secure; it records what the vendor claims and
  what the repository actually tests.
- That the implementation is free of exploitable bugs, timing leaks, or
  memory-disclosure issues.
- That the phone UI enforces the keystore policy end to end; the GUI was not
  launched in this pass, though the IPC paths are covered headlessly and the
  keystore itself has a dedicated test matrix (F4/F8 fixed).
- That a clean-machine install, upgrade, or backup/restore works; that is
  stage 4.
- That the server is safe to expose; it is not, without auth, TLS, an
  authorization model, and fixes for the remaining open findings.

## Recommended gate before engaging the auditor

1. Review the fixed F4/F8 keystore: single-file vault v2, pinned Argon2id
   parameters, persisted lockout, atomic PIN change/import/destroy, zeroization,
   and the documented residuals (offline PIN guessing, no memory locking,
   TFHE-rs zeroization limits).
2. Give the auditor this package plus `SECURITY.md`, [`EVIDENCE.md`](EVIDENCE.md)
   (hashes, TFHE-rs patch diff, reproduction commands), and the exact parameter
   pin tests.
3. Keep the external audit scoped to the lattice parameters, the TFHE-rs
   integration, envelope/key parsing, and the key lifecycle; UI packaging is a
   separate review.
4. Record the fixed F1/F2/F11 probes and tests as regression evidence; do not
   reopen those findings unless the code or deployment model changes.
