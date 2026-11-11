# Fungal Compute Swarm 🐚

A decentralized computing platform where browser nodes form a mycelial network to distribute, execute, and verify computational tasks using P2P connectivity, WebAssembly, and cryptographic integrity proofs.

## Quick Start

```bash
# Install dependencies
pnpm install

# Build all packages
pnpm build

# Run all tests
pnpm test

# Lint all packages
pnpm lint
```

## Architecture

```
┌─────────────────────────────────────────┐
│         Application Layer               │
├─────────────────────────────────────────┤
│         Consensus Layer                 │
├─────────────────────────────────────────┤
│      Incentive / Token Layer            │
├─────────────────────────────────────────┤
│     Integrity & Privacy Layer           │
├─────────────────────────────────────────┤
│      Task Distribution Layer            │
├─────────────────────────────────────────┤
│    Resource & Discovery Layer           │
├─────────────────────────────────────────┤
│   Signal Transduction Protocol          │
├─────────────────────────────────────────┤
│      Mycelial Mesh (P2P Layer)          │
└─────────────────────────────────────────┘
```

## Project Structure

```
fungal-compute-swarm/
├── packages/
│   ├── core/                  # Shared types, constants, utilities
│   ├── signaling-server/      # WebRTC signaling server (Node.js)
│   ├── mycelial-mesh/         # P2P WebRTC mesh networking
│   ├── wasm-executor/         # WebAssembly sandbox execution
│   ├── task-distribution/     # Fractal task splitting & distribution
│   ├── redundancy/            # Probabilistic redundancy (Poisson)
│   ├── resource-scoring/      # Entropy-based hardware scoring
│   ├── zk-integrity/          # zk-SNARKs proof verification
│   ├── homomorphic/           # Homomorphic encryption layer
│   ├── signal-transduction/   # Lightweight messaging protocol
│   ├── consensus/             # Mycelial consensus engine
│   ├── substrate-layer/       # Abstraction / adaptation layer
│   ├── incentive-token/       # L2 blockchain micropayments
│   └── client/                # Browser client entry point
├── apps/
│   ├── demo/                  # Demo application
│   └── dev-tools/             # Network visualizer, debug tools
├── planning/                  # Development plans & phase docs
├── docs/                      # Architecture, protocols, API
└── scripts/                   # Build, deploy, benchmark scripts
```

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Language | TypeScript (strict mode) |
| P2P Networking | WebRTC (simple-peer, libp2p) |
| Compute Runtime | WebAssembly (wasm-pack, rust/wat target) |
| Crypto | snarkjs, circom (zk-SNARKs), SEAL/BFV (HE) |
| Blockchain (L2) | Arbitrum/Optimism or custom L2 rollup |
| Build | Turborepo + Vite (client), tsup (libs) |
| Testing | Vitest, Playwright (e2e), Benchmark.js |

## License

MIT