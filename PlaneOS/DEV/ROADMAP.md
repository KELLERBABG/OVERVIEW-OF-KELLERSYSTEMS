# PlaneOS — Development Roadmap & Action Tasks

> This file is the authoritative internal development roadmap and task sequencing plan.
> **Language:** `Rust`
> **Summary:** A high-assurance Rust flight control and avionics supervisory research kernel (`src/controller.rs`, `src/dynamics.rs`, `src/envelope.rs`, `src/gate.rs`, `src/invariants.rs`, `src/sensors.rs`, `src/simulator.rs`, `src/solver.rs`, `src/spec.rs`, `src/state.rs`, `src/thrust.rs`, `tests/verification_chain.rs`) enforcing flight envelope protection and fail-closed actuation vetoes.

## Task Queue

- [ ] **TASK-01: Harden angle-of-attack stall and structural load factor nz safety gate in gate.rs**
  - **Target Files:** `src/gate.rs`, `src/envelope.rs`
  - **Context Files:** `src/dynamics.rs`, `src/state.rs`, `tests/verification_chain.rs`
  - **Verify:** `cargo test --quiet`
  - **Spec:** Strengthen `src/gate.rs` and `src/envelope.rs` so that commanded elevator and aileron deflections are evaluated against the current state vector $(\alpha, q, V_{\text{tas}}, n_z)$.

1. If applying the commanded deflection over the actuation horizon $\Delta t$ predicts an angle-of-attack $\alpha > \alpha_{\text{stall}} - 1.5^\circ$ or normal load $n_z > +2.5g$ (or $n_z < -1.0g$), clamp the actuator command to the maximum safe boundary envelope.
2. Add integration tests in `tests/verification_chain.rs` demonstrating that abrupt full-deflection stick pulls are cleanly clamped without structural or aerodynamic stall violation.

- [ ] **TASK-02: Implement Triple Modular Redundancy pitot-static cross-channel voting in sensors.rs**
  - **Target Files:** `src/sensors.rs`
  - **Context Files:** `src/state.rs`
  - **Verify:** `cargo test --quiet`
  - **Spec:** Enhance `src/sensors.rs` to take three independent pitot-static and alpha-vane sensor streams.

1. Implement median voting with dynamic variance thresholding. If one channel drifts $> 15\text{ knots}$ from the other two for $> 200\text{ ms}$ (simulating pitot icing), isolate the faulty channel, raise a sensor alert, and continue flight envelope enforcement using the surviving dual-redundant pair.
2. Add unit tests verifying sensor failover during simulated pitot icing.

- [ ] **TASK-03: Enforce deterministic execution cycle latency and monotonic spec logging in spec.rs**
  - **Target Files:** `src/spec.rs`, `src/controller.rs`
  - **Context Files:** `src/gate.rs`
  - **Verify:** `cargo test --quiet`
  - **Spec:** Ensure `src/spec.rs` generates a deterministic flight safety ledger recording timestamp, sensor voting status, raw pilot command, envelope bounds, clamped actuator output, and formal SMT solver status for every $50\text{ Hz}$ control cycle.

1. Add unit tests for ledger serialization and zero-allocation hot-path execution.
