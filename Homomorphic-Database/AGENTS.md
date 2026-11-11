# AGENTS.md — FHE-DB Agent Guide

> **Read this file first.** It is the single source of truth for how to work on
> this codebase: architecture, standards, build/test commands, and the
> completion bar you must clear.

---

## 1. Mission

FHE-DB is a database system that enables computation on **fully encrypted
data** using lattice-based cryptography (TFHE). The server stores and
aggregates ciphertexts **without ever seeing plaintext**; only the client holds
the secret key and can decrypt.

The single source of truth for project status, the measured quality baseline,
and the production roadmap is [`PROJECT_STATUS.md`](PROJECT_STATUS.md). Do not
duplicate status in other docs; link there instead. The internal audit package
that gates the external cryptographic review is
[`docs/audit/README.md`](docs/audit/README.md).

The scope of the current build is the encrypted core, database, API, SDK and
inference stack, implemented per the architectural notes in the crate sources:

- ✅ `fhe-core` — FHE primitives over TFHE-rs (keygen, encryption, ciphertext
  algebra, serialization envelopes)
- ✅ `fhe-db` — encrypted columnar database engine (schemas, storage,
  homomorphic SUM / AVG / COUNT)
- ✅ `fhe-server` — axum REST API exposing encrypted database operations
- ✅ `fhe-client` — HTTP client SDK (keygen, encrypt, insert, aggregate, decrypt)
- ✅ `examples/encrypted_sum` — end-to-end encrypted SUM/AVG/COUNT demo
- ✅ `fhe-core::nn` — FHE-optimized neural layers (polynomial activations,
  dense layers over encrypted weights, fixed-point encoding)
- ✅ `examples/encrypted_nn` — encrypted inference demo (obfuscated weights)
- ✅ `examples/encrypted_iris` — real-dataset (UCI iris) encrypted inference demo
- ✅ `apps/fhe-phone` — Android-style webview shell over the real embedded
  server and client SDK

---

## 2. The Standard — Boil the Ocean

Every task must be completed to a standard that is genuinely impressive, not
politely acceptable. The operational definition, from
`Boil-The-Ocean.md`:

- **The marginal cost of completeness is near zero with AI. Do the whole
  thing.**
- Do it right. Do it with tests. Do it with documentation.
- Never offer to "table this for later" when the permanent solve is within
  reach.
- Never leave a dangling thread when tying it off takes five more minutes.
- Never present a workaround when the real fix exists.
- Search before building. Test before shipping. Ship the complete thing.
- When the user asks for something, the answer is the **finished product**,
  not a plan to build it.
- Time is not an excuse. Fatigue is not an excuse. Complexity is not an
  excuse.

**Practical checklist before declaring any task done:**

- [ ] Builds clean with `cargo build --workspace` (and `--release`)
- [ ] `cargo test --workspace` passes (unit + integration + doc tests)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` is clean
- [ ] `cargo fmt --check` is clean
- [ ] New public API has rustdoc with `# Errors` / `# Panics` sections
- [ ] No `unwrap()`/`expect()`/`panic!()` in library code paths that can
      observe untrusted input (server/API layers)
- [ ] Errors are typed and first-class (no stringly-typed errors)
- [ ] No dangling stubs, TODO markers, or "implemented later" comments
- [ ] New behavior is covered by tests (including failure cases)

---

## 3. Workspace Layout

```
FHE-DB/
├── AGENTS.md                  ← you are here
├── Cargo.toml                 ← workspace manifest, shared deps, release profile
├── PROJECT_STATUS.md          ← authoritative status + production roadmap
├── docs/audit/               ← threat model, parameters, claims, evidence pack,
│                               engagement plan, format freeze
├── docs/operations/          ← operator runbooks (WAL recovery)
├── crates/
│   ├── fhe-core/              ← FHE primitives (ONLY crate that depends on tfhe)
│   │   └── src/
│   │       ├── lib.rs         ← ScalarWidth, module exports, prelude
│   │       ├── error.rs       ← FheError, FheResult
│   │       ├── keys.rs        ← ClientKey, ServerKey, ServerKeyGuard
│   │       ├── encrypt.rs     ← Encryptor (client-side encryption)
│   │       ├── ciphertext.rs  ← Ciphertext, envelope, EncryptedColumn, PlaintextValue
│   │       ├── algebra.rs     ← CiphertextAlgebra (server-side homomorphic ops)
│   │       └── nn/            ← FHE neural layers (fixed-point, activations, layers)
│   │           ├── mod.rs     ← re-exports + module overview
│   │           ├── fixed_point.rs ← signed Qm.n encode/decode
│   │           ├── activation.rs  ← polynomial activations (square, sigmoid)
│   │           └── layers.rs      ← DenseLayer, FeedForward (encrypted weights)
│   ├── fhe-db/                ← encrypted columnar database engine
│   │   └── src/
│   │       ├── lib.rs         ← crate root, re-exports
│   │       ├── schema.rs      ← ColumnType, ColumnDef, TableSchema, SchemaError
│   │       ├── database.rs    ← EncryptedDatabase (tables, persistence)
│   │       └── query.rs       ← QueryEngine (homomorphic SUM/AVG/COUNT)
│   ├── fhe-server/            ← axum REST API server
│   │   └── src/
│   │       ├── lib.rs         ← router construction, AppState
│   │       ├── main.rs        ← CLI entrypoint (clap)
│   │       └── routes.rs      ← HTTP handlers
│   └── fhe-client/            ← HTTP client SDK
│       └── src/
│           ├── lib.rs         ← FheClient, ClientError
│           └── client.rs      ← HTTP transport, key/ciphertext (de)serialization
├── apps/
│   └── fhe-phone/             ← Android-style phone shell (.exe)
│       ├── build.rs           ← deploys vendored WebView2Loader.dll beside the exe
│       ├── WebView2Loader.dll ← vendored WebView2 COM loader (wry runtime dep)
│       └── src/
│           ├── main.rs        ← boot embedded server + open the shell
│           ├── lib.rs         ← crate root, re-exports
│           ├── engine.rs      ← FHE session: real FheClient against the embedded server
│           ├── server.rs      ← embedded axum fhe-server + wire-tap middleware
│           ├── shell.rs       ← wry/tao window + IPC bridge
│           └── wire.rs        ← live wire-record ring for the Monitor app
│       └── assets/
│           └── index.html     ← Material-You Android UI (lock screen, launcher, 6 apps)
└── examples/
    ├── encrypted_sum/         ← end-to-end demo binary
    │   └── src/main.rs
    └── encrypted_nn/          ← encrypted XNOR inference demo
        └── src/main.rs
```

### Crate dependency rules

1. **`fhe-core` is the only crate that may depend on `tfhe`.** This isolates
   the whole workspace from TFHE-rs type/API changes and keeps the security
   boundary reviewable.
2. `fhe-db` depends on `fhe-core` only.
3. `fhe-server` depends on `fhe-core` + `fhe-db`.
4. `fhe-client` depends on `fhe-core` only (it does not need the DB engine;
   it talks to the server over HTTP).
5. Examples may depend on any of the above.

---

## 4. Security Model

This is a **zero-trust** architecture. The invariants are non-negotiable:

| Invariant | Enforcement |
|-----------|-------------|
| The server never receives plaintext **values** | Only public `ServerKey` is sent; values cross as envelopes. Schema, counts, timing, and the XNOR bias are visible metadata (see `docs/audit/THREAT_MODEL.md`). |
| Only the holder of `ClientKey` can decrypt | The key is never sent to the server; the phone stores it encrypted at rest and backups contain it encrypted. PIN strength governs backup confidentiality. |
| Computation results are **not** authenticated | FHE gives confidentiality, not malicious-server integrity; a wrong/replayed ciphertext decrypts without detection. No MAC/commitment/verifiable computation in this build. |
| Width mismatches never slip through | Versioned `CiphertextEnvelope` + width-checked `EncryptedColumn::push` |
| Out-of-range values rejected before encryption | `Encryptor` validates against `ScalarWidth` before FHE work |
| Errors are first-class, never panics | `FheError`/`SchemaError`/`DbError`/`ClientError` cover all fallible paths |
| No key material in debug output | `ClientKey`/`ServerKey` implement `Debug` as redacted structs |

**Product model (2026-11-11):** FHE-DB is a **single-user, local-first
encrypted vault**. The bearer token is an operator access control, not user
identity; there are no tenants or ACLs. Audit finding F9 is accepted only
under the loopback/single-OS-user conditions in
[`docs/audit/PRODUCT_MODEL.md`](docs/audit/PRODUCT_MODEL.md) and reopens as a
blocker for any shared, remote, or multi-user deployment.

**Operational flow (zero-trust):**

1. Client generates `(ClientKey, ServerKey)`.
2. Client sends only `ServerKey` to the server (`POST /keys/server`).
3. Client encrypts values and sends ciphertext envelopes to the server.
4. Server installs the server key and evaluates homomorphic aggregations —
   it sees ciphertexts plus schema/count metadata, never value plaintext.
5. Server returns encrypted results + public row counts.
6. Client decrypts with `ClientKey`; for AVG, divides by the public count.

---

## 5. Architecture Notes

### fhe-core

- **`ScalarWidth`** (`U8`/`U16`/`U32`/`U64`) is the runtime width tag. TFHE-rs
  has distinct types per width; this enum lets higher layers be width-agnostic.
- **`Ciphertext`** is the width-tagged enum wrapping the four concrete TFHE-rs
  ciphertext types. It is the "any ciphertext" passed across the workspace.
- **`CiphertextEnvelope`** is the versioned wire/storage format
  (`version` byte + width + payload). `CURRENT_ENVELOPE_VERSION = 1`.
- **`EncryptedColumn`** is the storage unit for `fhe-db`: a width-homogeneous
  list of ciphertexts.
- **`CiphertextAlgebra`** performs server-side homomorphic `add`, `mul`,
  `sum_column`, and `average_column`. `average_column` returns encrypted sum +
  public count because homomorphic division is prohibitively deep; the client
  divides after decryption (standard FHE practice).
- **`ServerKeyGuard`** installs the thread-local TFHE server key for the
  duration of a scope. Every request handler that computes on ciphertexts
  must create one.

### fhe-db

- Columnar, not row-based: homomorphic aggregation operates per column.
- `TableSchema` holds column *names* in plaintext (metadata) — only values are
  encrypted.
- `EncryptedDatabase` is in-memory with bincode persistence to disk.
- `QueryEngine` runs homomorphic SUM / AVG / COUNT with full error handling
  (unknown table/column, empty column, width mismatch).

### fhe-server

- axum 0.8 router. All state is in `AppState`.
- Endpoints (see §7 for the full table).
- Bearer-token auth middleware (`--auth-token` / `FHE_DB_AUTH_TOKEN`,
  constant-time comparison, `GET /health` exempt) — opt-in.
- Optional HTTPS via axum-server/rustls behind the default-off `tls` cargo
  feature (`--tls-cert/--tls-key`); CI builds + tests it on ubuntu.
- CORS enabled via `tower-http`; request tracing via `tracing-subscriber`.
- Clap CLI: `--bind`, `--port`, `--data-dir`, `--save-interval-secs`,
  `--auth-token`, `--tls-cert`, `--tls-key`.
- Hardened deserialization: envelope headers are hand-parsed and length
  prefixes validated before bincode allocates (length-prefix bombs are
  rejected — see `fhe_core::Ciphertext::from_envelope_bytes`); all key/DB
  decodes are bounded by the input size.

### fhe-client

- `FheClient` wraps `reqwest` and the `fhe-core` types.
- `keypair()` generates and returns `(ClientKey, ServerKey)`.
- `with_auth(url, token)` sends `Authorization: Bearer <token>` on every
  request for servers started with `--auth-token`.
- Uploads server key, pushes encrypted columns, runs aggregations, decrypts
  results. `sum_plaintext()` / `avg_plaintext()` / `count()` are the
  high-level end-user API.

---

## 6. Build, Test, and Quality Commands

Run from the workspace root (the checkout directory containing this file):

```bash
# Check (fast, no codegen)
cargo check --workspace

# Build (debug)
cargo build --workspace

# Build (release — recommended for FHE work; tfhe is much faster optimized)
cargo build --release --workspace

# Test everything (unit + integration + doc tests)
cargo test --workspace

# Test a single crate
cargo test -p fhe-core

# Lint (must be clean before shipping)
cargo clippy --workspace --all-targets -- -D warnings

# Format
cargo fmt --all
cargo fmt --all -- --check

# Run the end-to-end example
cargo run --release -p encrypted_sum

# Run the server (clap CLI: --bind/--port/--data-dir; see --help)
cargo run --release -p fhe-server -- --bind 127.0.0.1 --port 8080
cargo run --release -p fhe-server -- --bind 127.0.0.1 --port 8080 --data-dir ./data
# Require a bearer token (every route except GET /health)
cargo run --release -p fhe-server -- --auth-token "$(openssl rand -hex 32)"
# Serve HTTPS instead of HTTP (build with the tls cargo feature first)
cargo build --release --workspace --features fhe-server/tls
cargo run --release -p fhe-server -- --tls-cert cert.pem --tls-key key.pem

# Run the real-dataset encrypted classification demo (UCI iris)
cargo run --release -p encrypted_iris

# Fuzz the wire deserialization paths (needs clang; see fuzz/README.md)
cd fuzz && cargo fuzz run fuzz_envelope

# Benchmark the primitives (see scripts/check-bench-regression.sh)
cargo bench -p fhe-core --bench envelope -- --quick --noplot

# Run the Android-style phone shell (.exe): embedded fhe-server + real
# fhe-client SDK + Material-You webview UI
cargo run --release -p fhe-phone
```

> **Note:** property tests live in `fhe-core/tests/properties.rs` and are
> gated behind the `proptest` dev-dependency. Keep algebraic-invariant
> property tests in `fhe-core` only.

> **Release profile:** the workspace pins `codegen-units = 1` for maximum
> runtime performance, but the enormous `tfhe` crate is **scoped overridden**
> to `codegen-units = 16` (see `[profile.release.package.tfhe]` in the
> workspace `Cargo.toml`) so its first codegen completes within a single
> build window. All other crates keep the global setting. Do not remove the
> override; it is why `tfhe` first-builds reliably.

**Windows cmd note:** there is no `head`/`tail`/`grep`; use
`findstr`, `more`, or PowerShell equivalents when filtering command output.

> **Windows build environment (historical note, 2026-08):** the MSVC
> toolchain **cannot link** here — the Windows SDK `Lib` tree is not
> installed (no `kernel32.lib`/`dbghelp.lib` anywhere). The workspace
> therefore builds with the **self-contained GNU toolchain**, which rustup
> bundles completely:
>
> - `rustup default nightly-x86_64-pc-windows-gnu` is the active default.
> - GNU `dlltool.exe`/`as.exe` (binutils 2.43.1, mingw-w64 build from the
>   MSYS2 mirror) live in `~/.cargo/bin` — required by rustc's raw-dylib
>   import-lib generation for windows-gnu targets.
> - `.cargo/config.toml` pins `target-dir = "C:/fhe-target"`: the workspace
>   path contains a space and GNU `as` cannot write its temp files there
>   ("can't create C:\Users\INTAL: Permission denied").
>
> All documented commands below work as written on this machine.

> **Measured 2026-11-10:** on this checkout the default toolchain is
> `stable-x86_64-pc-windows-msvc` (rustc 1.97.1) and
> `cargo build/test --release --workspace` links and runs with it. The GNU
> workaround above is retained only for machines whose MSVC Windows SDK
> (`Lib` tree) is genuinely missing.

---

## 7. REST API Reference (fhe-server)

| Method | Path             | Body / Query                                   | Returns                                   |
|--------|------------------|------------------------------------------------|-------------------------------------------|
| POST   | `/keys/server`   | raw server key bytes (`application/octet-stream`) | `204 No Content`                        |
| POST   | `/tables`        | `{"name": "...", "columns":[{"name":"...","column_type":"U32"}]}` | `200 {"name": "...", "columns": [...]}` |
| GET    | `/tables`        | —                                              | `200` list of table schemas                |
| POST   | `/tables/{table}/columns` | `{"name":"...","column_type":"U32"}` | `200` updated schema            |
| POST   | `/tables/{table}/rows` | `{"column": "<name>", "ciphertexts": ["<base64 envelope>", ...]}` | `200 {"inserted": n}` |
| POST   | `/tables/{table}/aggregations/sum` | `{"column":"<name>"}` | `200 {"sum":"<base64 envelope>","count":n}` |
| POST   | `/tables/{table}/aggregations/avg` | `{"column":"<name>"}` | `200 {"sum":"<base64 envelope>","count":n}` |
| POST   | `/tables/{table}/aggregations/count` | `{"column":"<name>"}` | `200 {"count":n}`            |
| GET    | `/tables/{table}/ciphertexts/{column}` | — | `200 {"column":"...","count":n,"ciphertexts":["<base64 envelope>",...]}` |
| POST   | `/models/xnor`   | `{"weights":["<base64>",...],"bias":[...]}` | `204 No Content`            |
| POST   | `/models/xnor/infer` | `{"inputs":["<base64>","<base64>"]}` | `200 {"model":"xnor","output":"<base64 envelope>"}` |
| GET    | `/metrics`       | —                                              | `200` Prometheus-text counters (HTTP, rows, WAL recovery/checkpoint) |
| GET    | `/health`        | —                                              | `200 {"status":"ok"}`                      |

All ciphertexts crossing the wire are base64-encoded `CiphertextEnvelope`
bytes. Row counts (`count`) are public metadata.

**Authentication:** when the server is started with `--auth-token` (or
`FHE_DB_AUTH_TOKEN`), every request except `GET /health` must carry
`Authorization: Bearer <token>`; missing/wrong tokens get `401 UNAUTHORIZED`
with code `UNAUTHORIZED`. The SDK sends it automatically via
`FheClient::with_auth(base_url, token)`.

**Transport:** with `--tls-cert/--tls-key` (requires the `tls` cargo
feature) the server speaks HTTPS instead of HTTP.

**Error bodies:** every non-2xx response is
`{"code": "<stable-code>", "message": "<human text>"}`. The `code` string is
part of the wire contract — clients branch on it, never on `message` prose.
Codes include `UNKNOWN_TABLE`, `UNKNOWN_COLUMN`, `TABLE_EXISTS`,
`COLUMN_EXISTS`, `WIDTH_MISMATCH`, `EMPTY_COLUMN`, `INVALID_BASE64`,
`INVALID_ENVELOPE`, `HOMOMORPHIC_EVAL`, `NO_SERVER_KEY`, `NO_MODEL`,
`PAYLOAD_TOO_LARGE`, `RATE_LIMITED`, `SCHEMA_INVALID`,
`CORRUPTED_DATABASE`, `IO_ERROR`, `FHE_ERROR`, `SERIALIZATION_ERROR`,
`BAD_REQUEST`, `NOT_FOUND`, `CONFLICT`, `INTERNAL`. The full mapping lives in
`crates/fhe-server/src/error.rs`.

**DoS guards (see `fhe_server::routes::Limits`):** 2 MiB JSON body cap
(256 MiB for `/keys/server`), insert batch caps (10k envelopes, 4 MiB per
envelope), a 1M-row aggregation cap (the "max price" guard), schema size caps
(128-char names, 256 columns), and a rate limiter (1000 req/min general,
120 req/min on aggregations/inference; `429 RATE_LIMITED`). The limiter is
keyed by peer `IpAddr`, so the quota is shared across TCP connections from the
same IP; a new connection does not reset it. The peer address is trusted as
reported by `ConnectInfo`, so behind a reverse proxy all clients share the
proxy's bucket until a trusted forwarded-for policy is added (finding F1
fixed; regression `rate_limiter_shares_one_bucket_per_ip_across_connections`).

---

## 8. Coding Conventions

- **Edition/rust-version:** workspace manifest pins `edition = "2021"` and
  `rust-version = "1.91.1"`. The installed toolchain on this checkout is
  stable 1.97.1 (MSVC default; see §6).
- **Errors:** use `thiserror`; define a crate-local error enum and a
  `type XResult<T> = Result<T, X>`. Never return `String` errors.
- **`unwrap`/`expect`:**
  - Allowed in tests and examples.
  - In library/server code, only where the failure is a programmer error and
    the invariant is provably enforced upstream (e.g. after a width check).
  - In server request handlers, **never** on data parsed from the wire.
    Deserialize envelopes with `Ciphertext::from_envelope_bytes` and return
    `400` on error.
- **Server key pinning:** any handler that performs homomorphic ops wraps its
  work in `ServerKeyGuard::new(&server_key)`.
- **Docs:** every public item has rustdoc. Fallible functions document
  `# Errors`. Security-adjacent docs state the invariant.
- **Naming:** `snake_case` for functions/fields, `CamelCase` for types,
  `SCREAMING_SNAKE` for constants.
- **Comments:** explain *why*, not *what*. Security-relevant comments call out
  the invariant being protected.

---

## 9. Testing Strategy

| Layer | Coverage |
|-------|----------|
| `fhe-core` unit tests | envelope roundtrips (all widths), version/malformed rejection, width enforcement, randomized encryption, out-of-range rejection, all-width encryption |
| `fhe-core` format freeze | envelope version, golden-corpus SHA-256/lengths, pinned parameter values (`tests/format_freeze.rs`) |
| `fhe-db` format freeze | WAL magic/version/header constants and a byte-level fresh-header check (`tests/format_freeze.rs`) |
| `fhe-core` doc tests | `Encryptor` example in rustdoc |
| `fhe-db` unit tests | schema validation, insert/retrieve, persistence roundtrip |
| `fhe-db` integration | SUM on live ciphertexts (server key installed), AVG with client-side division, COUNT on empty column |
| `fhe-server` | axum handler tests via `tower::ServiceExt::oneshot` (no network) |
| `fhe-client` | deserialization of aggregation responses, error mapping |
| `fhe-phone` keystore | vault v2 roundtrip, pinned/recorded KDF, v1 migration, v1 backup import, persisted lockout, injected-commit-failure atomicity, authenticated destroy |
| `fhe-phone` engine | headless end-to-end flow over the embedded server, including lock/keygen/aggregate/read-back and IPC change-PIN/destroy |
| `examples/encrypted_sum` | full end-to-end check in `main`; run with real encryption |
| property tests | active in `fhe-core/tests/properties.rs` (`proptest`: additive identity, multiplicative identity, sum-of-two) |

**Golden rule:** when implementing a feature, add the test **that would have
caught the bug you are fixing** and the tests that pin the new behavior.

---

## 10. Troubleshooting

| Symptom | Likely cause / fix |
|---------|--------------------|
| `cargo build` very slow on first run | tfhe is enormous; use `--release` and expect a long first compile (10+ min). Subsequent builds are incremental. |
| Build killed by a 10-minute tool/harness timeout mid-`tfhe` | Cargo is incremental: the partially-written `.rcgu.o` object files survive. Simply re-run `cargo build --release --workspace`; each run resumes where the last left off until `tfhe` completes. Do **not** `cargo clean` — that discards the progress. |
| `failed to add native library ...\kernel32.dll_imports.lib: ... (os error 2)` while compiling `parking_lot_core` | The GNU `dlltool.exe`/`as.exe` in `~/.cargo/bin` crash at launch with `STATUS_DLL_NOT_FOUND` because mingw `libintl-8.dll` requires `libiconv-2.dll`, which is missing from `~/.cargo/bin`. rustc can therefore never generate the import library. Fix: `copy "C:\Program Files\Git\mingw64\bin\libiconv-2.dll" "%USERPROFILE%\.cargo\bin\"` (the Git copy depends only on system KERNEL32/msvcrt). See the note in `.cargo/config.toml`. |
| `ld: error: export ordinal too large: 165566` / `rust-lld: error: too many exported symbols (got 165566, max 65535)` while linking `tfhe` | `tfhe` declares `crate-type = ["lib", "staticlib", "cdylib"]`, so cargo must build its cdylib as a PE DLL. That DLL exports 165,566 symbols — beyond the PE/COFF export-table cap of 65,535 ordinals — so **no linker** (GNU ld.bfd nor LLVM lld) can produce it. The workspace links `tfhe` only as an rlib, so the impossible cdylib artifact is removed by the vendored patch: `vendor/tfhe` ships the exact 1.7.0 source with `crate-type = ["lib"]`, wired via `[patch.crates-io] tfhe = { path = "vendor/tfhe" }` in the workspace `Cargo.toml`. |
| Doctests fail to link: `lld: error: unable to find library -lbcrypt / -lkernel32 ...` | `rustdoc` does not receive `[target.*].rustflags`, so runnable doc examples link without `-C link-self-contained=yes` and rust-lld cannot find the mingw/system import libs in the rustdoc temp dir. Fixed by mirroring the flag under `[target.x86_64-pc-windows-gnu] rustdocflags` in `.cargo/config.toml` (see the comment there). If the flag is ever removed, re-add it to both `rustflags` and `rustdocflags`. |
| `WebView2Loader.dll was not found` when launching `fhe-phone.exe` | `wry` links against `WebView2Loader.dll`, but `webview2-com-sys`'s `build.rs` only copies that DLL into its private `OUT_DIR` for the linker — never beside the built exe. Fixed by `apps/fhe-phone/build.rs`, which auto-deploys the vendored `apps/fhe-phone/WebView2Loader.dll` next to the produced binary on every build. If the DLL is missing, run `cargo build --release -p fhe-phone` (or copy the vendored DLL manually). |
| "no server key installed" TFHE error | Create a `ServerKeyGuard` before homomorphic ops; or the server key was never uploaded. |
| `WidthMismatch` on insert | Column width differs from ciphertext width. Check the schema's `ColumnType` matches the `ScalarWidth` used at encryption. |
| `UnknownEnvelopeVersion` | Ciphertext produced by an older/newer build; `CURRENT_ENVELOPE_VERSION` changed. Re-encrypt. |
| `CorruptedCiphertext` | Packet corrupted in transit or base64 mangled. Re-send. |
| Port already in use | Pass `--port` to `fhe-server`. |
| Server returns `400` on aggregation | Envelope failed to deserialize or the table/column does not exist. Check the error body (`message` field). |

---

## 11. Roadmap Context

The authoritative roadmap is [`PROJECT_STATUS.md`](PROJECT_STATUS.md) §3, which
records measured status and exit criteria. As of 2026-11-10:

| Stage | Scope | Status |
|-------|-------|--------|
| 0. Trustworthy baseline | Real quality gate, measured results, consistent docs, crate boundary | **Done** |
| 1. Security & release blockers | External crypto audit, claim/threat-model and lifecycle review | **In progress** — internal 1.1–1.2 package done; F4/F8 fixed, F9 accepted for the single-user local vault; external audit **Blocked** |
| 2. Database & service reliability | Crash/restart recovery, semantics, limits, versioning, runbooks | **In progress** — durability slice landed (process-level crash/restart tests + WAL recovery runbook); load limits, versioning, and remaining semantics open |
| 3. Bounded query & inference | Documented query semantics, wider-width NN, bootstrapping-aware training | **Not started** |
| 4. Usable distribution | `fhe-phone` packaging, signing, clean-machine test | **Not started** |
| 5. Performance, operations, release | Measured optimization, operational checks, release review | **Not started** |

Managed cloud hosting is **not in scope**. Before designing new capabilities,
read the relevant stage and its exit criteria in `PROJECT_STATUS.md`.

---

## 12. Handoff Checklist

Before finishing any task, update this file if:

- [ ] The workspace layout changed (new crates/modules)
- [ ] The REST API surface changed
- [ ] The security model changed
- [ ] New invariants or conventions were established
- [ ] The build/test/quality commands changed
- [ ] Roadmap status changed

Then verify the §2 "Practical checklist" is fully checked.

---

## 13. Production-Readiness Gap List (current implemented scope)

FHE-DB is **not production-ready**. Live status, the measured baseline, and the
production roadmap are in [`PROJECT_STATUS.md`](PROJECT_STATUS.md) — do not
duplicate readiness claims here. This list covers only gaps in already
implemented scope, grouped by area with an effort rating (S/M/L). Planned
phases (SMPC, CUDA/GPU, managed cloud) are tracked in `PROJECT_STATUS.md`.

### Database durability & semantics
- [x] **WAL / crash recovery** — every mutation is validated against the locked database, then journaled to `fhe-db.log` *before* it is applied (write-ahead ordering); boot recovers valid committed state, migrates a legacy pre-WAL `database.bin` exactly once, and heals torn tails. A rejected mutation leaves no record (audit F2 fixed), pinned by `rejected_mutations_do_not_poison_the_wal`; complete-record corruption and torn-tail recovery remain pinned by `corrupt_complete_record_is_rejected` and `torn_tail_is_truncated`; checkpoint lock order is centralized in `AppState::checkpoint_wal` and pinned by `checkpoint_acquires_database_before_wal` (audit F11 fixed). `Wal::recover` also creates the log on first boot and decodes an empty snapshot as an empty DB (2026-08-05 pass). The stage-2 durability slice added process-level coverage against the real binary — `crates/fhe-server/tests/durability.rs`: graceful stop (SIGTERM / Ctrl-Break) exits 0 and compacts, acknowledged rows survive a hard kill, and a truncated tail heals on boot — plus the WAL crash-boundary matrix in `fhe-db`, and the operator runbook [`docs/operations/WAL_RECOVERY.md`](docs/operations/WAL_RECOVERY.md). Recovery and checkpointing are observable: `Wal::recover_with_report` / `Wal::checkpoint_with_report` feed one structured `wal.recover` / `wal.checkpoint` log line each and the `fhe_wal_*` `/metrics` counters (torn-tail bytes, records replayed/compacted, snapshot bytes, duration), pinned by `recovery_report_counts_records_snapshot_and_torn_tail`, `checkpoint_report_counts_only_appends_since_the_previous_checkpoint`, `wal_recovery_and_checkpoint_metrics_are_exposed`, and the durability output assertions. (L)
- [x] **Concurrency contract** — documented in `fhe-db` module docs + enforced by the `concurrent_inserts_are_serialized_and_counted` server test (8×25 concurrent inserts → 200 rows). (M)
- [x] **Transaction / batch semantics** — insert batches are all-or-nothing (validated before any write); pinned by `batch_insert_is_all_or_nothing` (DB layer) and `partial_batch_leaves_column_untouched` (server layer). `inserted` == batch length or the whole batch is rejected. (M)
- [x] **Backup / restore story** — `fhe-server --data-dir` recovers `fhe-db.log` on boot and checkpoints on an interval and graceful shutdown; the supported offline backup is a stopped-server copy of that log (not the legacy `database.bin`). Roundtrip pinned by `save_and_load_file_roundtrip`, `load_rejects_corrupt_file`, `restored_database_is_served`. F2 no longer makes restart unsafe after a rejected mutation; live-copy consistency is still not supported. Operator procedure: [`docs/operations/WAL_RECOVERY.md`](docs/operations/WAL_RECOVERY.md). (S)
- [x] **Cross-version data migration test** — golden corpus (`fhe-core/tests/golden/`: committed v1 key + envelope) fails the build if `CURRENT_ENVELOPE_VERSION` changes without a deliberate migration. (S)

### Server hardening (auth, DoS, observability)
- [x] **Authentication present; authorization intentionally absent** — `fhe-server --auth-token <token>` (or `FHE_DB_AUTH_TOKEN`) requires `Authorization: Bearer <token>` on every request except `GET /health`; the comparison is constant-time; `FheClient::with_auth(url, token)` sends the header. There are no users, tenants, ACLs, or ownership checks (audit F9). That is **accepted only for the single-user local-vault model** ([`docs/audit/PRODUCT_MODEL.md`](docs/audit/PRODUCT_MODEL.md)) and reopens as a blocker for any shared, remote, or multi-user deployment. Tests: `auth_gates_everything_except_health`, `auth_is_optional_by_default`. (L)
- [ ] **DoS resistance (partial)** — per-route body caps (2 MiB JSON / 256 MiB server-key), insert batch caps (envelope count + per-envelope size), aggregation row-cap (max-price guard), schema size caps, and a per-IP rate limiter with a stricter quota on expensive endpoints. F1 is fixed (IP-keyed buckets, shared across connections); the limiter still trusts the peer address (proxy caveat), and there is no admission queue. Remains partial. Tests: `insert_batch_cap_rejects_oversized_batches`, `aggregation_row_cap_rejects_large_columns`, `schema_caps_reject_oversized_schemas`, `oversized_json_body_rejected_with_413`, `rate_limiter_trips_after_quota`, `rate_limiter_shares_one_bucket_per_ip_across_connections`. (M)
- [x] **TLS / transport security** — `--tls-cert/--tls-key` serve HTTPS via rustls (axum-server), built with the default-off `tls` cargo feature (its aws-lc-rs provider needs a C compiler, so CI owns the build; the local host builds without it). Integration test `tls_serves_https_with_auth` runs over a real rustls listener with a committed test-only cert. (M)
- [x] **Metrics endpoint** — `GET /metrics` (Prometheus text, dependency-free counters: requests by status class, rate-limited, envelope bytes, rows inserted, key uploads, tables, columns, aggregations, inferences, models, read-backs) plus the `fhe_wal_*` recovery/checkpoint counters and the `fhe_wal_log_bytes` gauge. Tests: `metrics_expose_counters`, `wal_recovery_and_checkpoint_metrics_are_exposed`. (M)
- [x] **Structured error taxonomy on the wire** — every error body carries a stable machine-readable `code` (full list in §7 and `error.rs`); the client SDK exposes `code` on `ClientError::Server`. Test: `error_bodies_carry_stable_codes`. (S)

### Cryptography & audit
- [ ] **External security audit** — no FHE system ships without an independent lattice-crypto audit of parameters + implementation. (L, external)
- [ ] **Internal audit findings F1–F11** — F1 (connection-keyed limiter), F2 (WAL poisoning by rejected mutations), F4 (phone keystore pinning/atomicity/lockout/zeroization/destruction), F8 (PIN-change atomicity), and F11 (checkpoint lock-order inversion) are fixed with regression coverage; F9 (no authorization/tenancy) is **accepted** only for the single-user local-vault model; F3, F5, F6, F7, and F10 remain open. Evidence and severity: `docs/audit/README.md`.
- [x] **Parameter documentation + enforcement** — values pinned in `fhe_core::SecurityParameters` (918-dim LWE, 2048 polynomial, 4/4 msg/carry, p-fail 2^-129.58) and enforced by the `default_parameters_match_documented_pinning` gate test. The 128-bit IND-CPA^D label is TFHE-rs's vendor/Lattice-Estimator claim, not a project proof; see `docs/audit/PARAMETERS.md`. (S)
- [x] **Side-channel posture note** — `SECURITY.md` §4 states that keygen/encrypt are not constant-time in this build (TFHE-rs implements no side-channel mitigation) and gives deployment guidance; no mitigation is implemented. (M)

### Testing & benchmarks
- [x] **Enable property tests** — `proptest` properties active in `fhe-core/tests/properties.rs` (additive identity, multiplicative identity, sum-of-two; 16 cases each over random U8 plaintexts). (S)
- [x] **Fuzz harness** — standalone cargo-fuzz crate (`fuzz/`): `fuzz_envelope` (wire envelope deserialization) + `fuzz_db_snapshot`; CI job builds and smoke-runs both on ubuntu (libFuzzer needs clang). Local no-clang hosts pin the same invariants via proptest + bomb-rejection tests. The harness surfaced and fixed a real length-prefix bomb (bounded decodes + header validation, see `ciphertext.rs`). (M)
- [x] **Adversarial / DoS HTTP tests** — oversized bodies (2 MiB JSON cap, 256 MiB server-key route), oversized batches, aggregation row-cap, schema caps, rate-limit exhaustion, malformed envelopes, all with stable error codes. (M)
- [x] **Benchmark suite + regression guard** — criterion suite in `crates/fhe-core/benches/envelope.rs` (roundtrips all widths, encryption, homomorphic SUM, DoS-rejection path); hardware-robust ceiling guards in `tests/bench_guard.rs` run inside `cargo test`; `scripts/check-bench-regression.sh` for fixed-machine baseline comparisons; CI smoke job. (M)
- [x] **CI pipeline** — `.github/workflows/ci.yml` runs `build --release`, `test --release`, `clippy -D warnings`, `fmt --check` on push/PR. (M)

### fhe-phone as a product
- [x] **Persistent key storage (qualified)** — the `(ClientKey, ServerKey)` pair is stored in one `vault.json` under `%APPDATA%/FHE-DB`, encrypted with AES-256-GCM under an Argon2id PIN-derived key (`keystore.rs`). The vault records the **pinned** parameters (19 MiB, t=2, p=1, 32-byte output, v0x13), uses a 6–12 digit PIN, persists a 5-attempt lockout (30 s doubling to 1 h), and commits every save/change/import in one atomic write. Derived keys, plaintext key buffers, and the in-memory PIN are zeroized; a legacy v1 two-file store is migrated. F4/F8 fixed. Residuals: offline PIN guessing remains possible, there is no memory locking, and TFHE-rs key types expose no zeroization API. On unlock the server key is re-uploaded automatically. (L)
- [x] **App lock** — real 6–12 digit PIN gate on the lock screen (keypad, dots, wrong-PIN/lockout feedback); Settings → Security manages Set/Change PIN, Lock now, backup export/import, and Destroy keys. A legacy 4-digit PIN can still unlock an existing store. (M)
- [ ] **Packaging** — installer, code signing, WebView2 bootstrapper for fresh machines (loader DLL is deployed; the Evergreen runtime itself must be present). (M)
- [x] **On-device backups / export (qualified)** — `backup.export` writes a v2 backup containing the whole vault (PIN hash + AES-GCM ciphertext); `backup.import` accepts v2 and legacy v1 backups, validates KDF bounds, and replaces the vault in one atomic commit (Settings → Security). The backup is copyable and unauthenticated, so confidentiality still depends on the PIN against offline guessing; see `docs/audit/KEY_LIFECYCLE.md`. (M)
- [x] **Error surface in the UI** — verified already implemented: every IPC op returns `{ok:false, error: <actionable text>}` and the UI surfaces it via `toast(r.error)` (see `assets/index.html`); engine errors carry guidance (e.g. "No keys generated yet. Open the Vault app..."). (S)

### NN layer
- [ ] **Bootstrapping-aware training** — train/quantize weights with the square/sigmoid activation polynomial in the loop (see `PROJECT_STATUS.md` stage 3). *The principle is applied at demo scale by `examples/encrypted_iris` (quantization-aware: trains in f64 and quantizes to the exact Q15.0 inference scale).* (L)
- [x] **Wider-width strategy** — documented in `fhe-core::nn` module docs (Q15.0 exactness constraint, scale-corrected activation path for `frac_bits > 0`, U32/U64 or multi-slot packing plan). The code path itself is future work. (M)
- [x] **Real dataset demo** — `examples/encrypted_iris`: UCI iris (150 rows embedded), linear softmax trained on quantized features, encrypted weights, homomorphic inference, client-side argmax; encrypted accuracy equals plaintext exactly (96.7% test). (M)

### Practical checklist before declaring the repo "production-ready"
- [ ] All items above are either done or explicitly rejected with rationale.
- [ ] `cargo build --release --workspace` + `cargo test --workspace` + `clippy -D warnings` + `fmt --check` pass in CI.
- [ ] `fhe-phone.exe` runs from a clean machine (WebView2 runtime + signed exe + persisted key).
- [x] An internal security review package has been filed (`docs/audit/`); the
      independent external audit is still required and remains blocked.
- [ ] All audit findings F1–F11 are fixed or explicitly accepted with rationale.
- [ ] The §12 Handoff Checklist is re-run after each productionization PR.
