#!/usr/bin/env bash
# The lint gate (D3).
#
# This gate exists because the row that kept it open was written when `clippy`
# could not be installed for the pinned toolchain: `cargo-clippy.exe` was absent
# and the component would not install, so writing a `cargo clippy` step would have
# been CI configuration nobody had ever run. That is no longer true -
# `rustup component add clippy --toolchain nightly-2026-07-24-…` now succeeds and
# the command runs - so the lint half is a gate instead of a claim.
#
# What it enforces, and what it deliberately does not:
#
#   * It denies the lint groups that indicate a bug rather than a taste
#     (`clippy::correctness`, `clippy::suspicious`). Those are clean on this tree,
#     so the denial is enforced and any new one fails the build.
#   * It holds the remaining style warnings under a ceiling rather than denying
#     them. The tree carries 79 of them at f681f07, concentrated in test code
#     (`unnecessary_get_then_check`, `needless_borrow`) plus three
#     `too_many_arguments` on functions whose signature the analysis pass needs.
#     A `-D warnings` gate here would mean a 60-site refactor of tests in the same
#     commit as verification changes; the ceiling means the number cannot grow
#     silently, which is the part that matters for a regression gate. Clearing it
#     is a deliberate job of its own.
#
# The count is per-host. The development host (Windows/GNU) measures 79 at
# f681f07 (plus 0 in the two denied groups); a Linux
# runner prints its own number, so the ceiling has headroom rather than being
# pinned to one machine. Override with KELLER_CLIPPY_MAX_WARNINGS when a
# deliberate cleanup lowers it.
#
# Usage:
#   scripts/lint-gate.sh                       # gate against the ceiling
#   KELLER_CLIPPY_MAX_WARNINGS=0 scripts/…     # what the cleanup should reach
#
# Exit: 0 clean or under the ceiling, 1 a denied lint or a count over the ceiling,
# 2 the gate could not run (no clippy, no Z3) - a gate that cannot run is not a
# pass.
set -uo pipefail

readonly REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly CEILING="${KELLER_CLIPPY_MAX_WARNINGS:-90}"

cd "$REPO" || exit 2

# `build.rs` links Z3, so clippy needs the same environment a build does. The
# project-local install is preferred; Z3_HOME is honoured if it is already set.
if [[ -z "${Z3_HOME:-}" && -d "$REPO/.z3/current" ]]; then
  export Z3_HOME="$REPO/.z3/current"
fi
if [[ -n "${Z3_HOME:-}" ]]; then
  export Z3_SYS_Z3_HEADER="${Z3_SYS_Z3_HEADER:-$Z3_HOME/include/z3.h}"
  export PATH="$Z3_HOME/bin:$PATH"
fi

if ! cargo clippy --version >/dev/null 2>&1; then
  echo "[FAIL] lint gate cannot run: clippy is not installed for the pinned toolchain" >&2
  echo "       rustup component add clippy --toolchain $(rustup show active-toolchain | awk '{print $1}')" >&2
  exit 2
fi

# One run, two verdicts: the deny flags decide the correctness half, and the
# warning count decides the debt half. Warnings are counted from the `-->`
# location lines rustc prints once per diagnostic, not from the trailing
# "generated N warnings" summary, which counts duplicates differently.
output="$(cargo clippy --all-targets -- -D clippy::correctness -D clippy::suspicious 2>&1)"
status=$?
warnings="$(printf '%s\n' "$output" | grep -cE '^[[:space:]]*-->.*\.rs:[0-9]+:[0-9]+$')"

if [[ $status -ne 0 ]]; then
  printf '%s\n' "$output" | grep -E '^(error|error\[)' | head -20 >&2
  echo "[FAIL] lint gate: a correctness or suspicious lint is denied" >&2
  exit 1
fi

if [[ "$warnings" -gt "$CEILING" ]]; then
  echo "[FAIL] lint gate: $warnings style warnings, over the ceiling of $CEILING" >&2
  printf '%s\n' "$output" | grep -E '^[[:space:]]*-->.*\.rs:[0-9]+:[0-9]+$' | head -10 >&2
  echo "       Do not raise the ceiling to pass this; clear the warnings or say why in the commit." >&2
  exit 1
fi

echo "[PASS] lint gate: no correctness/suspicious denials, $warnings style warning(s) (ceiling $CEILING)"
