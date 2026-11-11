# AeroResonance — Development Roadmap & Action Tasks

> This file is the authoritative internal development roadmap and task sequencing plan.
> **Language:** `Rust`
> **Summary:** A dual-workspace Rust repository (`simulation/` and `firmware/`) modeling atmospheric electromagnetic/electrostatic energy harvesting and hybrid solar-cap observatory nodes on ESP32-C3. The simulation honestly demonstrates that pure atmospheric resonance misses Gate-1 (>1 mW/m²), requiring hybrid solar+atmospheric MPPT scheduling and supercapacitor leakage/aging management.

## Task Queue

- [ ] **TASK-01: Implement adaptive complex impedance matching and Q-factor tracking in simulation**
  - **Target Files:** `simulation/src/physics.rs`, `simulation/src/subsystems.rs`
  - **Context Files:** `simulation/src/config.rs`, `simulation/simulation_core/src/physics.rs`
  - **Verify:** `cargo test --manifest-path simulation/Cargo.toml --quiet`
  - **Spec:** Enhance `simulation/src/physics.rs` and `subsystems.rs` to model dynamic RLC tank tuning under ambient humidity and temperature drift.

1. Compute antenna radiation and reactive impedance $Z_{\text{ant}}(\omega, RH, T) = R_{\text{rad}} + R_{\text{loss}} + j(\omega L - \frac{1}{\omega C(RH)})$.
2. Implement an automatic varactor/switched-capacitor matching network that minimizes reflection coefficient $|\Gamma| = |\frac{Z_{\text{load}} - Z_{\text{ant}}^*}{Z_{\text{load}} + Z_{\text{ant}}}|$ and reports net harvested power.
3. Add unit tests verifying that automatic retuning recovers $>85\%$ of peak resonant power under $\pm 20\%$ capacitance drift.

- [ ] **TASK-02: Couple supercapacitor ESR aging and leakage into the no_std duty-cycle scheduler**
  - **Target Files:** `simulation/simulation_core/src/cap_aging.rs`, `simulation/simulation_core/src/scheduler.rs`
  - **Context Files:** `simulation/simulation_core/src/lib.rs`
  - **Verify:** `cargo test --manifest-path simulation/Cargo.toml -p simulation_core --quiet`
  - **Spec:** Wire `cap_aging.rs` directly into `scheduler.rs` in `simulation_core`.

1. Compute usable energy $E_{\text{usable}}(t) = \frac{1}{2} C_{\text{eff}}(t) (V_{\text{cap}}^2 - V_{\text{cutoff}}^2) - I_{\text{peak}}^2 R_{\text{ESR}}(t) \Delta t_{\text{tx}}$.
2. Enforce the invariant that a LoRa/BLE transmission burst is only scheduled when $V_{\text{cap}} - I_{\text{peak}} R_{\text{ESR}}(t) > V_{\text{brownout}} + 0.15\text{ V}$.
3. Add unit tests simulating a 5-year aged capacitor at $-15\,^\circ\text{C}$ (high ESR) proving the scheduler throttles burst length rather than triggering a brownout reset.

- [ ] **TASK-03: Implement cold-start hysteresis state machine in ESP32-C3 host-testable firmware core**
  - **Target Files:** `firmware/src/energy.rs`
  - **Context Files:** `firmware/src/lib.rs`, `firmware/src/telemetry.rs`, `firmware/tests/day_cycle.rs`
  - **Verify:** `cargo test --manifest-path firmware/Cargo.toml --quiet`
  - **Spec:** Update `firmware/src/energy.rs` to implement a formal 4-state hysteresis machine (`DeadStorage`, `ColdCharge`, `SurvivalSenseOnly`, `ActiveTelemetry`).

1. Require $V_{\text{cap}} \ge 2.85\text{ V}$ to transition from `ColdCharge` to `SurvivalSenseOnly`, and $V_{\text{cap}} \ge 3.30\text{ V}$ for `ActiveTelemetry`, with downward hysteresis thresholds at $2.50\text{ V}$ and $2.10\text{ V}$.
2. Add unit tests in `firmware/tests/day_cycle.rs` verifying zero oscillation chattering during dawn/dusk low-irradiance ramps.

- [ ] **TASK-04: Add atmospheric sferics impulse entropy extractor and NIST min-entropy estimator**
  - **Target Files:** `firmware/src/entropy.rs`, `simulation/simulation_core/src/entropy.rs`
  - **Context Files:** `simulation/simulation_core/src/crypto.rs`
  - **Verify:** `cargo test --manifest-path firmware/Cargo.toml --quiet`
  - **Spec:** Harden `entropy.rs` in both `firmware` and `simulation_core` to extract cryptographic nonces from inter-arrival times of atmospheric sferics impulses.

1. Implement Von Neumann debiasing followed by a health test (Repetition Count Test & Adaptive Proportion Test per NIST SP 800-90B).
2. Refuse to emit a seed if min-entropy $H_\infty < 0.75$ bits/sample, falling back to aflagged deterministic counter state.
3. Add unit tests for stuck-ADC and periodic EMI injection rejection.

- [ ] **TASK-05: Verify authenticated compact CBOR/binary telemetry framing with replay protection**
  - **Target Files:** `firmware/src/telemetry.rs`, `firmware/tests/decode_tool.rs`
  - **Context Files:** `firmware/src/bme280.rs`, `firmware/examples/decode.rs`
  - **Verify:** `cargo test --manifest-path firmware/Cargo.toml --quiet`
  - **Spec:** Ensure `firmware/src/telemetry.rs` packs BME280 environmental readings, capacitor voltage, harvested power, and monotonic boot/sequence counter into a fixed-size authenticated frame signed/MACed with key material derived from `simulation_core::crypto`.

1. Add tamper and replay rejection tests in `firmware/tests/decode_tool.rs`.
