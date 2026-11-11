# SECURITY.md — FHE-DB Security Posture

> **Status:** development build. The cryptography below is pinned and tested,
> but this project has **not** been independently audited and must not be used
> to protect real secrets yet (see [Audit status](#7-audit-status)).
>
> The internal audit package that gates the external review is in
> [`docs/audit/`](docs/audit/README.md). It records the actual protection
> boundary, per-claim evidence, parameter provenance, and key lifecycle.
> The authoritative status/roadmap remains [`PROJECT_STATUS.md`](PROJECT_STATUS.md).

---

## 1. Protection boundary

FHE-DB computes over **encrypted values**. The client encrypts locally and
uploads ciphertext envelopes plus the public evaluation key; the server stores
and aggregates them without receiving the values or the secret key.

The boundary is narrower than "the server sees nothing":

| The server **does** receive or observe | The server **does not** receive |
|---|---|
| Table and column names, declared types, schema shape | Plaintext field values |
| Row counts, insert batch sizes, read-back and aggregation targets | The secret `ClientKey` |
| Ciphertext width, envelope version, ciphertext byte size | Plaintext aggregation results |
| Request paths, status codes, timing, peer address (per-request logs only when tracing is enabled) | Plaintext neural inputs or weight values |
| The public `ServerKey` | The plaintext of a decrypted result |
| The XNOR model's **plaintext bias** and layer shape | — |

The server also persists metadata and ciphertexts in the WAL when
`--data-dir` is used. A reader of that log learns the same schema, counts, and
ciphertext sizes as the server. Full inventory:
[`docs/audit/THREAT_MODEL.md`](docs/audit/THREAT_MODEL.md).

The invariants below are enforced in code and pinned by tests:

| Invariant | Enforcement |
|-----------|-------------|
| The server never receives plaintext **values** | Only the public `ServerKey` is uploaded; values cross the wire as `CiphertextEnvelope` bytes. Metadata and plaintext bias are explicitly outside this invariant. |
| Only the holder of the `ClientKey` can decrypt | The key is never sent to the server. The phone stores it encrypted at rest and can export it inside a PIN-protected backup (the key material in that file stays encrypted); confidentiality then depends on the PIN (see §3). |
| Width mismatches never slip through | Versioned `CiphertextEnvelope` + width-checked `EncryptedColumn::push` |
| Out-of-range values rejected before encryption | `Encryptor` validates against `ScalarWidth` first |
| Wire-data errors do not panic request handlers | Typed `FheError` / `DbError` / `ServerError` / `ClientError` paths; malformed envelopes are rejected with `400` |
| No key material in debug output | `ClientKey`/`ServerKey` implement redacted `Debug` (tested in `fhe-core`) |
| Envelope format changes break the build | Golden-corpus test (`fhe-core/tests/golden_corpus.rs`) |
| Parameter drift breaks the build | Gate test `default_parameters_match_documented_pinning` |

## 2. Pinned cryptographic parameters

This build pins TFHE-rs 1.7.0's **default integer configuration**
(`ConfigBuilder::default()`), which resolves to the shortint parameter set
`PARAM_MESSAGE_2_CARRY_2_KS_PBS_TUNIFORM_2M128`:

| Parameter | Value | Meaning |
|-----------|-------|---------|
| `lwe_dimension` | 918 | LWE secret dimension |
| `glwe_dimension` | 1 | GLWE secret dimension |
| `polynomial_size` | 2048 | GLWE ring polynomial degree |
| `pbs_base_log` / `pbs_level` | 23 / 1 | Programmable-bootstrapping decomposition |
| `ks_base_log` / `ks_level` | 4 / 4 | Key-switching decomposition |
| `message_modulus` / `carry_modulus` | 4 / 4 | 2-bit message / 2-bit carry per shortint block |
| `max_noise_level` | 5 | Maximum noise level |
| `log2_p_fail` | ≈ −129.58 | TFHE-rs's estimated correctness-failure probability |

**What `log2_p_fail` does and does not mean.** It is the estimated probability
that a homomorphic operation produces a ciphertext that no longer decrypts to
the intended message because the noise budget was exhausted. It is **not** an
adversary-advantage or semantic-security bound. The "2M128" name and the
128-bit IND-CPA^D label come from TFHE-rs's parameter documentation and its
Lattice Estimator methodology (BDGL16); this repository neither reproduces
that estimate nor has it independently reviewed. The value is pinned, not
proven. See [`docs/audit/PARAMETERS.md`](docs/audit/PARAMETERS.md).

These values are documented in
[`fhe_core::SecurityParameters`](crates/fhe-core/src/keys.rs) and enforced by
the gate test `default_parameters_match_documented_pinning`. If a dependency
upgrade changes any of them, that test fails and the pin must be updated
**deliberately** (and re-documented here).

## 3. Key lifecycle

A summary; the evidence table is
[`docs/audit/KEY_LIFECYCLE.md`](docs/audit/KEY_LIFECYCLE.md).

- **Generation.** `ClientKey::new` generates a fresh pair in the client
  process using TFHE-rs's default configuration. There is no HSM, entropy
  health check, or key-ceremony procedure in this build.
- **Storage.** The CLI/library leaves custody to the caller. The phone shell
  stores `(ClientKey, ServerKey)` in one `vault.json` under `%APPDATA%/FHE-DB`
  (or `~/.fhe-db-phone`), encrypted with AES-256-GCM under an Argon2id
  PIN-derived key. The vault records the **pinned** Argon2id parameters
  (19 MiB, t=2, p=1, 32-byte output, v0x13) and the PHC PIN hash, so future
  dependency defaults cannot strand existing keys. Every commit is one
  atomic write (temp file, `sync_all`, rename); a legacy v1 two-file store is
  migrated on first open. New PINs are 6–12 digits; five failed unlocks start
  a persisted 30 s lockout that doubles to a one-hour cap.
- **Backup.** `backup.export` writes a JSON file containing the whole vault
  (PIN hash plus encrypted key pair). It is copyable, but confidentiality is
  only as strong as the PIN against offline guessing; the backup itself is not
  authenticated. v1 backups are still importable.
- **Restore.** `backup.import` validates the backup, resets the failure
  state, and replaces the vault in one atomic write. Treat an imported backup
  as a new root of trust.
- **Rotation.** There is no key-pair rotation, no ciphertext re-encryption, and
  no server-key versioning. Replacing `POST /keys/server` breaks evaluation of
  existing ciphertexts; changing the PIN re-encrypts the same key pair.
- **Destruction.** Settings → Security → Destroy keys calls
  `vault.destroy {pin}`: it authenticates through the lockout, erases the local
  vault and legacy files, and clears the in-memory key, PIN, and model flag.
  Ciphertexts on the server remain but become undecryptable (crypto-shredding).
  Exported backups, WAL entries, and OS copies are **not** erased. Derived
  keys, plaintext key buffers, and the in-memory PIN use `Zeroizing`; TFHE-rs
  key types expose no zeroization API, so the decrypted `ClientKey` is dropped
  but not explicitly wiped. Locking does the same for the in-memory handle.

## 4. Known posture and residual risks

- **Metadata leakage.** Schema names/types, row counts, batch sizes, which
  column is aggregated, read-back patterns, ciphertext width/size, request
  timing, peer address, and status codes are visible to the server and to
  anyone who can read the WAL or captured request logs (when per-request
  tracing is enabled). Traffic analysis can reveal workload shape even though
  values stay encrypted.
- **No result authentication.** FHE protects confidentiality, not correctness or
  authenticity. A malicious server can return a valid ciphertext that decrypts
  to a wrong value, replay another row's ciphertext, or omit rows. The client
  has no MAC, commitment, or verifiable-computation check to detect this. The
  protocol does not provide malicious-server integrity.
- **No authorization model.** Bearer-token auth is a single shared secret.
  There are no users, tenants, per-table ACLs, or ownership checks. Any caller
  with the token (or any caller at all when auth is unset) can read back any
  column, replace the server key, or overwrite the XNOR model.
- **Open by default.** The server binds to `127.0.0.1` and, without
  `--auth-token`, accepts any caller with port access. Loopback is not a
  security boundary between local processes or users. `app()` adds
  `CorsLayer::permissive()`; do not expose the service to a browser origin you
  do not control. This is accepted **only** for the single-user local-vault
  product model ([`docs/audit/PRODUCT_MODEL.md`](docs/audit/PRODUCT_MODEL.md));
  any shared, remote, or multi-user deployment reopens F9 as a blocker.
- **No authorization or tenancy.** The bearer token is one operator secret,
  not an identity system. There are no users, tables owned by principals,
  ACLs, or audit logs. Do not run the server for more than one person.
- **Rate limiting is per IP, with a proxy caveat.** Buckets are keyed by
  `IpAddr`, so opening a new TCP connection does not reset the quota. The
  limiter trusts the peer address from `ConnectInfo`; behind a reverse proxy,
  every client appears as the proxy IP and shares one bucket until a trusted
  forwarded-for policy is configured. NAT-ed clients likewise share a bucket.
  See `crates/fhe-server/src/routes.rs` and finding F1 (fixed).
- **WAL validation happens before the append.** The server validates each
  mutation against the locked database *before* journaling, so a rejected
  duplicate table/column or width-mismatched batch leaves no record that can
  block the next boot. A genuinely corrupt complete record is still rejected as
  corruption, and a torn tail is still healed by truncation. A log already
  poisoned by a pre-fix build is not silently repaired. See
  `docs/audit/README.md` finding F2 (fixed).
- **Side channels.** TFHE-rs 1.7's own README states that side-channel
  mitigation has not been implemented. This build also does not add
  constant-time key handling, memory locking, or zeroization. Key generation
  and encryption must not be treated as side-channel hardened.
- **Phone PIN strength (residual).** New PINs are 6–12 digits, so the space
  is at most 10^12 (and 10^6 at the floor). The persisted lockout bounds
  **online** guessing at the lock screen, but an attacker with a copied vault
  can guess offline; Argon2id (19 MiB, t=2) raises the per-guess cost without
  removing the risk. A longer alphanumeric passphrase would be stronger, at a
  UX cost. The backup contains the PIN hash by design.
- **Key-store memory and durability (residual).** Decrypted key material is
  not memory-locked, and TFHE-rs key types cannot be explicitly zeroized. The
  vault file is flushed before rename, but Windows has no directory fsync, so
  power-loss durability of the rename itself is not proven. The store relies
  on the OS account for file permissions; it does not set or verify ACLs.
- **Plaintext model bias.** The XNOR install request sends `bias` as a
  plaintext `i64` array; the server persists and uses it. Weight ciphertexts
  stay encrypted, but the model is not fully hidden.
- **Unaudited dependencies and build.** TFHE-rs and the Rust dependency tree
  are not audited here; the vendored `tfhe` patch changes its `crate-type` and
  must be reviewed as part of any supply-chain review.
- **DoS caps are local defaults, not a capacity model.** The 2 MiB JSON cap,
  256 MiB server-key cap, 10k-envelope batch cap, and 1,000,000-row
  aggregation cap bound individual requests, but 1M-row homomorphic work can
  still consume substantial CPU. There is no queue, admission control, or
  per-principal quota.
- **Backup consistency.** Copying `--data-dir` while the server is running is
  not a documented consistent snapshot. Stop the server (or use an
  OS/filesystem snapshot) before copying `fhe-db.log`; the WAL heals a torn
  tail but a live copy is not a supported backup procedure.
- **TLS is optional and not built on this host.** Without the `tls` feature and
  `--tls-cert/--tls-key`, traffic is plain HTTP. The committed test certificate
  is test-only.

## 5. Error-code contract

Every server error body is `{"code": "...", "message": "..."}` with a stable
machine-readable `code` (e.g. `UNKNOWN_TABLE`, `INVALID_ENVELOPE`,
`PAYLOAD_TOO_LARGE`, `RATE_LIMITED`). Codes are part of the wire contract —
clients must branch on `code`, never on `message` prose. The full mapping
lives in `crates/fhe-server/src/error.rs`.

## 6. Operational guidance

- **Run loopback-only**, or enable both `--auth-token` and TLS
  (`--features fhe-server/tls` build + `--tls-cert/--tls-key`) before any
  exposure beyond a private network.
- **Rotate the auth token** as any secret: generate it with a CSPRNG
  (`openssl rand -hex 32`), pass it via the `FHE_DB_AUTH_TOKEN` environment
  variable rather than the process list, and use per-deployment tokens.
- **Back up deliberately.** Stop the server, copy `fhe-db.log` from
  `--data-dir`, and test a restore on a separate directory before relying on
  it. Do not treat a live directory copy as a transactionally consistent
  backup.
- **Store client keys** in a secret store (OS keychain / HSM / encrypted
  file); never transmit them. On the phone, use a PIN longer than the 6-digit
  floor (12 digits maximum) and keep exported backups at least as protected as
  the device.
- **Do not expose the XNOR endpoint as a secret-model service.** Bias and model
  shape are plaintext; only weight values are encrypted.

## 7. Audit status

- **Internal audit package (roadmap stage 1.1–1.2): DONE — 2026-11-10.** The
  threat model, parameter dossier, key-lifecycle review, and per-claim
  inventory are in [`docs/audit/`](docs/audit/README.md). This is an internal
  review, not an independent audit.
- **External audit: NOT DONE (required before production).** No FHE system
  ships without an independent lattice-cryptography audit of parameters and
  implementation.
- **Fuzzing: implemented for wire and persistence decoding.** The `fuzz/`
  crate contains `fuzz_envelope` and `fuzz_db_snapshot`; CI smoke-runs both.
  Envelope deserialization is also covered by unit/property tests locally.
- **Side-channel audit: NOT DONE.** TFHE-rs documents that its own
  side-channel mitigations are not implemented; this project adds none.

## 8. Reporting

This is a development build, not a production service. For the audit and
hardening roadmap, see
`PROJECT_STATUS.md` (roadmap, measured baseline and open gaps) and the
Production-Readiness Gap List in `AGENTS.md` §13.
