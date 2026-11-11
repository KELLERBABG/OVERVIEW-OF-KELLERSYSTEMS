# FluidIP — Development Roadmap & Action Tasks

> This file is the authoritative internal development roadmap and task sequencing plan.
> **Language:** `Rust`
> **Summary:** A 7-crate Rust workspace (`core`, `crypto`, `contracts`, `p2p`, `storage`, `api`, `cli`, `tests`) implementing DAG-based intellectual property dependency graphs, novelty delta verification, SMT prior-art conflict checking, and recursive PageRank-style fractional royalty splits. Needs clean removal of legacy root `.rs` scratch files and full wiring between `core`, `contracts`, and `api`.

## Task Queue

- [ ] **TASK-01: Enforce exact fixed-point conservation in recursive DAG royalty distribution**
  - **Target Files:** `core/src/royalties.rs`, `contracts/src/royalties.rs`
  - **Context Files:** `core/src/graph.rs`, `core/src/contribution.rs`
  - **Verify:** `cargo test -p dfip-core -p dfip-contracts --quiet`
  - **Spec:** Ensure `core/src/royalties.rs` and `contracts/src/royalties.rs` compute transitive ancestral royalty payouts across deep citation DAGs such that the sum of payouts across all ancestor creators plus the leaf creator equals the exact input payment amount (using deterministic largest-remainder integer allocation).

1. Add property and unit tests verifying zero-loss conservation for arbitrary DAG depths and payment amounts.

- [ ] **TASK-02: Wire prior-art semantic entailment check into IP registry and execution gate**
  - **Target Files:** `core/src/delta.rs`, `contracts/src/registry.rs`, `contracts/src/gate.rs`
  - **Context Files:** `core/src/solver.rs`, `core/src/ir.rs`
  - **Verify:** `cargo test --workspace --quiet`
  - **Spec:** Connect `core::delta` and `core::solver` to `contracts::registry` so registering a derivative IP node automatically computes its novelty delta $\Delta_{\text{novelty}}$ against cited parents.

1. Reject registration if $\Delta_{\text{novelty}} < \theta_{\min}$ or if an undeclared prior-art conflict is detected by the constraint solver.
2. Verify in `contracts/src/gate.rs` that blueprint decryption keys are only released when `InfringementStatus::Clean` and `EscrowStatus::Settled` both hold.
