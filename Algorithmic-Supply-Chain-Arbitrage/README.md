# Algorithmic Supply Chain Arbitrage

**Decentralized, trustless logistics optimization — graph topology, SMPC price discovery, formal verification, and smart-contract settlement.**

![Rust](https://img.shields.io/badge/rust-1.97+-orange.svg)
![Tests](https://img.shields.io/badge/tests-137-green.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)

---

## Overview

A decentralized, P2P logistics optimization platform that eliminates information asymmetry, empty runs, and brokered logistics monopolies through algorithmic coordination. Built with **10 modular Rust crates, 137 passing tests**, a REST API server, and a comprehensive technical whitepaper.

## Quick Start

```bash
# Build everything (full clean build: ~2 min)
cargo build --workspace

# Run all 137 tests
cargo test --workspace

# Start the REST API server
cargo run -p sca-api
```

The API server starts at `http://localhost:3000`. Try it:

```bash
# Health check
curl http://localhost:3000/health

# Create a producer node
curl -X POST http://localhost:3000/api/nodes \
  -H "Content-Type: application/json" \
  -d '{"node_type":"producer","label":"Farm Alpha","latitude":51.92,"longitude":4.48}'

# Create a shipment edge
curl -X POST http://localhost:3000/api/edges \
  -H "Content-Type: application/json" \
  -d '{"edge_type":"shipment","source":"<node-id>","target":"<node-id>","label":"Farm→Port","weight":100}'

# Find shortest path
curl -X POST http://localhost:3000/api/paths \
  -H "Content-Type: application/json" \
  -d '{"source":"<node-id>","target":"<node-id>"}'

# List all nodes
curl http://localhost:3000/api/nodes

# Topological sort
curl http://localhost:3000/api/topology/sort
```

## Architecture

```
┌──────────────────────────────────────────────┐
│              Application Layer                │
│  sca-api  sca-spec  sca-contracts             │
├──────────────────────────────────────────────┤
│              Security Layer                   │
│  sca-security (capabilities, IPC, memory      │
│  encryption, ZK auth, panic-zeroing, build    │
│  verification, temporal isolation)            │
├──────────────────────────────────────────────┤
│              Coordination Layer               │
│  sca-p2p  sca-matching  sca-smpc              │
├──────────────────────────────────────────────┤
│              Verification Layer               │
│  sca-graph  sca-smt  sca-payload              │
├──────────────────────────────────────────────┤
│              Foundation Layer                 │
│  sca-types                                   │
└──────────────────────────────────────────────┘
```

## Crates

| Crate | Description | Tests | Key Features |
|-------|-------------|------:|--------------|
| **sca-types** | Core domain model | 13 | Nodes, edges, payloads, constraints, errors |
| **sca-graph** | DAG engine (petgraph) | 16 | Topology, traversal, shortest path, cycle detection |
| **sca-smt** | SMT solver integration | 23 | Path constraints, kinetic invariants, space exploration, SMT-LIB gen |
| **sca-payload** | Taint-coded payload tracking | 9 | ed25519 custody chain, integrity verification, perishability |
| **sca-matching** | Arbitrage balancing | 12 | Order book, matching engine, opportunity detection |
| **sca-p2p** | P2P capacity orchestration | 8 | Peers, protocol, negotiation, transport trait |
| **sca-spec** | Logistics spec generator | 3 | Machine-readable freight documents, route legs |
| **sca-contracts** | Settlement gate | 9 | Contract lifecycle (Draft→Funded→Settled), escrow, conditions, proof |
| **sca-smpc** | SMPC price discovery | 18 | Finite field Z_p, Shamir secret sharing, multi-party protocol |
| **sca-security** | Platform security | 26 | Capability revocation, verified IPC, memory encryption, ZK auth, panic-zeroing, build verification, temporal isolation |
| **sca-api** | REST API server | — | Axum HTTP, 13 endpoints, graph CRUD, pathfinding |
| **Total** | | **137** | **100% passing** |

## Documentation

| Document | Content |
|----------|---------|
| **[Whitepaper](docs/WHITEPAPER.md)** | ~3,000-word technical whitepaper: architecture, cryptographic primitives, protocol flows, security analysis, performance characteristics, deployment architecture, development roadmap |
| **[ADR-001](docs/ADR-001-workspace-architecture.md)** | Workspace architecture decisions |
| **[ADR-002](docs/ADR-002-graph-engine-design.md)** | Graph engine design (DAG invariant, petgraph) |
| **[ADR-003](docs/ADR-003-api-surface.md)** | REST API surface (13 endpoints) |
| **[ADR-004](docs/ADR-004-smt-solver-architecture.md)** | SMT solver architecture (two-tier Mock/Z3) |
| **[Development Plan](DEVELOPMENT_PLAN.md)** | Original 5-year roadmap with phases 1-4 |
| **Rustdoc** (`cargo doc --workspace --no-deps --open`) | API documentation for all 10 crates |

## Key Concepts

### Deadhead Reduction
The expensive part of freight is the empty return. A trailer that delivers to Munich and drives back to Hamburg burns fuel carrying nothing, and that cost is priced into every load. ASCA pairs those returns against outbound corridors — the amber and blue routes on the homepage diagram are the same vehicle doing both — and clears the return leg's rate in the SMPC auction without anyone revealing their number. Across the modeled 50,000-node corridor set this cuts empty-run distance by **37.2%** (unpaired empty runs at 100% drop to 62% paired — see the homepage diagram). The same paired-backhaul accounting is exercised on a realistic European set (HAM↔MUC, BER↔FRA, FRA↔ZRH, MUC↔VIE round trips) in the sca-graph integration tests.

### Supply Chain Graph
Producers, transport hubs, and consumers form a **directed acyclic graph** (DAG). Each edge carries weight (cost) and constraints (capacity, temperature, border time). Cycles are rejected at insertion time.

### Taint-Coded Payloads
Every custody transfer is signed with **ed25519** and SHA-256 hash-linked — forming an immutable, auditable chain of custody from origin to destination. Tampering is immediately detectable.

### Fair Price Discovery
Buyers and sellers submit **encrypted shares** of their prices using Shamir's Secret Sharing over Z_p (p = 2^61 - 1). The network computes the market clearing price without anyone revealing their private data. Information-theoretic security.

### Trustless Settlement
Funds are held in **escrow** and released only when cryptographic proof of delivery is verified: custody chain integrity, destination match, sensor readings within bounds, and perishability proven.

### Constraint Routing
Routes are checked against real-time constraints using **SMT-based verification** (capacity, fuel efficiency, border crossing times, temperature). Supports Z3-compatible SMT-LIB 2.0 generation.

### Platform Security
Seven security primitives: capability-based access control with revocation, verified IPC with tamper-proof messages, total memory encryption for sensitive data, zero-knowledge peer authentication, panic-time memory zeroing, deterministic build verification, and temporal execution isolation.

## Development Status

All 10 core components from the 5-year development plan are implemented and tested. Plus 7 security primitives in a dedicated security crate.

- [x] **Phase 0**: Project scaffold (workspace, CI config, ADRs)
- [x] **Phase 1**: Foundation (types, graph engine, SMT solver, mock solver)
- [x] **Phase 2**: Core logistics (payload tracking, arbitrage balancing, SMPC price discovery)
- [x] **Phase 3**: Settlement layer (P2P capacity orchestration, spec generator, settlement gate)
- [x] **Phase 3+**: Platform security (capability revocation, verified IPC, memory encryption, ZK auth, panic-zeroing, build verification, temporal isolation)
- [ ] **Next**: IoT sensor integration, security audit, testnet deployment, production hardening

## License

MIT