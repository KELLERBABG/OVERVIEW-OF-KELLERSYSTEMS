# OP-Roboter (Surgical Teleoperation Safety Kernel) — Comprehensive Architecture & Planning Documentation

> **Project Directory:** `OP-Roboter/`  
> **Primary Language / Toolchain:** `Rust`  
> **Verification Command:** `cargo test --workspace --quiet`  
> **Reference Sources:** `KELLERSYSTEMS Spine.md` · `PaySlip.canvas` · `Life Work.canvas`

---

## 1. Executive Spine & System Overview

**OP-Roboter (Surgical Teleoperation Safety Kernel)** — A high-assurance surgical teleoperation safety supervisor and real-time kinematics interlock kernel written in Rust to enforce virtual fixtures and collision avoidance in minimally invasive robotic surgery.

### Current Engineering Baseline
A 7-crate Rust surgical robotics safety workspace (`op-math`, `op-kinematics`, `op-constraints`, `op-energy`, `op-gate`, `op-solver`, `op-telemetry`) implementing Denavit-Hartenberg forward/inverse kinematics, Jacobian velocity mapping, virtual fixture anatomical keep-out zones, and an execution safety gate.

---

## 2. Canonical Specifications & Mathematical Invariants (`PaySlip.canvas` / `Spine`)

The following 12 architectural pillars and formal invariants define the target engineering state of the system:

- **Mathematical Trajectory Mapping:** Jede mechanische Bewegung des OP-Arms oder jede Änderung der Flugklappenstellung wird in ein formales mathematisches Modell übersetzt, das alle physikalisch erlaubten Zustände im dreidimensionalen Raum repräsentiert.
    
- **Kinetic Path Constraint Generation:** Während die Steuerungs-Engine die nächste Aktion (Schnitttiefe oder Schubumkehr) berechnet, sammelt sie die logischen und physikalischen Bedingungen und erstellt einen booleschen Ausdruck, der den gesamten Pfad beschreibt.
    
- **Critical State Space Exploration:** Systematische Untersuchung aller theoretisch möglichen System- und Umweltzustände (wie plötzlicher Herzstillstand des Patienten oder extreme atmosphärische Turbulenzen), um seltene lebensbedrohliche "Edge Cases" mathematisch vorab zu finden.
    
- **Real-Time SMT-Solver Integration (Z3):** Die gesammelten Pfadbedingungen der Aktuatoren werden in Echtzeit an einen High-Performance-Solver übergeben, der prüft, ob die logische Formel für einen fehlerhaften Zustand (z. B. Überreizung des Herzmuskels oder Strömungsabriss) erfüllbar ($SAT$) ist.
    
- **Lethal Invariant Enforcement:** Definition unumstößlicher Sicherheitsregeln (wie die physikalische Unmöglichkeit, dass die chirurgische Nadel eine vordefinierte Gewebegrenze überschreitet oder die Flughöhe den Korridor verlässt), die über die gesamte Laufzeit bewiesen werden.
    
- **Symbolic Pulse & Thrust Analysis:** Symbolische Berechnung der exakten elektrischen Ladungsabgabe des Schrittmachers oder der Triebwerksleistung für jede Millisekunde, um Hardware-Überlastungen oder funktionale Ausfälle vorab auszuschließen.
    
- **Taint-Coded Sensor Stream Audit:** Kontinuierliche Verfolgung des Datenflusses von unzuverlässigen externen Sensoren (wie EKG-Elektroden oder Staurohren) bis hin zu den kritischen lebenserhaltenden Systemänderungen, um Datenmanipulationen oder Sensorfehler sofort abzufangen.
    
- **Automated Medical/Aerospace Specification:** Generierung von formalen Spezifikationen direkt aus dem Systemverhalten zur Laufzeit, die als unbestechliche, maschinenlesbare mathematische Referenz für medizinische Zertifizierungen oder Flugschreiber dient.
    
- **Temporal Isolation Asymmetry:** Scheduling-Algorithmen, die verhindern, dass sekundäre Überwachungsprozesse (wie die Telemetriedatenübertragung) durch die Messung von CPU-Zyklen Informationen ableiten oder die kritische Echtzeit-Impulsabgabe verzögern.
    
- **Trustless Execution Gate:** Ein automatisiertes Kontrollsystem, das den physischen Impuls an den Herzmuskel oder den Steuerbefehl an die Hydraulik blockiert, solange kein gültiger mathematischer Beweis für die absolute Sicherheit der Aktion vorliegt.

---

## 4. Codebase Architecture & Module Inventory

Total tracked source and configuration files in `OP-Roboter/`: **41**

| File Path | Lines | Architectural Role |
| :--- | :---: | :--- |
| `Cargo.toml` | 47 | Build / Package Manifest |
| `crates/op-constraints/Cargo.toml` | 13 | Build / Package Manifest |
| `crates/op-constraints/src/expr.rs` | 163 | Core Implementation Module |
| `crates/op-constraints/src/generate.rs` | 278 | Core Implementation Module |
| `crates/op-constraints/src/lib.rs` | 21 | Core Implementation Module |
| `crates/op-energy/Cargo.toml` | 12 | Build / Package Manifest |
| `crates/op-energy/src/lib.rs` | 197 | Core Implementation Module |
| `crates/op-gate/Cargo.toml` | 17 | Build / Package Manifest |
| `crates/op-gate/src/lib.rs` | 377 | Core Implementation Module |
| `crates/op-gate/tests/stress_tests.rs` | 172 | Verification & Test Suite |
| `crates/op-kinematics/Cargo.toml` | 12 | Build / Package Manifest |
| `crates/op-kinematics/src/dh.rs` | 83 | Core Implementation Module |
| `crates/op-kinematics/src/forward.rs` | 59 | Core Implementation Module |
| `crates/op-kinematics/src/inverse.rs` | 190 | Core Implementation Module |
| `crates/op-kinematics/src/jacobian.rs` | 253 | Core Implementation Module |
| `crates/op-kinematics/src/lib.rs` | 49 | Core Implementation Module |
| `crates/op-math/Cargo.toml` | 10 | Build / Package Manifest |
| `crates/op-math/src/lib.rs` | 52 | Core Implementation Module |
| `crates/op-math/src/matrix.rs` | 134 | Core Implementation Module |
| `crates/op-math/src/quaternion.rs` | 176 | Core Implementation Module |
| `crates/op-math/src/transform.rs` | 125 | Core Implementation Module |
| `crates/op-math/src/vector.rs` | 223 | Core Implementation Module |
| `crates/op-sim/Cargo.toml` | 26 | Build / Package Manifest |
| `crates/op-sim/src/lib.rs` | 590 | Core Implementation Module |
| `crates/op-sim/src/main.rs` | 194 | Core Implementation Module |
| `crates/op-spec/Cargo.toml` | 12 | Build / Package Manifest |
| `crates/op-spec/src/lib.rs` | 250 | Verification & Test Suite |
| `crates/op-state-space/Cargo.toml` | 12 | Build / Package Manifest |
| `crates/op-state-space/src/lib.rs` | 330 | Core Implementation Module |
| `crates/op-taint/Cargo.toml` | 11 | Build / Package Manifest |
| `crates/op-taint/src/lib.rs` | 188 | Core Implementation Module |
| `crates/op-trajectory/Cargo.toml` | 13 | Build / Package Manifest |
| `crates/op-trajectory/src/lib.rs` | 35 | Core Implementation Module |
| `crates/op-trajectory/src/path.rs` | 148 | Core Implementation Module |
| `crates/op-trajectory/src/rcm.rs` | 87 | Core Implementation Module |
| `crates/op-trajectory/src/workspace.rs` | 255 | Core Implementation Module |
| `crates/op-verifier/Cargo.toml` | 21 | Build / Package Manifest |
| `crates/op-verifier/src/invariant.rs` | 80 | Core Implementation Module |
| `crates/op-verifier/src/lib.rs` | 385 | Core Implementation Module |
| `crates/op-verifier/src/mock.rs` | 82 | Core Implementation Module |
| `crates/op-verifier/src/z3.rs` | 107 | Core Implementation Module |

---

## 5. Gap Analysis & Production-Readiness Audit

- **Non-Trivial SMT/Geometric Safety Gate (`crates/op-gate/src/lib.rs` & `crates/op-constraints`):** Previously noted in the readiness audit that the gate needed full coupling to kinematic and anatomical virtual fixture constraints rather than trivial pass-throughs.
- **Jacobian Singularity & Kinetic Energy Bounding (`crates/op-kinematics/src/jacobian.rs` & `crates/op-energy/src/lib.rs`):** Must veto any teleoperation master command if the manipulator approaches a kinematic singularity ($\det(J J^T) < \varepsilon_{\text{sing}}$) or if end-effector kinetic energy $\frac{1}{2} m_{\text{eff}} \|v\|^2 > E_{\max}$ near delicate tissue boundaries.

---

## 7. Actionable Development & Verification Queue

Each task below is synchronized with `DEV/ROADMAP.md` and `DEV/TASKS.json` for automated or manual execution:

| Task ID | Title | Target Files | Verification Command |
| :---: | :--- | :--- | :--- |
| **TASK-01** | Wire anatomical virtual-fixture keep-out spheres and planes into op-gate | `crates/op-gate/src/lib.rs`, `crates/op-constraints/src/generate.rs` | `cargo test -p op-gate --quiet` |
| **TASK-02** | Enforce Jacobian manipulability singularity guard and kinetic energy cap | `crates/op-kinematics/src/jacobian.rs`, `crates/op-energy/src/lib.rs` | `cargo test -p op-kinematics -p op-energy --quiet` |
