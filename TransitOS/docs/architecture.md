# Transit OS Architecture

 **Version 1.0.0** — **New in 1.0.0**: Split-µ MRM braking, CoG noise filter, RSS fault model, MRM fallback | [Whitepaper](whitepaper.md) | [API Reference](api_reference.md) | [Integration Guide](integration.md)

## System Overview

Transit OS is a formally verified safety gate for autonomous vehicle control
systems supporting **passenger cars, heavy trucks, and articulated semi-trucks**.
It intercepts actuator commands and proves they are safe before they reach the
vehicle hardware, accounting for air brake latency, actuator lag, and trailer
articulation dynamics.

```
Sensor Data (Radar, LIDAR, Camera, GPS, IMU)
    |
    v
+------------------+
| Taint Analysis   |  -- detect spoofed/manipulated sensor data
+------------------+
    |
    v
+------------------+
| State Machine    |  -- bicycle/truck model + air brake lag + articulation
+------------------+
    |
    v
+------------------+
| Path Constraints |  -- generate boolean constraints for maneuver
+------------------+
    |
    v
+------------------+  -- swept-circle (cars) or SAT rectangle (trucks)
| SMT Solver       |  -- SAT/UNSAT collision check
+------------------+
    |
    v
+------------------+
| Invariant Check  |  -- rollover, jackknife, brake lag, force invariants
+------------------+
    |
    v
+------------------+
| Deployment Gate  |  -- ALLOW or BLOCK with configurable policy
+------------------+
    |
    v
 Actuator (steer, throttle, brake — with modelled lag)
```

## Vehicle Class Support

| Class | Mass | CoG Height | Wheelbase | Brake Type | Special |
|-------|------|-----------|-----------|------------|---------|
| PassengerCar | 1500 kg | 0.5 m | 2.8 m | Hydraulic | Default |
| HeavyTruck | 15,000 kg | 1.5 m | 6.0 m | Pneumatic | Air brake lag (350 ms build-up) |
| ArticulatedTruck | 25,000 kg | 1.5 m | 6.0 m + 9.0 m trailer | Pneumatic | Trailer articulation, jackknife risk |

## Crate Dependency Graph

```
state-machine  (foundation: vehicle state, bicycle + articulated truck model)
    |
    +-- abstract-interpretation  (symbolic expressions, interval + affine domains)
    |       |
    |       +-- dynamics-simulator  (time-stepped sim, truck dynamics, air brake)
    |               |
    |               +-- gas-brake-analysis  (energy verification)
    |               |
    |               +-- carla-integration   (CARLA bridge, scenario runner)
    |
    +-- path-constraints  (constraint system, SMT-LIB2 gen)
    |       |
    |       +-- smt-solver  (collision: swept-circle + SAT rectangle)
    |
    +-- verification-invariants  (safety invariants, proof, monitor)
    |
    +-- taint-analysis  (sensor trust, data flow tracking)
    |
    +-- state-space  (exploration, edge case detection)
    |
    +-- spec-generator  (specification mining from trajectories)
            |
            +-- deployment-gate  (interceptor, pipeline, policy, audit)
                    |
                    +-- ros2-integration  (ROS2 node, message bridge)
                    |
                    +-- monitoring  (dashboard, metrics, history)
                    |
                    +-- real-time  (WCET, latency budget, scheduling)
```

## Formal Verification Pipeline

Each actuator command passes through four verification stages:

1. **Taint Check** — Is the command derived from trusted sensor data? Sensors
   have trust levels (IMU: 0.95, LIDAR: 0.90, Camera: 0.75). Commands from
   low-trust sources are blocked under Strict/Normal policies.

2. **Invariant Check** — Seven safety invariants are proven against the current
   vehicle state, including rollover stability (geometry-dependent on CoG height)
   and jackknife detection for articulated vehicles.

3. **Collision Check** — For passenger cars, swept-circle intersection is used.
   For trucks and articulated vehicles, oriented rectangle collision (Separating
   Axis Theorem) provides accurate geometry for long/wide vehicles.

4. **Gate Decision** — ALLOW or BLOCK based on policy (Strict, Normal,
   Permissive). All decisions are audited with sub-millisecond latency targets.

4. **Gate Decision** — ALLOW or BLOCK with fallback. Every `Block` verdict
   carries a **Minimum Risk Maneuver (MRM)** command (straight-line brake,
   gradual decel, or pull-over) so actuators have a deterministic safe
   trajectory even when the control command is rejected.

5. **RSS Fault Model** — Before collision blocking, the gate determines
   fault via RSS (Responsibility-Sensitive Safety) rules. If an external
   vehicle cut in within minimum stopping distance, the fault is assigned
   to `Other` and the gate triggers an **MRM** to minimize impact energy.

## Key Design Principles

- **Multi-class vehicle support**: Passenger car, heavy truck, and articulated
  truck with class-specific physics, brake dynamics, and collision geometry.
- **Sound over-approximation**: Abstract interpretation over interval and affine
  domains guarantees no unsafe state is missed. Affine arithmetic eliminates
  the dependency problem (`x - x ≠ 0`) for multi-step integration.
- **Actuator-aware verification**: Air brake latency (pneumatic build-up time)
  and CAN-bus delays are explicitly modelled, so the gate accounts for the gap
  between "ALLOW" and actual brake application.
- **RSS-based fault assignment**: Collision threats are classified by fault
  (Ego, Other, Unavoidable). Only ego-fault conditions block the command;
  external-actor threats trigger MRM fallback.
- **Dynamic cargo adaptation**: Truck mass and CoG height can be updated at
  runtime from payload estimates, causing rollover thresholds to adjust
  automatically.
- **Pluggable backends**: SMT solver, collision detector, and gate policy are
  replaceable components.
