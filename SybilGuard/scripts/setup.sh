#!/usr/bin/env bash
set -euo pipefail

echo "=== Sybil-Resistentes Proof-of-Personhood Setup ==="

# ── Rust toolchain ──────────────────────────────────────────────
echo "→ Checking Rust toolchain..."
if ! command -v rustc &> /dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi
echo "  Rust: $(rustc --version)"

# ── Build ──────────────────────────────────────────────────────
echo "→ Building workspace..."
cargo build --workspace

# ── Run tests ──────────────────────────────────────────────────
echo "→ Running tests..."
cargo test --workspace

# ── Dashboard dependencies ──────────────────────────────────
if command -v npm &> /dev/null; then
    echo "→ Installing dashboard dependencies..."
    cd dashboard && npm install && cd ..
fi

echo ""
echo "=== Setup complete ==="
echo ""
echo "Quick start:"
echo "  cargo run -p sybil-cli -- serve          # Start API server"
echo "  cargo run -p sybil-cli -- generate-graph   # Generate test graph"
echo "  cargo run -p sybil-cli -- validate         # Run validation"
echo "  make test                                  # Run all tests"