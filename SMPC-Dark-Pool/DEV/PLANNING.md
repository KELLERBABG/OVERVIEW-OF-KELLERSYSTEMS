# SMPC Dark Pool — Comprehensive Architecture & Planning Documentation

> **Project Directory:** `SMPC-Dark-Pool/`  
> **Primary Language / Toolchain:** `Rust`  
> **Verification Command:** `cargo test --workspace --quiet`  
> **Reference Sources:** `KELLERSYSTEMS Spine.md` · `PaySlip.canvas` · `Life Work.canvas`

---

## 1. Executive Spine & System Overview

**SMPC Dark Pool** — A front-running- and MEV-resistant decentralized exchange where order books remain completely hidden using Shamir's Secret Sharing and Secure Multi-Party Computation (SMPC).

### Current Engineering Baseline
An 8-crate Rust decentralized exchange workspace (`matching-engine`, `commitments`, `front-running`, `homomorphic-compare`, `bft`, `node`, `api-server`, `tests`) implementing threshold cryptography, Shamir's Secret Sharing order fragmentation, Pedersen commitments, and PBFT validator consensus.

---

## 2. Canonical Specifications & Mathematical Invariants (`PaySlip.canvas` / `Spine`)

The following 12 architectural pillars and formal invariants define the target engineering state of the system:

- **Shamir's Secret Sharing:** Zerlegung von Orderdaten (Preis und Menge) in mathematische Fragmente (Shares). Kein einzelner Knoten im Netzwerk besitzt genügend Informationen, um den ursprünglichen Wert zu rekonstruieren.
    
- **Secure Multiparty Computation (SMPC):** Protokoll-Ebene, die es ermöglicht, Vergleiche und Berechnungen (z. B. "Ist Gebot A $\geq$ Gebot B?") direkt auf den verschlüsselten Shares durchzuführen, ohne diese jemals zu entschlüsseln.
    
- **Blind Order Matching:** Ein kryptographischer Mechanismus, der Käufer und Verkäufer automatisch zusammenführt, sobald ihre Bedingungen korrespondieren, während die Orderbücher für alle Teilnehmer – inklusive der Plattformbetreiber – unsichtbar bleiben.
    
- **Front-Running Protection:** Da Gebote während der Übertragung und Verarbeitung verschlüsselt sind, können Miner oder Nodes keine Transaktionen vorziehen, da sie den Inhalt und die Priorität der Orders nicht kennen.
    
- **Threshold Cryptography:** Festlegung einer Mindestanzahl an ehrlichen Knoten ($t$ von $n$), die kooperieren müssen, um einen Trade final zu validieren, was das System resistent gegen kolludierende Angreifer macht.
    
- **Zero-Knowledge Order Validation:** Händler beweisen kryptographisch, dass sie über ausreichende Mittel für ein Gebot verfügen, ohne ihren tatsächlichen Kontostand preiszugeben.
    
- **Commitment Schemes:** Sicherstellung, dass einmal abgegebene Gebote nicht nachträglich verändert werden können (Immutability), während sie im "blinden" Zustand auf einen Match warten.
    
- **MPC-Based Price Discovery:** Berechnung eines fairen Marktpreises basierend auf dem aggregierten, aber verschlüsselten Volumen, um Slippage zu minimieren, ohne Markttendenzen vorzeitig zu leaken.
    
- **Homomorphic Comparison Protocols:** Spezielle mathematische Funktionen, die eine Sortierung von Orders nach Preis-Zeit-Priorität erlauben, ohne die exakten Zeitstempel oder Preisdaten im Klartext offenzulegen.
    
- **Atomic Settlement Integration:** Direkte Kopplung des Dark Pools an eine Blockchain, sodass die Abwicklung (Settlement) nur dann erfolgt, wenn das SMPC-Protokoll einen gültigen Match beweist.

---

## 4. Codebase Architecture & Module Inventory

Total tracked source and configuration files in `SMPC-Dark-Pool/`: **84**

| File Path | Lines | Architectural Role |
| :--- | :---: | :--- |
| `Cargo.toml` | 54 | Build / Package Manifest |
| `crates/api-server/Cargo.toml` | 34 | Build / Package Manifest |
| `crates/api-server/src/lib.rs` | 8 | Core Implementation Module |
| `crates/api-server/src/main.rs` | 66 | Core Implementation Module |
| `crates/api-server/src/middleware.rs` | 106 | Core Implementation Module |
| `crates/api-server/src/routes.rs` | 206 | Core Implementation Module |
| `crates/api-server/src/state.rs` | 87 | Core Implementation Module |
| `crates/api-server/src/types.rs` | 50 | Core Implementation Module |
| `crates/api-server/src/ui.rs` | 35 | Core Implementation Module |
| `crates/api-server/src/ws.rs` | 44 | Core Implementation Module |
| `crates/api-server/src/bin/node-daemon.rs` | 234 | Core Implementation Module |
| `crates/bft/Cargo.toml` | 23 | Build / Package Manifest |
| `crates/bft/src/consensus.rs` | 339 | Core Implementation Module |
| `crates/bft/src/leader.rs` | 84 | Core Implementation Module |
| `crates/bft/src/lib.rs` | 6 | Core Implementation Module |
| `crates/bft/src/types.rs` | 132 | Core Implementation Module |
| `crates/bft/src/view_change.rs` | 148 | Core Implementation Module |
| `crates/commitments/Cargo.toml` | 21 | Build / Package Manifest |
| `crates/commitments/src/hash_commitment.rs` | 99 | Core Implementation Module |
| `crates/commitments/src/lib.rs` | 5 | Core Implementation Module |
| `crates/commitments/src/pedersen.rs` | 110 | Core Implementation Module |
| `crates/commitments/src/traits.rs` | 14 | Core Implementation Module |
| `crates/front-running/Cargo.toml` | 20 | Build / Package Manifest |
| `crates/front-running/src/commit_reveal.rs` | 131 | Core Implementation Module |
| `crates/front-running/src/lib.rs` | 5 | Core Implementation Module |
| `crates/front-running/src/mempool.rs` | 86 | Core Implementation Module |
| `crates/front-running/src/types.rs` | 43 | Core Implementation Module |
| `crates/homomorphic-compare/Cargo.toml` | 18 | Build / Package Manifest |
| `crates/homomorphic-compare/src/lib.rs` | 5 | Core Implementation Module |
| `crates/homomorphic-compare/src/order.rs` | 20 | Core Implementation Module |
| `crates/homomorphic-compare/src/priority.rs` | 50 | Core Implementation Module |
| `crates/homomorphic-compare/src/sort.rs` | 36 | Core Implementation Module |
| `crates/matching-engine/Cargo.toml` | 22 | Build / Package Manifest |
| `crates/matching-engine/src/book.rs` | 174 | Core Implementation Module |
| `crates/matching-engine/src/engine.rs` | 143 | Core Implementation Module |
| `crates/matching-engine/src/lib.rs` | 5 | Core Implementation Module |
| `crates/matching-engine/src/types.rs` | 73 | Core Implementation Module |
| `crates/node/Cargo.toml` | 25 | Build / Package Manifest |
| `crates/node/src/config.rs` | 121 | Core Implementation Module |
| `crates/node/src/discovery.rs` | 105 | Core Implementation Module |
| `crates/node/src/lib.rs` | 7 | Core Implementation Module |
| `crates/node/src/monitoring.rs` | 27 | Core Implementation Module |
| `crates/node/src/network.rs` | 505 | Core Implementation Module |
| `crates/node/src/reputation.rs` | 147 | Core Implementation Module |
| `crates/price-discovery/Cargo.toml` | 15 | Build / Package Manifest |
| `crates/price-discovery/src/aggregation.rs` | 77 | Core Implementation Module |
| `crates/price-discovery/src/discovery.rs` | 64 | Core Implementation Module |
| `crates/price-discovery/src/lib.rs` | 4 | Core Implementation Module |
| `crates/settlement/Cargo.toml` | 22 | Build / Package Manifest |
| `crates/settlement/src/contract.rs` | 91 | Core Implementation Module |
| `crates/settlement/src/engine.rs` | 143 | Core Implementation Module |
| `crates/settlement/src/lib.rs` | 6 | Core Implementation Module |
| `crates/settlement/src/proof.rs` | 71 | Core Implementation Module |
| `crates/settlement/src/types.rs` | 74 | Core Implementation Module |
| `crates/smpc-engine/Cargo.toml` | 22 | Build / Package Manifest |
| `crates/smpc-engine/benches/smpc_bench.rs` | 22 | Core Implementation Module |
| `crates/smpc-engine/src/add.rs` | 83 | Core Implementation Module |
| `crates/smpc-engine/src/compare.rs` | 102 | Core Implementation Module |
| `crates/smpc-engine/src/equality.rs` | 38 | Core Implementation Module |
| `crates/smpc-engine/src/lib.rs` | 8 | Core Implementation Module |
| *...and 24 additional files* | — | — |

---

## 5. Gap Analysis & Production-Readiness Audit

- **Private SMPC Order Matching vs. Plaintext Simulation (`crates/matching-engine` & `crates/homomorphic-compare`):** The readiness report highlighted that crossing logic must run securely on secret shares via garbled circuits or Beaver triples so neither validator nor sequencer ever sees individual limit prices or quantities before clearing.
- **Anti-Front-Running & MEV Prevention Invariant (`crates/front-running/src/commit_reveal.rs`):** Commit-reveal protocols must enforce strict block-hash timelocks and threshold commitments so a malicious validator cannot inject front-running orders after observing secret shares.
- **Atomic Settlement & Conservation of Value (`crates/matching-engine/src/engine.rs`):** Must mathematically prove zero token creation/loss during partial fill execution and fee deduction.

---

## 7. Actionable Development & Verification Queue

Each task below is synchronized with `DEV/ROADMAP.md` and `DEV/TASKS.json` for automated or manual execution:

| Task ID | Title | Target Files | Verification Command |
| :---: | :--- | :--- | :--- |
| **TASK-01** | Harden secure multiparty comparison protocol for private limit order matching | `crates/homomorphic-compare/src/order.rs`, `crates/homomorphic-compare/src/priority.rs` | `cargo test -p smpc-homomorphic-compare --quiet` |
| **TASK-02** | Enforce cryptographic commit-reveal timelocks and mempool anti-front-running invariants | `crates/front-running/src/commit_reveal.rs`, `crates/front-running/src/mempool.rs` | `cargo test -p smpc-front-running --quiet` |
| **TASK-03** | Enforce strict conservation of asset balances in matching-engine settlement | `crates/matching-engine/src/engine.rs`, `crates/matching-engine/src/book.rs` | `cargo test -p smpc-matching-engine --quiet` |
