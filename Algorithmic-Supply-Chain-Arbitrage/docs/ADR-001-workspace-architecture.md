# ADR-001: Workspace Architecture

## Status
Accepted

## Context
The Algorithmic Supply Chain Arbitrage platform consists of multiple
interconnected modules. A clear separation of concerns is needed to
maintain testability, compile times, and dependency boundaries.

## Decision
Use a Cargo workspace with the following crate structure:

```
sca-types/       — Core domain types (graph, payload, routing, errors)
sca-graph/       — Graph engine wrapping petgraph, DAG enforcement, traversal
sca-api/         — Axum REST API exposing graph operations over HTTP
(sca-smt/)       — Z3 SMT solver integration for path constraint verification
(sca-smpc/)      — SMPC price discovery on encrypted shares
(sca-p2p/)       — P2P layer using libp2p for capacity negotiation
(sca-contracts/) — Solidity smart contracts for settlement
```

Each crate owns its public API surface. Internal fields use `pub(crate)`
to expose to sibling modules within the same crate while keeping
implementation details hidden from consumers.

## Consequences
+ Compilation is incremental across crates.
+ Tests are isolated per crate.
+ Dependency graph is acyclic.
- Requires careful API design for inter-crate boundaries.

## Related
DEVELOPMENT_PLAN.md Phase 1 milestone.