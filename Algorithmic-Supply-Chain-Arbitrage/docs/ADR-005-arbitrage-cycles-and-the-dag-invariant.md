# ADR-005: Arbitrage Cycles and the DAG Invariant

## Status
Accepted

## Context

Two accepted decisions pull in opposite directions:

- **ADR-002** mandates a directed *acyclic* graph. Cycles are rejected at
  insertion because a cycle violates the physical direction of goods flow.
- The platform's stated premise (README, whitepaper) is **arbitrage**: profit
  extracted from price or cost discrepancies between nodes.

Arbitrage in a graph is precisely a *negative-cost cycle* — a closed loop whose
total traversal cost is negative, so running the loop returns more value than it
consumes. That is the exact structure a DAG forbids. A graph cannot be both
strictly acyclic and arbitrage-bearing.

This was not a hypothetical conflict. Two graph engines existed side by side:

| File | Behaviour | Built? |
|---|---|---|
| `crates/sca-graph/src/lib.rs` | General digraph; **no** cycle check on `add_edge` | yes |
| `crates/sca-graph/src/engine.rs` | Rejects cycles with `ScaError::CycleDetected` | no — never declared as a module |

So the documented invariant (ADR-002) lived only in dead code, while the shipped
engine silently permitted cycles. Neither file could satisfy both goals as
written.

## Decision

Treat the two graphs as different objects rather than compromising one of them:

1. **Physical route graph** — the freight topology that routing queries run over.
   It stays acyclic. The acyclic property is a *policy* to be enforced where
   routes are committed, not baked into `add_edge`.
2. **Cost graph** — the same corridors viewed as signed costs. It is a general
   directed graph, because signed corridor costs are what make backhaul pairing
   and arbitrage visible.

Arbitrage detection runs **Bellman-Ford negative-cycle detection** over the cost
graph (`GraphEngine::detect_arbitrage`), initialising every node's distance to
zero so a cycle is found wherever it sits, in `O(V * E)`.

`add_edge` does **not** reject cycles. Rejecting them would make `detect_arbitrage`
unreachable by construction.

## Consequences
+ Arbitrage is expressible: `detect_arbitrage` returns the concrete corridors and
  the per-lap profit, not just a boolean.
+ Code and documentation now agree — the shipped engine's behaviour is the
  documented behaviour.
+ Directed acyclicity remains available as an explicit validation step (e.g.
  before a route is committed), instead of an invisible insertion-time filter.
- Callers that assumed `add_edge` rejected cycles must call the check explicitly.
- The duplicate engine in `engine.rs` still needs to be reconciled or deleted;
  this ADR records the intended semantics so the merge has a target.

## Related
ADR-002 (graph engine design), `crates/sca-graph/src/lib.rs`,
`crates/sca-graph/src/engine.rs`, README "Key Concepts".
