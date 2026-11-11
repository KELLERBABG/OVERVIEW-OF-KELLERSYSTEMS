FROM rust:1.85-slim-bookworm AS builder
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY crates/ ./crates/
RUN cargo build --release --bin mesh-cli --bin mesh-api-server && \
    cp target/release/mesh-cli /usr/local/bin/ && \
    cp target/release/mesh-api-server /usr/local/bin/ && \
    strip /usr/local/bin/mesh-cli /usr/local/bin/mesh-api-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/bin/mesh-cli /usr/local/bin/
COPY --from=builder /usr/local/bin/mesh-api-server /usr/local/bin/
COPY config/default.toml /etc/credit-mesh/config.toml
EXPOSE 8899
VOLUME ["/data"]
WORKDIR /data
ENTRYPOINT ["mesh-api-server"]
