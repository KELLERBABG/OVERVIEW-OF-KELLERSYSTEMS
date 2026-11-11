#!/usr/bin/env bash
# ATA release packager — build both crates and assemble a deployable bundle.
#
# Usage:   bash scripts/release.sh [version]
# Env:     CARGO_TARGET_DIR  (optional; use a space-free path on Windows, e.g. C:/ata-target)
#
# Output:  dist/ata-<version>.tar.gz + .sha256 (binaries, config, seed data, docs)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

GATEWAY="crates/gateway"
FLEET="crates/fleet"
VERSION="${1:-$(grep '^version' "$GATEWAY/Cargo.toml" | head -1 | cut -d'"' -f2)}"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
DIST="dist/ata-$VERSION"

echo "==> Building gateway + fleet (release)..."
cargo build --release --manifest-path "$GATEWAY/Cargo.toml" --target-dir "$TARGET_DIR"
cargo build --release --manifest-path "$FLEET/Cargo.toml" --target-dir "$TARGET_DIR"

echo "==> Assembling $DIST ..."
rm -rf "$DIST"
mkdir -p "$DIST/bin" "$DIST/data_cache"

# Copy only the binaries (exclude cargo .d dep-info and .pdb debug files).
# Hard-fail if an expected binary is missing — a dist/ with no binary is not a release.
for b in "$TARGET_DIR/release/thermal-arbitrage-mvp" "$TARGET_DIR/release/thermal-fleet"; do
  if [ -f "$b.exe" ]; then
    cp "$b.exe" "$DIST/bin/"
  elif [ -f "$b" ]; then
    cp "$b" "$DIST/bin/"
  else
    echo "[ERROR] expected binary missing: $b (did the build succeed?)" >&2
    exit 1
  fi
done
cp "$GATEWAY/config.toml" "$DIST/"
[ -d "$GATEWAY/data_cache" ] && cp -r "$GATEWAY/data_cache" "$DIST/"
cp docs/whitepaper-ATA.md "$DIST/" 2>/dev/null || true
cp "$GATEWAY/DEPLOYMENT.md" "$DIST/" 2>/dev/null || true
cp "$GATEWAY/README.md" "$DIST/" 2>/dev/null || true

echo "==> Packing..."
mkdir -p dist
tar -C dist -czf "dist/ata-$VERSION.tar.gz" "ata-$VERSION"
if command -v sha256sum >/dev/null 2>&1; then
  sha256sum "dist/ata-$VERSION.tar.gz" > "dist/ata-$VERSION.tar.gz.sha256"
elif command -v shasum >/dev/null 2>&1; then
  shasum -a 256 "dist/ata-$VERSION.tar.gz" > "dist/ata-$VERSION.tar.gz.sha256"
fi

echo "==> Done: dist/ata-$VERSION.tar.gz"
ls -lh "dist/ata-$VERSION.tar.gz"
