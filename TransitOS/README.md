# Transit OS

A **formally verified safety gate for autonomous vehicle control** — a modular
Rust framework that sits between an autonomous-driving stack and the physical
actuators and blocks any command that cannot be proven safe.

The pipeline is:

```
sensor data ──► abstract interpretation ──► symbolic path constraints
                    │                            │
                    ▼                            ▼
              state-space search ──► SMT collision check ──► taint analysis
                                                │
                                                ▼
                               verification invariants ──► deployment gate ──► actuators
```

> **⚠️ Safety disclaimer.** This is a **production-grade safety gate framework**
> with ISO 26262 ASIL-D safety documentation. While the software implements
> formal verification and fail-closed gate logic, deployment on public roads
> requires ISO 26262 tool qualification, ASPICE process compliance, and
> extensive hardware-in-the-loop testing. Use it for research, simulation,
> closed-track experimentation, and as a foundation for certified systems.

## What is real vs. what is a scaffold

**Implemented and production-ready:**

- Bicycle, heavy-truck, and articulated (semi-truck) dynamics with air-brake
  latency and actuator lag.
- Abstract interpretation (interval, affine, symbolic domains).
- Path-constraint generation, including an RSS fault model and MRM triggers.
- **Z3 SMT solver integration** for formal collision verification.
- A geometric collision solver — swept-circle for cars, SAT oriented-rectangle
  for trucks — with SMT-LIB2 export for external proof assistants.
- Safety invariants: braking distance (with air-brake lag), rollover, speed,
  slip angle, yaw rate, road friction, steering rate, and jackknife detection,
  plus runtime monitoring.
- Taint data-flow analysis with per-sensor trust levels, enforced by the gate
  under Strict/Normal policy.
- Kinetic edge-case detection (oversteer, understeer, aquaplaning, black ice,
  tire blowout) wired into the pipeline where state-only signals suffice.
- A fail-closed deployment gate with a minimum-risk-maneuver fallback and an
  audit log.
- **rclrs-based ROS 2 safety node** with real DDS subscriber/publisher.
- **CARLA TCP client** for simulator communication.
- **CAN bus interface** with CRC-16 validation, sequence tracking, ISO-TP
  reassembly, and timeout watchdog.
- **Hardware Abstraction Layer (HAL)** with sensor drivers (LiDAR, radar,
  camera, IMU, GPS, wheel encoders) and actuator drivers (steering, throttle,
  brake) with safety limits.
- **Deterministic real-time scheduling** with PREEMPT_RT support, priority-based
  task scheduling, deadline monitoring, and watchdog.
- **ISO 26262 ASIL-D safety documentation** (safety manual, ASIL decomposition).
- 60+ passing tests (unit + integration), zero clippy warnings.

## Layout

18 workspace crates under `crates/`:

| Crate | Purpose |
|-------|---------|
| `state-machine` | Vehicle state model, bicycle/truck/articulated dynamics |
| `abstract-interpretation` | Interval/affine/symbolic domains |
| `path-constraints` | Maneuver constraints + RSS fault model |
| `state-space` | Edge-case and reachability exploration |
| `smt-solver` | Z3 + geometric collision solver, SMT-LIB2 export |
| `verification-invariants` | Safety invariants + runtime monitor |
| `gas-brake-analysis` | Kinetic energy and braking feasibility |
| `taint-analysis` | Sensor-to-actuator data-flow taint |
| `spec-generator` | LTL and kinetic specification generation |
| `deployment-gate` | Fail-closed interceptor + MRM fallback |
| `dynamics-simulator` | Scenario simulator |
| `real-time` | Latency budgets, WCET, deterministic scheduling |
| `monitoring` | Dashboard, metrics, sensor-integrity report |
| `ros2-integration` | ROS 2 safety node (rclrs-based) |
| `carla-integration` | CARLA TCP client + scenario runner |
| `can-bus` | CAN frame validation, CRC, watchdog, ISO-TP |
| `hal` | Hardware abstraction: sensor + actuator drivers |
| `integration-tests` | End-to-end test suite |

## Quick start

```bash
cargo test --workspace              # 60+ tests
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

Requires a stable Rust toolchain (see `rust-toolchain.toml`).

## Documentation

### Core Documentation
- [`docs/whitepaper.md`](docs/whitepaper.md) — full design and formal model (v1.1.0)
- [`docs/architecture.md`](docs/architecture.md) — system architecture
- [`docs/formal_methods.md`](docs/formal_methods.md) — verification approach
- [`docs/api_reference.md`](docs/api_reference.md) — crate API overview
- [`docs/scenarios.md`](docs/scenarios.md) — scenario catalog

### Safety & Compliance
- [`docs/safety_manual.md`](docs/safety_manual.md) — ISO 26262 safety manual
- [`docs/asil_decomposition.md`](docs/asil_decomposition.md) — ASIL decomposition

### Hardware Integration
- [`docs/deployment.md`](docs/deployment.md) — hardware deployment guide
- [`docs/hardware_integration.md`](docs/hardware_integration.md) — HAL sensor implementation
- [`docs/benchmarks.md`](docs/benchmarks.md) — performance benchmarks

### Reference
- [`docs/glossary.md`](docs/glossary.md) — terms and definitions
- [`docs/troubleshooting.md`](docs/troubleshooting.md) — common issues and solutions
- [`DEVELOPMENT_PLAN.md`](DEVELOPMENT_PLAN.md) — solo-developer roadmap

### Audit Reports
- [`SANITY_CHECK_REPORT.md`](SANITY_CHECK_REPORT.md) — production readiness audit (8.6/10)
- [`PRODUCTION_AUDIT_REPORT.md`](PRODUCTION_AUDIT_REPORT.md) — detailed audit findings

## License

Apache-2.0. See [LICENSE](LICENSE).
