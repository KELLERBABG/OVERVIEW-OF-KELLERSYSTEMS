# Energy Harvesting via Atmospheric Resonance (EHAR) — Comprehensive Architecture & Planning Documentation

> **Project Directory:** `AeroResonance/`  
> **Primary Language / Toolchain:** `Rust`  
> **Verification Command:** `cargo test --manifest-path simulation/Cargo.toml --workspace --quiet`  
> **Reference Sources:** `KELLERSYSTEMS Spine.md` · `PaySlip.canvas` · `Life Work.canvas`

---

## 1. Executive Spine & System Overview

**Energy Harvesting via Atmospheric Resonance (EHAR)** — A hybrid atmospheric electromagnetic resonance and solar energy harvesting system and environmental observatory node built for low-power, off-grid telemetry.

### Current Engineering Baseline
A dual-workspace Rust repository (`simulation/` and `firmware/`) modeling atmospheric electromagnetic/electrostatic energy harvesting and hybrid solar-cap observatory nodes on ESP32-C3. The simulation honestly demonstrates that pure atmospheric resonance misses Gate-1 (>1 mW/m²), requiring hybrid solar+atmospheric MPPT scheduling and supercapacitor leakage/aging management.

---

## 2. Canonical Specifications & Mathematical Invariants (`PaySlip.canvas` / `Spine`)

The following 12 architectural pillars and formal invariants define the target engineering state of the system:

- **Ionospheric Plasma Coupling:** Direkte mathematische Abstimmung der Empfangsschaltungen auf die Resonanzfrequenzen der Ionosphäre, um die elektromagnetische Potentialdifferenz zwischen Erdoberfläche und Plasma-Schichten anzuzapfen.
    
- **Near-Vertical Skywave Inductor:** Steuerung von Antennen-Arrays im NVIS-Verfahren (Near Vertical Incidence Skywave), um atmosphärische Schwingungsenergie steil nach oben zu projizieren und die reflektierte Resonanzenergie als Induktionsstrom einzufangen.
    
- **Maxwellian Field Relaxation:** Kontinuierliche Berechnung der elektrodynamischen Feldgleichungen in Echtzeit, um die Impedanz des Empfängers exakt an die wandernden Ladungsdichten der Atmosphäre anzupassen.
    
- **Acoustic-Atmospheric Transduction:** Umwandlung von barometrischen Druckwellen und makroskopischen Luftdruckfluktuationen der Atmosphäre in elektrische Mikrostrome über piezoelektrische Gitterstrukturen.
    
- **Non-Linear Wave Modulation:** Experimentelle Codierung und Gleichrichtung von hochfrequenten atmosphärischen Störsignalen, um die unregelmäßige Energie der kosmischen Hintergrundstrahlung in stabilen Gleichstrom zu transformieren.
    
- **Cyclostationary Feature Filtering:** Algorithmische Isolierung von periodischen atmosphärischen Signalmustern (wie solaren Zyklen oder Gewitter-Entropie) aus dem globalen elektromagnetischen Rauschen.
    
- **Plasma-Wake Energy Buffering:** Kurzzeitige Speicherung und Stabilisierung von transienten Überspannungsspitzen, die durch ionisierende Blitzeinschläge oder solare Teilchenströme in der Atmosphäre induziert werden.
    
- **Resonant Substrate Adaptation:** Abstraktionsebene im Kernel, die sicherstellt, dass die mathematische Frequenznachführung zur Resonanzoptimierung vollkommen deterministisch und ohne Jitter auf der Steuerungshardware läuft.
    
- **Thermodynamic Capacity Gradient:** Mathematische Ausnutzung des Temperaturgefälles zwischen bodennahen Luftschichten und der höheren Atmosphäre zur Erzeugung eines kontinuierlichen, passiven Energiestroms.
    
- **Static Invariant Discharge:** Sicherheits-Interrupt-Logik, die bei Überschreitung kritischer physikalischer Grenzwerte oder atmosphärischer Überspannung die überschüssige Energie sofort kontrolliert ableitet, um die lokale Hardware zu schützen.

---

## 4. Codebase Architecture & Module Inventory

Total tracked source and configuration files in `AeroResonance/`: **29**

| File Path | Lines | Architectural Role |
| :--- | :---: | :--- |
| `firmware/Cargo.toml` | 14 | Build / Package Manifest |
| `firmware/esp32c3/Cargo.toml` | 21 | Build / Package Manifest |
| `firmware/esp32c3/src/main.rs` | 232 | Core Implementation Module |
| `firmware/examples/decode.rs` | 87 | Core Implementation Module |
| `firmware/src/bme280.rs` | 289 | Core Implementation Module |
| `firmware/src/energy.rs` | 311 | Core Implementation Module |
| `firmware/src/entropy.rs` | 80 | Core Implementation Module |
| `firmware/src/lib.rs` | 42 | Core Implementation Module |
| `firmware/src/telemetry.rs` | 193 | Core Implementation Module |
| `firmware/tests/day_cycle.rs` | 105 | Verification & Test Suite |
| `firmware/tests/decode_tool.rs` | 66 | Verification & Test Suite |
| `firmware/tests/fuzz.rs` | 94 | Verification & Test Suite |
| `simulation/Cargo.toml` | 32 | Build / Package Manifest |
| `simulation/benches/run_simulation.rs` | 24 | Core Implementation Module |
| `simulation/simulation_core/Cargo.toml` | 14 | Build / Package Manifest |
| `simulation/simulation_core/src/cap_aging.rs` | 74 | Core Implementation Module |
| `simulation/simulation_core/src/crypto.rs` | 124 | Core Implementation Module |
| `simulation/simulation_core/src/entropy.rs` | 251 | Core Implementation Module |
| `simulation/simulation_core/src/lib.rs` | 20 | Core Implementation Module |
| `simulation/simulation_core/src/physics.rs` | 87 | Core Implementation Module |
| `simulation/simulation_core/src/scheduler.rs` | 175 | Core Implementation Module |
| `simulation/src/config.rs` | 226 | Core Implementation Module |
| `simulation/src/lib.rs` | 10 | Core Implementation Module |
| `simulation/src/main.rs` | 350 | Core Implementation Module |
| `simulation/src/physics.rs` | 268 | Core Implementation Module |
| `simulation/src/simulation.rs` | 215 | Core Implementation Module |
| `simulation/src/subsystems.rs` | 654 | Core Implementation Module |
| `simulation/tests/fuzz.rs` | 108 | Verification & Test Suite |
| `simulation/tests/simulation_tests.rs` | 117 | Verification & Test Suite |

---

## 5. Gap Analysis & Production-Readiness Audit

- **Gate-1 Hybrid Energy Reconciliation (`simulation/`):** Pure Schumann/sferics RF harvesting yields nanowatts to microwatts per square meter; the simulation and firmware must formally enforce the hybrid solar + atmospheric energy budget invariant so the node never browns out during 7-day severe-weather darkness.
- **Dynamic Impedance Matching & MPPT (`simulation/src/subsystems.rs` & `firmware/src/energy.rs`):** Needs adaptive complex conjugate impedance matching $Z_{\text{load}} = Z_{\text{ant}}^*$ across variable atmospheric capacitance and humidity.
- **Supercapacitor Aging & Cold-Start Hysteresis (`simulation_core/src/cap_aging.rs`):** Must couple Arrhenius thermal degradation and ESR rise into the duty-cycle scheduler (`scheduler.rs`) to guarantee zero brownout resets over a 10-year maintenance-free deployment.

---

## 7. Actionable Development & Verification Queue

Each task below is synchronized with `DEV/ROADMAP.md` and `DEV/TASKS.json` for automated or manual execution:

| Task ID | Title | Target Files | Verification Command |
| :---: | :--- | :--- | :--- |
| **TASK-01** | Implement adaptive complex impedance matching and Q-factor tracking in simulation | `simulation/src/physics.rs`, `simulation/src/subsystems.rs` | `cargo test --manifest-path simulation/Cargo.toml --quiet` |
| **TASK-02** | Couple supercapacitor ESR aging and leakage into the no_std duty-cycle scheduler | `simulation/simulation_core/src/cap_aging.rs`, `simulation/simulation_core/src/scheduler.rs` | `cargo test --manifest-path simulation/Cargo.toml -p simulation_core --quiet` |
| **TASK-03** | Implement cold-start hysteresis state machine in ESP32-C3 host-testable firmware core | `firmware/src/energy.rs` | `cargo test --manifest-path firmware/Cargo.toml --quiet` |
| **TASK-04** | Add atmospheric sferics impulse entropy extractor and NIST min-entropy estimator | `firmware/src/entropy.rs`, `simulation/simulation_core/src/entropy.rs` | `cargo test --manifest-path firmware/Cargo.toml --quiet` |
| **TASK-05** | Verify authenticated compact CBOR/binary telemetry framing with replay protection | `firmware/src/telemetry.rs`, `firmware/tests/decode_tool.rs` | `cargo test --manifest-path firmware/Cargo.toml --quiet` |
