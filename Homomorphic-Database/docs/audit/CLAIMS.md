# Claim Inventory

Every capability or security claim in `README.md`, `SECURITY.md`, and
`PROJECT_STATUS.md` is listed below with the evidence that supports it and a
verdict. This is the audit trail for the corrections made in this pass.

**Verdicts**

- **Supported** — code, a test, or an observed run supports the claim.
- **Qualified** — the claim is true only with the stated narrowing; the
  document now says so (or the claim is marked here as needing a wording
  change).
- **Vendor claim** — the statement is TFHE-rs's documented claim, not a proof
  produced by this repository.
- **Not verified** — cannot be checked from this checkout/run.
- **Corrected** — the original wording was unsupported and has been changed.

**Status:** internal review, 2026-11-10. F1, F2, and F11 were fixed after the
review; F3, F5, F6, F7, and F10 remain open; F9 is accepted only for the single-user local vault. No external audit.

## README.md

| ID | Claim (source section) | Evidence | Verdict |
|---|---|---|---|
| C01 | "Encrypted aggregation and encrypted-weight neural inference over TFHE" (title) | SUM/AVG/COUNT in `crates/fhe-db/src/query.rs`; NN in `crates/fhe-core/src/nn`; demos pass | Supported |
| C02 | Client encrypts locally; server stores/evaluates without seeing value plaintext; only the key holder decrypts (Overview) | `crates/fhe-core/src/encrypt.rs`; `routes.rs` accepts envelopes; `ClientKey` never uploaded | Supported with the metadata/bias qualification now stated in README |
| C03 | Encrypted aggregation SUM / AVG / COUNT (implemented surface) | `QueryEngine::sum/avg/count`; `encrypted_sum` measured run | Supported |
| C04 | Encrypted read-back of stored ciphertexts (implemented surface) | `get_column_ciphertexts` (`routes.rs:1055-1075`); read-back test | Supported |
| C05 | Encrypted inference over encrypted weights, XNOR + iris demos; bias/shape plaintext | `DenseLayer.obfuscated_weights`; `InstallXnorRequest.bias: Vec<i64>` (`routes.rs:937-971`); `encrypted_nn`/`encrypted_iris` | Qualified; README now states bias/shape are plaintext |
| C06 | Optional bearer auth, optional TLS, body/schema caps, per-IP rate limiting (transport hardening) | Auth/TLS/caps implemented; limiter keyed by `IpAddr` (`routes.rs:239-317`) and tested across connections | Supported; proxy/NAT caveat documented |
| C07 | Filtering, joins, general SQL, SMPC, GPU are not implemented | Route list (`routes.rs:506-538`); no such modules | Supported |
| C08 | Columnar storage; row counts are public metadata | `Table.columns`; `AggregateResult.count` (`query.rs:19-24`) | Supported |
| C09 | "Zero host value plaintext"; server gets public key and envelopes, never client key or field value | Server key upload; envelope-only rows; no client-key route | Supported; README now excludes metadata/bias explicitly |
| C10 | "Measured, not assumed": build/test/lint/format/demo results recorded with commands | `PROJECT_STATUS.md` §1; baseline run | Supported |
| C11 | Not production-ready: independent audit and clean-machine packaging outstanding | Stage 1 external audit blocked; stage 4 not started | Supported |
| C12 | "Live Gateway" link | External deployment, not probed here | Not verified (not a security claim) |
| C13 | License badge "BSD-3-Clause-Clear" | Workspace `Cargo.toml:18`; no `LICENSE` file | Qualified; manifest declaration only, distribution gap noted in `PROJECT_STATUS.md` |

## SECURITY.md

| ID | Claim (source section) | Evidence | Verdict |
|---|---|---|---|
| C14 | The server never receives plaintext values; only the public `ServerKey` and envelopes cross the wire | `POST /keys/server` accepts only server-key bytes; rows accept only base64 envelopes (`routes.rs:732-786`) | Supported |
| C15 | Only the holder of `ClientKey` can decrypt; the key is never sent to the server | No client-key route; `ClientKey` is not serialized to the server path (client only); phone stores it encrypted | Supported; backup confidentiality depends on the PIN (C24) |
| C16 | Width mismatches never slip through (versioned envelope + width-checked column) | `CiphertextEnvelope`; `EncryptedColumn::push`; `WIDTH_MISMATCH` probe | Supported |
| C17 | Out-of-range values rejected before encryption | `Encryptor::encrypt_*` range checks; `rejects_out_of_range` test | Supported |
| C18 | Wire-data errors do not panic request handlers | Handler paths return typed errors; malformed-envelope tests | Supported for the tested/handler paths; not a proof for every allocation path |
| C19 | No key material in `Debug` output | Redacted `Debug` for `ClientKey`/`ServerKey`; test asserts no `0x` | Supported |
| C20 | Envelope format changes break the build | `fhe-core/tests/golden_corpus.rs`; committed v1 key/envelope | Supported |
| C21 | Parameter drift breaks the build | `default_parameters_match_documented_pinning` (`keys.rs:289-345`) | Supported as a value pin |
| C22 | Pinned parameter table (918 LWE, 2048 polynomial, 4/4, p-fail −129.58) | `keys.rs:148-161`; vendor parameter source | Supported |
| C23 | `log2_p_fail` ≈ −129.58 implies "128-bit IND-CPA^D security" | TFHE-rs docs say failure probability ≤2^-128 and IND-CPA^D; Lattice Estimator/BDGL16 (`vendor/tfhe/README.md:198-208`). `p_fail` itself is correctness, not advantage. | **Vendor claim, not proof**; corrected wording in SECURITY/keys docs |
| C24 | Phone PIN: AES-256-GCM under a pinned Argon2id (19 MiB, t=2, p=1) key, 6–12 digit policy, persisted unlock lockout; backup is copyable and PIN-dependent | `keystore.rs` vault v2 (`KdfParams::PINNED`, `check_pin_at`, `atomic_write`); 12 keystore/wire tests + engine flow | **Supported** for the local-vault scope (F4/F8 fixed 2026-11-11); offline guessing remains possible and is disclosed |
| C25 | Side channels: keygen/encrypt are not constant-time; TFHE-rs has no side-channel mitigation | `vendor/tfhe/README.md:215-219`; no project zeroization/memory locking | Supported (as a limitation) |
| C26 | Loopback default; auth optional; TLS optional | `main.rs` CLI defaults; `auth_token: None`; TLS feature default-off | Supported |
| C27 | DoS posture: body caps, batch caps, schema caps, aggregation row cap, "per-IP" limiter | Caps implemented (`Limits`); limiter keyed by `IpAddr` (`routes.rs:239-317`); cross-connection regression and live probe | Supported for the stated limits; proxy/NAT sharing and no admission queue remain limitations |
| C28 | Durability: every mutation journaled before apply; a crash loses nothing acknowledged | `WalRecord::validate` runs under the database write lock before `Wal::append`; regression proves rejected mutations leave the WAL unchanged and committed rows recover | Supported; F2 fixed |
| C29 | Fuzzing implemented for envelope and DB snapshot decoding; CI smoke-runs both | `fuzz/` targets; `.github/workflows/ci.yml` fuzz job | Supported; not run locally |
| C30 | External audit NOT DONE; side-channel audit NOT DONE | No external engagement; stage 1 blocked | Supported |
| C31 | Back up the data directory; copying `database.bin` is the supported backup | Current server uses `fhe-db.log`; `database.bin` is legacy. Live-copy consistency is not guaranteed. | **Corrected**; AGENTS/SECURITY now say stopped-server `fhe-db.log` copy |
| C32 | No result authentication; FHE does not provide malicious-server integrity | No MAC/commitment/verification in client or server; ciphertexts are malleable | Supported (as a limitation) |
| C50 | Generation: fresh pair in the client, no HSM/entropy health check/key ceremony | `ClientKey::new` (`keys.rs:39-43`); phone `keygen` (`engine.rs:182-230`); no HSM/ceremony code | Supported as a limitation |
| C51 | Key-pair rotation is absent; replacing the server key breaks old ciphertext evaluation | No key-id field in `CiphertextEnvelope`/schema; `POST /keys/server` replaces `AppState.server_key` (`routes.rs:580-600`); no re-encryption API | Supported as a limitation |
| C52 | Destruction is absent: `KeyStore::clear` exists but is not wired; lock does not zeroize | `keystore.rs:375-382`; only definition, no IPC/UI call; `engine.rs:324-329` drops the Arc | Supported as a limitation |
| C53 | Restore used to replace both files without authenticating the backup or an atomic commit | Old `import_backup` wrote `pin.json` then `keys.json` directly | **Fixed 2026-11-11**: v2 import validates KDF bounds and commits one vault atomically; backups are still unauthenticated |
| C54 | Backup contains the PHC PIN hash and the encrypted key pair; confidentiality depends on PIN strength | `export_backup` embeds the v2 vault; no signature/MAC | Supported as a limitation (unchanged) |
| C55 | Auth-token rotation is operational only: restart with a new token, no online rotation/key ID | Single `auth_token` in `AppState` (`routes.rs:430-432`); no rotation route | Supported as a limitation |

## PROJECT_STATUS.md

| ID | Claim (source section) | Evidence | Verdict |
|---|---|---|---|
| C33 | Baseline: release build/test/clippy/fmt and three demos pass; 119 passed, 1 ignored, including the format-freeze gates and phone keystore matrix | `PROJECT_STATUS.md` §1 commands and observed run | Supported |
| C34 | Test breakdown 38/31/27/10/13 passed across `fhe-core`/`fhe-db`/`fhe-server`/`fhe-client`/`fhe-phone` (plus 1 ignored golden fallback) | Release test output; `PROJECT_STATUS.md` §1 table | Supported |
| C35 | Only `fhe-core` depends on `tfhe` | `cargo tree -i tfhe --workspace --edges normal` | Supported |
| C36 | Stage 0 (trustworthy baseline) is Done | Baseline recorded; docs reconciled | Supported |
| C37 | Stage 1 is Blocked because the external audit is not engaged | No external audit exists; internal package now added | **Qualified**: internal 1.1–1.2 Done, external audit still Blocked |
| C38 | `fhe-server` has rate limits and body/schema caps | Caps in `Limits`; limiter is per IP and binds across connections (F1 fixed) | Supported |
| C39 | WAL recovery is delivered | Valid mutations recover; rejected mutations are validated before append and cannot poison replay (F2 fixed) | Supported |
| C40 | `apps/fhe-phone` has a PIN-encrypted key store and PIN-protected backups | `keystore.rs` single-file v2 vault: AES-GCM, pinned Argon2id, persisted lockout, atomic change/import/destroy; legacy v1 migration | Supported for the single-user local scope; F4/F8 fixed 2026-11-11; residual offline PIN guessing and vendor zeroization limits documented |
| C41 | Production readiness is not claimed | Stage 1 and 4 open | Supported |
| C42 | No `LICENSE` file while `Cargo.toml` declares BSD-3-Clause-Clear | `ls LICENSE` absent; `Cargo.toml:18` | Supported |
| C43 | TLS, fuzzing, and benchmarks have CI coverage but were not exercised locally | `.github/workflows/ci.yml` jobs; baseline "not executed" section | Supported |
| C44 | `fhe-core::nn` uses encrypted weights | `DenseLayer.obfuscated_weights`; XNOR bias is plaintext (C05) | Qualified; weights only |
| C45 | Stage 2 (database/service reliability) is in progress | Durability slice landed: `crates/fhe-server/tests/durability.rs` (process-level graceful stop, hard kill, torn tail), the `fhe-db` crash-boundary matrix, and `docs/operations/WAL_RECOVERY.md`; load limits, versioning, and remaining semantics still open | Supported as a status |
| C46 | `fhe-server` has ciphertext read-back and encrypted XNOR install/infer | Routes/handlers; tests | Supported |
| C47 | `fhe-phone` GUI was not exercised in the baseline | Baseline note; no GUI run in this pass | Supported |
| C48 | Managed cloud hosting is not in scope | Product decision recorded; no cloud code | Supported |
| C49 | "No stage-1 security work was performed in this pass" (old §3) | Internal audit package now exists | **Corrected**: removed; stage 1 is In progress |

## Fixes made after the review

| Finding | Fix | Regression evidence |
|---|---|---|
| F1 — connection-keyed limiter | `RateLimitState` now keys on `IpAddr`; middleware takes `ConnectInfo<SocketAddr>::ip()`. | `rate_limiter_shares_one_bucket_per_ip_across_connections`; live 125-connection probe (120 handle, 5 `429`). |
| F2 — WAL poisoning | `WalRecord::validate` runs against the locked database before `Wal::append` in every server mutation and `WalBackedDatabase` wrapper. | `rejected_mutations_do_not_poison_the_wal`; live duplicate-table/column and golden width-mismatch probes with unchanged WAL and forced restart. |
| F11 — checkpoint lock order | `AppState::checkpoint_wal` takes `db.read` then `wal.write`; periodic and shutdown paths use it. | `checkpoint_acquires_database_before_wal`; live 2,100-mutation probe at 1s checkpoints, final WAL snapshot with zero record bytes. |

## Corrections made in this pass

| Original wording | Problem | Now |
|---|---|---|
| "per-IP rate limiter" (README, AGENTS, SECURITY, rustdoc) | Was keyed by `SocketAddr` with ephemeral port; new connections bypassed it | Fixed: keyed by `IpAddr`, quota shared across connections, proxy caveat documented |
| "ClientKey never leaves the client process/your phone" (SECURITY, AGENTS, engine/keystore docs) | Phone persists it encrypted and exports it in backups | "never sent to the server; stored encrypted at rest; backup confidentiality depends on PIN" |
| "server never sees the model" / "learns neither data nor model" (server/NN/example/phone rustdoc) | XNOR bias and layer shape are plaintext | "weight plaintext never seen; plaintext bias and shape visible" |
| `p_fail ≈ 2^-129.58` → "128-bit IND-CPA^D security" (SECURITY, keys rustdoc, AGENTS) | `p_fail` is a correctness-failure estimate; the security label is TFHE-rs/Lattice-Estimator vendor claim, unaudited | Separated the two; marked vendor claim; link `PARAMETERS.md` |
| "Argon2id tuned for ~100 ms; strong against short-PIN offline brute force" (old keystore docs) | No parameter benchmark, and the old floor was 4 characters | Corrected and fixed: pinned 19 MiB/t=2 parameters recorded in the vault, 6–12 digit PIN, persisted lockout; a short PIN is still guessable offline |
| "Every value is encrypted twice" (phone DB note) | Only TFHE application-layer encryption is applied | "encrypted once as a TFHE envelope" |
| "Wire tap proves the server only handled ciphertexts" (phone docs) | Tap records method/path/status only, lossy, matched routes only | "traffic metadata view, not payload proof" |
| "Backup = copy `database.bin`" (AGENTS §13) | Current store is `fhe-db.log`; legacy file is migration-only | "stopped-server copy of `fhe-db.log`" |
| "Encrypted backup; only the PIN can open it" / "safe to copy anywhere" (engine rustdoc, phone UI) | The JSON file is copyable and contains the PHC PIN hash; only the key material is encrypted, and a short PIN is guessable offline | "PIN-protected backup; key material stays encrypted; short PIN is guessable offline" |
| "Traces every request at INFO level" / "access logs" (server `app()` docs, SECURITY) | `TraceLayer`'s default events are DEBUG and the CLI default filter is `info,fhe_server=debug`, so no per-request log is emitted by default | "tracing installed; enable `tower_http=debug` for per-request logs" |
| "No stage-1 security work performed" (PROJECT_STATUS §3) | Internal 1.1–1.2 now done | "In progress: internal package Done; external Blocked" |

## Residual claims that are intentionally not made

- FHE-DB does **not** claim malicious-server integrity, verifiable
  computation, side-channel resistance, multi-tenant authorization, secure
  erase, or production readiness.
- It does **not** claim that the TFHE parameters are independently proven
  secure; see [`PARAMETERS.md`](PARAMETERS.md).
- It does **not** claim that the phone keystore is a hardware-backed key store;
  it is a PIN-derived software keystore.
