#!/usr/bin/env bash
# Development wrapper for the EHAR simulation workspace (Git Bash / MSYS).
# Same purpose as dev.cmd: redirects the cargo target dir to a space-free
# path so the GNU toolchain can build windows-sys (dev-dependency of
# criterion) even when the repo path contains spaces.
#
# Usage:  ./dev.sh test --workspace
#         ./dev.sh clippy --workspace --all-targets -- -D warnings
#         ./dev.sh bench --bench run_simulation
#
# Override the target directory by setting CARGO_TARGET_DIR yourself.
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-C:/ehsim-target}"
exec cargo "$@"
