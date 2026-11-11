# SMPC Dark Pool — Documentation

A fully private, front-running-resistant decentralized exchange using Shamir's Secret Sharing, Secure Multiparty Computation (SMPC) for blind order matching, threshold cryptography for validation, zk-SNARK order validation, and atomic blockchain settlement.

> **Status:** Latest release **[v0.1.0](https://github.com/KELLERBABG/SMPC-DP/releases/tag/v0.1.0)** ·
> **License:** Proprietary — all rights reserved. See [LICENSE](../LICENSE).

## Documents

| Document | Description |
|----------|-------------|
| [Whitepaper](whitepaper.md) | Full technical whitepaper with protocol design, security analysis, and implementation details |
| [Protocol Specification](protocol-spec.md) | Detailed specification of all cryptographic primitives, protocols, and data structures |
| [API Reference](api-reference.md) | REST and WebSocket API documentation |
| [Architecture](architecture.md) | System architecture, component descriptions, data flow diagrams, and deployment architecture |
| [Getting Started](getting-started.md) | Build instructions, CLI usage, development workflow, and troubleshooting guide |

## Quick Reference

```bash
# Build
cargo build --release

# Test (103 tests across 14 crates)
cargo test

# Lint
cargo clippy -- -D warnings

# Generate docs
cargo doc --no-deps --open

# Run CLI diagnostics
cargo run --release -- diag

# Start API server
cargo run -p api-server

# Start cluster (Docker)
docker compose up --build
```
