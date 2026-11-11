# AquaMOF — Development Roadmap & Action Tasks

> This file is the authoritative internal development roadmap and task sequencing plan.
> **Language:** `Rust`
> **Summary:** A Rust simulation and sizing engine (`src/isotherm.rs`, `src/kinetics.rs`, `src/condenser.rs`, `src/economics.rs`, `src/telemetry.rs`) modeling passive desert atmospheric water harvesting using MOF-303 (Al-3,5-pyrazoledicarboxylate). Needs full diurnal 24-hour thermodynamic state-machine coupling, dust-storm cut-off gate logic, and WHO potability mineralization modeling.

## Task Queue

- [ ] **TASK-01: Implement Clausius-Clapeyron temperature-dependent MOF-303 step isotherm**
  - **Target Files:** `src/isotherm.rs`
  - **Context Files:** `src/lib.rs`, `src/kinetics.rs`
  - **Verify:** `cargo test --quiet`
  - **Spec:** Enhance `src/isotherm.rs` to model the cooperative S-shaped water uptake $q(RH, T)$ (in $\text{g H}_2\text{O / g MOF}$) of MOF-303 across both adsorption ($15\text{--}35\,^\circ\text{C}$) and solar desorption ($75\text{--}95\,^\circ\text{C}$).

1. Model the step inflection relative humidity $RH_{\text{step}}(T)$ shifting upward with temperature according to $\Delta H_{\text{ads}} = -52.0\text{ kJ/mol}$.
2. Provide `working_capacity(rh_night: f64, t_night_c: f64, rh_chamber_day: f64, t_desorb_c: f64) -> f64` returning net cyclic water release per kg of MOF-303.
3. Add unit tests verifying that at $20\%\text{ RH}, 25\,^\circ\text{C}$ night and $85\,^\circ\text{C}$ day desorption, net working capacity exceeds $0.32\text{ L/kg/cycle}$.

- [ ] **TASK-02: Implement 24-hour diurnal adsorption-desorption-condensation state machine**
  - **Target Files:** `src/kinetics.rs`, `src/condenser.rs`, `src/lib.rs`
  - **Context Files:** `src/isotherm.rs`
  - **Verify:** `cargo test --quiet`
  - **Spec:** Couple `isotherm.rs`, `kinetics.rs`, and `condenser.rs` into a deterministic 24-hour diurnal simulator in `src/lib.rs`.

1. Define `DiurnalState { NightAdsorption, MorningPreHeat, SolarDesorption, EveningCooldown, DustStormSealed }`.
2. In `condenser.rs`, calculate the exact dew point $T_{\text{dew}}$ of the hot desorbed vapor stream using the Magnus-Tetens equation and compute liquid condensate yield based on ambient heat-sink rejection capacity.
3. Add unit tests verifying mass conservation (adsorbed mass == desorbed vapor + residual loading) across a 24-hour desert weather profile.

- [ ] **TASK-03: Implement dust-storm shutter protection gate and WHO potability audit in telemetry**
  - **Target Files:** `src/telemetry.rs`, `src/economics.rs`
  - **Context Files:** `src/lib.rs`
  - **Verify:** `cargo test --quiet`
  - **Spec:** Expand `src/telemetry.rs` (32 lines) and `src/economics.rs` (55 lines).

1. In `telemetry.rs`, implement `EnvironmentalGate` that triggers `ShutterState::Sealed` when ambient particulate concentration exceeds $500\,\mu\text{g/m}^3$ (haboob/dust storm) or night RH $< 8\%$, protecting MOF nanopores from fouling.
2. Implement `PotabilityAudit` checking condensate TDS ($10\text{--}250\text{ mg/L}$ post-mineralization cartridge) and thermal pasteurization invariance ($T_{\text{desorb}} \ge 75\,^\circ\text{C}$ for $\ge 30\text{ min}$).
3. Add unit tests for dust-storm sealing and pasteurization verification.
