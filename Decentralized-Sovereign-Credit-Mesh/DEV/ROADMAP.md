# Decentralized-Sovereign-Credit-Mesh — Development Roadmap & Action Tasks

> This file is the authoritative internal development roadmap and task sequencing plan.
> **Language:** `Rust`
> **Summary:** A 10-crate Rust workspace (`graph-core`, `wot`, `reputation`, `smpc`, `zkp`, `credit`, `settlement`, `api-server`, `cli`, `tests`) implementing mutual credit clearing, Web-of-Trust vouching, SMPC solvency checks, and cycle-clearing debt compression. Needs non-forgeable Pedersen/Schnorr range commitments in `zkp`, SMPC share verification in `smpc`, and graph cycle-compression flow conservation proofs in `graph-core`.

## Task Queue

- [ ] **TASK-01: Enforce strict net-position conservation in cyclic debt clearing**
  - **Target Files:** `crates/graph-core/src/algorithms.rs`, `crates/settlement/src/pool.rs`
  - **Context Files:** `crates/graph-core/src/graph.rs`, `crates/settlement/src/contract.rs`
  - **Verify:** `cargo test -p dscm-graph-core -p dscm-settlement --quiet`
  - **Spec:** Harden `crates/graph-core/src/algorithms.rs` and `crates/settlement/src/pool.rs` so cycle-detection and multilateral debt compression enforce two exact invariants:

1. **Net Position Invariance:** For every vertex $v \in V$, net credit position $\text{Net}(v) = \sum_{u} w(u, v) - \sum_{w} w(v, w)$ before compression equals $\text{Net}(v)$ after compression.
2. **Monotonic Gross Debt Reduction:** Total gross credit exposure $\sum_{e \in E} w(e)$ strictly decreases by $L \times w_{\min}$ for each cleared cycle of length $L$.
3. Add unit tests verifying 3-hop and 5-hop debt cycle compression and panicking/returning `Err` if any edge weight underflows.

- [ ] **TASK-02: Harden zero-knowledge solvency proofs and Pedersen commitment binding**
  - **Target Files:** `crates/zkp/src/commitment.rs`, `crates/zkp/src/proof.rs`
  - **Context Files:** `crates/credit/src/constraints.rs`, `crates/tests/tests/attack_simulation.rs`
  - **Verify:** `cargo test -p dscm-zkp -p dscm-tests --quiet`
  - **Spec:** Upgrade `crates/zkp/src/commitment.rs` and `proof.rs` so solvency range proofs (`assets - liabilities >= required_margin`) use non-malleable Fiat-Shamir transcripts bound to the epoch ID and borrower public key.

1. Reject any proof with mismatched blinding factors, negative reserve slack, or replayed epoch challenges.
2. Add unit and attack-simulation tests verifying that an insolvent node cannot forge a valid solvency proof.

- [ ] **TASK-03: Verify k-hop default taint propagation and voucher stake slashing**
  - **Target Files:** `crates/reputation/src/taint.rs`, `crates/wot/src/vouching.rs`
  - **Context Files:** `crates/reputation/src/scoring.rs`, `crates/wot/src/limits.rs`
  - **Verify:** `cargo test -p dscm-reputation -p dscm-wot --quiet`
  - **Spec:** Ensure `crates/reputation/src/taint.rs` and `crates/wot/src/vouching.rs` propagate default penalties along directed endorsement edges with distance attenuation $\gamma^d$ ($0 < \gamma < 1$, hop distance $d \in \{1, 2, 3\}$).

1. Automatically reduce credit issuance limits in `wot/src/limits.rs` for endorsers of a defaulted debtor.
2. Add unit tests confirming Sybil ring isolation.

- [ ] **TASK-04: Add verifiable Feldman VSS share integrity checks to SMPC solvency aggregation**
  - **Target Files:** `crates/smpc/src/secret_sharing.rs`, `crates/smpc/src/solvency.rs`
  - **Context Files:** `crates/smpc/src/field.rs`
  - **Verify:** `cargo test -p dscm-smpc --quiet`
  - **Spec:** Extend `crates/smpc/src/secret_sharing.rs` and `solvency.rs` with verifiable polynomial commitments so malicious dealers submitting inconsistent shares during multiparty liquidity aggregation are identified and rejected before Lagrange reconstruction.
