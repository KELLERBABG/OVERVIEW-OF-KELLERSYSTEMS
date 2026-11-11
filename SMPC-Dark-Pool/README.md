# SMPC Dark Pool

A fully private, front-running-resistant decentralized exchange built on threshold cryptography.

Orders are fragmented with **Shamir's Secret Sharing**, matched with **secure multiparty computation (SMPC)** so the order book is never revealed, validated with **zk-SNARKs**, finalized with **BLS threshold signatures**, and settled atomically — coordinated by a **PBFT** validator network.

[![Release](https://img.shields.io/github/v/release/KELLERBABG/SMPC-DP?sort=semver)](https://github.com/KELLERBABG/SMPC-DP/releases)
[![CI](https://github.com/KELLERBABG/SMPC-DP/actions/workflows/ci.yml/badge.svg)](https://github.com/KELLERBABG/SMPC-DP/actions/workflows/ci.yml)

> **Proprietary software.** All rights reserved. This repository is not open source.
> See [LICENSE](LICENSE) for the terms.

---

## Features

- **Blind order matching** — crossing orders are found by SMPC without ever revealing the book
- **Shamir's Secret Sharing** — order price/quantity/timestamp split across validator shares
- **Homomorphic price–time priority** — orders sorted on secret-shared data
- **Commit-reveal + encrypted mempool** — front-running resistant order submission
- **BLS threshold signatures** — t-of-n nodes must cooperate to finalize a trade
- **zK-SNARK order validation** — prove sufficient funds without revealing balance
- **Atomic settlement** — on-chain verification of SMPC match proofs
- **PBFT consensus** — Byzantine fault tolerant validator liveness
- **MPC price discovery** — equilibrium price from aggregate encrypted volume
- **REST + WebSocket API** with an embedded single-page trading console

## Architecture

The workspace is a Rust monorepo (`edition 2024`, nightly toolchain pinned via
[`rust-toolchain.toml`](rust-toolchain.toml)):

| Crate | Purpose |
|-------|---------|
| `sss` | Shamir's Secret Sharing over the BN254 scalar field |
| `smpc-engine` | Secure addition, multiplication, comparison, equality on shares |
| `homomorphic-compare` | Price–time priority sorting on secret-shared orders |
| `commitments` | Pedersen + hash commitments for order immutability |
| `matching-engine` | Blind order matching (crossing detection without revealing the book) |
| `price-discovery` | MPC equilibrium price from aggregate encrypted volume |
| `threshold-crypto` | BLS threshold signatures for t-of-n finalization |
| `zk-validation` | Groth16 circuits + proof system for order validation |
| `front-running` | Commit-reveal and encrypted mempool ordering |
| `settlement` | Atomic settlement proofs verified on-chain |
| `bft` | PBFT consensus for the validator network |
| `node` | P2P networking, node discovery, reputation |
| `api-server` | REST + WebSocket API and embedded trading console (`smpc-cli` companion) |

## Quick Start

```bash
git clone https://github.com/KELLERBABG/SMPC-DP.git
cd SMPC-DP

# The Rust toolchain is pinned via rust-toolchain.toml — rustup installs it automatically.
cargo build --release
cargo test                          # 100+ tests across all crates
cargo run --release -- diag         # SSS, SMPC, BLS, settlement, BFT, reputation diagnostics
cargo run -p api-server             # REST/WS trading node on http://0.0.0.0:8080
docker compose up --build           # 5-node validator cluster + Ganache testnet
```

Two matching models are available:

- **Immediate** (default) — crossing orders match as soon as they are submitted.
- **Periodic** — orders are buffered and matched on a cycle:
  `MATCH_MODE=periodic MATCH_INTERVAL_MS=1000 cargo run -p api-server`

## Trading Console

`cargo run -p api-server` serves a self-contained single-page console at `http://localhost:8080/`
with live order submission, book snapshot, and a WebSocket trade feed — no separate build step.

## Documentation

| Document | Description |
|----------|-------------|
| [Whitepaper](docs/whitepaper.md) | Protocol design, security analysis, implementation details |
| [Protocol Specification](docs/protocol-spec.md) | Cryptographic primitives, protocols, data structures |
| [API Reference](docs/api-reference.md) | REST and WebSocket API reference |
| [Architecture](docs/architecture.md) | System architecture and data flow |
| [Getting Started](docs/getting-started.md) | Build, run, and develop |

## Releases

Latest release: **[v0.1.0](https://github.com/KELLERBABG/SMPC-DP/releases/tag/v0.1.0)**

## License

**Proprietary.** Copyright © 2026 KELLERBABG. All rights reserved. See [LICENSE](LICENSE).