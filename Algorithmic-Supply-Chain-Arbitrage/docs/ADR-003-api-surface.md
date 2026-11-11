# ADR-003: REST API Surface

## Status
Accepted

## Context
The graph engine needs a network-accessible API for integration with
other components (UI, IoT, other services).

## Decision
Use **Axum 0.8** with:

| Endpoint | Method | Description |
|---|---|---|
| `/health` | GET | Health check |
| `/api/nodes` | GET/POST | List/create nodes |
| `/api/nodes/{id}` | GET/PATCH/DELETE | CRUD single node |
| `/api/nodes/{id}/edges/outgoing` | GET | Outgoing edges |
| `/api/nodes/{id}/edges/incoming` | GET | Incoming edges |
| `/api/nodes/{id}/reachable` | GET | BFS reachable nodes |
| `/api/edges` | GET/POST | List/create edges |
| `/api/edges/{id}` | GET/PATCH/DELETE | CRUD single edge |
| `/api/paths` | POST | Shortest path (Dijkstra) |
| `/api/topology/sort` | GET | Topological sort |
| `/api/topology/subgraph` | POST | Extract subgraph |

State is shared via `Arc<Mutex<GraphEngine>>` — adequate for the
prototype phase.

## Consequences
+ Familiar REST semantics — easy to test with curl/httpie.
+ Axum's extractors make route handlers clean.
- Mutex contention under high concurrency (will need sharding later).

## Related
Axum 0.8 docs