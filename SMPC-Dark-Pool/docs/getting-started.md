# SMPC Dark Pool — Getting Started Guide

---

## Prerequisites

- **Rust**: Nightly, pinned via [`rust-toolchain.toml`](../rust-toolchain.toml) (edition 2024) — rustup installs it automatically
- **Cargo**: Bundled with the pinned toolchain
- **Docker** + **Docker Compose** (for cluster deployment)
- **Node.js** (for optional EVM contract deployment tools)

## Quick Start

### 1. Clone and Build

```bash
git clone https://github.com/KELLERBABG/SMPC-DP.git smpc-dark-pool
cd smpc-dark-pool
cargo build --release
```

### 2. Run Tests

```bash
# Full test suite (103 tests across 14 crates)
cargo test

# Run specific crate tests
cargo test -p sss
cargo test -p matching-engine
cargo test -p bft
cargo test -p zk-validation
```

### 3. Run CLI Diagnostics

```bash
cargo run --release -- diag
```

This runs tests for:
- SSS split/reconstruct
- SMPC addition
- BLS signatures
- Settlement engine
- BFT consensus (4 nodes)
- Reputation system

### 4. Start the API Server

```bash
# Without TLS (development)
cargo run -p api-server

# With TLS (requires cert + key files)
$env:TLS_CERT_PATH="cert.pem"
$env:TLS_KEY_PATH="key.pem"
cargo run -p api-server
```

The server starts on `http://0.0.0.0:8080`.

### 5. Submit an Order

```bash
curl -X POST http://localhost:8080/api/orders \
  -H "Content-Type: application/json" \
  -d '{"party_id": 1, "side": "bid", "price": "100", "quantity": "10"}'
```

### 6. Check the Order Book

```bash
curl http://localhost:8080/api/book
```

### 7. Run with Docker Compose

```bash
# Start 5-node cluster + Ganache
docker compose up --build

# Health check
curl http://localhost:8080/api/health
```

---

## Project Layout

```
smpc-dark-pool/
├── Cargo.toml                # Workspace root
├── crates/
│   ├── sss/                  # Shamir's Secret Sharing
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── sss.rs       # split_secret, reconstruct
│   │   │   ├── polynomial.rs# Lagrange interpolation
│   │   │   └── field.rs     # Random scalar generation
│   │   └── Cargo.toml
│   ├── smpc-engine/          # SMPC operations
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── add.rs       # secure_add
│   │   │   ├── mul.rs       # secure_mul
│   │   │   ├── compare.rs   # secure_less_than, min, max, equal
│   │   │   ├── equality.rs  # secure equality test
│   │   │   └── types.rs     # SecretShare
│   │   └── Cargo.toml
│   ├── homomorphic-compare/  # Secret-shared comparison
│   ├── commitments/          # Pedersen + hash commitments
│   ├── matching-engine/      # Blind order matching
│   ├── price-discovery/      # MPC price discovery
│   ├── threshold-crypto/     # BLS threshold signatures
│   ├── zk-validation/        # Groth16 zk-SNARKs
│   ├── front-running/        # Commit-reveal + mempool
│   ├── settlement/           # Settlement proofs
│   ├── bft/                  # PBFT consensus
│   ├── node/                 # Networking + reputation
│   └── api-server/           # REST + WebSocket API
├── contracts/
│   └── Settlement.sol        # Solidity settlement contract
├── smpc-cli/                 # CLI tool
├── tests/                    # Integration tests
├── benches/                  # Benchmarks
├── scripts/                  # Deployment scripts
├── docs/                     # Documentation
├── Dockerfile                # Multi-stage build
└── docker-compose.yml        # Multi-node orchestration
```

---

## Development Workflow

### Code Quality

```bash
# Check compilation
cargo check

# Lint (deny all warnings)
cargo clippy -- -D warnings

# Run all tests
cargo test

# Generate documentation
cargo doc --no-deps --open

# Run benchmarks
cargo bench
```

### Adding a New Crate

1. Create `crates/<name>/` with `Cargo.toml` and `src/lib.rs`
2. Add to workspace members in root `Cargo.toml`
3. Add dependencies using `path = "../<dep>"` for internal crates
4. Add `#![forbid(unsafe_code)]` to `lib.rs`

### Testing Conventions

- **Unit tests**: Inline `#[cfg(test)] mod tests` in each source file
- **Integration tests**: In `tests/` directory at workspace root
- **Property-based tests**: Use `proptest` or `quickcheck` for mathematical invariants

---

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | `info` | Logging level |
| `LISTEN_ADDR` | `0.0.0.0:8080` | API server address |
| `TLS_CERT_PATH` | (none) | Path to TLS certificate (optional) |
| `TLS_KEY_PATH` | (none) | Path to TLS key (optional) |
| `API_KEY` | (none) | API key for authenticated endpoints |
| `NODE_ID` | (auto-generated) | UUID for this node |
| `PEERS` | (none) | Comma-separated peer addresses |

## Configuration Tuning

### Network (`NetworkConfig`)

Network parameters are set via `NodeConfig::net_config`. Key tunables for latency-sensitive deployments:

| Parameter | Tuning Guidance |
|-----------|-----------------|
| `batch_timeout_ms` | Lower (1-5ms) for HFT; higher (10-50ms) for throughput |
| `enable_pipelining` | Enable for latency-sensitive broadcast; disable for reliability |
| `initial_retry_interval_ms` | Lower (100ms) for fast failover; higher (5s) to avoid reconnect storms |

### Consensus (`ConsensusConfig`)

PBFT parameters configured via `Consensus::with_config()`:

| Parameter | Tuning Guidance |
|-----------|-----------------|
| `view_change_timeout_ms` | Lower (1s) for fast leader rotation; higher (10s) for stability |
| `jitter_range_ms` | Increase to prevent simultaneous view changes in large clusters |
| `min_peers_for_quorum` | Increase to avoid view changes during network partitions |

### zk-SNARK Proving

Proof generation strategy selected via `Prover::with_strategy()`:

| Strategy | Use Case |
|----------|----------|
| `Cpu` | Development, low-volume deployments |
| `Gpu` | Production with NVIDIA GPUs (roadmap) |
| `Remote` | Thin clients, mobile wallets (roadmap) |


---

## Troubleshooting

### Build Failures

| Error | Solution |
|-------|----------|
| `edition 2024` not supported | Update to nightly 1.97.0+: `rustup default nightly` |
| `could not find crate` | Run `cargo build` from workspace root |
| `blocking waiting for file lock` | Delete `~/.cargo/.package-cache` |

### Runtime Issues

| Issue | Check |
|-------|-------|
| API server won't start | Port 8080 already in use? |
| Docker compose fails | Docker daemon running? Ports available? |
| Test hangs | Network test waiting for TCP connection? Timeout? |

---

## License

**Proprietary.** Copyright © 2026 KELLERBABG. All rights reserved. Not open source.
See [LICENSE](../LICENSE) for the terms.

## Next Steps

1. Read the [Whitepaper](whitepaper.md) for protocol design
2. Read the [Protocol Specification](protocol-spec.md) for implementation details
3. Read the [API Reference](api-reference.md) for REST/WS documentation
4. Explore the [Architecture](architecture.md) overview
