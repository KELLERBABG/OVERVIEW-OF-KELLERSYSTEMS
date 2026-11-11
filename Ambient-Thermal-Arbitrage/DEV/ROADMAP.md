# Ambient-Thermal-Arbitrage — Development Roadmap & Action Tasks

> This file is the authoritative internal development roadmap and task sequencing plan.
> **Language:** `Rust`
> **Summary:** A 2-crate Rust workspace (`crates/gateway` and `crates/fleet`) implementing closed-loop Modbus RTU/TCP industrial refrigeration control, thermodynamic RC building envelopes, EPEX Spot / FCR droop frequency regulation, and multi-site fleet coordination. The edge gateway is mature, while fleet-level N-1 compressor dispatch, defrost collision avoidance, and cryptographic savings ledger proofs need hardening.

## Task Queue

- [ ] **TASK-01: Enforce strict HACCP core-temperature and compressor short-cycle safety interlocks**
  - **Target Files:** `crates/gateway/src/control/mod.rs`, `crates/gateway/src/watchdog.rs`
  - **Context Files:** `crates/gateway/src/thermal/mod.rs`, `crates/gateway/src/config.rs`
  - **Verify:** `cargo test -p ata-gateway --quiet`
  - **Spec:** Harden the safety interlock layer in `crates/gateway/src/control/mod.rs` and `watchdog.rs`.

1. Enforce an unbypassable hardware-envelope check before any Modbus coil write: if `product_core_temp_c >= config.max_safe_temp_c - 0.5`, force Cooling ON regardless of electricity price or FCR shedding signal.
2. Enforce minimum compressor ON time (`min_on_secs >= 180`) and minimum OFF time (`min_off_secs >= 180`) to prevent liquid slugging and motor starter burnout.
3. Add unit tests verifying that extreme price spikes (e.g. 5000 EUR/MWh) cannot keep compressors off once `product_core_temp_c` reaches the safety threshold.

- [ ] **TASK-02: Implement online thermal capacity fingerprinting and door-open anomaly detection**
  - **Target Files:** `crates/gateway/src/thermal/mod.rs`, `crates/gateway/src/sensor/mod.rs`
  - **Context Files:** `crates/gateway/src/alerts.rs`
  - **Verify:** `cargo test -p ata-gateway --quiet`
  - **Spec:** Extend `crates/gateway/src/thermal/mod.rs` to estimate the effective thermal time constant $\tau = R_{\text{th}} C_{\text{th}}$ from passive warm-up slopes during curtailed load-shedding windows.

1. Implement `ThermalFingerprint::update_from_coast(&mut self, t_start: f64, t_end: f64, t_ambient: f64, dt_secs: f64)` computing $\hat{\tau} = -\frac{\Delta t}{\ln((T_{\text{end}} - T_{\text{amb}})/(T_{\text{start}} - T_{\text{amb}}))}$.
2. Flag a `DoorOpenOrInsulationBreach` alert if instantaneous $\frac{dT}{dt}$ exceeds $2.5\times$ the historical baseline warm-up rate, immediately aborting load-shedding.
3. Add unit tests for nominal coasting vs. door-open thermal runaway.

- [ ] **TASK-03: Implement counterfactual baseline simulator and auditable arbitrage savings ledger**
  - **Target Files:** `crates/gateway/src/financial/mod.rs`
  - **Context Files:** `crates/gateway/src/thermal/mod.rs`, `crates/gateway/src/logging/mod.rs`
  - **Verify:** `cargo test -p ata-gateway --quiet`
  - **Spec:** Enhance `crates/gateway/src/financial/mod.rs` to run a parallel shadow thermostat (conventional bang-bang deadband around $T_{\text{set}}$) alongside the ATA pre-cooling controller.

1. Compute hourly counterfactual energy consumption $E_{\text{baseline}}(t)$ and cost $C_{\text{baseline}}(t) = E_{\text{baseline}}(t) \cdot P_{\text{spot}}(t)$ using the same ambient temperature and thermal RC model.
2. Produce a deterministic SHA-256 hash-chained `DailySettlementReceipt` recording actual kWh, shadow baseline kWh, net EUR saved, and minimum/maximum core product temperatures.
3. Add unit tests proving positive arbitrage savings over a synthetic 24-hour solar duck-curve price profile.

- [ ] **TASK-04: Implement priority-ranked multi-site VPP dispatch in the fleet engine**
  - **Target Files:** `crates/fleet/src/control.rs`, `crates/fleet/src/physics.rs`
  - **Context Files:** `crates/fleet/src/store_types.rs`, `crates/fleet/src/simulation.rs`
  - **Verify:** `cargo test -p ata-fleet --quiet`
  - **Spec:** Enhance `crates/fleet/src/control.rs` to allocate a grid-level power curtailment or pre-cooling target $P_{\text{target\_kw}}$ across $N$ heterogeneous cold-storage facilities.

1. Rank stores by their remaining safe thermal coasting time $t_{\text{slack}, i} = \tau_i \ln\left(\frac{T_{\text{amb}, i} - T_{\text{core}, i}}{T_{\text{amb}, i} - T_{\text{max}, i}}\right)$.
2. Shed load first from stores with the largest $t_{\text{slack}, i}$, and pre-cool first in stores with highest COP and thermal mass.
3. Add unit tests in `crates/fleet/src/control.rs` verifying that fleet dispatch tracks a 2.5 MW FCR step response with zero individual store safety violations.

- [ ] **TASK-05: Add anti-synchronization defrost stagger scheduler across compressor banks**
  - **Target Files:** `crates/gateway/src/control/mod.rs`
  - **Context Files:** `crates/gateway/src/config.rs`
  - **Verify:** `cargo test -p ata-gateway --quiet`
  - **Spec:** Implement a deterministic defrost & compressor stagger coordinator in `crates/gateway/src/control/mod.rs` so that after a fleet-wide load-shedding window ends, compressors do not all restart on the same second (preventing transformer inrush spikes) and electric defrost heaters are barred during peak tariff quartiles.

1. Add unit tests verifying staggered recovery ramps and peak-price defrost deferral.
