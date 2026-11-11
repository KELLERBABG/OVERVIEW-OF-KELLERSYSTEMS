# ─────────────────────────────────────────────────────────────────────
# City OS — Makefile
#
# Quick-start:
#   make build        — debug build (fast)
#   make release      — optimised release build
#   make test         — run the full test suite
#   make check        — build + clippy + fmt (CI gate)
#   make demo         — run the host boot demo
#   make binary-size  — report binary size (release)
#   make clean        — remove build artifacts
#
# Cross-compilation:
#   make cross-aarch64          — release build for aarch64 Linux
#   make cross-thumbv7em        — release build for bare-metal Cortex-M
#
# ─────────────────────────────────────────────────────────────────────

.PHONY: build release test check demo fmt clippy binary-size clean \
        cross-aarch64 cross-thumbv7em

# ── Host builds ───────────────────────────────────────────────────────

build:
	cargo build --workspace

release:
	cargo build --workspace --release

test:
	cargo test --workspace

test-release:
	cargo test --workspace --release

# ── CI gate: build + lint + format check ──────────────────────────────

check:
	cargo fmt --all -- --check
	cargo clippy --workspace -- -D warnings
	cargo test --workspace
	cargo run -p city-node --release

fmt:
	cargo fmt --all

clippy:
	cargo clippy --workspace -- -D warnings

# ── Demo ──────────────────────────────────────────────────────────────

demo: release
	cargo run -p city-node --release

# ── Binary size ──────────────────────────────────────────────────────

binary-size: release
	@echo "─ city-node binary size ─"
	@ls -lh target/release/city-node.exe 2>/dev/null || \
	 ls -lh target/release/city-node 2>/dev/null || \
	 echo "Binary not found (build first)"
	@echo ""
	@echo "─ All library crate .rlib sizes ─"
	@ls -lh target/release/deps/lib*.rlib 2>/dev/null | head -20 || \
	 echo "No release deps found"

# ── Cross-compilation ────────────────────────────────────────────────
# Requires: rustup target add <target>
#           apt install gcc-aarch64-linux-gnu  (for aarch64 linker)

cross-aarch64: release
	@echo "Building for aarch64-unknown-linux-gnu (release) ..."
	cargo build --workspace --release --target aarch64-unknown-linux-gnu
	@echo ""
	@echo "Binary: target/aarch64-unknown-linux-gnu/release/city-node"
	@ls -lh target/aarch64-unknown-linux-gnu/release/city-node 2>/dev/null

cross-thumbv7em: release
	@echo "Building for thumbv7em-none-eabihf (release, bare-metal) ..."
	cargo build --workspace --release --target thumbv7em-none-eabihf
	@echo ""
	@echo "Binary: target/thumbv7em-none-eabihf/release/city-node"

# ── Housekeeping ─────────────────────────────────────────────────────

clean:
	cargo clean
