.PHONY: all build test check clean docs run serve cli

# ─── Build ──────────────────────────────────────────────────────────
all: build

build:
	cargo build --workspace

release:
	cargo build --release --workspace

check:
	cargo check --workspace

# ─── Test ───────────────────────────────────────────────────────────
test:
	cargo test --workspace

test-core:
	cargo test -p sybil-core

test-graph:
	cargo test -p sybil-graph

test-sybil:
	cargo test -p sybil-sybil

test-reputation:
	cargo test -p sybil-reputation

test-wot:
	cargo test -p sybil-wot

test-did:
	cargo test -p sybil-did

test-nullifier:
	cargo test -p sybil-nullifier

test-all: test

# ─── Lint ───────────────────────────────────────────────────────────
clippy:
	cargo clippy --workspace -- -D warnings

fmt:
	cargo fmt --all

lint: clippy fmt

# ─── Run ────────────────────────────────────────────────────────────
serve:
	cargo run -p sybil-cli -- serve

cli:
	cargo run -p sybil-cli --

# ─── Generation ─────────────────────────────────────────────────────
generate-graph:
	cargo run -p sybil-cli -- generate-graph

validate:
	cargo run -p sybil-cli -- validate

# ─── Clean ──────────────────────────────────────────────────────────
clean:
	cargo clean
	rm -rf target/
	rm -rf dashboard/node_modules/
	rm -rf mobile/node_modules/

# ─── Documentation ──────────────────────────────────────────────────
docs:
	cargo doc --workspace --no-deps
	cargo doc --open --workspace --no-deps

# ─── Dashboard ──────────────────────────────────────────────────────
dashboard-install:
	cd dashboard && npm install

dashboard-build:
	cd dashboard && npm run build

dashboard-dev:
	cd dashboard && npm run dev

# ─── Mobile ─────────────────────────────────────────────────────────
mobile-install:
	cd mobile && npm install

# ─── Docker ─────────────────────────────────────────────────────────
docker-build:
	docker build -t sybil-resistentes -f Dockerfile .

# ─── Coverage ───────────────────────────────────────────────────────
coverage:
	cargo tarpaulin --workspace --out Html