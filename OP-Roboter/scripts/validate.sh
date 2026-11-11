#!/usr/bin/env bash
# OP-Roboter Automated Production Validation Script
# Runs all compiler lints, unit tests, property stress tests, latency benchmarks, and scenario suites.

set -euo pipefail

echo "=========================================================="
echo "   OP-Roboter — Production Pre-Ship Validation Pipeline   "
echo "=========================================================="

echo -e "\n[1/7] Running Workspace Tests..."
cargo test --workspace

echo -e "\n[2/7] Running 10,000-Iteration Property Stress Tests..."
cargo test -p op-gate --test stress_tests -- --nocapture

echo -e "\n[3/7] Running Clippy with Strict Warnings (-D warnings)..."
cargo clippy --workspace -- -D warnings

echo -e "\n[4/7] Generating Rust Documentation..."
cargo doc --workspace --no-deps

echo -e "\n[5/7] Executing Clinical Surgical Scenarios..."
for s in safe-insertion tissue-breach force-overload rcm-deviation tainted-sensor patient-motion; do
    echo "  -> Running scenario: $s"
    cargo run -p op-sim -- scenario "$s"
done

echo -e "\n[6/7] Replaying FDA MAUDE Adverse Event Benchmark Suite..."
cargo run -p op-sim -- explore

echo -e "\n[7/7] Running 1,000-Cycle Real-Time Latency Benchmark..."
cargo run --release -p op-sim -- benchmark --iterations 1000

echo -e "\n=========================================================="
echo "   ✔ ALL PRODUCTION CHECKS PASSED — READY TO SHIP!       "
echo "=========================================================="
