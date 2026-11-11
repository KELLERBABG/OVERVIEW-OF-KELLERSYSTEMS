# Troubleshooting Guide

**Version 1.0.0** | **Common issues and solutions**

## Build Issues

### Z3 Compilation Fails

**Symptom:**
```
error: failed to run custom build script for `z3-sys`
```

**Cause:** Missing C++ compiler or Z3 development libraries.

**Solution:**
```bash
# Ubuntu/Debian
sudo apt install cmake g++ libclang-dev

# Or use geometric solver instead
let solver = SmtSolver::new();  // No Z3 dependency
```

### Cargo Build Slow

**Symptom:** Build takes >5 minutes.

**Solution:**
```bash
# Enable parallel compilation
export CARGO_BUILD_JOBS=8

# Use mold linker (faster than ld)
cargo install mold
mkdir -p .cargo
echo '[target.x86_64-unknown-linux-gnu]\nlinker = "clang"\nrustflags = ["-C", "link-arg=-fuse-ld=mold"]' > .cargo/config.toml
```

## Runtime Issues

### High Latency (>1ms)

**Symptom:** Verification takes too long, missing deadlines.

**Diagnosis:**
```rust
use transit_real_time::latency::LatencyMonitor;

let mut monitor = LatencyMonitor::new();
// ... run verification
monitor.record("verify", elapsed);

if monitor.p99_latency("verify").as_micros() > 1000 {
    tracing::error!("Verification latency too high!");
}
```

**Solutions:**
1. Run on PREEMPT_RT kernel
2. Isolate CPU cores for safety gate
3. Use geometric solver instead of Z3
4. Reduce number of invariants

### CAN Bus Errors

**Symptom:**
```
BusError::Full { max_depth: 1024 }
```

**Cause:** Buffer overflow due to high message frequency.

**Solutions:**
```rust
// Increase buffer size
let bus = InMemoryCanBus::with_capacity(4096);

// Or process messages faster
// ...
```

### ISO-TP Sequence Errors

**Symptom:**
```
IsoTpError("ISO-TP sequence gap: expected 1, got 3")
```

**Cause:** Missing or out-of-order CAN frames.

**Solutions:**
1. Check CAN bus wiring
2. Verify ECU configuration
3. Reduce bus load

### Z3 Timeout

**Symptom:** Z3 solver returns `SolverResult::Unknown`.

**Cause:** Complex collision scenario with many obstacles.

**Solutions:**
1. Use geometric solver for real-time
2. Limit number of obstacles checked
3. Increase Z3 timeout (if not real-time)

## Memory Issues

### Unbounded Growth

**Symptom:** Memory usage increases over time.

**Diagnosis:**
```rust
// Check buffer sizes
println!("CAN TX: {}, RX: {}", bus.tx_len(), bus.rx_len());
```

**Solutions:**
1. Use bounded buffers: `InMemoryCanBus::with_capacity(1024)`
2. Drain buffers periodically
3. Check for memory leaks in sensor drivers

## Sensor Issues

### Sensor Not Responding

**Symptom:**
```
SensorError::NotResponding("lidar_0")
```

**Solutions:**
1. Check hardware connections
2. Verify driver initialization
3. Run self-test: `driver.self_test()?`

### Low Trust Level Blocking

**Symptom:** Commands blocked due to untrusted sensors.

**Diagnosis:**
```rust
if !actuator_command.is_trusted() {
    tracing::warn!("Command blocked: untrusted sensor data");
}
```

**Solutions:**
1. Check sensor trust levels
2. Verify sensor sources are correct
3. Use `TaintedValue::clean()` for trusted sources

## Logging Issues

### No Log Output

**Symptom:** No tracing output.

**Solution:**
```bash
# Set log level
RUST_LOG=info cargo run

# Or in code
tracing_subscriber::fmt()
    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
    .init();
```

### Too Verbose

**Symptom:** Log output is overwhelming.

**Solution:**
```bash
RUST_LOG=warn cargo run
```

## Performance Profiling

### Measure Verification Time

```rust
use std::time::Instant;

let start = Instant::now();
let decision = pipeline.verify(&state, &params, &cmd, &query);
let elapsed = start.elapsed();

tracing::info!(μs = elapsed.as_micros(), "Verification completed");
```

### WCET Analysis

```rust
use transit_real_time::wcet::WcetAnalyzer;

let mut analyzer = WcetAnalyzer::new();
for _ in 0..1000 {
    let start = Instant::now();
    // ... run verification
    analyzer.record("verify", start.elapsed());
}

println!("WCET: {:?}", analyzer.wcet("verify"));
```

## Getting Help

1. Check this troubleshooting guide
2. Review [API Reference](api_reference.md)
3. Search GitHub Issues
4. Join Discord community
