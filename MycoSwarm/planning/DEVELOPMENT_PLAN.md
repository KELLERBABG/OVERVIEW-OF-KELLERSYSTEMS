# Fungal Compute Swarm — Development Plan

## Overview
A decentralized computing platform where browser nodes form a mycelial network to distribute, execute, and verify computational tasks using P2P connectivity, WebAssembly, and cryptographic integrity proofs.

> **Per-phase detailed plans are in separate files linked below.** Each contains exact file structures, interface definitions, algorithm details, test strategies, and step-by-step build instructions.

## Phase Quick Reference

| Phase | File | Est. Duration |
|-------|------|---------------|
| 0 — Foundation | [`phase-0-foundation.md`](./phase-0-foundation.md) | Weeks 1–2 |
| 1 — P2P Mesh Network | [`phase-1-p2p-mesh.md`](./phase-1-p2p-mesh.md) | Weeks 3–5 |
| 2 — Resource Scoring & Task Distribution | [`phase-2-distribution.md`](./phase-2-distribution.md) | Weeks 6–9 |
| 3 — Integrity & Privacy (zk-SNARKs, HE) | [`phase-3-integrity.md`](./phase-3-integrity.md) | Weeks 10–13 |
| 4 — Consensus & Incentives | [`phase-4-consensus-incentives.md`](./phase-4-consensus-incentives.md) | Weeks 14–17 |
| 5 — Client, Demo & Polish | [`phase-5-client-polish.md`](./phase-5-client-polish.md) | Weeks 18–20 |

## Project Structure (Monorepo)

```
fungal-compute-swarm/
├── packages/
│   ├── core/               # Shared types, constants, utilities
│   ├── signaling-server/   # WebRTC signaling server (Node.js)
│   ├── mycelial-mesh/      # P2P WebRTC mesh networking
│   ├── wasm-executor/      # WebAssembly sandbox execution
│   ├── task-distribution/  # Fractal task splitting & distribution
│   ├── redundancy/         # Probabilistic redundancy (Poisson)
│   ├── resource-scoring/   # Entropy-based hardware scoring
│   ├── zk-integrity/       # zk-SNARKs proof verification
│   ├── homomorphic/        # Homomorphic encryption layer
│   ├── signal-transduction/ # Lightweight messaging protocol
│   ├── consensus/          # Mycelial consensus engine
│   ├── substrate-layer/    # Abstraction / adaptation layer
│   ├── incentive-token/    # L2 blockchain micropayments
│   └── client/             # Browser client entry point
├── apps/
│   ├── demo/               # Demo application
│   └── dev-tools/          # Network visualizer, debug tools
├── docs/
│   ├── architecture.md
│   ├── protocols.md
│   └── api-reference.md
├── tests/                  # Integration & e2e tests
├── scripts/                # Build, deploy, benchmark scripts
├── package.json
├── tsconfig.json
└── README.md
```

## Tech Stack

| Layer              | Technology                                      |
|--------------------|-------------------------------------------------|
| Language           | TypeScript (strict mode)                        |
| P2P Networking     | WebRTC (simple-peer, libp2p)                    |
| Compute Runtime    | WebAssembly (wasm-pack, rust/wat target)        |
| Crypto             | snarkjs, circom (zk-SNARKs), SEAL/BFV (HE)      |
| Blockchain (L2)    | Arbitrum/Optimism or custom L2 rollup           |
| Messaging          | Custom thin protocol over WebRTC data channels  |
| Build              | Turborepo + Vite (client), tsup (libs)          |
| Testing            | Vitest, Playwright (e2e), Benchmark.js          |
| CI/CD              | GitHub Actions                                   |

## Development Phases (Summary)

Each phase has a dedicated file with full detail. Click through for complete specifications.

| Phase | File | Key Deliverables |
|-------|------|-----------------|
| **0** Foundation | [`phase-0-foundation.md`](./phase-0-foundation.md) | Monorepo, TypeScript, `@fungal/core` types, CI, docs skeleton |
| **1** P2P Mesh | [`phase-1-p2p-mesh.md`](./phase-1-p2p-mesh.md) | Signaling server, WebRTC mesh manager, binary codec, gossip protocol |
| **2** Distribution | [`phase-2-distribution.md`](./phase-2-distribution.md) | Wasm sandbox, Bio-Score benchmarks, fractal splitter, Poisson redundancy |
| **3** Integrity | [`phase-3-integrity.md`](./phase-3-integrity.md) | Circom circuits, snarkjs prover/verifier, TFHE-rs Wasm HE, probabilistic fallback |
| **4** Economy | [`phase-4-consensus-incentives.md`](./phase-4-consensus-incentives.md) | Saturation monitor, voting engine, Solidity contracts, micropayments, substrate adapters |
| **5** Polish | [`phase-5-client-polish.md`](./phase-5-client-polish.md) | Browser client UI, demo apps, fuzz testing, E2E tests, ADRs, deployment guide |

## Protocol Layers (Communication Stack)

```
┌─────────────────────────────────────────┐
│         Application Layer               │
│  (Task Submission, Result Collection)   │
├─────────────────────────────────────────┤
│         Consensus Layer                 │
│  (Priority Voting, Conflict Resolution) │
├─────────────────────────────────────────┤
│      Incentive / Token Layer            │
│  (Micropayments, Staking, Rewards)      │
├─────────────────────────────────────────┤
│     Integrity & Privacy Layer           │
│  (zk-SNARK Verification, Homomorphic)   │
├─────────────────────────────────────────┤
│      Task Distribution Layer            │
│  (Fractal Split, Assignment, Redundancy)│
├─────────────────────────────────────────┤
│    Resource & Discovery Layer           │
│  (Bio-Score, Capability Advertisement)  │
├─────────────────────────────────────────┤
│   Signal Transduction Protocol          │
│  (Gossip, Status Events, Lightweight)   │
├─────────────────────────────────────────┤
│      Mycelial Mesh (P2P Layer)          │
│  (WebRTC, Signaling, NAT Traversal)     │
└─────────────────────────────────────────┘
```

## Files Reference

| File | Content |
|------|---------|
| [`DEVELOPMENT_PLAN.md`](./DEVELOPMENT_PLAN.md) | This file — overview, tech stack, phases summary, protocol stack, risk register, success criteria |
| [`phase-0-foundation.md`](./phase-0-foundation.md) | Monorepo scaffold, tsconfig, eslint, turbo.json, `@fungal/core` types, CI, Vitest, docs skeleton |
| [`phase-1-p2p-mesh.md`](./phase-1-p2p-mesh.md) | Signaling server (WS), RoomManager, MeshManager, PeerConnectionWrapper, topology strategies, binary codec, gossip engine |
| [`phase-2-distribution.md`](./phase-2-distribution.md) | WasmExecutor sandbox, CPU/memory benchmarks, entropy Bio-Score, fractal splitter, scheduler, reassembler, Poisson redundancy |
| [`phase-3-integrity.md`](./phase-3-integrity.md) | Circom circuits, snarkjs prover/verifier, key management, batch verification, TFHE-rs Wasm HE, probabilistic fallback |
| [`phase-4-consensus-incentives.md`](./phase-4-consensus-incentives.md) | SaturationMonitor, SaturationAwareQueue, VotingEngine, ConflictResolver, FungalToken/ComputeMarket contracts, micropayment channels, reward calculator, SubstrateAdapter |
| [`phase-5-client-polish.md`](./phase-5-client-polish.md) | Browser client boot sequence, Svelte UI components, compute Worker, demo apps (prime/mandelbrot), dev tools, fuzz testing, chaos testing, Sybil resistance, rate limiter, E2E Playwright tests, Docker, ADRs |

## Risk Register

| Risk                          | Mitigation                                        |
|-------------------------------|---------------------------------------------------|
| Browser tab closes mid-task   | Probabilistic redundancy + short timeouts         |
| zk-SNARK proving too slow     | Optimize circuits; fallback to probabilistic re-check |
| WebRTC mesh doesn't scale     | Sparse mesh topology + relay nodes                |
| Homomorphic encryption too heavy | Hybrid approach: encrypt only sensitive subtasks |
| Sybil / malicious nodes       | Stake-based reputation + verification proofs      |
| Browser compatibility gaps    | Feature detection + graceful fallback to JS       |

## Success Criteria v1.0

- [ ] 10+ browser nodes form a working mesh
- [ ] A non-trivial computation (e.g. 1M primes) distributes and completes 2× faster than single-node
- [ ] zk-SNARK verification under 1s per task
- [ ] Token rewards correctly reflect contributed compute
- [ ] Network recovers from >50% node dropout mid-computation
