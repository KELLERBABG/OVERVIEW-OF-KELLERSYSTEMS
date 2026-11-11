# SmokeShift Tracker (AeroDose) — Comprehensive Architecture & Planning Documentation

> **Project Directory:** `AeroDose/`  
> **Primary Language / Toolchain:** `Rust`  
> **Verification Command:** `cargo test --quiet`  
> **Reference Sources:** `KELLERSYSTEMS Spine.md` · `PaySlip.canvas` · `Life Work.canvas`

---

## 1. Executive Spine & System Overview

**SmokeShift Tracker (AeroDose)** — A clip-on wearable sensor (nRF52 + Sensirion SPS30) that tracks cumulative PM2.5 particulate intake during shifts for outdoor construction and forestry crews to meet workplace safety and employer liability standards.

### Current Engineering Baseline
The project has a working audit hash-chain (Ed25519 + SHA-256), a compliance report generator, and a dose tracker skeleton. Missing are the thermodynamic drift compensation engine, taint-coded sensor stream validation, non-disableable haptic emergency veto state machine, full CLI wiring, and integration test coverage.

---

## 2. Canonical Specifications & Mathematical Invariants (`PaySlip.canvas` / `Spine`)

The following 12 architectural pillars and formal invariants define the target engineering state of the system:

- **Cumulative PM2.5 Inhalation Dosimetry:** Kontinuierliche zeitliche Integration der Feinstaubkonzentration über die gesamte Schichtdauer zur präzisen Berechnung der akkumulierten alveolengängigen Partikeldosis für outdoor arbeitende Forstarbeiter, Bau- und Feuerwehrkolonnen.
    
- **Optical Particle Spectrometry (Sensirion SPS30):** Laserbasierte Streulichtmessung zur Erfassung aerodynamischer Massenkonzentrationen (PM1.0, PM2.5, PM4.0, PM10) und Partikelanzahlkonzentrationen im Bereich von 0,3 bis 10 µm mit fortschrittlicher Selbstreinigungsfunktion gegen Rußablagerungen.
    
- **Nordic nRF52 Ultra-Low-Power Engine:** Energieeffiziente ARM Cortex-M4 Architektur mit adaptivem Schlaf- und Tastverhältnis, die die Sensorheizung und optische Kammer nur in dynamisch angepassten Intervallen bestromt, um wochenlange Akkulaufzeit im rauen Feldeinsatz zu gewährleisten.
    
- **Deterministic Regulatory Invariant Enforcement:** Lokale Überwachung internationaler und nationaler Arbeitsplatzgrenzwerte (wie OSHA, EU-Richtlinien und BG-Vorgaben) und mathematischer Nachweis, dass bei Überschreiten toxischer Expositionsschwellen ohne Latenz eine haptische und akustische Warnung initiiert wird.
    
- **Cryptographic Shift Logbook (Anti-Tamper Flash):** Lückenlose Speicherung signierter Expositions-Zeitreihen in einem geschützten Ringpuffer des On-Chip-Flash-Speichers, der Manipulationen durch Arbeitgeber oder Dritte ausschließt und als unbestechlicher forensischer Beweis für Berufsgenossenschaften dient.
    
- **Zero-Cloud BLE Peer Synchronization:** Vollständig lokale Peer-to-Peer Bluetooth Low Energy Synchronisation mit dem Handgerät des Vorarbeiters, wodurch der Nachweis des Schichtverlaufs auch in tiefen Funklöchern ohne jede Mobilfunk- oder Cloud-Anbindung garantiert ist.
    
- **Dynamic Thermodynamic Drift Compensation:** Deterministische Algorithmen kompensieren Temperatur- und Luftfeuchtigkeitsschwankungen in Echtzeit und verhindern fehlerhafte Partikelgrößenbestimmungen durch hygroskopisches Tröpfchenwachstum bei hoher Luftfeuchte.
    
- **Taint-Coded Sensor Stream Audit:** Kontinuierliche Verfolgung des Rohdatenstroms vom Sensorelement bis zum internen Schwellenwert-Komparator, um verstopfte Ansaugkanäle, verklebte optische Fenster oder gefälschte Telemetriedaten mathematisch unbestechlich abzufangen.
    
- **Worker Liability & Insurance Proofs (zk-Dose):** Erzeugung von kryptographischen Zero-Knowledge-Zertifikaten, die nachweisen, dass die persönliche Schutzmasken-Tragepflicht oder Schichtzeitgrenze eingehalten wurde, ohne detaillierte Bewegungsprofile der Arbeiter preiszugeben.
    
- **Real-Time Haptic Emergency Veto:** Autonomer Sicherheitsmechanismus, der bei akut toxischen Rauchgasspitzen (z. B. durch Drehen des Windes bei Waldbränden) unüberhörbare Vibrations- und Piezosignale erzwingt, die softwareseitig nicht vom Träger stummgeschaltet werden können.
    
- **Autonomous Battery Lifecycle & Solar Harvesting:** Intelligentes Power-Management mit Unterstützung für kleine Energy-Harvesting-Solarzellen auf dem Helm oder der Schulterklappe zur kontinuierlichen Pufferung der Pouch-Zelle im Freilandeinsatz.
    
- **Trustless Shift Settlement Gate:** Automatisiertes Protokoll zur Erzeugung eines digital versiegelten Übergabeprotokolls am Schichtende, das direkt in die Lohnabrechnung und das Gefährdungsbeurteilungskataster der Berufsgenossenschaft einfließt.

---

## 4. Codebase Architecture & Module Inventory

Total tracked source and configuration files in `AeroDose/`: **10**

| File Path | Lines | Architectural Role |
| :--- | :---: | :--- |
| `Cargo.toml` | 25 | Build / Package Manifest |
| `src/audit.rs` | 97 | Core Implementation Module |
| `src/compliance.rs` | 40 | Core Implementation Module |
| `src/dose.rs` | 105 | Core Implementation Module |
| `src/lib.rs` | 73 | Core Implementation Module |
| `src/main.rs` | 62 | Core Implementation Module |
| `src/sensor.rs` | 41 | Core Implementation Module |
| `src/taint.rs` | 274 | Core Implementation Module |
| `src/thermo.rs` | 252 | Core Implementation Module |
| `src/veto.rs` | 349 | Core Implementation Module |

---

## 5. Gap Analysis & Production-Readiness Audit

- **Completed Invariants (TASK-01 to TASK-03):** `src/thermo.rs`, `src/taint.rs`, and `src/veto.rs` are implemented with hygroscopic growth compensation, rolling variance/jump taint detection, and non-mutable emergency haptic veto states.
- **Integration & Pipeline Wiring (`TASK-04`):** `src/lib.rs` needs end-to-end integration tests connecting `ThermoCompensator -> TaintDetector -> DoseTracker -> HapticVeto -> AuditLog`.
- **CLI & Settlement Interface (`TASK-05`):** `src/main.rs` (49 lines) needs complete subcommands for shift dose simulation, cryptographic log verification, OSHA/EU compliance report export, and zk-Dose proof generation.

---

## 7. Actionable Development & Verification Queue

Each task below is synchronized with `DEV/ROADMAP.md` and `DEV/TASKS.json` for automated or manual execution:

| Task ID | Title | Target Files | Verification Command |
| :---: | :--- | :--- | :--- |
| **TASK-01** | Implement thermodynamic drift compensation module | `src/thermo.rs` | `cargo check --quiet` |
| **TASK-02** | Implement taint-coded sensor stream audit module | `src/taint.rs` | `cargo check --quiet` |
| **TASK-03** | Implement non-disableable haptic emergency veto state machine | `src/veto.rs` | `cargo check --quiet` |
| **TASK-04** | Wire thermo, taint, and veto modules into lib.rs with integration tests | `src/lib.rs` | `cargo check --quiet` |
| **TASK-05** | Complete CLI with subcommands for dose, audit, compliance, and veto simulation | `src/main.rs` | `cargo check --quiet` |
| **TASK-06** | Add property-based edge-case tests for dose integration and audit chain | `src/dose.rs`, `src/audit.rs` | `cargo test --quiet` |
