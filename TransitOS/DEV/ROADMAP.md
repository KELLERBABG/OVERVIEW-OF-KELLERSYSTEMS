# TransitOS — Development Roadmap & Action Tasks

> This file is the authoritative internal development roadmap and task sequencing plan.
> **Language:** `Rust`
> **Summary:** A 16-crate Rust safety gateway workspace for autonomous vehicle control (`crates/abstract-interpretation`, `crates/can-bus`, `crates/deployment-gate`, `crates/dynamics-simulator`, `crates/gas-brake-analysis`, `crates/carla-integration`) placing a verified invariant gate between AI path planners and physical actuators.

## Task Queue

- [ ] **TASK-01: Harden dynamic TTC and emergency stopping distance invariants in deployment-gate**
  - **Target Files:** `crates/deployment-gate/src/gate.rs`, `crates/deployment-gate/src/policy.rs`
  - **Context Files:** `crates/abstract-interpretation/src/physics.rs`, `crates/deployment-gate/src/verdict.rs`
  - **Verify:** `cargo test -p transit-deployment-gate --quiet`
  - **Spec:** Update `crates/deployment-gate/src/gate.rs` so that every incoming actuation command $(a_{\text{cmd}}, \delta_{\text{cmd}})$ is validated against the dynamic physical state of the vehicle.

1. Compute stopping distance $d_{\text{stop}} = v \cdot t_{\text{brake\_delay}} + \frac{v^2}{2 a_{\max\_brake}(\mu)}$ under current road friction coefficient $\mu$.
2. If $d_{\text{stop}} \ge d_{\text{obstacle}}$ or Time-To-Collision $TTC = \frac{d_{\text{rel}}}{v_{\text{rel}}} < 1.8\text{ s}$, veto the planner command with `GateVerdict::VetoedCollisionRisk` and substitute the autonomous Minimum Risk Maneuver (MRM) braking profile.
3. Add unit tests in `deployment-gate` verifying veto triggers under sudden lead-vehicle deceleration.

- [ ] **TASK-02: Implement fail-closed Minimum Risk Maneuver (MRM) and CAN watchdog interlock**
  - **Target Files:** `crates/deployment-gate/src/mrm.rs`, `crates/can-bus/src/watchdog.rs`
  - **Context Files:** `crates/can-bus/src/shutdown.rs`, `crates/can-bus/src/frame.rs`
  - **Verify:** `cargo test -p transit-deployment-gate -p transit-can-bus --quiet`
  - **Spec:** Enhance `crates/deployment-gate/src/mrm.rs` and `crates/can-bus/src/watchdog.rs`.

1. If no valid planner command arrives within $50\text{ ms}$ (watchdog timeout) or if 3 consecutive commands are vetoed, engage `MRMState::Active` emitting a jerk-limited deceleration profile ($-3.5\text{ m/s}^2$) straight to a complete stop.
2. In `can-bus/watchdog.rs`, assert that MRM frames take absolute priority on the CAN bus.
3. Add unit tests for watchdog timeout transitions and smooth MRM stopping curves.
