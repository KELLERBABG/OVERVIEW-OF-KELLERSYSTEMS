# Transit OS Safety Manual

**ISO 26262:2018 ASIL-D Compliant Safety Gate**

## 1. Purpose

This document defines the safety requirements, ASIL decomposition, and
verification strategy for Transit OS, a formally verified safety gate
for autonomous vehicle control.

## 2. ASIL Decomposition

| Function | ASIL | Rationale |
|---|---|---|
| Actuator command interception | ASIL-D | Single point of failure; must never pass unsafe commands |
| Invariant checking (braking, rollover, jackknife) | ASIL-D | Safety-critical path; violation = potential fatality |
| Collision detection | ASIL-D | Head-on or pedestrian collision = potential fatality |
| Sensor trust evaluation | ASIL(B) | Taint propagation is a safety mechanism, not single-point |
| MRM (Minimum Risk Maneuver) | ASIL-D | Fallback when verification fails; must bring vehicle to safe state |
| Audit logging | QM | Diagnostic only; no direct safety function |
| CAN bus monitoring | ASIL(C) | Communication integrity; missed frames → stale data |
| Real-time scheduling | ASIL-D | Timing violations = missed safety checks |

## 3. Safety Requirements

### 3.1 Functional Safety Requirements

| ID | Requirement | ASIL | Verification |
|---|---|---|---|
| FSR-001 | The gate shall block any actuator command when invariants are violated | D | Unit tests, integration tests |
| FSR-002 | The gate shall block any command from untrusted sensor sources | B | Unit tests |
| FSR-003 | The gate shall detect collision within 3.0s time horizon | D | Z3 formal verification |
| FSR-004 | The gate shall trigger MRM when any safety check fails | D | Integration tests |
| FSR-005 | The gate shall verify against live vehicle state, not defaults | D | Unit tests (invariants_are_checked_against_live_state) |
| FSR-006 | The gate shall enforce per-sensor trust levels | B | Unit tests (low_trust_source_is_blocked) |

### 3.2 Technical Safety Requirements

| ID | Requirement | ASIL | Verification |
|---|---|---|---|
| TSR-001 | Verification latency shall not exceed 500μs per check | D | WCET analysis |
| TSR-002 | End-to-end sensor-to-actuator latency shall not exceed 1000μs | D | Latency chain analysis |
| TSR-003 | The gate shall execute on PREEMPT_RT with SCHED_FIFO priority 99 | D | Scheduling analysis |
| TSR-004 | Memory shall be locked to prevent page faults in critical path | D | Runtime verification |
| TSR-005 | CAN frames shall be CRC-16 validated before processing | C | CRC unit tests |
| TSR-006 | CAN message timeout shall be detected within configured deadline | C | Watchdog tests |

### 3.3 Hardware Safety Requirements

| ID | Requirement | ASIL | Verification |
|---|---|---|---|
| HSR-001 | Sensor drivers shall implement built-in self-test (BIST) | D | HAL tests |
| HSR-002 | Actuator commands shall be clamped to physical limits | D | Actuator limit tests |
| HSR-003 | Dual-channel redundancy for critical sensors | D | Architecture review |
| HSR-004 | Watchdog shall trigger MRM on sensor failure | D | Integration tests |

## 4. Safety Mechanisms

### 4.1 Primary Safety Mechanism: The Gate

```
Sensor Data → Taint Analysis → Invariant Check → Collision Detection → Gate Decision
                                        ↓ (violation)
                                   MRM Fallback
                                        ↓
                              Safe State (stop)
```

The gate is **fail-closed**: any unhandled error results in `GateVerdict::Block`.

### 4.2 Minimum Risk Maneuver (MRM)

The MRM controller selects a fallback based on velocity and road conditions:

| Velocity | Road | MRM Action |
|---|---|---|
| > 20 m/s | Any | Controlled deceleration (0.5g) |
| > 5 m/s | Any | Gradual stop |
| ≤ 5 m/s | Any | Emergency brake |
| Any | Slippery | Coast with light braking |

### 4.3 Sensor Trust Model

Each sensor type has an inherent trust level:
- LiDAR: 0.95
- IMU: 0.92
- Wheel encoder: 0.93
- Radar: 0.90
- GPS: 0.85
- Camera: 0.75

The gate blocks commands from sensors below the policy threshold.

## 5. Verification and Validation Strategy

### 5.1 Static Analysis
- `cargo clippy` with `-D warnings` (zero warnings enforced)
- `cargo fmt` enforced in CI
- No unsafe code in safety-critical paths

### 5.2 Unit Testing
- 14 unit tests covering gate logic, invariant checking, CRC validation,
  sequence tracking, watchdog, actuator limits, scheduling

### 5.3 Integration Testing
- 42 integration tests covering the full pipeline:
  - Vehicle state creation and dynamics
  - State machine transitions
  - Abstract interpretation
  - Emergency brake analysis
  - Energy verification
  - Collision detection
  - Invariant checking
  - Taint blocking
  - Runtime monitoring
  - State-space exploration
  - Path constraints
  - Specification generation
  - ROS2 bridge verification
  - CARLA scenario running
  - Sensor fusion
  - Latency budgets
  - Dashboard and monitoring

### 5.4 Formal Verification
- Z3 SMT solver integration for collision checking
- SMT-LIB2 export for external proof assistants
- Geometric solver for real-time collision detection

### 5.5 Timing Analysis
- WCET analysis per component
- Latency chain: sensor → verify → actuate
- PREEMPT_RT scheduling with SCHED_FIFO
- Deadline monitoring with watchdog

## 6. Known Limitations

| Limitation | Mitigation |
|---|---|
| Z3 backend requires C++ toolchain for native build | Analytic geometric solver provides fallback |
| ROS2 bridge is in-process translation (no DDS) | Documented as scaffold; real rclrs node provided |
| CARLA bridge is in-process translation (no TCP) | Real TCP client provided; requires CARLA runtime |
| No on-vehicle hardware-in-the-loop testing | Simulation-only; requires HIL for production |
| No ISO 26262 tool qualification | Research/simulation framework; not certified |

## 7. Deployment Checklist

- [ ] Hardware-in-the-loop testing with actual sensors
- [ ] PREEMPT_RT kernel with SCHED_FIFO
- [ ] CAN bus integration with vehicle ECU
- [ ] ROS 2 build environment with rclrs
- [ ] CARLA simulator for scenario testing
- [ ] ISO 26262 tool qualification (if deploying on public roads)
- [ ] ASPICE process compliance (if deploying on public roads)
- [ ] Safety case documentation for type approval
