# SMPC-Dark-Pool — Development Roadmap & Action Tasks

> This file is the authoritative internal development roadmap and task sequencing plan.
> **Language:** `Rust`
> **Summary:** An 8-crate Rust decentralized exchange workspace (`matching-engine`, `commitments`, `front-running`, `homomorphic-compare`, `bft`, `node`, `api-server`, `tests`) implementing threshold cryptography, Shamir's Secret Sharing order fragmentation, Pedersen commitments, and PBFT validator consensus.

## Task Queue

- [ ] **TASK-01: Harden secure multiparty comparison protocol for private limit order matching**
  - **Target Files:** `crates/homomorphic-compare/src/order.rs`, `crates/homomorphic-compare/src/priority.rs`
  - **Context Files:** `crates/matching-engine/src/engine.rs`, `crates/commitments/src/pedersen.rs`
  - **Verify:** `cargo test -p smpc-homomorphic-compare --quiet`
  - **Spec:** Upgrade `crates/homomorphic-compare` so that evaluating `bid_price >= ask_price` across secret-shared orders executes via a verified 2-party/3-party comparison protocol (e.g. DGK or Beaver-triple bit-decomposition) without reconstructing the plain prices.

1. Output only a 1-bit encrypted share indicating whether a trade crossed.
2. Add unit tests proving that order books match correctly at the exact market clearing price while individual order prices remain hidden from all participating nodes.

- [ ] **TASK-02: Enforce cryptographic commit-reveal timelocks and mempool anti-front-running invariants**
  - **Target Files:** `crates/front-running/src/commit_reveal.rs`, `crates/front-running/src/mempool.rs`
  - **Context Files:** `crates/commitments/src/hash_commitment.rs`
  - **Verify:** `cargo test -p smpc-front-running --quiet`
  - **Spec:** Strengthen `crates/front-running/src/commit_reveal.rs` so that orders must be committed as $C = \text{Pedersen}(P, Q, r)$ within epoch $T$.

1. The mempool must reject any reveal phase message submitted before the commit phase has closed and been locked by a PBFT block header.
2. Reject any reveal whose commitment opening fails or whose timestamp exceeds the reveal deadline.
3. Add unit tests for front-running insertion attempts and late reveal cancellations.

- [ ] **TASK-03: Enforce strict conservation of asset balances in matching-engine settlement**
  - **Target Files:** `crates/matching-engine/src/engine.rs`, `crates/matching-engine/src/book.rs`
  - **Context Files:** `crates/matching-engine/src/types.rs`
  - **Verify:** `cargo test -p smpc-matching-engine --quiet`
  - **Spec:** Ensure `crates/matching-engine/src/engine.rs` enforces exact integer conservation: for every matched fill of base quantity $Q$ at price $P$, buyer base asset balance increases by $Q - \text{fee}_{\text{base}}$, seller quote asset balance increases by $(Q \cdot P) - \text{fee}_{\text{quote}}$, and the sum of all balances plus exchange fee vault equals the pre-trade total exactly.

1. Add unit and fuzz tests verifying zero underflow or balance drift.
