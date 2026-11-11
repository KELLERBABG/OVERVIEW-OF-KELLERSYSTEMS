# Multi-stage Dockerfile for Transit OS
# Development/CI image with full toolchain
# Production image with minimal dependencies

# ── Stage 1: Builder ──────────────────────────────────────────────
FROM rust:1.82-bookworm AS builder

WORKDIR /app

# Cache dependencies: copy manifests first, create dummy src
COPY Cargo.toml Cargo.lock rust-toolchain.toml rustfmt.toml ./
COPY crates/ /app/crates/

# Build dependencies only (cached layer)
RUN cargo build --release 2>/dev/null || true

# Copy full source and build
COPY . .
RUN cargo fmt --check && \
    cargo clippy --workspace --all-targets -- -D warnings && \
    cargo test --workspace && \
    cargo build --release

# ── Stage 2: Runtime (minimal) ───────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Copy built binaries (if any bins exist)
COPY --from=builder /app/target/release/ /usr/local/bin/

# Copy library artifacts for downstream consumers
COPY --from=builder /app/target/release/*.rlib /usr/local/lib/ 2>/dev/null || true

WORKDIR /app
COPY LICENSE README.md CHANGELOG.md ./

# Non-root user for security
RUN groupadd -r transit && useradd -r -g transit transit
USER transit

LABEL org.opencontainers.image.title="Transit OS"
LABEL org.opencontainers.image.description="Formally verified safety gate for autonomous vehicle control"
LABEL org.opencontainers.image.licenses="Apache-2.0"
