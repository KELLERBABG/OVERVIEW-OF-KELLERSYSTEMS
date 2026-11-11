# FHE-DB — Roadmap & Status (authoritative)

> **This file is the single source of truth for project status, the measured
> quality baseline, and the production roadmap.** Other documents link here
> instead of duplicating status claims.
>
> **Last measured:** 2026-11-11, on commit `3408d55` plus the F1/F2/F4/F8/F11
> fixes, the B4/C5/C6 audit work, and the D7/D8/E9 keystore and product-model
> work in this worktree, on this checkout.
>
> The internal audit package for stage 1.1–1.2 is
> [`docs/audit/README.md`](docs/audit/README.md). It is an internal review, not
> an independent audit.

Status values used throughout: **Done**, **In progress**, **Blocked**,
**Not started**, **Not in scope**.

---

## 1. Measured quality baseline — 2026-11-11

Environment actually used for the run below:

| Item | Value |
|------|-------|
| OS | Windows |
| `rustc` / `cargo` | 1.97.1 (`8bab26f4f` / `c980f4866`) |
| Default toolchain | `stable-x86_64-pc-windows-msvc` |
| Build target dir | `C:/fhe-target` (pinned in `.cargo/config.toml`) |
| Workspace root | `C:\Users\lukas\Downloads\SYSTEMS & CREATIONS\Homomorphic-Database` |

Every command below was executed in this checkout. Exit statuses were captured
explicitly (the tested command's status, not a pipeline's).

| Command | Result | Observed |
|---------|--------|----------|
| `cargo build --release --workspace` | **PASS** (exit 0) | Incremental release build |
| `cargo test --release --workspace` | **PASS** (exit 0) | 119 passed, 0 failed, 1 ignored |
| `cargo test --release -p fhe-core -p fhe-db --test format_freeze` | **PASS** (exit 0) | Envelope v1, WAL v1, golden corpus, and parameter pins unchanged |
| `cargo clippy --workspace --all-targets -- -D warnings` | **PASS** (exit 0) | No warnings |
| `cargo fmt --all -- --check` | **PASS** (exit 0) | No diff |
| `cargo run --release -p encrypted_sum` | **PASS** (exit 0) | SUM 325000, AVG 65000, COUNT 5/5 |
| `cargo run --release -p encrypted_nn` | **PASS** (exit 0) | XNOR truth table 4/4 |
| `cargo run --release -p encrypted_iris` | **PASS** (exit 0) | plaintext 29/30 = encrypted 29/30 (96.7%) |
| `cargo tree -i tfhe --workspace --edges normal` | **PASS** | `tfhe`'s only workspace parent is `fhe-core` |

### Test breakdown

| Target | Passed | Notes |
|--------|-------:|-------|
| `fhe-core` unit (`src/lib.rs`) | 25 | |
| `fhe-core` `format_freeze` | 3 | Envelope version, golden-corpus hashes/lengths, pinned parameter values |
| `fhe-core` `bench_guard` | 2 | Hardware-robust ceiling guards |
| `fhe-core` `golden_corpus` | 2 | 3 total; 1 ignored: `regenerate_golden_corpus` writes fixtures and is run only on a deliberate format change |
| `fhe-core` `properties` | 4 | `proptest` algebraic identities |
| `fhe-core` doc tests | 2 | |
| `fhe-db` unit | 29 | Includes DB-layer integration tests in `src/lib.rs`, the WAL crash-boundary matrix (`crash_before_append_*`, `crash_after_append_*`, `checkpoint_ignores_a_stale_partial_tmp_file`), and the recovery/checkpoint report counts |
| `fhe-db` `format_freeze` | 2 | WAL magic/version/header constants plus a byte-level fresh-header check |
| `fhe-server` unit | 24 | axum handlers via `tower::ServiceExt::oneshot`; includes the WAL-rejection, cross-connection rate-limit, checkpoint lock-order, and `fhe_wal_*` metrics regressions |
| `fhe-server` `durability` | 3 | Spawns the real binary: graceful stop (SIGTERM / Ctrl-Break) exits 0, compacts, and emits `wal.checkpoint`; acknowledged rows survive a hard kill and boot emits `wal.recover`; a torn tail heals on boot and is reported |
| `fhe-client` unit | 10 | |
| `fhe-phone` unit + `engine_flow` | 12 + 1 | Vault v2/migration/lockout/atomicity/destroy plus the full lock -> keygen -> insert -> aggregate -> read-back flow and IPC change-PIN/destroy |
| Examples | 0 | Demos are binaries, verified by running them (table above) |

### Not executed in this baseline

These are explicitly **unverified on this machine**, not silently passing:

- **`fhe-phone` GUI** — the crate builds, but the native webview shell was not
  launched; it is not a headless demo.
- **TLS feature** (`cargo build/test -p fhe-server --features tls`) — needs a C
  compiler for `aws-lc-rs`; owned by the Ubuntu CI job.
- **Fuzz targets** (`fuzz/`) — need `clang` and `cargo-fuzz`; owned by CI.
- **Criterion benchmarks** — not run in this pass.

---

## 2. Delivered capability — **Done**

Everything below is implemented in this checkout and covered by the measured
baseline above. "Delivered" does **not** mean production-ready; see §3.

| Area | Delivered | Evidence |
|------|-----------|----------|
| `fhe-core` primitives | Keygen, width-checked encryption, versioned ciphertext envelopes, `EncryptedColumn`, `CiphertextAlgebra`, `ServerKeyGuard` | 39 tests (38 passing + 1 ignored golden fallback; includes 2 doc tests) |
| `fhe-core::nn` | Fixed-point encoding, polynomial activations, dense/feed-forward layers over encrypted weights | Unit tests + `encrypted_nn` demo |
| `fhe-db` | Columnar encrypted storage, schemas, WAL recovery, bincode persistence, homomorphic SUM / AVG / COUNT | 31 tests (29 unit/integration + 2 format-freeze; incl. the crash-boundary matrix) |
| `fhe-server` | axum REST API, bearer-token auth, optional TLS (CI-built), per-IP rate limiting and body/schema caps, `/metrics`, ciphertext read-back, encrypted XNOR install/infer | 24 passing handler tests + 3 process-level durability tests |
| `fhe-client` | Async HTTP SDK: keygen, key upload, encrypted insert, aggregation, read-back, inference, client-side decryption | 10 passing tests |
| `examples/encrypted_sum` | End-to-end encrypted SUM / AVG / COUNT | Measured run above |
| `examples/encrypted_nn` | Encrypted XNOR inference with obfuscated weights | Measured run above |
| `examples/encrypted_iris` | Real-dataset encrypted inference, Q15.0 quantization-aware training | Measured run above |
| `apps/fhe-phone` | Embedded server + real client SDK + Material-You webview UI, single-file PIN vault (pinned KDF, persisted lockout, atomic change/import/destroy, v1 migration), PIN-protected backups | Build + 12 unit tests + 1 headless engine flow; GUI not exercised here; F4/F8 fixed, residuals documented |
| Tooling | WAL, fuzz targets, benchmark guards, golden corpus, CI workflow | `fuzz/`, `crates/fhe-core/benches/`, `crates/fhe-core/tests/`, `.github/workflows/ci.yml` |

Dependency rule still enforced: **only `fhe-core` depends on `tfhe`**
(verified with `cargo tree -i tfhe`).

---

## 3. Production roadmap

The project is being taken to a releasable encrypted database. Managed cloud
hosting is explicitly out of scope.

| Stage | Scope | Status |
|-------|-------|--------|
| **0. Trustworthy baseline** | Run the real quality gate, record measured results, keep status docs consistent, preserve the crate boundary | **Done — 2026-11-10** |
| **1. Security & release blockers** | Independent cryptographic audit of parameters/implementation; claim and threat-model review; key lifecycle review; service-exposure review; supply-chain review; fuzzing as a release gate | **In progress** — internal 1.1–1.2 package is **Done** (`docs/audit/`); the C5/C6 engagement package and format freeze are **Done** (`ENGAGEMENT.md`, `EVIDENCE.md`, `FORMAT_FREEZE.md`); external audit is **Blocked** (package ready, not sent, no quote). Findings **F1, F2, F4, F8, and F11 are fixed** with regression coverage; **F9 is accepted** only for the single-user local-vault model ([`docs/audit/PRODUCT_MODEL.md`](docs/audit/PRODUCT_MODEL.md)); F3, F5, F6, F7, and F10 remain open. |
| **2. Database & service reliability** | Crash/restart recovery tests beyond round-trips; operational semantics; resource limits under load; API/persistence versioning; runbooks | **In progress** — the durability slice landed: process-level graceful-stop and crash/restart tests (`crates/fhe-server/tests/durability.rs`), the WAL crash-boundary matrix in `fhe-db`, and the operator recovery runbook (`docs/operations/WAL_RECOVERY.md`). Recovery and checkpointing are now observable: one structured `wal.recover` / `wal.checkpoint` line each plus the `fhe_wal_*` `/metrics` counters (torn-tail bytes, records replayed/compacted, snapshot bytes, duration). Load limits, API/persistence versioning policy, and the remaining operational semantics are still open. |
| **3. Bounded query & inference** | Specify and add only queries whose semantics and leakage are documented; query planning; wider-width NN path; bootstrapping-aware training; reproducible benchmarks | **Not started** |
| **4. Usable distribution** | `fhe-phone` installer, code signing, WebView2 story, clean-machine test, recovery/backup verification, onboarding | **Not started** (packaging gap remains open) |
| **5. Performance, operations, release** | Measured optimization, supported workload envelopes, release-level operational checks, final release review | **Not started** |
| **Managed cloud hosting** | Hosted control plane, tenant provisioning, managed multi-tenant service | **Not in scope** (product decision 2026-11-10) |

**Production readiness is not claimed.** The blocking gate is stage 1: an
independent security review plus the open packaging work in stage 4.

### Known open gaps

- External cryptographic audit — **Blocked** (external dependency). The C5/C6
  package is ready (`docs/audit/ENGAGEMENT.md`, `EVIDENCE.md`,
  `FORMAT_FREEZE.md`, `evidence.sha256`); it has **not been sent**, no reviewer
  has been engaged, and no quote has been received.
- Internal audit findings F1, F2, F4, F8, and F11 — **fixed**: the limiter is
  keyed by IP, rejected mutations are validated before the WAL append,
  checkpointing acquires the database before the WAL, and the phone keystore
  now has a pinned/recorded KDF, a 6–12 digit PIN policy, a persisted lockout,
  single-atomic-commit PIN change/import/save, zeroized buffers, a wired
  authenticated destroy action, and a v1 migration. Pinned by the named
  regressions in §1. **F9 is accepted** only for the single-user local vault
  ([`docs/audit/PRODUCT_MODEL.md`](docs/audit/PRODUCT_MODEL.md)) and reopens as
  a blocker for any shared/remote deployment. F3, F5, F6, F7, and F10 remain
  **open**: plaintext XNOR bias, unaudited parameters, metadata leakage, no
  side-channel hardening, and a lossy wire-tap claim. Details and evidence in
  `docs/audit/README.md`.
- `fhe-phone` packaging (installer, signing, clean machine) — **Not started**.
- TLS, fuzzing and benchmarks have CI coverage but were **not exercised
  locally** in this baseline.
- The repository has no `LICENSE` file while the workspace manifest declares
  `BSD-3-Clause-Clear` — a distribution task for stage 5.

---

## 4. Reproduce this baseline

Run from the workspace root:

```bash
cargo build --release --workspace
cargo test --release --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check

cargo run --release -p encrypted_sum
cargo run --release -p encrypted_nn
cargo run --release -p encrypted_iris

# Dependency boundary: only fhe-core may list a direct tfhe dependency.
cargo tree -i tfhe --workspace --edges normal
```

FHE workloads are slow in debug builds; use `--release` as above. CI
(`.github/workflows/ci.yml`) additionally owns the TLS build/test, fuzz
smoke-runs, and the benchmark smoke job.

---

## 5. History

- **2026-08-05** — earlier snapshots recorded production-hardening and security
  passes, plus a four-phase roadmap in a `DEVELOPMENT_PLAN.md` file. That file
  is **not present in the repository**; all references to it were replaced by
  this document, and the four-phase numbering is superseded by §3. The
  earlier counts and readiness estimates are historical and are no longer
  asserted; §1 is the measured baseline.
- **2026-11-10** — baseline re-measured on this checkout; status documents
  reconciled to this file. Internal audit package 1.1–1.2 added under
  `docs/audit/`, with live probes reproducing the WAL-poisoning and
  per-connection limiter findings; unsupported claims corrected in README,
  SECURITY, AGENTS, and rustdoc. External audit remains blocked.
- **2026-11-10 (follow-up)** — fixed audit F1 (limiter buckets now keyed by
  `IpAddr`, so the quota binds across TCP connections), F2 (mutations are
  validated under the database write lock before the WAL append), and F11
  (checkpointing now takes the database before the WAL through one helper used
  by the periodic and shutdown paths). Added the server regressions
  `rate_limiter_shares_one_bucket_per_ip_across_connections`,
  `rejected_mutations_do_not_poison_the_wal`, and
  `checkpoint_acquires_database_before_wal`; live probes confirmed 120
  expensive requests allowed then `429`, a forced restart after rejected
  mutations recovered the committed table, and 2,100/2,100 mutations succeeded
  at a one-second checkpoint interval. Baseline is now 99 passing tests, 1
  ignored.
- **2026-11-10 (B4/C5/C6)** — observability and audit readiness. B4: recovery
  and checkpointing now report what they did. `Wal::recover_with_report` and
  `Wal::checkpoint_with_report` return typed reports (snapshot bytes, records
  and bytes replayed/compacted, torn-tail bytes, pre/post log size, duration);
  the server emits exactly one structured `wal.recover` or `wal.checkpoint`
  line per event and folds the same values into the new `fhe_wal_*` Prometheus
  counters, so a silent heal or a failed checkpoint is visible. C6: the auditor
  evidence pack (`docs/audit/EVIDENCE.md` plus `evidence.sha256`) records the
  artifact hashes, the re-runnable verification that the vendored TFHE-rs 1.7.0
  source is byte-identical to the published archive except `crate-type`, the
  parameter provenance, wire/key formats, and reproduction commands. C5: the
  engagement package (`docs/audit/ENGAGEMENT.md`) defines scope, exclusions,
  deliverables, an RFQ template, an outreach set, and the parallel-work rules;
  `docs/audit/FORMAT_FREEZE.md` freezes the envelope v1 frame, WAL v1
  header/record frame, golden corpus, and parameter set, enforced by
  `crates/fhe-core/tests/format_freeze.rs` and
  `crates/fhe-db/tests/format_freeze.rs`. Sending the package and collecting
  quotes remain owner actions. Baseline counts are in §1.
- **2026-11-10 (durability)** — stage-2 durability slice. Added the
  process-level tests in `crates/fhe-server/tests/durability.rs` (graceful
  stop compacts and exits 0; acknowledged rows survive `TerminateProcess` /
  SIGKILL; a truncated tail heals on boot), the WAL crash-boundary matrix in
  `fhe-db` (crash before append, after append before apply, mid-checkpoint
  with a stale `.tmp`), and `docs/operations/WAL_RECOVERY.md`. The server now
  also treats Windows Ctrl-Break as a graceful stop, so console hosts and
  `GenerateConsoleCtrlEvent` drain and checkpoint instead of aborting.
  Baseline is now 105 passing tests, 1 ignored.
- **2026-11-11 (D7/D8/E9)** — phone keystore and product model. D7: the
  two-file `pin.json`/`keys.json` store became a single atomic `vault.json`
  (format v2) with recorded, pinned Argon2id parameters; PIN change re-encrypts
  the key pair in the same commit; the PIN policy is now 6–12 digits, matching
  the numeric keypad. D8: failed unlocks are persisted (5 free attempts, then
  30 s doubling to a 1 h cap), derived keys/plaintext buffers/the in-memory PIN
  are zeroized, and an authenticated `vault.destroy` action is wired through
  IPC and Settings. A legacy v1 store and v1 backups are migrated/imported.
  E9: [`docs/audit/PRODUCT_MODEL.md`](docs/audit/PRODUCT_MODEL.md) records the
  single-user local-vault decision; F9 is accepted only under its deployment
  conditions and reopens for any shared/remote deployment. F4 and F8 are now
  fixed; residual offline PIN guessing and TFHE-rs zeroization limits remain
  documented. `fhe-phone` now has 12 unit tests + 1 headless engine flow (up
  from 6 + 1).
