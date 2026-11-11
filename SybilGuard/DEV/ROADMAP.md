# SybilGuard — Development Roadmap & Action Tasks

> This file is the authoritative internal development roadmap and task sequencing plan.
> **Language:** `Rust`
> **Summary:** A 5-crate Rust decentralized zero-knowledge Proof-of-Personhood workspace (`core`, `did`, `graph`, `api`, `cli`) with 80 source files implementing social graph topology mapping, betweenness centrality, community clustering, unique human nullifiers, and decentralized jury arbitration.

## Task Queue

- [ ] **TASK-01: Implement Cheeger conductance spectral clustering to isolate Sybil bot farms**
  - **Target Files:** `crates/graph/src/community.rs`, `crates/graph/src/clustering.rs`
  - **Context Files:** `crates/graph/src/graph.rs`, `crates/graph/src/topology.rs`
  - **Verify:** `cargo test -p sybilguard-graph --quiet`
  - **Spec:** Enhance `crates/graph/src/community.rs` with spectral bisection / conductance auditing $\Phi(S) = \frac{|\partial S|}{\min(\text{vol}(S), \text{vol}(\bar{S}))}$.

1. If a subgraph $S$ exhibits internal edge density $> 0.85$ but boundary conductance $\Phi(S) < 0.05$ (indicating an isolated clique of bots cross-endorsing through a narrow bottleneck of compromised honest accounts), flag the entire cluster as `SybilSubnet` and damp its collective voting weight to zero.
2. Add unit tests in `crates/graph` verifying that synthetic bot farms with 100 bots and 2 honest bridge nodes are identified and quarantined.

- [ ] **TASK-02: Harden scoped nullifier generation and replay prevention in api and core**
  - **Target Files:** `crates/core/src/primitives.rs`, `crates/api/src/handlers/nullifier.rs`
  - **Context Files:** `crates/core/src/types.rs`
  - **Verify:** `cargo test -p sybilguard-core -p sybilguard-api --quiet`
  - **Spec:** Ensure `crates/core/src/primitives.rs` computes scoped action nullifiers $N = \text{Hash}(\text{sk}_{\text{user}} \parallel \text{scope\_id})$.

1. In `api/src/handlers/nullifier.rs`, maintain an in-memory/persisted nullifier store rejecting duplicate requests with HTTP 409 Conflict (`NullifierAlreadySpent`).
2. Add unit tests for double-voting prevention and scope isolation (the same user generates distinct, un-linkable nullifiers for two different elections/actions).

- [ ] **TASK-03: Enforce quadratic stake slashing in decentralized jury arbitration**
  - **Target Files:** `crates/core/src/jury.rs`, `crates/core/src/economics/mod.rs`
  - **Context Files:** `crates/core/src/error.rs`
  - **Verify:** `cargo test -p sybilguard-core --quiet`
  - **Spec:** Enhance `crates/core/src/jury.rs` so that when a challenge reveals a verified Sybil identity, jurors who voted to certify the bot lose their staked trust score quadratically: $\Delta \text{stake} \propto (\text{bot\_confidence})^2$.

1. Add unit tests verifying that collusive minority jurors are financially and reputationally liquidated.
