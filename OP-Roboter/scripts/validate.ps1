# OP-Roboter Automated Production Validation Script
# Runs all compiler lints, unit tests, property stress tests, latency benchmarks, and scenario suites.

$ErrorActionPreference = "Stop"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "   OP-Roboter - Production Pre-Ship Validation Pipeline   " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

Write-Host "`n[1/7] Running Workspace Tests..." -ForegroundColor Yellow
cargo test --workspace
if ($LASTEXITCODE -ne 0) { Write-Error "Unit tests failed"; exit 1 }

Write-Host "`n[2/7] Running 10,000-Iteration Property Stress Tests..." -ForegroundColor Yellow
cargo test -p op-gate --test stress_tests -- --nocapture
if ($LASTEXITCODE -ne 0) { Write-Error "Stress tests failed"; exit 1 }

Write-Host "`n[3/7] Running Clippy with Strict Warnings (-D warnings)..." -ForegroundColor Yellow
cargo clippy --workspace -- -D warnings
if ($LASTEXITCODE -ne 0) { Write-Error "Clippy lints failed"; exit 1 }

Write-Host "`n[4/7] Generating Rust Documentation..." -ForegroundColor Yellow
cargo doc --workspace --no-deps
if ($LASTEXITCODE -ne 0) { Write-Error "Cargo doc failed"; exit 1 }

Write-Host "`n[5/7] Executing Clinical Surgical Scenarios..." -ForegroundColor Yellow
$scenarios = @("safe-insertion", "tissue-breach", "force-overload", "rcm-deviation", "tainted-sensor", "patient-motion")
foreach ($s in $scenarios) {
    Write-Host "  -> Running scenario: $s"
    cargo run -p op-sim -- scenario $s
    if ($LASTEXITCODE -ne 0) { Write-Error "Scenario $s failed"; exit 1 }
}

Write-Host "`n[6/7] Replaying FDA MAUDE Adverse Event Benchmark Suite..." -ForegroundColor Yellow
cargo run -p op-sim -- explore
if ($LASTEXITCODE -ne 0) { Write-Error "MAUDE explore failed"; exit 1 }

Write-Host "`n[7/7] Running 1,000-Cycle Real-Time Latency Benchmark..." -ForegroundColor Yellow
cargo run --release -p op-sim -- benchmark --iterations 1000
if ($LASTEXITCODE -ne 0) { Write-Error "Benchmark failed"; exit 1 }

Write-Host "`n==========================================================" -ForegroundColor Green
Write-Host "   [OK] ALL PRODUCTION CHECKS PASSED - READY TO SHIP!     " -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Green
