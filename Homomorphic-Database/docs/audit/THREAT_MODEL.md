# FHE-DB Threat Model

**Status:** internal review, 2026-11-10. This document states the boundary the
code actually implements; it does not assert that the implementation is secure.
See [`README.md`](README.md) for findings and [`CLAIMS.md`](CLAIMS.md) for the
claim-by-claim evidence.

## 1. System and data flow

```
client / phone                         server process                     disk
┌──────────────────┐   HTTP(S)   ┌──────────────────────────┐   WAL   ┌──────────┐
│ ClientKey        │ ──────────> │ ServerKey (public)       │ ──────> │ fhe-db.log│
│ Encryptor        │  envelopes  │ EncryptedDatabase        │         │ schema +  │
│ decrypt locally  │ <────────── │ QueryEngine / XNOR       │         │ envelopes │
└──────────────────┘  ciphertext │ request metadata/metrics │         └──────────┘
                                  └──────────────────────────┘
```

1. The client generates `(ClientKey, ServerKey)` and keeps the client key.
2. The client uploads the public server key (`POST /keys/server`) and encrypted
   values (`POST /tables/{table}/rows`).
3. The server stores ciphertexts, evaluates SUM/AVG/COUNT and the XNOR model,
   and returns ciphertexts or public counts.
4. The client decrypts locally. The server never receives the client key or
   value plaintext.

This is confidentiality of **values**. It is not confidentiality of metadata,
and it is not integrity or verifiability of the computation (see §5).

## 2. Assets

| Asset | Where it lives | Primary risk |
|---|---|---|
| Plaintext field values | Client only (transient) | Disclosure via client compromise, not the server |
| `ClientKey` | Client; phone encrypted store + backups | Theft of device/backup plus PIN guessing |
| `ServerKey` | Public, server memory | Substitution/DoS, not confidentiality |
| Auth bearer token | Server config/env, client config | Disclosure via process list, logs, or plain HTTP |
| Model weight plaintext | Client only | Disclosure via client compromise |
| Model bias / architecture | Server request and memory | Direct disclosure to server operator |
| Schema, counts, access patterns | Server memory, WAL, logs, metrics | Metadata/traffic analysis |
| Database availability | Server process, WAL | DoS, WAL poisoning, disk-full |
| Code/dependency integrity | Build and runtime | Supply-chain compromise |

## 3. Actors

| Actor | Capability assumed |
|---|---|
| Data owner / client | Holds `ClientKey`; may be malicious toward the server |
| Server operator | Controls the server process, memory, logs, WAL, and responses |
| Network attacker | Can observe/modify plain HTTP; can attempt TLS interception only if the client does not verify |
| Local process / local user | Can reach loopback ports; may read the same user's files |
| Holder of a stolen backup | Has the v2 `vault.json` (PIN hash + encrypted key pair); can guess the PIN offline |
| Disk/WAL reader | Has `fhe-db.log` or a filesystem snapshot |
| Authenticated API caller | Has the one shared bearer token; no per-table identity |
| Supply-chain attacker | Can influence a dependency, build, or the vendored `tfhe` patch |

## 4. Trust boundaries

1. **Client process / phone.** The `ClientKey` and plaintext exist here. The
   phone keystore protects them at rest with a PIN-derived AES-GCM key; the
   OS, WebView2, and the user's device are trusted.
2. **HTTP boundary.** With TLS, the transport is confidential and integrity
   protected against a network attacker. Without TLS it is plain HTTP.
3. **Server process.** The server is trusted for availability and for returning
   the computation it claims to have run. FHE does not make a malicious server
   detectable by the client.
4. **Persistence boundary.** The WAL stores schema and ciphertexts in plaintext
   form (ciphertext values, plaintext metadata). There is no at-rest encryption
   of the log.
5. **Dependency boundary.** `fhe-core` is the only crate that depends on
   `tfhe`; the workspace vendors TFHE-rs 1.7.0 with a `crate-type` patch. The
   patch and the dependency tree are part of the trusted computing base.

## 5. What the server observes

This is the core of the audit. "Encrypted values" does not mean "no
information".

| Observation | How / evidence | Consequence |
|---|---|---|
| Table and column names, declared types | `TableSchema` fields are plaintext (`crates/fhe-db/src/schema.rs:74-90`); `GET /tables` returns them (`routes.rs:685-687`); paths contain names (`routes.rs:520-538`). | Business schema and entity relationships leak. |
| Row counts and batch sizes | COUNT is public (`query.rs:19-24`); aggregation responses include `count` (`routes.rs:862-935`); insert returns `inserted` (`routes.rs:781-786`); read-back returns `count` (`routes.rs:1055-1075`). | Dataset size, ingestion rate, and query selectivity leak. |
| Which column is aggregated / read back | Request path and `{"column": ...}` body; request metadata in process memory, and trace logs only when enabled (`main.rs:78-82`, `lib.rs:32-38`). | Workload and interest leak. |
| Ciphertext width and size | Envelope header `version`, `width` (`crates/fhe-core/src/ciphertext.rs:40-42`, `:272-276`); base64 length. | Width class and approximate payload volume leak. |
| Request timing and concurrency | `TraceLayer::new_for_http()` (`lib.rs:38`) is installed, but its default events are `DEBUG` and the CLI filter is `info,fhe_server=debug` (`main.rs:78-82`), so no per-request access log is emitted unless `tower_http=debug` is enabled; homomorphic work is timing-variable. | Traffic analysis and side-channel surface. |
| Peer address | `ConnectInfo<SocketAddr>` in the rate limiter (`routes.rs:295-317`); buckets keyed by `IpAddr` (`:239-267`). | Source IP (not source port); behind a proxy/NAT the bucket is shared. F1 fixed. |
| Plaintext XNOR bias and shape | `InstallXnorRequest.bias: Vec<i64>`; stored as `bias: req.bias.clone()` (`routes.rs:940-980`); used in `DenseLayer` (`:996-1030`). | The model is not fully hidden. |
| Error existence | `UNKNOWN_TABLE` / `UNKNOWN_COLUMN` distinguish absent objects; all errors carry stable codes (`crates/fhe-server/src/error.rs`). | Existence oracle for schema names. |
| Auth token presence | `Authorization` header is parsed and compared (`routes.rs:361-399`); token content is not logged, but the token is a bearer secret. | Token theft grants full API access. |
| Aggregate operation counters | `/metrics` (`routes.rs:564-571`). | Aggregate usage, not per-table values. |
| WAL contents | `WalRecord` stores schema and ciphertexts (`crates/fhe-db/src/wal.rs:59-78`); no encryption. | Disk reader sees metadata and ciphertexts. |

**Not observed by the server:** plaintext field values, the `ClientKey`,
plaintext aggregation results, plaintext neural inputs, and plaintext weight
values. It also does not receive the model bias in encrypted form: the bias is
plaintext by design.

## 6. Attack scenarios and current controls

| Scenario | Impact | Current control | Gap |
|---|---|---|---|
| Network attacker on plain HTTP | Read/modify/replay requests; steal the token | Optional TLS (feature-gated); optional bearer auth | TLS is off by default and not built on this host; no replay protection. |
| Any local process reaches loopback | Full API when auth is unset; schema reads, read-back, key/model replacement | None by default | Loopback is not a user boundary; `CorsLayer::permissive()`. **Accepted only** for the single-user local vault (F9, [`PRODUCT_MODEL.md`](PRODUCT_MODEL.md)); a blocker for any shared/remote deployment. |
| Authenticated caller exceeds quota | CPU exhaustion from 1M-row aggregations | Per-IP rate limiter and caps | F1 fixed: the quota binds across connections. Residual: proxy/NAT clients share the peer IP, and there is no queue/admission control. |
| Invalid mutation followed by crash | Server refuses to boot | Validation-under-lock before WAL append; crash recovery | F2 fixed: rejected records are never appended. Genuine complete-record corruption is still fatal; torn tails are healed. |
| Disk thief / stolen WAL | Schema, counts, ciphertexts; traffic analysis | None at rest | No log encryption, no secure erase. |
| Stolen phone backup | Offline PIN guessing; eventual key recovery | AES-256-GCM + pinned Argon2id (19 MiB, t=2); 6–12 digit PIN; persisted online lockout | F4 fixed: the KDF is pinned and recorded, and unlock failures are rate-limited. Residual: the backup contains the PIN hash, so offline guessing remains possible; no memory locking or TFHE-rs key zeroization. |
| Malicious server returns a wrong/replayed ciphertext | Client decrypts an incorrect value and cannot tell | FHE confidentiality only | No result authentication, MAC, commitment, or verifiable computation; ciphertexts are malleable. |
| Malicious client with the token | Overwrite server key/model; DoS; read all tables | One shared token | No users, ACLs, ownership, or per-tenant isolation. |
| Side-channel observer on keygen/encrypt | Potential key recovery | None | TFHE-rs README says side-channel mitigation is not implemented; no project hardening. |
| Supply-chain compromise | Arbitrary code/data access | Vendored TFHE-rs patch; lockfile | No independent review of the patch or dependency tree; no reproducible-build verification here. |
| Traffic analysis | Infer schema, counts, workload | None | Metadata is intentionally public in the current design. |

## 7. Security objectives and non-goals

**Objectives**

- Confidentiality of plaintext values against the server and disk readers,
  subject to the metadata leakage above.
- Confidentiality of the client key against the server.
- Correct width/format handling on the wire.
- Availability of acknowledged, valid mutations across a clean restart.

**Non-goals of this build**

- Hiding schema, row counts, access patterns, ciphertext sizes, or request
  timing.
- Protecting against a compromised client endpoint, OS, or WebView2 runtime.
- Protecting against coercion, physical device seizure, or a malicious user
  who shares the `ClientKey`.
- Verifiable computation or malicious-server detection.
- Multi-tenant authorization or per-user isolation.
- Side-channel-resistant key generation/encryption.
- Secure erase or "crypto-shredding".
- Managed cloud hosting (explicitly out of scope).

## 8. Questions the external reviewer should answer

1. Are the pinned 2M128 parameters, noise distributions, and decomposition
   choices consistent with the claimed IND-CPA^D target under the cited
   Lattice Estimator methodology?
2. Does the `tfhe` vendored `crate-type` patch change any behavior relevant to
   key generation, encryption, or evaluation?
3. Are the envelope parser and key parsers free of length-prefix, allocation,
   and deserialization issues beyond the existing fuzz/proptest coverage?
4. Is the phone keystore's AES-GCM construction (salt/nonce handling, no AAD,
   KDF parameter recording) sound for the claimed threat model?
5. Does the PIN-change and backup/restore flow have a safe failure mode under
   crash, disk-full, and concurrent access?
6. Is the lack of result authentication acceptable for the intended use, or
   does it require a MAC/commitment/verifiable-computation layer?
7. What side-channel testing is required before keygen/encrypt can be used on
   an untrusted host?
