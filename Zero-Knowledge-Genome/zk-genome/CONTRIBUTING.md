# Contributing to Zero-Knowledge Genome

## Development Setup

### Prerequisites
- Rust nightly 2026-05-20 (`rustup install nightly-2026-05-20`)
- Node.js >= 18
- Git LFS (for test datasets)

### Quick Start
```bash
# Build all crates
cargo build --workspace

# Run all tests
cargo test --workspace

# Run lints
cargo clippy --workspace
cargo fmt --all --check
```

### Pre-commit Hooks
```bash
pip install pre-commit
pre-commit install
```

## Project Structure
```
core/           — Rust workspace (all cryptography & genomics)
mobile/         — React Native app + Rust FFI
contracts/      — Solidity smart contracts (Phase 3)
api/            — Rust Axum API server (Phase 3)
dashboard/      — Pharma research dashboard (Phase 3)
docs/           — Architecture, threat model, ADRs
```

## Coding Standards
1. All Rust code is `#![no_std]` compatible where possible
2. All public APIs are fully documented (`#![deny(missing_docs)]`)
3. No unsafe code (`#![deny(unsafe_code)]`)
4. All signatures use CRYSTALS-Dilithium hybrid mode
5. All hashing uses SHA-3 family (post-quantum)

## Pull Request Process
1. Ensure all tests pass: `cargo test --workspace`
2. Ensure lints pass: `cargo clippy --workspace`
3. Update documentation if API changes
4. Add ADR for architectural decisions