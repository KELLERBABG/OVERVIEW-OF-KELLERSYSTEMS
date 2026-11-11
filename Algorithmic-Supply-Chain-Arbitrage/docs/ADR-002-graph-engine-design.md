# ADR-002: Graph Engine Design

## Status
Accepted

## Context
The supply chain is modeled as a directed graph. The plan specifies
a DAG (directed acyclic graph) invariant — cycles violate the
physical direction of goods flow and would cause infinite loops in
path routing.

## Decision
Use `petgraph::graph::DiGraph` as the backing store, with:

- **Node weights** = `sca_types::Node` (producer, transport hub, consumer)
- **Edge weights** = `sca_types::Edge` (shipment, custody transfer)
- **DAG enforcement**: before adding an edge, check
  `has_path_connecting(target, source)` — if a reverse path exists,
  the edge would create a cycle, and it's rejected with
  `ScaError::CycleDetected`.
- **Bidirectional ID maps**: `HashMap<NodeId, NodeIndex>` and
  `HashMap<EdgeId, (NodeIndex, NodeIndex)>` for O(1) lookups by UUID.

## Consequences
+ DAG invariant is enforced at the API boundary — callers cannot
  accidentally create cycles.
+ UUID-based lookup is O(1) regardless of graph size.
- The cycle check adds O(V+E) overhead per edge insertion.

## Related
petgraph 0.6 documentation