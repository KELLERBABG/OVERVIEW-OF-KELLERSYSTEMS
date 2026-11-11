# Setup script for SMPC Dark Pool development environment
Write-Host "=== SMPC Dark Pool Setup ===" -ForegroundColor Cyan

# Check Rust toolchain
$rustc = rustc --version
if ($?) {
    Write-Host "✓ Rust: $rustc" -ForegroundColor Green
} else {
    Write-Host "✗ Rust not found. Install from https://rustup.rs" -ForegroundColor Red
    exit 1
}

# Add wasm target (for future contract work)
rustup target add wasm32-unknown-unknown 2>$null
if ($?) {
    Write-Host "✓ WASM target added" -ForegroundColor Green
}

# Install system components
$components = @("clippy", "rustfmt", "rust-analyzer")
foreach ($c in $components) {
    rustup component add $c 2>$null
    if ($?) {
        Write-Host "✓ Component: $c" -ForegroundColor Green
    }
}

# Build workspace
Write-Host "`nBuilding workspace..." -ForegroundColor Cyan
cargo build 2>&1
if ($?) {
    Write-Host "✓ Build successful" -ForegroundColor Green
} else {
    Write-Host "✗ Build failed" -ForegroundColor Red
    exit 1
}

# Run tests
Write-Host "`nRunning tests..." -ForegroundColor Cyan
cargo test 2>&1
if ($?) {
    Write-Host "✓ All tests passed" -ForegroundColor Green
} else {
    Write-Host "✗ Tests failed" -ForegroundColor Red
    exit 1
}

# Run linter
Write-Host "`nRunning clippy..." -ForegroundColor Cyan
cargo clippy -- -D warnings 2>&1
if ($?) {
    Write-Host "✓ Clippy passed" -ForegroundColor Green
} else {
    Write-Host "✗ Clippy warnings found" -ForegroundColor Yellow
}

Write-Host "`n=== Setup Complete ===" -ForegroundColor Cyan
Write-Host "`nAvailable commands:" -ForegroundColor White
Write-Host "  cargo build           - Build all crates" -ForegroundColor Gray
Write-Host "  cargo test            - Run all tests" -ForegroundColor Gray
Write-Host "  cargo clippy          - Lint" -ForegroundColor Gray
Write-Host "  cargo doc --open      - Build docs" -ForegroundColor Gray
Write-Host "  cargo run --bin smpc-cli diag - Run diagnostics" -ForegroundColor Gray
