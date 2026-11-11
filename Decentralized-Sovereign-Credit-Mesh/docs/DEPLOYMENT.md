# Deployment Guide

## Prerequisites

- Rust 1.85+ (stable)
- Docker + Docker Compose (optional, for containerized deployment)
- 32 GB RAM recommended for large graphs (>10k nodes)
- GitHub repo with CI (see [README](../README.md)) for automated builds/releases

## Releases

Cutting a release is fully automated via GitHub Actions. Tag a version and push:

```bash
git tag v0.2.0
git push origin v0.2.0
```

The `Release` workflow builds native binaries (`mesh-cli`, `mesh-api-server`)
for **linux-x86_64, linux-aarch64, windows-x86_64, macos-aarch64** and publishes
a GitHub Release with tarballs + SHA256 checksums. Every push/PR also runs the
`CI` workflow (fmt, clippy `-D warnings`, build, tests, cargo-deny).

## Local Development

```bash
# Build all crates
cargo build --release

# Initialize a node (CLI mode)
cargo run --release --bin mesh-cli -- init "MyNode"

# Add users and build trust graph
cargo run --release --bin mesh-cli -- add-user "Alice" --id alice_01
cargo run --release --bin mesh-cli -- vouch alice_01 --weight 0.9

# Start API server (with auth token)
MESH_API_TOKEN=$(openssl rand -hex 32) cargo run --release --bin mesh-api-server
```

## Single-Node Deployment

```bash
# Build release binaries
cargo build --release

# Copy binaries
cp target/release/mesh-cli /usr/local/bin/
cp target/release/mesh-api-server /usr/local/bin/

# Create data directory
mkdir -p /var/lib/credit-mesh
cd /var/lib/credit-mesh

# Initialize and start with auth
export MESH_API_TOKEN=$(openssl rand -hex 32)
export MESH_API_HOST=0.0.0.0
mesh-cli init "MainNode"
mesh-api-server &
```

> **Security:** always set `MESH_API_TOKEN`. Without it the API binds with no
> authentication — acceptable only for loopback testing. Signing keys are stored
> separately from state in `mesh-keys/` (0600 permissions on unix); never commit
> that directory or `*_state.json` files.

## Docker Deployment

```bash
# Build and start the API node
MESH_API_TOKEN=$(openssl rand -hex 32) docker-compose up --build

# node listens on :8899
# API requires: Authorization: Bearer <MESH_API_TOKEN>
```

Override the dev default token in `docker-compose.yml` (the compose file uses
`${MESH_API_TOKEN:-change-me}` — replace `change-me` for anything exposed
beyond localhost).

> **Note:** there is no P2P/multi-node networking yet. Each container/process is
> an independent node. Multi-node sync is on the roadmap.

## Configuration

All runtime settings are environment variables (prefixed `MESH_`). The
`config/default.toml` file is a reference for tuning knobs; the API server reads
its settings from the environment:

| Variable | Description | Default |
|----------|-------------|---------|
| `MESH_STATE_FILE` | Path to state file | `mesh_server_state.json` |
| `MESH_KEYRING_FILE` | Path to private signing-key store | `mesh-keys/server.keyring.json` |
| `MESH_API_PORT` | API server port | `8899` |
| `MESH_API_HOST` | API bind address | `127.0.0.1` |
| `MESH_API_TOKEN` | Bearer token required by all endpoints except `/health` | (none → unauthenticated) |
| `MESH_CORS_ORIGINS` | Comma-separated allowed origins; empty = same-origin only | (none) |

## Performance Tuning

For production with large graphs:

```toml
# config/default.toml
[graph]
pagerank_iterations = 200
pagerank_tolerance = 1e-8

[trust]
vouching_max_depth = 5
vouching_max_breadth = 100

[credit]
smpc_default_parties = 7
smpc_default_threshold = 4
```

## Health Check

```bash
# API health endpoint (no auth required)
curl http://localhost:8899/health

# Expected response:
# {"status":"ok","nodes":42,"edges":156,"loans":3}
```

## Example API session (with auth)

```bash
TOKEN=your-token

# Add users
curl -X POST http://localhost:8899/users \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"name":"Alice","id":"alice_01"}'

# Vouch
curl -X POST http://localhost:8899/vouch \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"voucher_id":"alice_01","target_id":"bob_01","weight":0.9,"relationship":"friend"}'

# Create a loan (registers a double-borrow nullifier)
curl -X POST http://localhost:8899/loans \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"borrower":"alice_01","lender":"bob_01","principal":2000,"purpose":"expansion"}'
```
