# Glossary

**Version 1.0.0** | **Terms and definitions for Transit OS**

## A

**Abstract Interpretation**
A static analysis technique that soundly over-approximates program behavior
using abstract domains (intervals, affine forms). Used in Transit OS to
verify vehicle state constraints.

**Actuator**
A device that converts control signals into physical motion (steering,
throttle, brake). In Transit OS, actuators are modeled with first-order
lag and rate limits.

**Affine Arithmetic**
An abstract domain that tracks linear dependencies between variables via
noise symbols. Eliminates the dependency explosion problem of pure intervals
(`x - x = 0`).

**ASIL (Automotive Safety Integrity Level)**
ISO 26262 risk classification from A (lowest) to D (highest). Transit OS
targets ASIL-D for the safety gate.

**Audit Log**
A chronological record of all gate decisions (allow/block) with latency
measurements and check identifiers.

## B

**BIST (Built-In Self-Test)**
Hardware diagnostic test run at startup to verify sensor/actuator health.

**Brake Lag**
The delay between brake command and actual brake force application,
especially significant in air brake systems (up to 350ms).

## C

**CAN (Controller Area Network)**
Serial bus standard for vehicle electronics. Transit OS supports CAN 2.0B
and CAN FD with CRC validation.

**Collision Model**
The geometric representation used for collision detection:
- **Circle:** Swept-circle for passenger cars
- **Rectangle:** SAT oriented-rectangle for trucks

**CoG (Center of Gravity)**
Height of the vehicle's center of mass, critical for rollover calculation.

## D

**Deployment Gate**
The final decision point that allows or blocks actuator commands based on
all verification checks.

**DDS (Data Distribution Service)**
ROS2 middleware for real-time publish/subscribe communication.

## E

**Edge Case Detection**
Identification of dangerous vehicle states: oversteer, understeer,
aquaplaning, black ice, tire blowout.

**EDF (Earliest Deadline First)**
Scheduling algorithm that prioritizes tasks by deadline urgency.

## F

**Fail-Closed**
Safety principle: any unhandled error results in `GateVerdict::Block`.
Transit OS is fail-closed by design.

**First-Order Lag**
Actuator response model: `output(t) = output(t-1) + (target - output(t-1)) * (1 - e^(-dt/τ))`.

## G

**Gate Decision**
The output of the deployment gate: `Allow { check_id }` or
`Block { check_id, reason, mrm_command }`.

**Geometric Solver**
Real-time collision detection using swept-circle or SAT rectangle
intersection. Faster than Z3 but less formal.

## H

**HAL (Hardware Abstraction Layer)**
Transit OS module providing sensor/actuator driver templates.

**HIL (Hardware-in-the-Loop)**
Testing methodology where real hardware is connected to simulated environment.

## I

**Interval Arithmetic**
Abstract domain representing values as `[lo, hi]` ranges. Sound but
suffers from dependency explosion.

**ISO-TP (ISO 15765-2)**
Transport protocol for sending multi-frame messages over CAN (up to 4095 bytes).

**Interceptor**
The component that intercepts actuator commands and runs verification checks.

**Invariant**
A property that must always hold for safe operation (e.g., braking distance,
rollover threshold).

## J

**Jackknife**
Dangerous articulation of a semi-truck where trailer angle exceeds safe
limits (>0.4 rad), potentially causing loss of control.

## L

**Latency Budget**
Maximum allowed time for the complete sensor→verify→actuate chain.

**LTL (Linear Temporal Logic)**
Formal logic for specifying temporal properties of systems.

## M

**MRM (Minimum Risk Maneuver)**
Fallback action when verification fails: controlled deceleration to safe stop.

**Multi-Frame**
ISO-TP term for messages spanning multiple CAN frames.

## P

**PREEMPT_RT**
Linux kernel patch for real-time determinism. Required for safety-critical
operation.

**Policy**
Gate configuration: `Strict` (block on any doubt), `Normal` (balanced),
`Permissive` (allow most).

## R

**Rate Limiting**
Actuator constraint: maximum change per time step (e.g., steering rate ≤0.8 rad/s).

**RM (Rate Monotonic)**
Scheduling algorithm that prioritizes tasks by period (shorter = higher priority).

**RSS (Responsibility-Sensitive Safety)**
Formal model for safe autonomous driving distance calculations.

**Rollover Threshold**
Maximum lateral acceleration before vehicle tips: `(track_width / (2 * CoG_height)) * g`.

## S

**SAT (Satisfiability)**
Z3 solver result indicating a collision is possible.

**Safety Invariant**
A condition checked every cycle (braking distance, rollover, jackknife, etc.).

**Sensor Fusion**
Combining data from multiple sensors to create unified vehicle state.

**SMT (Satisfiability Modulo Theories)**
Extended SAT with theory solvers (reals, integers, arrays). Used by Z3.

**Span**
Tracing concept: a named time interval with correlation IDs for log analysis.

**Stale Data**
Sensor data older than the watchdog timeout, potentially unsafe to use.

## T

**Taint Analysis**
Tracking data origin and trust level from sensor to actuator command.

**TCC (Time-Controllable Collision)**
Analytic collision model checking if collision occurs within time horizon.

**Trust Level**
Numeric confidence in sensor data (0.0-1.0). Below threshold → block.

## U

**UNSAT (Unsatisfiable)**
Z3 solver result indicating no collision is possible (safe).

## V

**Vehicle State**
Complete representation: pose, dynamics, articulation, actuators, tires, road.

**Verification Pipeline**
Sequential checks: taint → invariants → edge cases → collision → gate.

## W

**Watchdog**
Timer that detects stale CAN messages or missed deadlines.

**WCET (Worst-Case Execution Time)**
Maximum time a task takes, critical for real-time guarantees.
