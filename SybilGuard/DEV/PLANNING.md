# SybilGuard (Zero-Knowledge Proof-of-Personhood) — Comprehensive Architecture & Planning Documentation

> **Project Directory:** `SybilGuard/`  
> **Primary Language / Toolchain:** `Rust`  
> **Verification Command:** `cargo test --workspace --quiet`  
> **Reference Sources:** `KELLERSYSTEMS Spine.md` · `PaySlip.canvas` · `Life Work.canvas`

---

## 1. Executive Spine & System Overview

**SybilGuard (Zero-Knowledge Proof-of-Personhood)** — A decentralized zero-knowledge proof-of-personhood protocol that mathematically verifies unique individual human identity without storing or centralizing biometric credentials.

### Current Engineering Baseline
A 5-crate Rust decentralized zero-knowledge Proof-of-Personhood workspace (`core`, `did`, `graph`, `api`, `cli`) with 80 source files implementing social graph topology mapping, betweenness centrality, community clustering, unique human nullifiers, and decentralized jury arbitration.

---

## 2. Canonical Specifications & Mathematical Invariants (`PaySlip.canvas` / `Spine`)

The following 12 architectural pillars and formal invariants define the target engineering state of the system:

- **Social Graph Topology:** Nutzung der Graphentheorie, um die Struktur menschlicher Beziehungen abzubilden. Da echte soziale Netzwerke eine "Small World"-Eigenschaft besitzen, können Bot-Armeen durch ihre unnatürlich hohe Cluster-Dichte und isolierte Konnektivität identifiziert werden.
    
- **Web of Trust (WoT) Anchoring:** Vertrauen wird durch gegenseitige Bestätigungen ("Vouching") zwischen bereits verifizierten Knotenpunkten aufgebaut, wodurch eine organisch gewachsene Vertrauenskette entsteht.
    
- **Privacy-Preserving Proofs (zk-SNARKs):** Ein kryptographisches Verfahren, das beweist, dass ein Nutzer Teil eines legitimen Vertrauens-Clusters ist, ohne den Klarnamen, den exakten Standort im Graphen oder die Identität der verbundenen Freunde preiszugeben.
    
- **Sybil-Resistance by Graph Entropy:** Analyse der Netzwerk-Entropie, um künstlich erzeugte Identitäts-Cluster zu erkennen, die versuchen, das System durch massive Duplizierung zu infiltrieren.
    
- **Recursive Vouching Limits:** Mathematische Begrenzung der "Vertrauens-Power", die ein einzelner Knoten weitergeben kann, um zu verhindern, dass ein kompromittierter Account unendlich viele Fake-Identitäten legitimiert.
    
- **Decentralized Identity (DID) Integration:** Verknüpfung des Vertrauensstatus mit einer selbstsouveränen Identität, die lokal auf dem Gerät gespeichert wird und dem Nutzer die alleinige Kontrolle über seinen digitalen "Menschlichkeits-Beweis" gibt.
    
- **Attack-Vector Mitigation (Anti-Sybil-Cost):** Implementierung einer ökonomischen oder zeitlichen Barriere für das Erstellen neuer Verbindungen, die für echte Menschen vernachlässigbar, für Bot-Betreiber bei Tausenden Identitäten jedoch ruinös ist.
    
- **Graph-Embedded Reputation:** Ein dynamischer Score, der sich aus der Tiefe und Qualität der Einbettung im globalen Graphen berechnet, anstatt auf statischen biometrischen Merkmalen zu basieren.
    
- **Deterministic Identity Nullifiers:** Technischer Mechanismus innerhalb von Zero-Knowledge-Proofs, der verhindert, dass dieselbe reale Person mehrere Accounts mit dem gleichen Vertrauens-Status erstellt (Double-Voting Protection).
    
- **Community-Led Verification Events:** Periodische Synchronisations-Events, bei denen die Lebendigkeit des Netzwerks durch zeitgleiche, dezentrale Interaktionen validiert wird, um statische Bot-Farmen auszuschließen.

---

## 4. Codebase Architecture & Module Inventory

Total tracked source and configuration files in `SybilGuard/`: **79**

| File Path | Lines | Architectural Role |
| :--- | :---: | :--- |
| `Cargo.toml` | 50 | Build / Package Manifest |
| `crates/api/Cargo.toml` | 24 | Build / Package Manifest |
| `crates/api/src/lib.rs` | 21 | Core Implementation Module |
| `crates/api/src/middleware.rs` | 17 | Core Implementation Module |
| `crates/api/src/routes.rs` | 79 | Core Implementation Module |
| `crates/api/src/server.rs` | 19 | Core Implementation Module |
| `crates/api/src/handlers/did.rs` | 76 | Core Implementation Module |
| `crates/api/src/handlers/graph.rs` | 59 | Core Implementation Module |
| `crates/api/src/handlers/mod.rs` | 12 | Core Implementation Module |
| `crates/api/src/handlers/nodes.rs` | 75 | Core Implementation Module |
| `crates/api/src/handlers/nullifier.rs` | 45 | Core Implementation Module |
| `crates/api/src/handlers/reputation.rs` | 30 | Core Implementation Module |
| `crates/api/src/handlers/sybil.rs` | 63 | Core Implementation Module |
| `crates/api/src/handlers/wot.rs` | 94 | Core Implementation Module |
| `crates/cli/Cargo.toml` | 22 | Build / Package Manifest |
| `crates/cli/src/main.rs` | 115 | Core Implementation Module |
| `crates/core/Cargo.toml` | 19 | Build / Package Manifest |
| `crates/core/src/constants.rs` | 127 | Core Implementation Module |
| `crates/core/src/error.rs` | 128 | Core Implementation Module |
| `crates/core/src/jury.rs` | 294 | Core Implementation Module |
| `crates/core/src/lib.rs` | 28 | Core Implementation Module |
| `crates/core/src/primitives.rs` | 144 | Core Implementation Module |
| `crates/core/src/traits.rs` | 153 | Core Implementation Module |
| `crates/core/src/types.rs` | 323 | Core Implementation Module |
| `crates/core/src/economics/mod.rs` | 210 | Core Implementation Module |
| `crates/core/src/events/mod.rs` | 203 | Core Implementation Module |
| `crates/did/Cargo.toml` | 16 | Build / Package Manifest |
| `crates/did/src/lib.rs` | 12 | Core Implementation Module |
| `crates/did/src/manager.rs` | 104 | Core Implementation Module |
| `crates/did/src/resolver.rs` | 20 | Core Implementation Module |
| `crates/did/src/storage.rs` | 40 | Core Implementation Module |
| `crates/graph/Cargo.toml` | 18 | Build / Package Manifest |
| `crates/graph/src/cache.rs` | 195 | Core Implementation Module |
| `crates/graph/src/centrality.rs` | 148 | Core Implementation Module |
| `crates/graph/src/clustering.rs` | 68 | Core Implementation Module |
| `crates/graph/src/community.rs` | 158 | Core Implementation Module |
| `crates/graph/src/graph.rs` | 235 | Core Implementation Module |
| `crates/graph/src/lib.rs` | 26 | Core Implementation Module |
| `crates/graph/src/storage.rs` | 53 | Core Implementation Module |
| `crates/graph/src/synthetic.rs` | 160 | Core Implementation Module |
| `crates/graph/src/topology.rs` | 118 | Core Implementation Module |
| `crates/nullifier/Cargo.toml` | 11 | Build / Package Manifest |
| `crates/nullifier/src/lib.rs` | 16 | Core Implementation Module |
| `crates/nullifier/src/manager.rs` | 125 | Core Implementation Module |
| `crates/nullifier/src/store.rs` | 41 | Core Implementation Module |
| `crates/reputation/Cargo.toml` | 13 | Build / Package Manifest |
| `crates/reputation/src/decay.rs` | 64 | Core Implementation Module |
| `crates/reputation/src/engine.rs` | 174 | Core Implementation Module |
| `crates/reputation/src/lib.rs` | 15 | Core Implementation Module |
| `crates/sdk/Cargo.toml` | 14 | Build / Package Manifest |
| `crates/sdk/src/client.rs` | 33 | Core Implementation Module |
| `crates/sdk/src/error.rs` | 23 | Core Implementation Module |
| `crates/sdk/src/lib.rs` | 18 | Core Implementation Module |
| `crates/sdk/src/types.rs` | 10 | Core Implementation Module |
| `crates/sybil/Cargo.toml` | 15 | Build / Package Manifest |
| `crates/sybil/src/detector.rs` | 221 | Core Implementation Module |
| `crates/sybil/src/entropy.rs` | 175 | Core Implementation Module |
| `crates/sybil/src/lib.rs` | 21 | Core Implementation Module |
| `crates/sybil/src/metrics.rs` | 98 | Core Implementation Module |
| `crates/sybil/src/privacy.rs` | 152 | Core Implementation Module |
| *...and 19 additional files* | — | — |

---

## 5. Gap Analysis & Production-Readiness Audit

- **Sybil Region Detection & Graph Partitioning (`crates/graph/src/community.rs` & `centrality.rs`):** Must detect dense algorithmic bot clusters that attempt to cross-vouch. Must prove that the ratio of inter-cluster to intra-cluster conductance satisfies the Cheeger inequality bound so Sybil clusters cannot bridge into the honest Web of Trust.
- **Zero-Knowledge Nullifier Uniqueness & Replay Prevention (`crates/core/src/primitives.rs` & `crates/api/src/handlers/nullifier.rs`):** Proof-of-personhood action nullifiers ($N = \text{Poseidon}(\text{identity\_secret}, \text{action\_scope})$) must prove one-human-one-vote without revealing social graph connections or biometric identity.
- **Economic Invariant Enforcement (`crates/core/src/economics/mod.rs`):** Slashing and stake forfeiture for colluding Sybil juries.

---

## 7. Actionable Development & Verification Queue

Each task below is synchronized with `DEV/ROADMAP.md` and `DEV/TASKS.json` for automated or manual execution:

| Task ID | Title | Target Files | Verification Command |
| :---: | :--- | :--- | :--- |
| **TASK-01** | Implement Cheeger conductance spectral clustering to isolate Sybil bot farms | `crates/graph/src/community.rs`, `crates/graph/src/clustering.rs` | `cargo test -p sybilguard-graph --quiet` |
| **TASK-02** | Harden scoped nullifier generation and replay prevention in api and core | `crates/core/src/primitives.rs`, `crates/api/src/handlers/nullifier.rs` | `cargo test -p sybilguard-core -p sybilguard-api --quiet` |
| **TASK-03** | Enforce quadratic stake slashing in decentralized jury arbitration | `crates/core/src/jury.rs`, `crates/core/src/economics/mod.rs` | `cargo test -p sybilguard-core --quiet` |
