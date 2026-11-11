# Production-Readiness Assessment — Full Spectrum Report

**Scope:** all **27 projects** in `C:\Users\LNegenborn\Documents\Projekte`. The initial pass (2026-08-10) covered 21 projects; the six previously labeled "done" — Atmospheric Broadcast OS (ABOS), Sybil-Resistentes Proof-Of-Personhood, FHE-DB, Global Ghost Net (GGN), Atomic Mesh OS (AMOS), Ambient Thermal Arbitrage (ATA) — were assessed on 2026-08-11 and are included as §3.22–3.27.
**Method:** read-only review of every project's documentation (READMEs, whitepapers, development plans, status/changelog docs, Obsidian canvases) cross-checked against the actual file tree (crates/modules/tests/CI). No builds or test suites were executed — every verdict is doc- and tree-based evidence with `file:line` citations.
**Date of assessment:** 2026-08-10 / 2026-08-11.
**Update 2026-08-12 (City OS integration round):** the City OS workspace grew by four bridge crates (`city-bridge`, `city-smartmatter`, `city-ata`, `city-ehar`) connecting the sibling modules (Smart Matter, ATA, EHAR) and the KELLER kernel (ring-3 task spawning) to the City services; City OS was **built and tested** and `sovereign-core` was **compile-checked** — the only executed builds in this report (see §4). Affected: §1 rows 3/7/16/21/27, §3.3, §3.7, §3.16, §3.21, §3.27, §4.

---

## 1. Executive Summary

| # | Project | Verdict | Shortest justification |
|---|---------|---------|------------------------|
| 1 | Algorithmic Supply Chain Arbitrage | ❌ NOT ready | Docs claim 10 crates/137 tests; 5 crates are empty, workspace does not compile, 62 tests exist |
| 2 | Autonomous Proof-of-Damage Layer | ❌ NOT ready | "SMT" is a hand-rolled evaluator, "zk" is value-revealing hash commitment, contract pseudo-code, no CI/git |
| 3 | City OS | ❌ NOT ready | `no_std` prototype (41 test binaries green / 0 failed on 2026-08-12) + 4 bridge crates + ring-3 task-spawn scaffolding, but hardware paths are no-op stubs, crypto is FNV-1a/LCG, no CI/VCS |
| 4 | Decentralized Sovereign Credit Mesh | ❌ NOT ready | ZKP forgeable, SMPC commitments unverified, **no networking code at all**, broken Docker |
| 5 | Decoupled Fluid IP Network | ❌ NOT ready | Great simulation core (~97 tests) but HE/ZK/P2P/storage all simulated, CLI stub, roadmap unchecked |
| 6 | Differentially Private Synthesizer | 🟡 PARTIALLY | Real package, 160 tests, CI; but Pre-Alpha, no deployment layers, license contradiction |
| 7 | Energy Harvesting via Atmospheric Resonance | 🟡 PARTIALLY | Runnable simulation MVP (10 subsystems) but concept-stage physics, pre-Gate-1, dead code, stale config crashes; City OS budget seam `city-ehar` added (2026-08-12) |
| 8 | Formal Verification Platform (Keller-Verify) | ❌ NOT ready | Coherent rustc+MIR+Z3 prototype but zero tests, no CI, documented-but-absent artifacts |
| 9 | Fungal Compute Swarm | 🟡 PARTIALLY | Real TS monorepo (~149 tests, CI, Docker) but zk/HE/token are mocks and demo is a placeholder |
| 10 | Herzschrittmacher | 🟡 PARTIALLY | Strong research toolchain (101 Z3 tests, consistent docs); not software-production, explicitly not a medical device |
| 11 | OP-Roboter | ❌ NOT ready | Gate proves trivial `True`, Z3 compiled but never invoked, pipeline not wired |
| 12 | Plane OS | ❌ NOT ready | Honest research prototype (63 tests) but mock dynamics, not certified, no CI |
| 13 | Planetary Entropy Mining | 🟡 PARTIALLY | P0 MVP complete & rigorously tested (SP 800-90B); P1/P2 (~40%) unbuilt, toy zk |
| 14 | Quantum Entanglement Link | 🟡 PARTIALLY | Research-grade simulation core (81 tests); headline Ghost-Net layer unimplemented, Phase-3 untested |
| 15 | SMPC Dark Pool | ❌ NOT ready | Real crypto primitives + 103 tests, but blind matching & settlement are plaintext simulations, broken front-run crypto |
| 16 | Smart Matter | ❌ NOT ready | Protocol logic in `no_std` Rust (56 tests) but firmware never builds, bootloader empty, hardware dirs empty; City OS bridge `city-smartmatter` added (2026-08-12) |
| 17 | Symbolic Execution Engine (Smart Contracts) | ❌ NOT ready | Everything downstream of the parser is stubbed — no SMT query ever runs, no contract analyzable end-to-end |
| 18 | Transit OS | 🟡 PARTIALLY | Coherent research prototype (16 crates, 39 integration tests, CI) but gate checks a zeroed state, no Z3 |
| 19 | Vantablack | ❌ NOT ready | Security product with **no key exchange** (master key sent in-band), zero tests, docs describe a different product |
| 20 | Zero-Knowledge Genome | ❌ NOT ready | Solid bioinformatics core (~150 tests) but every security component is a stub/simulation |
| 21 | sovereign-core (KELLER OS) | ❌ NOT ready | Bootable QEMU microkernel (26 self-tests) + City/Bridge task spawning (2026-08-12, compile-verified) but still no user-mode ring-3, hardcoded keys, no CI/git |
| 22 | Atmospheric Broadcast OS (ABOS) | ❌ NOT ready | Honest v0.1.0 "simulation-capable core": full DSP/FEC/protocol chain works in sim, but all SDR/GPIO/GPSDO surfaces are simulated stubs — nothing has touched RF |
| 23 | Sybil-Resistentes Proof-of-Personhood | 🟡 PARTIALLY | Core graph/sybil/reputation/WoT engines real (~93 tests) + Axum API; Phase 3–4 (governance, zk, SDK, mobile, deployment) unfinished, zk is a toy |
| 24 | FHE-DB | 🟡 PARTIALLY | Best engineering in the ecosystem (4-job CI, real fuzzing, benchmarks, TLS/auth/DoS hardening, WAL) but self-assessed ~40%, no external crypto audit, bootstrapping unwired |
| 25 | Global Ghost Net (GGN) | 🟡 PARTIALLY | Genuinely tested mesh daemon (~121 tests, live 2-node smoke, pentest round, cross-compile CI) but stale docs, dead 288 KB duplicate, broken fuzz feature, TUN stub, placeholder bootstrap seeds |
| 26 | Atomic Mesh OS (AMOS) | ❌ NOT ready | Offline verification workbench works end-to-end (38 tests, bundled z3) but 5 crates unwired, no CI/git, no hardware, no signatures — pre-release scaffold |
| 27 | Ambient Thermal Arbitrage (ATA) | 🟡 PARTIALLY | Mature pilot candidate (~70 gateway tests, E2E harness, deployment guide, shipped v0.2.0) but Phase 2 "in progress", go-live checklist unchecked, no field validation, fleet engine untested; City OS adapter `city-ata` added (2026-08-12) |

**Scoreboard: 0 production-ready · 11 partially ready · 16 not ready (of all 27 projects).**

**The single most important finding:** in almost every project the **middle layer is genuinely implemented** (parsers, graph/DAG engines, physics models, protocol logic, unit tests) while the **headline security/crypto features are simulations** — zk-SNARKs are fake hash proofs, "FHE" is XOR obfuscation, "SMT/Z3" is a boolean evaluator or never-invoked code, and hardware paths are stubs. No project has a production substrate: none has a CI that has demonstrably ever run (most have no git repository at all), none has releases, most lack licenses, and fuzzing/benchmarks are essentially absent everywhere.

**Second finding — the "done" six are themselves prototypes:** the projects labeled finished (ABOS, Sybil, FHE-DB, GGN, AMOS, ATA) follow the same pattern, just further along. The most mature (FHE-DB, ATA, GGN) explicitly self-disclaim production readiness: FHE-DB has no external crypto audit and rates itself ~40%, ATA's go-live checklist is unchecked and unfield-tested, GGN ships placeholder bootstrap seeds and no tagged release. "Done" here means "scaffold + simulation complete", not "deployable".

---

## 2. Cross-Cutting Findings

### 2.1 What is THERE (common strengths across the spectrum)

- **Honest planning culture.** Nearly every project has a multi-phase, month-budgeted solo-developer plan (40–62 months) that explicitly defers production (external audit, testnet, regulatory) to a final phase — and most docs correctly label the current state as "prototype/MVP/scaffold".
- **Real middle-layer engineering.** Concrete, working implementations exist in most projects: graph/DAG engines, streaming parsers (VCF, G-code), physics/thermal models, MPC/share math, B-tree query indexes, no_std kernels, mesh routing, QKD protocol simulation.
- **Unit tests in most projects.** Ranges from 17 (DSCM) to ~160 (DPSynth); several match their documented counts exactly (City OS 43, Plane OS 63, QEL 81, Herzschrittmacher 101, SMPC 103).
- **A few genuine CI workflow files** exist (DPSynth, FCS, SMPC, Smart Matter, SymEx, Transit OS, ZKG, ABOS, FHE-DB, GGN, Sybil, ATA) — though none is verifiable as ever having run (see 2.2).
- **Exceptions that raise the bar:** FHE-DB (4-job CI incl. fuzzing + benchmarks + TLS, 2 real libFuzzer targets, criterion regression guards, WAL, auth/TLS/DoS hardening), ABOS (clippy-clean CI, deterministic seeds, structured JSON errors, shipped prebuilt binary), GGN (live 2-node smoke test, authorized pentest round, cross-compile CI) — all still self-label pre-production.
- **Discipline where it counts:** `#![forbid(unsafe_code)]`/`deny(unsafe_code)` and `missing_docs` in the formal-methods family (OP-Roboter, Plane OS, Atomic Mesh); fail-closed execution gates in the verification projects; root-caused-bug regression guards (PEM's SP 800-90B work).
- **Safety disclaimers are ubiquitous and appropriate** in the medical/aerospace/automotive projects ("NOT a medical device", "not certified DO-178C/ISO 26262", "must not control a physical device").

### 2.2 What is MISSING (the production substrate, project-wide)

- **CI that has ever run: 0/27.** Workflow files exist in several projects, but none can demonstrably have run: most have **no git repository** (no `.git`); GGN's release workflow only triggers on tags and the repo has no tags; ATA has 11 commits but no git remote; several docs describe CI that does not exist on disk (FVP, ASCA).
- **Releases, changelogs, tags: essentially none.** A handful of CHANGELOGs exist (ATA—excluded, DPSynth, Smart Matter's sibling, EHAR none…); only DPSynth has a tagged v0.1.0 entry. Most crates are pinned at `0.1.0` forever.
- **Licenses:** the majority have no LICENSE file at all; two have **contradictory** license stories (DPSynth: Proprietary LICENSE file vs Apache-2.0 claims; ASCA: MIT badge with no file; several: Apache-2.0 claim with no file).
- **Integration/E2E tests:** `tests/` directories are empty in ASCA, PoDL, SMPC (root), SymEx fixtures empty; integration tests are usually in-crate or in one file.
- **Fuzzing: ~0.** Only sovereign-core has an in-kernel coverage-guided fuzzer (256 iterations as a boot self-test). Proptest/cargo-fuzz/quickcheck are declared-but-unused in PoDL, SMPC.
- **Benchmarks: ~0.** DSCM has 3 Criterion benches with no committed results; SMPC has 2 with no report; everywhere else none — yet several whitepapers publish performance figures with no backing artifact.
- **Deployment:** Docker/compose files exist in ~5 projects and are **broken in all of them** (DSCM: non-existent subcommand; SMPC: rust:1.77 vs edition-2024, ships only CLI; ZKG: none despite plan).
- **Git discipline:** most projects are not git repositories at all; two have git dirs with zero commits (ASCA, sovereign-core); ZKG has 12 commits from one author in one ~19-hour session.

### 2.3 What is PRIMITIVE (the "toy-grade" layer — where docs overstate)

- **zk-SNARKs are fake almost everywhere.** Forgeable hash tricks (DSCM), value-revealing commitments (PoDL, FCS), SHA-3 "simulated proof data" + verifier that accepts any non-empty blob (ZKG), toy circom with placeholder gates and hardcoded secrets (PEM), fake Groth16 from a 32-bit JS hash (FCS).
- **Homomorphic encryption = XOR obfuscation** (FCS — explicitly disclaimed in its own docs; DFIP — "pure-Rust simulation layer").
- **SMT/Z3 is a ground boolean evaluator or dead code** (PoDL, OP-Roboter: compiled never invoked; SymEx: both backends return Unknown; Transit OS: no Z3 at all; FVP: works but unwired type-awareness).
- **Home-grown non-standard crypto:** hand-rolled ChaCha20-Poly1305 variants in EHAR and Smart Matter (both non-RFC-8439), FNV-1a "MACs" in City OS and sovereign-core, 32-bit-seed LCG "entropy" in City OS, plaintext or hardcoded keys (DSCM plaintext state file, sovereign-core compile-time constants, PEM hardcoded secrets, Smart Matter `[0x42;32]`).
- **Hardware/embedded paths are stubs:** City OS NIC `wr()` empty, SDR no-op; Smart Matter firmware never builds, bootloader functions empty; EHAR `hal/` empty; sovereign-core no real NIC/DMA; QEL IPC "scaffolding"; ZKG RadioShield always reports "Isolated".
- **Smart contracts are pseudo-code/placeholders:** PoDL `PayoutInvariants.sol` non-compiling; SMPC `Settlement.sol` `hash[0] != 0` check; ZKG `Payment.sol` never transfers; DFIP Rust-native "would be compiled to ink!/Solidity".
- **Stale/duplicated artifacts everywhere:** monoliths + backups (DFIP), byte-identical README==WHITEPAPER + nested snapshot + orphaned root files (Vantablack), stale status docs (ZKG's "zero code exists" with 20k+ LOC present), docs claiming tests/CI that don't exist (FVP), test counts that no longer match (ASCA 137→62, OP-Roboter 47→42, GGN 66 vs 121, Sybil 84 vs ~93).
- **The "done" six are the same pattern**: ABOS's SDR/GPIO/GPSDO surfaces are simulated type-aliases (nothing has touched RF), AMOS's real-time pipeline and 5 crates are unwired, GGN's TUN mode is a stub with placeholder bootstrap seeds and a dead 288 KB `VANTABLACK.rs`, Sybil's zk circuits are self-labeled "toy"/"simplified educational", FHE-DB's bootstrapping is unwired and it has no external crypto audit, ATA's go-live checklist is unchecked.

### 2.4 What is TO BE DONE (common next steps before anything ships)

1. **Reconcile docs with reality** — every project's docs overstate completion somewhere; fix test counts, remove stale files, mark simulations honestly (this is the cheapest, highest-value fix).
2. **Init git + CI in every project** (build, clippy `-D warnings`, test, coverage) and make CI **non-blocking-proof** (Smart Matter's `|| echo` firmware swallows failures — fix that pattern).
3. **Replace or honestly rescope the crypto layer**: real KEMs/AEAD (RFC 8439), real zk libraries (arkworks/snarkjs/circom) or delete the claim, real MPC (Beaver triples) or label it simulation.
4. **Add integration/E2E tests** for the wired pipelines, plus at least one fuzz target and one benchmark per project.
5. **Licenses, changelogs, release process** (version bumps, tags, artifacts).
6. **Fix the deployment story** (Docker/compose, env-var/config wiring, host binding) or remove it.
7. For the safety-critical family (OP-Roboter, Plane OS, Transit OS, Herzschrittmacher, sovereign-core): keep them as research artifacts; **do not** attempt certification without a team and regulator.

---

## 3. Per-Project Detail

---

### 3.1 Algorithmic Supply Chain Arbitrage — ❌ NOT ready (confidence: high)

**What it is (per docs):** a decentralized, trustless logistics-optimization platform ("SCA") — DAG routing (graph theory), SMPC price discovery, SMT formal verification, smart-contract settlement. README claims "10 modular Rust crates, 137 passing tests", phases 0–3+ complete, only IoT integration/audit/testnet/hardening remaining.

**What's THERE (code):**
- 5 crates with real code, 62 unit tests: `sca-smpc` (field/shamir/protocol, 18 tests — only crate with no `sca_*` imports), `sca-smt` (23), `sca-payload` (9), `sca-spec` (3), `sca-contracts` (9, pure-Rust state machine).
- Detailed, coherent whitepaper + ADRs + README quick-start; `rustfmt.toml`, edition 2024, `thiserror` error handling, well-formed state machines.

**What's MISSING / PRIMITIVE (the critical part):**
- **5 of 10 workspace members are empty skeletons** (`sca-types`, `sca-graph`, `sca-matching`, `sca-p2p`, `sca-security` — `src/` empty, only Cargo.toml).
- **The workspace cannot compile**: coded crates import types from the empty crates; `sca-api` declares `pub mod nodes/edges/paths/topology` that don't exist (routes.rs:3-6).
- Claimed Z3 backend behind a `z3-backend` feature flag — **the feature flag doesn't exist**; claimed AES-256-CTR memory encryption — no code; libp2p — no dependency; Solidity contracts — aspirational comment only (sca-contracts/src/lib.rs:17-18).
- No CI, no integration tests (`tests/` empty), no fuzzing/benchmarks, no changelog/LICENSE, empty `docs/openapi/` referenced in `.gitignore`.
- `target/debug/.fingerprint/` shows all 10 crates compiled at an earlier point — **source was removed afterwards**; git repo has no commits ever.

**What's TO BE DONE:** restore or rebuild the 5 empty crates; fix `sca-api` route modules; wire the Z3 feature or drop the claim; decide and implement the real contract story; CI + fuzzing + audit + testnet (as the docs themselves list as "Next/Future": WHITEPAPER.md:514-525).

---

### 3.2 Autonomous Proof-of-Damage Layer — ❌ NOT ready (confidence: high)

**What it is (per docs):** a decentralized insurance-claims system: real-time sensor telemetry → symbolic physics matching → SMT damage verification → zk-SNARK privacy proofs → autonomous smart-contract payouts. 4-phase/60-month plan; code covers ~partial Phase 1 + Phase 2 with Phase-3 stubs. (Folder spelled "Autnomous".)

**What's THERE (code):**
- ~113 unit tests across crates + `crates/physics-logic/tests/end_to_end.rs` (4 tests).
- `telemetry-graph` (petgraph DiGraph, CSV ingest, window aggregation), `incident-audit` (boolean constraint traces `trace.rs:128-148`, state-space windows), `sensor-integrity` (ed25519 signing chain, replay/injection/temporal-anomaly checks), `report` (DamageReport JSON), `pod-cli` (9 subcommands), `pod-dashboard` (axum + static HTML).

**What's MISSING / PRIMITIVE:**
- **"SMT" is a hand-rolled ground boolean/arithmetic evaluator** — no Z3 in Cargo.lock (solver.rs:63-67,145-165); `StructuralShift` damage detection is a hardcoded stub always returning `false` (conditions.rs:67-69) and is skipped by the solver (solver.rs:217); `RateOfChange` also skipped.
- **"zk-SNARK" is a SHA-256 commitment that reveals the value and blinding** (privacy/src/proof.rs:73-78) — not a SNARK, not privacy-preserving.
- `PayoutInvariants.sol` is **non-compiling pseudo-code** referencing nonexistent `DamageProof.Data`; `RiskPool.releasePayout` being `onlyGovernance` contradicts invariant I2.
- `audit_chain_links` passes **empty public keys** so signatures are never actually verified (taint.rs:128-136).
- Settlement is an in-memory `MockChainClient`; `privacy`+`settlement` not wired into any binary.
- No README/docs, no CI, no git, no `target/` (never built), heavy `.unwrap()`/`.expect()`, `proptest` declared but unused.

**What's TO BE DONE:** real Z3 or honest rescope; real zk; fix/remove the Solidity; actually verify signatures; wire privacy+settlement; CI + build instructions. Plan itself flags fraud-resistance as untested (DEVELOPMENT_PLAN.md:106-111).

---

### 3.3 City OS — ❌ NOT ready (confidence: high)

**What it is (per docs):** "a decentralized urban operating system" — DAG ledger, edge computing, identity-less service access (BRP), localized sharding, dead-drop infrastructure, autonomous resource allocation — built **on top of KELLER OS** (`../sovereign-core/`), which it reframes as "City OS Phase 1/2". Docs claim L0/L1/L2 layers, kernel wiring and the hardware bridge "complete"; only Phase 4 hardware remains.

**What's THERE (code):**
- All **20 library crates** + `city-node` binary exist as `no_std` code; host boot demo; exact kernel-ABI mirror (`SYS_CITY_*` opcodes, `CAP_CITY_*` slots, ring-3 registry in `sovereign-core/src/city.rs`); byte-level protocol specs; threat-model→test mapping; **zero external crates** (Cargo.lock = workspace only). **Executed 2026-08-12:** `cargo test --workspace` → 41 test binaries green, 0 failed (~68 unit tests, incl. the previously verified 43); `cargo run -p city-node` → `[HARNESS] CITY OS BOOT: PASS`.
- **Integration bridges (2026-08-12, +25 tests):** `city-bridge` (BridgeFrame 28-byte wire body + MAC + per-bridge salts; capability mapping 7/10/16 · 11/15/7 · 11), `city-smartmatter` (Smart-Matter slice parser CRC16-CCITT → IntegrityReport → `city-dag`; `GeoBox` geo-ownership → `city-shard`; quantized readings → `city-entropy` OTP; replay guard), `city-ata` (status → `city-alloc` demand/scarcity; MAC-signed credits → `city-incentive`; grid-balancing events → DAG), `city-ehar` (per-subsystem harvest budget → `city-alloc` supply). `city-node` boots them as `[L3] integration bridges`; living spec in `docs/INTEGRATION_PLAN.md` (wire formats, ring-3 spawn plan, QEMU/RPi hardware path §7).

**What's MISSING / PRIMITIVE:**
- `Nic::wr()` is an **empty function** (city-p2p/src/hw.rs:56-62), `Sdr::tx()` returns `true` without transmitting (hw.rs:113-121) — the "implemented" hardware bridge is a no-op.
- Ring-3 task spawning is **scaffolded but not user-mode**: `sovereign-core` now starts the ten City services (slots 11–20) + three bridges (21–23) as **kernel-mode** heartbeat tasks (`scheduler::start_task`, `MAX_TASKS` 24, cap-gated dispatch; compile-verified) — but the ring-3 `iret` transition (`user.rs::enter_user`, `proc_create`) is still never called and city-p2p is still not bridged onto kernel IPC.
- **Crypto is not cryptographically sound:** the "salted MAC" is 32-bit FNV-1a with a hardcoded salt `0xC17DECAF` (city-crypto/src/mac.rs:14-35) — forgeable; "zero-knowledge" entropy is a deterministic LCG from a 32-bit seed (osrng.rs:13-21) — 2³² unlinkability.
- No CI, **no version control**, no changelog; `DEVELOPMENT_PLAN.md` is stale/superseded (libp2p P2P, zk voting, ML demand prediction, admin dashboard, 10× RPi testnet — none built as described); placeholder `repository = "https://example.invalid/city-os"`.

**What's TO BE DONE:** implement the NIC/SDR seam or remove the claim; **complete the ring-3 transition** (user-mode `iret`, QEMU `run-tests.ps1` log); replace FNV-1a/LCG with real MAC/entropy (or downgrade the security claims); git init + CI; reconcile DEVELOPMENT_PLAN with ROADMAP.

---

### 3.4 Decentralized Sovereign Credit Mesh — ❌ NOT ready (confidence: high)

**What it is (per docs):** a trust-based, privacy-preserving credit system: social-graph topology, web-of-trust vouching, SMPC solvency, zk cluster proofs, taint-coded reputation, nullifiers, trustless settlement. Whitepaper self-labels it "a functional prototype with ~4,000 lines of Rust". 58-month plan; production milestone 58 = audit+testnet+regulation (not reached).

**What's THERE (code):**
- All 11 crates present: `graph-core` (PageRank, clustering, entropy, Sybil detection), `wot` (ed25519, vouching, limits, nullifier), `reputation` (scoring, taint), `credit` (constraints, loan lifecycle), `smpc` (Shamir SSS over GF(2³¹−1)), `zkp` (commitments, Merkle), `settlement`, `api-server` (axum), `cli` (29 subcommands); 17 tests; 3 Criterion benches (no results committed); docs consistent (CLI/API/test-count claims verified).

**What's MISSING / PRIMITIVE:**
- **ZKP is not zero-knowledge:** `ClusterMembershipProof` exposes cluster size/reputation in plaintext; the "range proof" is a plain SHA-256 of a prover-controlled string — forgeable (zkp/src/proof.rs:53-62,99-106; acknowledged at WHITEPAPER.md:319-327).
- **SMPC commitments never verified** — `verify_solvency` ignores them and hardcodes `entity_id = "verified"` (smpc/src/solvency.rs:115-126).
- **No networking code at all** — `MESH_PEERS` env vars (`ws://…`), TLS, multi-node: nothing (no websocket/tungstenite/reqwest/hyper/TcpStream anywhere).
- Broken deployment: `docker-compose.yml` invokes a **non-existent `mesh-cli api-server` subcommand**; API hardcodes `127.0.0.1:8899` (unreachable in container); `config/default.toml` is dead (nothing parses TOML); env vars never read.
- API wide open (`CorsLayer::permissive()`, no auth/rate-limit); signing keys stored **plaintext**; `VouchingLimit` depth implemented as `pagerank * 10 as usize` (semantically wrong); clustering coefficient approximates via shortest path.
- `deny.toml` typo `unlicensed = "denon"` breaks cargo-deny; no CI, no git, no LICENSE.

**What's TO BE DONE (docs' own priority order):** (1) real zk-SNARK or honest re-scope, (2) wire config/env vars + fix/remove Docker, (3) API hardening, (4) enforce SMPC commitment verification, (5) CI + fuzzing + license.

---

### 3.5 Decoupled Fluid IP Network — ❌ NOT ready (confidence: high)

**What it is (per docs):** decentralized IP management replacing text patents with machine-verifiable formal specs: invention DAG, SMT delta verification (Δ = ¬Original ∧ Derivative), homomorphic royalty discovery, taint tracking, trustless smart-contract "ExecutionGate". Whitepaper v1.0 but §10 labels production parts "Future Work"; a 14-phase `Step-by-Step.txt` roadmap has **every checkbox unchecked**, ending with "Shall I proceed with implementation?".

**What's THERE (code):**
- Working Rust: `PatentDag` (petgraph + Merkle audit), `InventionIr`, Delta/AuditTrail, solver abstraction (BuiltIn + subprocess Z3/Bitwuzla/CVC5), royalties, taint lattice, `.ipspec`, Rust-native contracts mirror, actix REST API, **~89 unit tests + 8 property-based tests** with committed proptest regression file.
- Unusually self-critical whitepaper (full §7 "Limitations & Response" with three named critique modules).

**What's MISSING / PRIMITIVE:**
- **Every trust/network-critical component is a simulation:** HE (`crypto/src/he.rs` "pure-Rust simulation layer"), ZK (`ZkProofType::Simulated`), P2P (in-memory, no libp2p), storage (in-memory, no IPFS/Arweave), contracts (Rust mirror, "would be compiled to ink!/Solidity").
- CLI is a **usage-text stub** (no commands implemented); API is in-memory-only, no auth, `solver_time_ms: 0.0` hardcoded (api/main.rs:263); protobuf schema not wired into the build (no build.rs).
- Repo hygiene: `dfip_monolith.rs` (divergent snapshot), `dfip_monolith_backup.rs` (older), `temp_new.rs` (concatenation of 3 files) — none compiled; orphaned `tests/src/proptests.rs`; stale `WHITEPAPER_backup.md`; `cargo install z3` is not a valid install command.
- No README, no docs/, no LICENSE, no CI, no `target/` (never compiled here); `unreachable!()` in `ExternalSolver::new` (solver.rs:197).

**What's TO BE DONE:** implement real HE/ZK/P2P/storage or re-scope the project as an honest simulator; implement the CLI; persistence + auth in the API; delete stale monolith/backup files; CI.

---

### 3.6 Differentially Private Synthesizer — 🟡 PARTIALLY ready (confidence: high)

**What it is (per docs):** a Python package generating synthetic datasets with formal differential-privacy guarantees (DP-GAN, gradient clipping, noise injection, membership-inference protection). 40-month plan; core library (Phases 1–2) built; self-declared **"Development Status :: 2 - Pre-Alpha"** (pyproject.toml:23); single release v0.1.0.

**What's THERE (code):**
- **All 10 canvas components implemented:** RDP privacy accountant, gradient clipping (fixed/adaptive/per-sample), Gaussian/Laplace noise, `CTGAN_DP` (DP-SGD discriminator + PAC), membership-inference evaluation, utility/correlation evaluation, exponential mechanism, post-processing invariance.
- **160 tests across 9 files** (verified via pytest nodeids), ~96% coverage claim, CI workflow (test matrix Python 3.11–3.13 + import check), full docs (api.md, tutorial, whitepaper, CONTRIBUTING), valid pyproject/Makefile.

**What's MISSING / PRIMITIVE:**
- Phase 3–4 not built: REST API, web UI, Docker, PostgreSQL connector, time-series generator, audit dashboard — all absent.
- `optimizer.py:170` vmap strategy raises `NotImplementedError` (falls back to manual); post-processing "verification" is a distance heuristic (ε ≈ −log(mean normalized distance)), not formal RDP.
- **License contradiction:** `LICENSE` file = Proprietary/VANTABLACK, while CHANGELOG/WHITEPAPER/plan claim Apache-2.0 — "production use" is legally constrained regardless of code quality.
- opacus hard dependency vs "optional" in docs; CHANGELOG claims property-based tests but no `hypothesis` import exists; unreproducible empirical AUC numbers (no benchmark code); no git repo; stale pytest cache.

**What's TO BE DONE:** resolve the license first; add deployment layers (API/Docker); reproducible benchmarks; verify the DP amplification correctness formally; git + release process.

---

### 3.7 Energy Harvesting via Atmospheric Resonance — 🟡 PARTIALLY ready (confidence: high)

**What it is (per docs):** a concept project to passively extract usable electrical energy from atmospheric EM/thermodynamic phenomena (10 subsystems: ionospheric plasma coupling, NVIS skywave, Maxwellian matching, piezo infrasound, non-linear wave modulation, cyclostationary filtering, plasma-wake buffering, resonant-substrate RTOS, thermodynamic gradient, static invariant discharge). Status: **"Concept & Planning Phase"**, at Phase 1 (Research & Simulation), pre-Gate-1 (gate = theoretical energy density > 1 mW/m², unmet & unmeasured).

**What's THERE (code):**
- A runnable Rust **simulation MVP** (`atmospheric-resonance-sim` + `simulation_core`): all 10 documented subsystems modeled as structs, 3 JSON scenarios, deterministic CSV output, 6 unit tests, lockfile; an unusually thorough plan (physics background, BOM ~23.5k–48k €, risk register R1–R8, IEC/FCC compliance, references).

**What's MISSING / PRIMITIVE:**
- **Physics unvalidated and self-defeating:** the plan's own numbers show infrasound ~0.01–1 μW/m² and TEG ΔT ≈ 0.2°C at 30 m; the sim "solves" this with an unvalidated ground-coupling assumption "guarantees minimum 2.5°C" (physics.rs:214-225). Risk R1 = "Harvested energy density too low (< 1 mW/m²) — High/Critical".
- Planned FDTD Maxwell solver is **dead code** (never instantiated); `severe_weather.json` is stale and **panics** the CLI (missing 9 config fields); undocumented 11th subsystem (Solar PV); plan says MATLAB/Python, code is Rust.
- Hand-rolled **non-conformant ChaCha20-Poly1305** (no reduction mod 2¹³⁰−5, MAC key derived from keystream block 0) and a fake "BLAKE3" XOR-feedback sponge (entropy.rs:75,126 admit it).
- No hardware/FPGA/DSP/embedded code (`hal/` empty); no CI, no git, no README; `chrono` dep unused; many config fields never read.

**What's TO BE DONE:** validate or revise the physics gate before any further engineering; fix the stale config; remove dead FDTD; replace the crypto or drop it; document the sim; CI.

**Update 2026-08-12:** a City OS budget seam now exists — `city-ehar` estimates per-subsystem harvestable power from the plan's own tables (Schumann 100, NVIS 500, Infrasound 500, TEG 100 000 nW/m² ≈ 0.1 mW/m²) and feeds the `city-alloc` supply side (4 tests). The module itself is unchanged (concept stage, pre-Gate-1, physics unvalidated).

---

### 3.8 Formal Verification Platform (Keller-Verify) — ❌ NOT ready (confidence: high)

**What it is (per docs):** a Rust compiler plugin that runs as `RUSTC_WRAPPER`, intercepts MIR, extracts bounds/overflow/div-zero assertions, encodes them as Z3 SMT constraints, and reports `[PASS]/[FAIL]/[WARN]`. The author's own `UPGRADE_RECOMMENDATIONS.md:3` frames the outstanding work as "to make this production-ready".

**What's THERE (code):**
- Coherent single-crate prototype: CLI + rustc-wrapper mode; MIR→Z3 for `BoundsCheck`, `Overflow(Add/Sub/Mul/Shl/Shr)`, `DivisionByZero`; constant propagation; reachability; JSON report; Z3 path detection in build.rs; 6 clean modules.

**What's MISSING / PRIMITIVE:**
- **Zero tests** (no `#[test]`, no `tests/` dir, no dev-dependencies); **no CI** — README documents `tests/safe_code`, `tests/overflow_fail`, `tests/divzero_fail` and `.github/workflows/verify.yml` that **do not exist on disk**.
- Type-aware bounds are **dead code** — callers pass `None`, `infer_int_type` is never called; the effective path is the i64 fallback (constraint.rs:98-114).
- **Exit-code contract broken:** FAIL events never set exit code 1 (report.rs:34-40); `u128` clamped to `i128::MAX` ("close enough"); Z3 context per-crate; report path mismatch (README vs USAGE); `is_panic_internal` filters any fn containing "Panic".
- Checked-in `report.json` still shows 4 known false-positive FAILs; `USAGE.md`/`keller-verify.cmd` hardcode machine-specific paths to a nonexistent release binary; `.cargo/config.toml` hardcodes `C:\z3-4.16.0-x64-win`; no LICENSE/VCS; compile against pinned nightly unverified.

**What's TO BE DONE:** add the documented tests and CI; fix the exit-code contract; wire type-aware bounds; fix report path; de-hardcode machine paths; verify compilation on `nightly-2026-07-24`.

---

### 3.9 Fungal Compute Swarm — 🟡 PARTIALLY ready (confidence: high)

**What it is (per docs):** a decentralized browser-compute platform: WebRTC "mycelial mesh", fractal task splitting, sandboxed WebAssembly, zk-SNARK + homomorphic verification, L2 FUNGAL token, leaderless consensus. Whitepaper claims "**fully implemented**, 147 tests across 33 test files"; the v1.0 release checklist is entirely unchecked; version 0.1.0, `private: true`.

**What's THERE (code):**
- 14 packages + 2 apps, **~149 tests across 34 files**: `core` (32), `signaling-server` (real WS + RoomManager), `mycelial-mesh` (RTCPeerConnection, Full/Sparse/Hybrid topologies, binary codec), `signal-transduction` (codec+gossip), `wasm-executor`, `task-distribution` (11), `redundancy` (10, Poisson PMF), `resource-scoring` (CPU/mem benchmarks + entropy Bio-Score), `consensus` (17), `client` (boot, submitter, rate-limiter/sybil-resistance/fingerprint), `substrate-layer`, `dev-tools`.
- Turborepo/tsup build, CI workflow, Dockerfile + docker-compose, 6 detailed phase-plan docs with checklists, 3 ADRs.

**What's MISSING / PRIMITIVE:**
- **zk-integrity is a mock:** snarkjs loader *always throws*, proofs fall back to a fake Groth16 from a 32-bit JS hash; `verify()` only checks `pi_a[0]` non-empty. No circom/wasm/zkey anywhere.
- **"Homomorphic encryption" is XOR/AND obfuscation** — explicitly disclaimed ("should not be relied upon for production secrets", whitepaper.md:167-169).
- **Token layer is a stub:** zero-address wallet, fake tx hashes, fabricated signatures; no Solidity contracts, yet `docs/deployment.md` documents a hardhat deploy path.
- `consensus/engine.ts:108-117` injects **hardcoded fake metrics**; `client/boot.ts` uses a hardcoded score-50 node registry; demo app is a `console.log` placeholder; worker-sandbox `compile()` resolves a field the worker never posts (sandbox.ts:53-57).
- No E2E/fuzz/chaos/benchmarks; docs contradict each other (WHAT_WAS_DELETED.md stale); JSON-over-WS vs documented 28-byte binary protocol; no git/LICENSE/CHANGELOG.

**What's TO BE DONE:** real snarkjs/circom or remove the claim; real FHE or remove the claim; real chain contracts; wire consensus to actual mesh data; functional UI; E2E + fuzz + chaos; license/release process; reconcile docs with code.

---

### 3.10 Herzschrittmacher — 🟡 PARTIALLY ready (confidence: high)

**What it is (per docs):** "Formally Verified Pacemaker Real-Time OS — Research Toolchain" (Python + Z3). **Phase 2 prototype (Verification Engine)**; every doc carries an explicit **NOT A MEDICAL DEVICE** disclaimer; 56-month plan; self-declared Pre-Alpha.

**What's THERE (code):**
- NBG modes (VVI/AAI/DDD/VDD/DDDR/OFF), TimingCycles with validation floors, CardiacState, kinetic path constraints, pacing FSM, **5 lethal invariants**, real Z3 SAT/UNSAT engine + envelope search, fail-closed execution gate, symbolic pulse analysis, taint audit, temporal-isolation schedulability, CLI.
- **101 pytest tests (verified via nodeids)** including real Z3 integration and gate fail-closed tests; formal model doc matches code closely; docs internally consistent (5 invariants, 101 tests, ~11 ms solver, DDD subset); LICENSE present; valid pyproject.

**What's MISSING / PRIMITIVE:**
- **Mode switching is a stub** (path-constraint generator only — no transition FSM, no `set_mode`, no SAT analysis of the transition graph); recall validation (plan 2.7) not done.
- **Real-time budget: ~10–11 ms Z3 per decision vs 200 µs budget / 250 µs pulse window** — a 10–55× gap (documented in upgrades.txt, verification.md:141-150, whitepaper.md:298-300).
- No RTOS/embedded/HIL code (Phase 3); DDDR rate-response is an enum flag only; no external proof-checker ("solver trust" future work).
- No CI, no git, no changelog; `upgrades.txt` references a Rust `main.rs` that **does not exist** (100% Python — stale doc); committed `egg-info`; title "Formally Verified" overstates the body's "conditionally safe within the model".

**What's TO BE DONE:** mode-transition SAT analysis; proof-checker independence; real-time-budget engineering or an honest statement that the budget is unmet; CI; fix `upgrades.txt`.

---

### 3.11 OP-Roboter — ❌ NOT ready (confidence: high)

**What it is (per docs):** "Formally Verified Surgical Robot Control OS" — prove every actuator command safe (Model → Constrain → Verify → Execute) via SMT/Z3. **Phase 1 (current): kinematics, trajectory, simulator, mock SMT scaffold**; explicitly "must not be used to control a physical surgical device outside of a research simulation". 58-month plan; version 0.1.0.

**What's THERE (code):**
- All 11 crates: `op-math`, `op-kinematics` (DH, FK/IK, Jacobian with Gram-determinant ground-truth tests, singularities), `op-trajectory` (workspace/RCM/segment checks), `op-constraints`, `op-verifier` (mock solver + invariants), `op-gate`, `op-state-space`, `op-energy`, `op-taint`, `op-spec`, `op-sim` (3D simulator + CLI).
- 42 unit tests (docs claim 47 — stale); `#![forbid(unsafe_code)]` + `#![warn(missing_docs)]`; thorough honest docs incl. risk register and certification-gap mapping; reproducible getting-started; Cargo.lock shows a prior build.

**What's MISSING / PRIMITIVE:**
- **The execution gate is a scaffold stub**: `command_to_constraints` returns `ConstraintExpr::True` (op-gate/src/lib.rs:92-103) — every non-empty command is "proved" and released. The demonstrated "release=true" is a **tautology, not a proof**.
- **Z3 is compiled but never invoked** — `check_fault_reachable` has zero callers; `Verifier` hardcodes `MockSolver`.
- **Pipeline not wired:** `op-sim` generates a real `ConstraintSet` but `main.rs:47-51` discards it; even if wired, the verifier's `build_env` never binds the quantities `generate.rs:74-113` produces → would return `Unknown` on any real plan.
- Deadline guard checked once *before* `solver.check()` (z3.rs:49-64) — can't interrupt a long solve; no CI/LICENSE/CHANGELOG/fuzz/bench; no git.

**What's TO BE DONE:** wire the end-to-end pipeline; implement real gate constraints; connect Z3; fix env binding; CI; then Phase 2.7 validation + Phase 3 RTOS/HIL per plan.

---

### 3.12 Plane OS — ❌ NOT ready (confidence: high)

**What it is (per docs):** a "formally verified aerospace flight control **prototype**" — symbolic NRA formula DSL → canonical control-cycle environment → trustless execution gate (block actuator commands unless every lethal hazard is proven UNSAT) → hash-chained flight recorder. **Explicitly not deployable/certified (DO-178C/ARP4754A/CS-25)**; corresponds to a prototype of Phase-2 deliverables in a 62-month plan; Apache-2.0, v0.1.0.

**What's THERE (code):**
- **Every documented module exists:** `formula.rs` (NRA DSL), `solver.rs` (StubSolver + optional Z3 feature), `dynamics.rs` (mock 6-DoF), `state.rs` (canonical env), `envelope.rs` (V-N/Mach/alt/terrain), `thrust.rs` (limits + reverser policy), `sensors.rs` (taint audit), `controller.rs` (mock fly-by-wire), `gate.rs` (5 standard cases), `invariants.rs`, `constraints.rs` (horizon=8), `spec.rs` (recorder), `simulator.rs` + demo; **63 tests match the docs exactly** (56 unit + 7 e2e); honest, consistent docs; dependency-light (only optional `z3`).

**What's MISSING / PRIMITIVE:**
- Mock dynamics + ground-evaluation default solver (not symbolic SMT); FNV-1a recorder hash (non-cryptographic, acknowledged); horizon prediction assumes the command persists unchanged; completeness not guaranteed.
- No real reachability (Flow*/CORA/JuliaReach), no ARINC 653 partitioning/WCET scheduling, no HIL — all future work per plan Phase 3.
- No CI, no changelog/releases; timing figures disagree across docs (30–140 µs / 23–142 µs / 30–100 µs); latent dead code (`mag.env.alt_reverser_max` never seeded → any use yields UNKNOWN).

**What's TO BE DONE:** as a research artifact it is in good shape — remaining value-adds: Z3-in-CI, SHA-256 recorder, seed the reverser env. Do **not** pursue certification solo (docs themselves say it's not certifiable).

---

### 3.13 Planetary Entropy Mining — 🟡 PARTIALLY ready (confidence: high)

**What it is (per docs):** harvests environmental entropy (thermal/RF/seismic) into cryptographically verified certificates — **explicitly "everything runs on a laptop. No hardware, no network sockets, no real money"** (README.md:9-11). P0 MVP = capture→estimate→harden→nullifier→zk→signed cert; P1 = erasure/onion/DTN; P2 = dark pool/contracts/dashboard. Status doc: "P0 core complete and green… ~60% overall" (PROJECT_STATUS.md:8-34).

**What's THERE (code):**
- **P0 is complete and unusually rigorous:** deterministic 3-source capture sim with ground truth, FFT/noise-floor/Shannon analysis, **all 10 NIST SP 800-90B min-entropy estimators**, validation vs exact analytic ground truth (OVERALL: PASS report), GF(256) polynomial hardening, deterministic nullifier, versioned signed cert, toy circom circuit + e2e proof, CLI demo, ~93 tests + slow tier, root-caused bugs fixed with regression guards.
- Honest, specific docs: PROJECT_STATUS.md (handoff doc with run commands, repo map, bug post-mortems, API sketches for remaining modules), AGENTS.md with Definition of Done, NIST estimator documentation.

**What's MISSING / PRIMITIVE:**
- **P1/P2 (~40%) not built:** `erasure/`, `net/` (sphinx, dtn), `darkpool/`, `contracts/`, dashboard — API sketches only (PROJECT_STATUS.md:24-30, 122-154); README layout table lists dirs that don't exist.
- **zk is a toy:** placeholder gate (`window_id == 42`), hardcoded witness secrets, fixed Powers-of-Tau (circuits/toy_entropy_audit.circom:19-21,40-41; toy_prove_verify.mjs:77-79,101-106); hardcoded `ISSUER_KEY` and hardening key fallback.
- RF/seismic validated on structural invariants only (no closed-form ground truth); no CI, no git, no changelog; stray 1.8 MB `toy_entropy_audit.wasm` at root; no lint config; `node_modules` absent so the JS zk stage isn't runnable (demo skips it).

**What's TO BE DONE:** build P1/P2 modules (erasure coding, Sphinx framing, DTN mesh, dark pool, settlement, dashboard); replace the toy circuit; real key management; git + CI + benchmarks.

---

### 3.14 Quantum Entanglement Link — 🟡 PARTIALLY ready (confidence: medium)

**What it is (per docs):** a modular, simulation-first quantum-communication network stack in Python (density-matrix formalism): QKD, teleportation, superdense coding, entanglement swapping, error correction, distillation, memory buffering, and (planned) a Ghost-Net topology layer. Whitepaper v1.2 claims Phases 0–2 and 4 implemented, Phase 3 foundations complete, **Ghost-Net integration next**.

**What's THERE (code):**
- Phase 0 core (qubit/gate/measurement/channel/noise), Phase 1 protocols (bb84/e91/teleportation/superdense/swapping/bell), Phase 2 (shor/steane/distillation/memory), Phase 3 foundations (stabilizer/scheduler/physical/ipc_node), CLI + demo + generated notebook.
- **81 tests (verified count)**; `lastfailed` cache empty (last local run green); seeded-RNG design for reproducibility; meaningful tests (exhaustive single-error correction loops); MIT license.

**What's MISSING / PRIMITIVE:**
- **The headline next milestone — Ghost-Net topology layer — is entirely unimplemented** (topology optimizer, fidelity-constrained path finding, multi-hop swap schedules, visualization; only a stale `# --- ghost-net ---` comment in cli.py:251).
- `StabilizerState.from_density()` is a `pass` stub; IPC node explicitly "scaffolding"; BB84 privacy amplification, Steane transversal gates, no-cloning checks — all missing.
- **Zero tests for all Phase 3 modules** (stabilizer/scheduler/physical/ipc); latent bug: `qubit.py:20` auto-dims collapse any non-4×4 matrix to a single qubit; versioning mismatch (0.1.0 vs "1.2"); documented API drift (BB84/E91 return contracts); no CI/changelog; stale egg-info manifest.

**What's TO BE DONE:** implement the Ghost-Net layer (the documented next milestone); tests for Phase 3 modules; privacy amplification; fix the qubit dims bug; reconcile versioning/API docs.

---

### 3.15 SMPC Dark Pool — ❌ NOT ready (confidence: high)

**What it is (per docs):** "a fully private, front-running-resistant decentralized exchange" — Shamir Secret Sharing, SMPC blind order matching, threshold cryptography, zk-SNARK order validation, atomic blockchain settlement. 56-month plan; **whitepaper.md:312 claims "The implementation is complete, with 100+ tests, zero compiler warnings, and production-hardened networking" — contradicted by the code**.

**What's THERE (code):**
- Real arkworks-based primitives: SSS over BN254 (real polynomial + Lagrange), Pedersen + SHA-256 commitments, BLS sign/verify/aggregate, a real Groth16 circuit (prove/verify), genuine TCP node networking (framing/batching/pipelining/backoff), axum API + WS.
- **~103 tests (verified count)** including a real TCP integration test and a BFT 4-node consensus test; 6 coherent docs; a CI workflow (fmt/clippy/test/doc); 2 Criterion benches.

**What's MISSING / PRIMITIVE (the defining security claims):**
- **Blind SMPC matching is a plaintext simulation** — `secure_compare`/`secure_min`/`equality` read raw share values (compare.rs:6-12), `secure_mul` has **no Beaver triples** (mul.rs:4-7) — mathematically incorrect MPC multiplication; matching engine wraps plaintext values in shares.
- **Front-running "encryption" is broken:** `encrypt_payload` keys off `nonce‖payload` while `decrypt_payload` keys off `nonce‖"decrypt"` (commit_reveal.rs:58-72) — decryption can never recover the payload; tests never check round-trip.
- **Settlement is a placeholder:** `verify_match_proof` only checks hex length == 64; `Settlement.sol` verification is `hash[0] != 0`; threshold signatures length-checked; API builds proofs from zero-filled signatures.
- BFT "signatures" = SHA-256 of digest, `verify` = non-empty check; DKG is a **trusted dealer** (no distributed key gen); zk parameters generated per-proof (no trusted setup/CRS ceremony).
- Docker broken (rust:1.77 vs edition-2024; image ships only the interactive CLI while compose healthchecks an API port); API auth docs contradict code (docs say no auth, code 401s everything when `API_KEY` set); axum 0.7 `{id}` route syntax bug; no web UI; proptest/bincode promised but absent; no git.

**What's TO BE DONE:** real MPC (Beaver triples, multi-party) or an honest "simulation" label; fix commit-reveal; real settlement verification; proper DKG + trusted setup; fix Docker; UI; fuzzing.

---

### 3.16 Smart Matter — ❌ NOT ready (confidence: high)

**What it is (per docs):** infrastructure-embedded intelligent materials — energy-autonomous nodes in concrete/asphalt harvesting kinetic energy (piezo), ad-hoc mesh, encrypted dead-drop storage, structural-integrity self-monitoring, RSSI-based geographic data locality. Target HW: STM32G031J6 (Cortex-M0+, 64 KB flash/8 KB SRAM), nRF24L01+/CC1101, LIS3DH, BQ25504, 1 F supercap, PZT-5A. 54-month plan; MVP = harvesting + mesh + dead-drop (~12 months).

**What's THERE (code):**
- `no_std` Rust protocol logic: gradient mesh routing, dead-drop store/fragment/assemble/gc, broadcast RSSI geo-fencing + dedup, thermal/integrity/energy models, entropy pool, wear-leveling simulation, ProtocolSlice types; **56 tests** (43 core + 6 + 7 integration, all pure logic).
- Unusually thorough docs: energy model with flash-lifetime analysis, ITU-R P.2040 RF propagation, protocol slice layout, routing metric formulas, security threat model, BOM (~$14.15/node), build/schematic guides; CI workflow file.

**What's MISSING / PRIMITIVE:**
- **The firmware does not build** — no HAL, no `memory.x`, no `.cargo/config.toml`/build.rs; CI swallows firmware failures with `|| echo "...non-blocking"` (ci.yml:39-45); bootloader's two functions are comment-only stubs (`loop {}`).
- Sensors read **hardcoded zeros** (`main.rs:84-89` "In real implementation, this reads from I2C/SPI"); `RouteAdv` payloads **never serialize entries** → no multi-hop learning over the wire; no framing/CRC/fragmentation (radio max 32 B vs 151 B slice); retrieval-by-DataAck unimplemented.
- **Non-standard crypto:** hand-rolled ChaCha20-Poly1305 (tag keyed directly with the ChaCha20 key, no AAD) — not RFC 8439, not interoperable; hardcoded `[0x42; 32]` key in the desktop tool; no key management.
- Contradictions: 1 F (docs) vs **100 mF** (code) energy storage; 64×144 B = 9,216 B dead-drop fragments vs 8 KB SRAM; 12 KB vs 16 KB flash layout; `hardware/compute-node/` is **empty** (no PCB/KiCad/gerbers); desktop tool/flasher/network-simulator print hardcoded/random values; no git/LICENSE/changelog; no field or bench data.

**What's TO BE DONE:** real HAL + memory layout + buildable firmware; serialize mesh entries; standard AEAD + key provisioning; implement bootloader/OTA; hardware design; resolve the capacity contradictions.

**Update 2026-08-12:** a City OS bridge now exists — `city-smartmatter` (slice parser byte-compatible with the protocol spec, IntegrityReport → DAG, `GeoBox` geo-ownership, entropy OTP, replay guard; 8 tests) and the QEMU/HW feed path (network-simulator → SDR/e1000 → CC1101 Sub-1-GHz sink) is specified in `City OS/docs/INTEGRATION_PLAN.md` §3.4/§7. The module itself is unchanged (firmware still does not build, non-standard crypto remains).

---

### 3.17 Symbolic Execution Engine for Smart Contracts — ❌ NOT ready (confidence: high)

**What it is (per docs):** a formal-verification engine for EVM smart contracts: symbolic execution, SMT (Z3), state-space exploration, vulnerability detection, taint, gas analysis, concolic execution, spec generation. Version 0.1.0, **Phase 1 "Core Symbolic Engine (Current — Months 1–12)"**; whitepaper concludes the scaffold is "ready for Phase 2".

**What's THERE (code):**
- Genuinely implemented: opcode table (Frontier–Shanghai), bytecode parser, core types, gas-cost utilities with tests, Keccak concrete + symbolic fresh-var model, basic (linear) IR/CFG + selector detection, a partial symbolic executor, CLI shell; ~57 unit + 4 integration smoke tests; CI workflow; Apache-2.0 LICENSE.

**What's MISSING / PRIMITIVE (the docs systematically describe stubs as implemented):**
- **No SMT query ever runs**: `SmtSolver::check_sat` always returns `Unknown` (smt/mod.rs:81-82); both Z3 backends are stubs; the `z3` dependency is used by no crate.
- **All 6 vulnerability detectors return `Vec::new()`**; `StateExplorer::explore` returns 0 paths; `SpecGenerator` returns 3 hardcoded candidates; concolic has no negation/solve loop; verification always `Inconclusive("not yet implemented")`; gas analysis is a placeholder (no EIP-2929/2200 applied by the executor).
- **Most decisive:** `AnalysisEngine::analyze_bytecode` never feeds parsed bytecode into any pass (analysis/mod.rs:108-140 — all passes called with `&[]`). **No contract can be analyzed end-to-end today.**
- Executor gaps: SWAP is off-by-one (executor.rs:311-315), JUMPI ignores its condition, KECCAK/CALL/LOG/block opcodes unsupported, default gas `u64::MAX` (OutOfGas never triggers).
- No test fixtures (empty `tests/fixtures/`); no fuzz/bench; **no git repo** (CI never ran); no user guide/changelog; README/whitepaper roadmap tables mark stubs as "✅ Implemented".

**What's TO BE DONE:** wire real Z3; implement detectors/explorer/concolic/spec-gen; fix executor bugs; add real-contract fixtures and semantic tests — only then is Phase 1 complete per its own plan (DEVELOPMENT_PLAN.md:22-35).

---

### 3.18 Transit OS — 🟡 PARTIALLY ready (confidence: high)

**What it is (per docs):** a "formally verified operating system for autonomous vehicles" — abstract interpretation, symbolic path constraints, real-time SMT collision verification, taint tracking, trustless deployment gate. 58-month solo plan, explicitly a **research/formal-verification framework** ("A solo developer cannot legally deploy this on public roads"); docs present v1.0.0 as complete (15 crates/39 integration tests).

**What's THERE (code):**
- **16 crates as code:** state-machine (bicycle + articulated + air-brake), abstract-interpretation (interval/affine/symbolic), path-constraints (incl. RSS), state-space, verification-invariants (7), gas-brake-analysis, taint-analysis, spec-generator (LTL + kinetic), deployment-gate, dynamics-simulator, real-time, monitoring.
- 39 integration tests (verified); CI workflow (fmt/clippy `-D warnings`/test on stable); rust-toolchain; 8 documents (~75 KB) incl. scenario catalog and an honest `idea.txt` design critique.

**What's MISSING / PRIMITIVE (critical):**
- **The deployment gate never sees live vehicle state** — `Interceptor::intercept` and `VerificationPipeline::verify` check invariants against `let state = Default::default();` (gate.rs:52, pipeline.rs:47) — a zeroed state (velocity 0, yaw 0), making the invariant stage **vacuous and disconnected from runtime**.
- ROS2 bridge **discards its input** and fabricates zeroed values with `latency_us: 0.0` (bridge.rs:16-44); CARLA bridge never connects (static translate functions only); **no Z3** (analytic TCC solver + SMT-LIB2 serialization; manual submission per whitepaper.md:548-552).
- **No unit tests in any crate** (only the 39 integration tests); no fuzz/bench; no LICENSE despite Apache-2.0 claim; version drift (1.0.0 manifests vs `v0.2.0` in-code docs); whitepaper "15 crates" vs 16 actual; `Cargo.lock` gitignored; `scenarios/` empty; no README.

**What's TO BE DONE:** wire the real runtime state into the gate; real ROS2/CARLA middleware; Z3 integration; unit tests; license; closed-track test plan (Phase 4).

---

### 3.19 Vantablack — ❌ NOT ready (confidence: high)

**What it is (per docs):** "Post-Quantum, Cryptographically Layered P2P Chat" (BRP + Black Hole Storage, 7-layer "GHOST" stack) built in Rust over raw UDP; claims hybrid X25519+Kyber512, ChaCha20-Poly1305, Shamir SSS, Reed-Solomon; resistance to passive interception, traffic analysis, MITM, quantum adversaries. Branded "V2.0", Cargo version 0.3.0, **no alpha/beta/PoC disclaimer**. **The current source (`src/main.rs:36`) is a "VANTABLACK VPN — Post-Quantum Encrypted Tunnel" (server/client) — a different product than the docs' P2P chat.**

**What's THERE (code):**
- ChaCha20-Poly1305 AEAD, Reed-Solomon (2,1), Shamir (2,3), partial Ed25519 handshake signing, SessionGuard sliding-window anti-replay + timeouts, UDP transport with retransmission + shard buffering, X25519 + Kyber512 **public-key generation**.

**What's MISSING / PRIMITIVE (severe — this is a security product):**
- **No key exchange at all.** The "master key" is a random 32-byte key, Shamir-split, and **transmitted in-band with the handshake** (main.rs:49-51,88-91). The X25519 secret is dropped without ECDH and the Kyber secret discarded (`crypto.rs:100-108` — "extend this struct if ECDH completion is required later"). The public keys are decorative.
- **This defeats the documented threat model:** a passive observer capturing one handshake burst (3 packets carrying 2-of-3 key shares) + one message burst decrypts the traffic — exactly the "practically infeasible" interception the README claims (README.md:87,528-537).
- The Ed25519 signature covers only the Kyber pubkey bytes, **not the Shamir-share field** — an active MITM can substitute shares; the **client never authenticates the server**.
- `original_len` stored in a `u8` (messages >255 B corrupt decryption); unbounded shard HashMap (memory-exhaustion DoS); pervasive `.unwrap()`/silently-ignored `send_to` errors.
- **Zero tests, no CI, no releases**; `target/` contains only dependency artifacts from another machine — no evidence the crate ever compiled here.
- Docs/duplicates: `WHITEPAPER-VANTABLACK.md` is **byte-identical to README.md**; nested `Vantablack-main/` old snapshot; root-level orphaned `.rs` copies; `vantablack-vpn/` = build output only; README tells users to "download the EXE" — no artifact exists.

**What's TO BE DONE:** implement a real hybrid KEM (X25519+Kyber encapsulation), integrity-bind the key material, client→server authentication, fix the length field, bound the shard pool, add tests/CI, delete duplicates — or honestly re-scope and label the current design a prototype. (Forward secrecy/Double Ratchet already acknowledged as missing at README.md:57.)

---

### 3.20 Zero-Knowledge Genome — ❌ NOT ready (confidence: high)

**What it is (per docs):** a privacy-preserving genomic-data marketplace: genomes on-device (Secure Enclave + SQLCipher), mutation presence proven via zk-SNARKs, local pharma queries, L2 payments, DID consent, differential-privacy aggregation, post-quantum crypto "from day one". 56-month plan; README status: Phase 1 "🚧 In Progress", Phases 2–4 "📋 Planned"; version 0.1.0-alpha; 12 commits by one author in ~19 hours.

**What's THERE (code):**
- A **genuinely solid bioinformatics core (~150+ tests, 20k+ LOC Rust)**: genome types, streaming VCF v4.2 parser, normalizer, hg19/hg38 reference, delta/bit-pack compressor, annotator, **B-tree variant query engine**, DTC-import module; thin mobile/native bridge; 4 skeleton Solidity contracts; minimal Axum API; Python + TypeScript SDKs; 3 CI workflows (the rust.yml is solid: check/fmt/clippy `-D pedantic -D nursery`/test/coverage/`cargo audit`); 8 integration tests; extensive docs (735-line EXECUTION_PLAN, ARCHITECTURE, STRIDE THREAT_MODEL, PRIVACY, v1.2 WHITEPAPER, engineering-challenges with P0/P1/P2 priorities).

**What's MISSING / PRIMITIVE (every security component is a stub):**
- **zk prover** = "Generate simulated proof data" (SHA-3 of inputs, no Groth16/PLONK); **zk verifier accepts any non-empty blob** (verifier/mod.rs:100-120); `.circom` circuits `include "poseidon.circom"` which **does not exist** → circuits won't compile.
- "Dilithium" signature verification is a **byte-length check** (genome-ingestion/src/lib.rs:229-230); **RadioShield (signal isolation — a core security invariant) always reports "Isolated"** and its clock returns 0s (radio_shield.rs:203,236-244); query crypto returns empty ciphertext; DP noise is deterministic (`u = 0.5`); Payment.sol **never transfers**; ProofRegistry has no access control; API: queries stub, stats hardcoded zeros, fake auth UUIDs, CORS permissive.
- Mobile app = **empty directories** (no package.json → the react-native CI workflow fails); no dashboard/datasets/compliance/ADRs/OpenAPI/Docker; zero contract/API/mobile tests; no fuzz/bench; no LICENSE; `EXECUTION_PLAN.md:3` says "Zero code exists" (stale — 20k+ LOC present) and every phase success criterion is unchecked; whitepaper disclaims external audit ("Use at your own risk", WHITEPAPER.md:451).

**What's TO BE DONE:** real zk prover/verifier/circuits; real PQ signature verification; truthful RadioShield; real payments; the mobile app; contract/API tests; license. The existing bioinformatics core is a strong foundation — the crypto layer is the entire remaining distance.

---

### 3.21 sovereign-core (KELLER OS) — ❌ NOT ready (confidence: high)

**What it is (per docs):** a "sovereign microkernel" — capability-based, zero-knowledge-authenticated, allocation-free (`no_std`, no `alloc`), the trusted foundation for a future City OS node runtime. i686/QEMU today; Raspberry Pi ARM64 + x86-64 in Phase 4. Docs: "Phase D substantially complete; Phase E hardware-gated"; WORK_PLAN marks Phases A–D ✅, Phase E (SME/TDX, E-Fuse, IOMMU, Coq/Lean) ❌ deferred.

**What's THERE (code):**
- Boot/multiboot, GDT/IDT/PIC/PIT, bitmap PMM + identity paging + **per-task CR3**, fixed-slot RR scheduler, capability table with O(1) revocation, `int 0x80` cap-gated syscall dispatcher, salted-MAC IPC, serial/PS2/VGA drivers, **AES-XTS vault with NIST-KAT-verified AES**, 256-bit bigint + EC + **Schnorr auth + honeypot**, loopback net stack, e1000 PCI detection, in-kernel coverage-guided fuzzer, HW-feature detection, shell, **26 boot-time self-tests** (AES KAT, EC 2G vector, Schnorr roundtrip, tamper detection) via the `run-tests.ps1` QEMU harness, City service registry, TLA+ specs, host tools.

**What's MISSING / PRIMITIVE:**
- **Still no user-mode ring-3 processes**: `enter_user` trampoline and `proc_create` are **never called** (grep verified) — no user binary is ever loaded or run. **Update 2026-08-12:** kernel-mode task spawning was added — the ten City services (slots 11–20) + three bridges (21–23) start via `city::spawn_services`/`spawn_bridges` (`scheduler::start_task`, `MAX_TASKS` 24, cap-gated heartbeat dispatch; `cargo check` passes, compile-verified only, no QEMU run). "ring-3 isolation" is still CR3-value-difference checks only; ASLR = canary only; no read-only root FS; City DAG = a counter-stub registry; real NIC/DMA absent (`net.rs:3-4` "No real NIC/DMA in this seed").
- **Security claims are demonstrational:** the Schnorr `PROVER_SECRET` and vault AES keys are **compile-time constants** in the binary (auth.rs:44-48, vault.rs:20-27); IPC/net MACs are salted **FNV-1a** (non-cryptographic); TLA+ invariants are effectively vacuous (constant mismatch: comments 64/7, config 8/4, kernel 64/17).
- `repro-build.ps1` hashes one build's output but **does not prove reproducibility** (no pinned toolchain/date; `rust-toolchain.toml` pins "nightly" with no date); `.cargo/config.toml` + `keller-i686.json` hardcode a Windows LLVM path; **no CI, no unit tests, no git commits, no LICENSE**; version mismatch (0.3.0 docs vs 0.1.0 crate); stale docs (DEVELOPMENT_PLAN "Stage 0" with a fixed bug, shell comment referencing "real ZK Schnorr protocol is Phase 3.1"). **Update 2026-08-12:** `.cargo/config.toml` was fixed for current nightlies (`-Zjson-target-spec` was removed from rustc; `[unstable] json-target-spec` + `-Zunstable-options` now satisfy cargo/rustc) — the Windows LLVM path and undated `nightly` remain.

**What's TO BE DONE:** real ring-3 process loading; key management (no compile-time secrets); cryptographic MACs; proper TLA+ modeling with matching constants; pinned reproducible toolchain; CI + license. Phase E hardware security remains explicitly deferred.

---

### 3.22 Atmospheric Broadcast OS (ABOS) — ❌ NOT ready (confidence: high)

**What it is (per docs):** a software-defined radio (SDR) OS in Rust for covert, resilient long-range HF/NVIS communication — spread-spectrum stealth, cognitive radio, DTN, ionospheric sounding — and the **HF radio backhaul for Global Ghost Net** (GGN moves encrypted frames over the internet mesh; ABOS over the air, joined by a file-based JSON contract, `docs/GGN_BRIDGE.md`). Stated status: **v0.1.0 "simulation-capable core"** (README.md:10-15); real SDR front-ends are the next milestone. Private/proprietary.

**What's THERE (code):**
- All 14 workspace crates; the **core simulation chain is complete and tested**: file → Reed-Solomon shards → AES-256-GCM DTN bundle → QPSK → DSSS → burst framing → AWGN/multipath channel → acquisition → despread → decrypt → reconstruct (`abos-cli sim`/`bridge`).
- DSP (DDC/decimation/AGC/IQ, Costas+timing, OFDM), PHY (DSSS/FHSS, burst framing, pulse shaping, beamforming), FEC (LDPC, RS, BICM, LLR, CRC), protocol (shard/DTN/bounce/schedule/routing/store on sled).
- **~83 unit tests + 6 cross-crate integration tests** (full pipeline at 4–40 dB, GTF 512 B/1472 B frames, structured low-SNR failures); CI (fmt, clippy `-D warnings`, test, release build); deterministic `--seed`; structured JSON errors; shipped `dist/abos-cli.exe`; unusually honest docs (README/WHITEPAPER/HARDWARE/GGN_BRIDGE).

**What's MISSING / PRIMITIVE:**
- **Every external surface is simulated**: `USRPSDR`/`LimeSDR`/`HackRFSDR` are type aliases of `SimulatedSDR` (abos-hal/src/sdr.rs:121-131); GPIO/GPSDO are state-recording stubs; **nothing has ever touched real RF** (HARDWARE.md:320-324 "not yet wired").
- Costas loop implemented but **not wired** (carrier offsets beyond ~one QPSK quadrant unsupported, abos-bridge/src/lib.rs:417-419); Doppler tracking, SIMD decimation, PAPR reduction missing.
- Cognitive radio (scanner 26 LOC), stealth (mask/phase/amp/burst), and iono sounding (single-point formulas) are **thin, zero tests, not integrated into the pipeline**.
- Mesh/Ghost-node coordination (plan phase 13) not implemented; GUI is a library only (no binary); no CHANGELOG, no fuzzing, no benchmarks; building-plan 12–18-month timeline stale; "Plasma-Wave Modulation" exists only in the canvas, not in any doc/code; SDR recommendation conflicts (HackRF vs USRP B210).

**What's TO BE DONE:** real SDR front-end integration (HackRF/USRP), wire the Costas loop, integrate + test cognitive/stealth/iono, DSP optimization, key revocation without a central authority, HF-license legal compliance, then a first real link with GGN.

### 3.23 Sybil-Resistentes Proof-Of-Personhood — 🟡 PARTIALLY ready (confidence: high)

**What it is (per docs):** decentralized proof-of-personhood — social-graph topology analysis (small-world/Watts–Strogatz), web-of-trust vouching (ed25519), graph-entropy Sybil detection, graph-embedded reputation, BLAKE3 nullifiers, planned zk-SNARK privacy. Stated: "84/84 tests — Phase 2 Complete"; Phase 3 under development, Phase 4 planned (production ~month 50).

**What's THERE (code):**
- Core engines real + tested: graph (petgraph, centrality, community, synthetic gen; 19 tests), sybil (entropy, combined score, ring-lattice; 13), reputation (decay, reactivation; 12), wot (anchors, limits, trust chains; 16), nullifier (BLAKE3; 6); working Axum REST API; dashboard + mobile scaffolds; CI workflow (fmt, clippy `-D warnings`, build, test, dashboard npm build); docs (README/WHITEPAPER/ARCHITECTURE/SPECIFICATION/openapi.yaml).

**What's MISSING / PRIMITIVE:**
- **zk is a toy**: a custom `secret^3+42` hash (2 tests); the circom circuit self-labels "simplified educational circuit", "toy construction for demonstration only", "not real ed25519" (circuits/cluster_membership.circom:17-18,55,61,134).
- DID manager: **0 tests**, **base64 as a base58 placeholder**, signing keys stored unencrypted; CLI `analyze-graph`/`create-did` print "(not yet implemented)".
- Phase 3–4 not done: economics/events/jury scaffold-level ("needs token integration / P2P networking / DAO governance"), dispute smart contracts not started, JS/TS SDK not started (Rust `sybil-sdk` exposes only `health()`), integration-test crate **empty**, dashboard renders synthetic nodes, mobile app is a static placeholder.
- No fuzzing, benchmarks, changelog, LICENSE file (AGPL-3.0 only in Cargo.toml), no git repo; openapi.yaml has paths but no schemas; README "13 crates"/"84 tests" stale vs 11 crates / ~93 tests; whitepaper lists zk as "Future Work (Phase 2)" while README claims Phase 2 complete.

**What's TO BE DONE:** real zk or honest defer to v2 (its own plan recommends deferring zk-SNARKs — DEVELOPMENT_PLAN.md:110), DID/key hardening, Phase-3 governance + smart contracts, JS SDK, live dashboard, integration tests, then deployment + audit.

### 3.24 FHE-DB — 🟡 PARTIALLY ready (confidence: high)

**What it is (per docs):** a database computing on **fully encrypted data** with TFHE (TFHE-rs 1.7) — zero-trust: the server stores/aggregates ciphertexts and never sees plaintext. Phase 1 (encrypted SUM/AVG/COUNT DB) complete+verified; Phase 2 (encrypted NN + Iris demo + `fhe-phone` app) implemented; Phases 3–4 (query planner, SMPC, verifiable FHE, CUDA, audit, open-source release) future. Self-assessed **~40% production readiness** (AGENTS.md §13).

**What's THERE (code):**
- `fhe-core` (keys, encrypt, ciphertext/envelope, algebra, nn), `fhe-db` (schema, database, query, **write-ahead log**, error), `fhe-server` (12 endpoints incl. auth, rate limiting, metrics), `fhe-client`, examples (`encrypted_sum`/`encrypted_nn`/`encrypted_iris`), `apps/fhe-phone` (Argon2id+AES-256-GCM keystore, shell, wire, tests).
- **2 real libFuzzer targets** with documented invariants + CI smoke runs; **criterion benchmarks with hardware-robust ceiling guards** + regression script; golden corpus; vendored TFHE; **4-job CI** (quality / fuzz / bench / TLS).
- Strong security posture for a research project: pinned parameters with a build-blocking gate test, constant-time bearer auth, default-off TLS (rustls), DoS limits/rate limiting, redacted Debug for keys, WAL durability, a documented length-prefix-bomb fix. Excellent build docs incl. the vendored-tfhe Windows linker workaround.

**What's MISSING / PRIMITIVE:**
- **No external cryptographic audit — an explicit blocker** (SECURITY.md §6, AGENTS.md §13:472); the final production-readiness checklist (AGENTS.md §13:495-500) is all unchecked.
- **Bootstrapping not wired** into aggregation paths (deep circuits unsupported, SECURITY.md §3:102-106); keygen/encryption not constant-time; side-channel audit planned.
- Query planner (SELECT/JOIN/GROUP BY), SMPC, verifiable computing/zk, CNN/MNIST, Python bindings, CUDA/100× acceleration — future.
- Loopback default open unless `--auth-token`/TLS; permissive CORS ("tighten for public deployments"); test-only TLS keypair intentionally public; `fhe-phone` not packaged (no installer/code signing).
- No root README/docs/CHANGELOG/LICENSE; license declared BSD-3-Clause-Clear in Cargo.toml vs Apache-2.0 in the plan; stale absolute machine paths (`c:\Users\INTAL Admin\...`); no git repo.

**What's TO BE DONE:** external crypto audit → wire bootstrapping (deep-circuit support) → constant-time keygen/encryption → query planner → packaging/release/license — then declare production per its own checklist.

### 3.25 Global Ghost Net (GGN / "Vantablack") — 🟡 PARTIALLY ready (confidence: high)

**What it is (per docs):** v0.4.0 "post-quantum WAN mesh routing node" — a single Rust executable (~1.1–1.38 MB) where every node is router+relay: hybrid X25519 + Kyber-512/ML-KEM-512 key exchange, ChaCha20-Poly1305, Reed-Solomon (2,1) sharding, SOCKS5 exit, 512-byte jitter-padded UDP frames. Git initial commit: "v0.4.0: post-quantum mesh — shipping-ready". Companion to ABOS (HF bridge) and QEL.

**What's THERE (code):**
- Full CLI (10 commands), **multi-hop relay (verified live)**, beacon/peer/status/reputation commands, SessionGuard anti-replay, **signed beacons, exit allowlist, GHOST_PSK wired into HKDF salt** (CHANGELOG "shipping round").
- **~121 Rust tests** (≈72 unit in `src/`, 44 in `tests/layer_tests.rs`, + sim/virtual-net; README's "66" is stale); **live 2-node smoke test** (real UDP/crypto/HTTP, 4/4 then 5/5 checks); **authorized pentest round** with hardening fixes + honest root-cause CHANGELOG; `examples/attack_harness.rs` (8 attack modes); cross-compile CI (Windows msvc / Linux musl / macOS x86_64+aarch64, release artifacts); good secrets hygiene (`identity.key` gitignored).

**What's MISSING / PRIMITIVE:**
- **Stale/contradictory docs**: README vs CHANGELOG (test counts 66 vs 121; "remaining design gaps" listed open in README were fixed per CHANGELOG); **MIT LICENSE file vs README "Proprietary — no license is granted"** — a direct legal conflict.
- **288 KB dead `VANTABLACK.rs`** at root — a stale duplicate of the old `src/main.rs` (imports removed pqcrypto crates; cannot compile; zero references).
- **Fuzz targets exist but are broken**: `fuzz/Cargo.toml` requires a `fuzzing` feature the root Cargo.toml doesn't define; not in CI.
- **TUN mode is a stub** ("wintun not available"; wintun not a dependency); **bootstrap seeds are placeholders** (`vps1.example.com:2270` — "Replace with real VPS addresses before shipping", src/main.rs:54-57).
- **No actual tagged release** (single commit, no tags; the tag-triggered release workflow has never run); `report.json` holds 16 static-analysis FAILs triaged as false positives only in a `.reasonix` log, no remediation ticket.
- Cross-check: this review initially reported QEL's Ghost-Net topology layer as present; on-disk verification found **no such source files** (only a stale `ghost_net.*.pyc`) — QEL Phase 3 remains unimplemented (see §3.14).

**What's TO BE DONE:** replace bootstrap seeds, delete `VANTABLACK.rs`, reconcile README/LICENSE docs, fix the `fuzzing` feature and run fuzz in CI, finish TUN mode, tag a real release, remediate/track the `report.json` findings.

### 3.26 Atomic Mesh OS (AMOS) — ❌ NOT ready (confidence: high)

**What it is (per docs):** originally "an operating system for formal verification of atomic-scale 3D printing", honestly re-scoped (STRATEGIC_REALITY_CHECK.md:36-54) to a **formal-verification platform for existing continuum-scale FDM/SLA/SLM printers (10 µm–1 mm)**: G-code → lattice model → thermal simulation → SMT invariant checks → proof-gated fabrication interlock → quality certificates. Status: **"v0.1.0-dev — Pre-release scaffolding phase"** (README.md:7); every phase labeled "Scaffolded"; production at month 60 of a 5-year plan.

**What's THERE (code):**
- All 14 crates: `amos-core` (materials/traits), `amos-lattice`, `amos-gcode`, `amos-thermal` (1D+2D solvers), `amos-smt` (3 backends: SystemCommand/FileDump/Stub; bundled z3 4.13.0), `amos-invariant`, `amos-constraint`, `amos-explorer`, `amos-crystallization`, `amos-taint` (blake3 chain; **ed25519 signing not implemented**), `amos-spec` (JSON certs), `amos-gate` (fail-closed interlock; `serial.rs` stub), `amos-rt` (scaffold, solver not wired), `amos-app` (offline end-to-end `verify_gcode()` workbench).
- **38 tests** (verified; matches CHANGELOG "38/38") incl. adversarial suites (gate fail-closed, parser sanitization, bounded solver output); `#![deny(unsafe_code)] + #![deny(missing_docs)]` in all crates; unusually honest docs (incl. a STRATEGIC_REALITY_CHECK that admits the plan/canvas contradiction and unused deps); CHANGELOG; LICENSE (proprietary).

**What's MISSING / PRIMITIVE:**
- `amos-constraint`/`amos-explorer`/`amos-crystallization`/`amos-taint`/`amos-rt` are **referenced by no other crate** — only the offline workbench is wired, and it doesn't use them; the real-time proof-gating loop doesn't run end-to-end.
- **No CI, no git repo**; **no benchmarks** (criterion declared but unused) — the single biggest documented technical risk is unbenchmarked real-time SMT feasibility (STRATEGIC_REALITY_CHECK.md:103).
- No hardware integration (serial stub); certificates "not yet cryptographically signed — a record, not a seal" (USER_GUIDE.md:96-97); no experimental validation; "open-source" in the Abstract contradicts the proprietary LICENSE.

**What's TO BE DONE:** wire pipeline→solver, implement ed25519 signing, per-layer gate permits, real wall-clock deadlines, a solver benchmark suite, git init + CI, experimental validation — or remain an honest scaffold (its own roadmap says exactly this).

### 3.27 Ambient Thermal Arbitrage (ATA) — 🟡 PARTIALLY ready (confidence: high)

**What it is (per docs):** turns temperature-controlled facilities (supermarket freezers, cold-storage warehouses, data centers) into "virtual batteries" that shift refrigeration load for energy arbitrage / grid balancing / demand-side management. Two Rust crates: `thermal-arbitrage-mvp` **v0.2.0** ("production pilot gateway": control loop, HTTP API, Modbus PLC, alerts, OTA, live data) + `thermal-fleet` **v2.0.0** (21-facility-type 8760 h simulator). Phase 0+1 complete; **Phase 2 Production Pilot "in progress — majority implemented" (5/8 checklist items)**.

**What's THERE (code):**
- 6-mode cascade + MPC + ASCT governor, RC thermal model + dynamic COP + RLS, sensor abstraction, financial tracking, Axum API, HMAC-SHA256 auth + secret redaction, native Modbus TCP client, alerts (webhook/retry), OTA (Ed25519+SHA-256), live data (ENTSO-E/energy-charts/SMARD/Open-Meteo + staleness + grid frequency), Prometheus metrics, CSV logging, 8760 h simulator, real NRW 2025 dataset (1.6 MB), fleet engine, **~70 gateway tests**.
- CI workflow + **automated E2E harness against a fake Modbus PLC (5 checks)**, release packager with sha256, **shipped v0.2.0 release bundle**, DEPLOYMENT.md (systemd/Docker/Windows service/RPi cross-compile + go-live checklist), pentest output + hardening-plan (fixes marked done), `overflow-checks = true`, fail-closed auth, TLS half-config refusal.

**What's MISSING / PRIMITIVE:**
- **Go-live checklist entirely unchecked** (DEPLOYMENT.md:306-320); field validation vs a real PLC + real ENTSO-E key is "Next" (CHANGELOG); **E2E uses a fake PLC**; **CI never ran** (no git remote; 11 commits, no tags).
- **Fleet engine has zero tests**; no Grafana/field deployment; gateway is on/off only (continuous VFD modulation exists only in the fleet engine).
- Documented "Known Limitations": MPC pre-cooling historically inactive in live mode, `POST /price` transient in sim, CSV no rotation, no stale-price guard, Modbus read-optimistic (README.md:208-224) — partially contradicted by the newest CHANGELOG.
- Default secret `"change-me-in-production"` in the shipped config; placeholder webhook URL in DEPLOYMENT.md; doc-version skew (whitepaper v3.0 vs mvp WHITEPAPER v1.0; test counts 49 vs 68 vs 69); `report.json` FAIL blocks triaged as serde false-positives.

**What's TO BE DONE:** real-PLC + real-market field test (its own "Next"), complete the go-live checklist, add fleet-engine tests, run CI on a host, reconcile the three test counts and doc versions, then Phase 2.5 hardening.

**Update 2026-08-12:** a City OS adapter now exists — `city-ata` maps the gateway `/status` snapshot to `city-alloc` demand/scarcity, MAC-signs flex credits into `city-incentive`, and authors grid-balancing events into the `city-dag` (5 tests; host-side HTTP/HMAC glue is specified in `City OS/docs/INTEGRATION_PLAN.md` §2.4 but not implemented to keep the City workspace dependency-free). The ATA module itself is unchanged (go-live checklist still unchecked, no field validation).

---

## 4. Methodology & Caveats

- **Evidence basis:** documentation (README/WHITEPAPER/DEVELOPMENT_PLAN/PROJECT_STATUS/CHANGELOG/AGENTS.md/Obsidian `.canvas` JSON) cross-checked against the file tree (workspace members, `src/` contents, `#[test]` counts via grep, CI workflows, Dockerfiles, git metadata). All `file:line` citations come from the per-project reviews.
- **Read-only assessment:** no `cargo build`/`cargo test`/`pytest`/`pnpm` runs were executed, so "compiles" and "tests pass" claims are verified statically (test counts, CI config, code structure) rather than by execution — where a project had never been built (no `target/`), that is noted.
- **2026-08-12 exception:** during the City OS integration round, `cargo test --workspace` and `cargo run -p city-node` (City OS) and `cargo check` (`sovereign-core`) were executed — the only executed builds in this report. Their test/build claims are therefore execution-verified; all other projects remain static-review-only.
- **All 27 projects are covered.** The six initially labeled "done" were assessed on 2026-08-11 (§3.22–3.27). Cross-check note: the GGN review initially reported QEL's Ghost-Net topology layer as present; on-disk verification shows **no `src/quantumnet/topology/` source and no `ghost_net.py`** (only a stale `__pycache__/ghost_net.*.pyc`), confirming the earlier QEL verdict that Ghost-Net is unimplemented (§3.14).
- **Interpretation of "production ready":** for the safety-critical family (medical/aerospace/automotive/kernel), production readiness would additionally require regulatory certification, which no project claims and no solo developer can obtain — those are assessed against their own stated research-toolchain standards.
