# =============================================================================
# Stage 1: Build
# =============================================================================
FROM rust:1.77-slim-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        && rm -rf /var/lib/apt/lists/*

# Copy manifests first for layer caching
COPY Cargo.toml Cargo.lock ./
COPY smpc-cli/Cargo.toml smpc-cli/
COPY crates/sss/Cargo.toml crates/sss/
COPY crates/smpc-engine/Cargo.toml crates/smpc-engine/
COPY crates/homomorphic-compare/Cargo.toml crates/homomorphic-compare/
COPY crates/commitments/Cargo.toml crates/commitments/
COPY crates/matching-engine/Cargo.toml crates/matching-engine/
COPY crates/price-discovery/Cargo.toml crates/price-discovery/
COPY crates/threshold-crypto/Cargo.toml crates/threshold-crypto/
COPY crates/zk-validation/Cargo.toml crates/zk-validation/
COPY crates/front-running/Cargo.toml crates/front-running/
COPY crates/settlement/Cargo.toml crates/settlement/
COPY crates/bft/Cargo.toml crates/bft/
COPY crates/node/Cargo.toml crates/node/
COPY crates/api-server/Cargo.toml crates/api-server/

# Build dependencies only (dummy main to cache layers)
RUN mkdir -p smpc-cli/src crates/sss/src crates/smpc-engine/src \
    crates/homomorphic-compare/src crates/commitments/src \
    crates/matching-engine/src crates/price-discovery/src \
    crates/threshold-crypto/src crates/zk-validation/src \
    crates/front-running/src crates/settlement/src \
    crates/bft/src crates/node/src crates/api-server/src \
    && echo "fn main() {}" > smpc-cli/src/main.rs \
    && for d in sss smpc-engine homomorphic-compare commitments \
               matching-engine price-discovery threshold-crypto \
               zk-validation front-running settlement bft node api-server; do \
        echo "pub fn dummy() {}" > "crates/$d/src/lib.rs"; \
    done \
    && cargo build --release 2>/dev/null || true

# Copy actual source code
COPY . .

# Build the release binary
RUN cargo build --release --bin smpc-cli

# =============================================================================
# Stage 2: Runtime
# =============================================================================
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/smpc-cli /usr/local/bin/smpc-cli

# Expose API and P2P node ports
EXPOSE 8080
EXPOSE 9000-9004

# Set logging level
ENV RUST_LOG=info

# Default command
CMD ["smpc-cli"]
