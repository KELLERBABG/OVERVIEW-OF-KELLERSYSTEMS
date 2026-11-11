#!/usr/bin/env bash
# Benchmark regression guard for a FIXED machine.
#
# Criterion's timings are only comparable on the same hardware, so this
# script implements the "regression guard" workflow documented in
# crates/fhe-core/benches/envelope.rs:
#
#   On a fixed machine, commit a baseline once:
#       scripts/check-bench-regression.sh save
#
#   Then check regressions before shipping:
#       scripts/check-bench-regression.sh compare
#
# `compare` runs the criterion suite with --baseline saved, which prints a
# change percentage per benchmark; any benchmark that got *slower* is
# flagged. Use `-- --verbose` (after the command) to see the change lines,
# or run `cargo bench -p fhe-core --bench envelope -- --baseline saved`
# directly for the full report.
#
# Note: this is advisory on CI (runner hardware varies between runs); the
# hard, hardware-robust guard lives in crates/fhe-core/tests/bench_guard.rs,
# which runs in the normal `cargo test` flow.

set -euo pipefail

cd "$(dirname "$0")/.."

case "${1:-}" in
  save)
    cargo bench -p fhe-core --bench envelope -- --save-baseline saved --noplot
    ;;
  compare)
    cargo bench -p fhe-core --bench envelope -- --baseline saved --noplot
    ;;
  *)
    echo "usage: $0 {save|compare}" >&2
    exit 1
    ;;
esac
