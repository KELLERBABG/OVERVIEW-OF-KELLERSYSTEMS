# Transit OS Deployment Guide

**Version 1.0.0** | **Target: Hardware-in-the-loop and on-vehicle deployment**

## Overview

This guide covers deploying Transit OS on real autonomous vehicle hardware,
from development boards to production ECUs.

## Prerequisites

### Hardware Requirements

| Component | Minimum | Recommended |
|---|---|---|
| CPU | 4-core ARM Cortex-A53 | 8-core ARM Cortex-A72 or x86_64 |
| RAM | 2 GB | 8 GB |
| Storage | 16 GB eMMC | 128 GB NVMe SSD |
| CAN Interface | 1x CAN 2.0B | 2x CAN FD |
| Ethernet | 1x 100 Mbps | 2x 1 GbE (sensor fusion) |
| ROS2 | Humble or later | Iron (latest LTS) |

### Software Requirements

- **OS:** Ubuntu 22.04 LTS with PREEMPT_RT kernel (or equivalent RTOS)
- **Rust:** 1.82+ stable toolchain (see `rust-toolchain.toml`)
- **ROS2:** Humble/Iron with `rclrs` for Rust integration
- **Z3:** 4.12+ (optional, for formal collision verification)

## Installation

### 1. Build from Source

```bash
git clone https://github.com/<your-org>/transit-os.git
cd transit-os
cargo build --release
```

### 2. Cross-Compilation (ARM64)

For embedded ARM64 targets (e.g., NVIDIA Jetson, Raspberry Pi 4):

```bash
# Install cross-compilation toolchain
rustup target add aarch64-unknown-linux-gnu

# Create .cargo/config.toml
mkdir -p .cargo
cat > .cargo/config.toml << 'EOF'
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"
EOF

# Build for ARM64
cargo build --release --target aarch64-unknown-linux-gnu
```

### 3. Docker Deployment

```bash
# Build the image
docker build -t transit-os:latest .

# Run with CAN bus passthrough (Linux only)
docker run --rm --privileged \
  --device=/dev/can0 \
  --device=/dev/can1 \
  transit-os:latest
```

## PREEMPT_RT Configuration

For deterministic latency guarantees:

```bash
# Install PREEMPT_RT kernel
sudo apt install linux-image-rt-arm64

# Configure CPU isolation (isolate cores 2-3 for safety gate)
sudo grub-editenv -s GRUB_CMDLINE_LINUX_DEFAULT="isolcpus=2,3 nohz_full=2,3 rcu_nocbs=2,3"

# Lock memory (requires CAP_IPC_LOCK or root)
# Transit OS does this automatically via PreemptRtConfig
```

## CAN Bus Setup

### SocketCAN Configuration

```bash
# Enable CAN interface
sudo ip link set can0 up type can bitrate 500000

# For CAN FD
sudo ip link set can0 up type can bitrate 500000 dbitrate 2000000 fd on
```

### Transit OS CAN Integration

```rust
use transit_hal::bus::{InMemoryCanBus, BusAdapter};
use transit_can_bus::frame::{CanActuatorCommand, CanSensorData};

// Create bounded bus (max 1024 frames in queue)
let mut bus = InMemoryCanBus::with_capacity(1024);
bus.init()?;

// Read sensor data from CAN
if let Some((id, data)) = bus.read()? {
    let frame = CanFrame::new(id, data);
    let sensor = CanSensorData::from_frame(&frame)?;
    // Feed into verification pipeline
}
```

## ROS2 Integration

### Node Configuration

```yaml
# transit_safety.yaml
transit_safety_node:
  ros__parameters:
    policy: "Normal"  # Strict, Normal, Permissive
    max_speed: 50.0
    safety_gap: 100.0
    vehicle_class: "PassengerCar"  # or HeavyTruck, ArticulatedTruck
```

### Launch File

```python
from launch import LaunchDescription
from launch_ros.actions import Node

def generate_launch_description():
    return LaunchDescription([
        Node(
            package='transit_ros2_integration',
            executable='transit_safety_node',
            name='transit_safety',
            parameters=['transit_safety.yaml']
        )
    ])
```

## Z3 Integration

### Building Z3 from Source

```bash
# Clone Z3
git clone https://github.com/Z3Prover/z3.git
cd z3
python scripts/mk_make.py
cd build
make -j$(nproc)
sudo make install
```

### Using Z3 Backend

```rust
use transit_smt_solver::SmtSolver;
use transit_smt_solver::query::CollisionQuery;

// Use geometric solver (real-time, no Z3 dependency)
let solver = SmtSolver::new();

// Use Z3 for formal verification (slower, but provably correct)
let solver_z3 = SmtSolver::z3();

let query = CollisionQuery::new(ego_pos, velocity, heading, obstacles, 3.0);
let result = solver_z3.check_collision(&query);
```

## Graceful Shutdown

```rust
use transit_can_bus::shutdown::ShutdownCoordinator;

let coordinator = ShutdownCoordinator::with_timeout(Duration::from_secs(5));
let token = coordinator.token();

// Spawn verification thread
std::thread::spawn(move || {
    while token.is_running() {
        // Process commands
    }
    // Cleanup
});

// Handle SIGTERM/SIGINT
ctrlc::set_handler(move || {
    coordinator.shutdown();
})?;

// Wait for shutdown
coordinator.wait_for_shutdown();
```

## Monitoring

### Health Checks

```rust
use transit_can_bus::watchdog::SafetyCanMonitor;

let mut monitor = SafetyCanMonitor::new();
monitor.watchdog.register(0x100, Duration::from_millis(50));

// Check health periodically
if !monitor.is_healthy() {
    tracing::error!("Safety monitor unhealthy: CRC={}, sequence={}, stale={}",
        monitor.crc_failures,
        monitor.sequence_gaps,
        monitor.stale_events);
}
```

### Prometheus Metrics

```rust
use transit_monitoring::metrics::SafetyMetrics;

let mut metrics = SafetyMetrics::new();
metrics.record_check(true, 150.0); // passed, 150μs latency
metrics.record_check(false, 200.0); // blocked, 200μs latency
```

## Troubleshooting

### Common Issues

| Issue | Cause | Solution |
|---|---|---|
| High latency (>1ms) | Not running on PREEMPT_RT | Configure PREEMPT_RT kernel |
| CAN bus errors | Incorrect bitrate | Match bitrate with other ECUs |
| Z3 timeout | Complex collision scenario | Use geometric solver for real-time |
| Memory growth | Unbounded buffers | Use `with_capacity()` on all buses |

### Debug Mode

```bash
RUST_LOG=debug cargo run --release
```

## Safety Considerations

1. **Always test on closed track first** before public road deployment
2. **Monitor the audit log** for blocked commands — high block rate indicates sensor issues
3. **Watchdog timeout** should be < 10ms for safety-critical applications
4. **Dual-channel redundancy** recommended for ASIL-D compliance
