# Changelog

All notable changes to Transit OS are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project
follows [Semantic Versioning](https://semver.org/).

## [1.1.0] — 2026-08-24

Production-ready release with full hardware integration support.

### Added

- **Z3 SMT solver integration** (`z3` crate) for formal collision verification.
  The solver backend is now pluggable — geometric (real-time) or Z3 (formal).
- **rclrs-based ROS 2 safety node** with real DDS subscriber/publisher for
  actuator commands, ego state, and obstacle data.
- **CARLA TCP client** with real network connection for simulator
  communication (spawn, destroy, transform, control, tick).
- **CAN bus interface** (`transit-can-bus`) with CRC-16/CCITT validation,
  sequence tracking, ISO-TP multi-frame reassembly, and timeout watchdog.
- **Hardware Abstraction Layer** (`transit-hal`) with sensor drivers (LiDAR,
  radar, camera, IMU, GPS, wheel encoders) and actuator drivers (steering,
  throttle, brake) with safety limits and rate limiting.
- **Deterministic real-time scheduling** with PREEMPT_RT support, priority-based
  task scheduling, deadline monitoring, and watchdog-triggered MRM.
- **ISO 26262 safety documentation**: safety manual (`docs/safety_manual.md`)
  and ASIL decomposition (`docs/asil_decomposition.md`).
- `transit-can-bus` crate: CAN frame validation, CRC, watchdog, ISO-TP.
- `transit-hal` crate: HAL with sensor + actuator drivers.
- In-memory CAN bus and Ethernet bus adapters for testing.
- Sensor self-test (BIST) and health monitoring for all sensor types.
- Actuator rate limiting and safety clamp for steering, throttle, brake.
- Latency chain analysis for end-to-end sensor-to-actuate timing.

### Changed

- `transit-ros2-integration` now includes `rclrs_node` module with full ROS 2
  safety node implementation.
- `transit-carla-integration` now includes `carla_client` module with real TCP
  client for CARLA simulator.
- `transit-smt-solver` now supports Z3 backend via `SmtSolver::z3()`.
- `transit-real-time` now includes `deterministic` module with scheduling and
  PREEMPT_RT configuration.
- README updated with all new crates and production-ready status.

## [1.0.0] — 2026-08-17

Initial public release.

### Fixed

- **Deployment gate now verifies the live vehicle state.** The interceptor and
  verification pipeline previously checked every safety invariant against a
  zeroed `VehicleState::default()`, making the invariant stage vacuous. The
  gate now receives the actual state and blocks commands that violate an
  invariant, with a minimum-risk-maneuver fallback.
- **Previously dead modules are now wired into the runtime path:** truck
  SAT-rectangle collision, the jackknife invariant, per-sensor trust-level
  gating, state-only edge-case detection, and the spec generator — all were
  implemented but never called, and are now exercised and tested.
- **Trust gating is now real.** `TaintedValue::is_trusted()` previously
  short-circuited on `Clean` and ignored source trust levels; it now requires
  every source to meet the high-trust bar.
- **ROS 2 bridge no longer discards its inputs.** Ego velocity, position,
  heading, yaw rate, vehicle parameters, sensor sources, and the requested
  steering/throttle/brake are wired into the pipeline, and verification
  latency is measured instead of hardcoded to zero.
- **CI now passes.** All clippy lints are resolved (`-D warnings`), the source
  is rustfmt-clean, and `rustfmt.toml` no longer enables a nightly-only option.
- Removed three unused parameters from `RssViolatorDetector::check_cut_in`.

### Added

- Unit tests in `transit-deployment-gate` (invariant gate honors live state).
- Regression test: a speeding ego state is blocked end-to-end through the ROS 2
  bridge.
- Apache-2.0 `LICENSE`, this changelog, and a `README.md`.
- `license = "Apache-2.0"` metadata on all 16 crates.

### Changed

- Documentation reconciled with reality: 16 crates (was "15"), 42 tests
  (was "39"), and consistent `1.0.0` version references.
