# Smart Matter — Comprehensive Architecture & Planning Documentation

> **Project Directory:** `Smart-Matter/`  
> **Primary Language / Toolchain:** `Rust`  
> **Verification Command:** `cargo test --workspace --quiet`  
> **Reference Sources:** `KELLERSYSTEMS Spine.md` · `PaySlip.canvas` · `Life Work.canvas`

---

## 1. Executive Spine & System Overview

**Smart Matter** — Autonomous, kinetic-energy-harvesting compute nodes embedded inside concrete and asphalt infrastructure that communicate over an ad-hoc mesh to monitor structural integrity and store encrypted messages.

### Current Engineering Baseline
A multi-crate embedded Rust workspace (`firmware/core`, `firmware/bootloader`, `firmware/node`, `mesh`, `storage`, `broadcaster`, `desktop-tool`, `tools/network-simulator`) for autonomous, piezoelectric kinetic-energy-harvesting compute nodes embedded inside concrete and asphalt.

---

## 2. Canonical Specifications & Mathematical Invariants (`PaySlip.canvas` / `Spine`)

The following 12 architectural pillars and formal invariants define the target engineering state of the system:

- **Piezoelectric Kinetic Extraction:** Umwandlung mikroskopischer Druckwellen, seismischer Schwingungen und urbaner Trittschall-Energie in nutzbare elektrische Mikrostrome direkt innerhalb der physischen Gussmasse.
    
- **Crystalline Lattice Resonance:** Feinabstimmung piezoelektrischer Kristallstrukturen auf die spezifischen Vibrationsfrequenzen von Straßen, Brücken oder Server-Racks, um den Wirkungsgrad der Energieerzeugung zu maximieren.
    
- **Embedded Compute Nodes:** Integration mikroskopisch kleiner, robuster und energieeffizienter Rechenkerne direkt in die physische Materialsubstanz der städtischen Infrastruktur.
    
- **Environmental Signature Sampling:** Lokale Umgebungsdaten wie kontinuierliches seismisches Rauschen oder elektromagnetische Hintergrundfelder werden als unvorhersehbare physikalische Entropiequelle für Einmal-Schlüssel genutzt.
    
- **Ad-hoc Mesh Formations:** Autonome, drahtlose Vernetzung der im Material eingegossenen Knoten untereinander, sobald physischer Druck oder Vibrationen die Energieversorgung aktivieren.
    
- **Decentralized Dead-Drop Storage:** Nutzung der intelligenten Materialknoten (z. B. in Straßenlaternen) als asynchrone, physische Speicherpunkte, an denen Datenfragmente für Empfänger hinterlegt werden.
    
- **Low-Power Protocol Slicing:** Extrem leichtgewichtige Datenübertragung, die darauf optimiert ist, Statusänderungen in winzigen Millisekunden-Fenstern der piezoelektrischen Stromerzeugung zu versenden.
    
- **Thermal Inertia Coupling:** Mathematische Abstimmung der lokalen Rechenlast im Material auf die thermische Kapazität und Ableitung der umgebenden Beton- oder Asphaltstrukturen.
    
- **Material Integrity Auditing:** Kontinuierliche Selbstüberwachung der Knotenverbindungen; strukturelle Schäden oder Risse im Material verändern das piezoelektrische Signalmuster und triggern sofortige Warnungen.
    
- **Sovereign Localized Broadcaster:** Lokale Verteilung städtischer Datenfragmente ausschließlich über die physische Reichweite des betroffenen Materialviertels, um Daten-Lokalität zu erzwingen.

---

## 4. Codebase Architecture & Module Inventory

Total tracked source and configuration files in `Smart-Matter/`: **44**

| File Path | Lines | Architectural Role |
| :--- | :---: | :--- |
| `Cargo.toml` | 39 | Build / Package Manifest |
| `broadcaster/Cargo.toml` | 10 | Build / Package Manifest |
| `broadcaster/src/lib.rs` | 107 | Core Implementation Module |
| `desktop-tool/Cargo.toml` | 16 | Build / Package Manifest |
| `desktop-tool/src/main.rs` | 363 | Core Implementation Module |
| `firmware/bootloader/Cargo.toml` | 33 | Build / Package Manifest |
| `firmware/bootloader/build.rs` | 23 | Core Implementation Module |
| `firmware/bootloader/src/boot.rs` | 415 | Core Implementation Module |
| `firmware/bootloader/src/host.rs` | 159 | Core Implementation Module |
| `firmware/bootloader/src/main.rs` | 46 | Core Implementation Module |
| `firmware/bootloader/src/platform.rs` | 137 | Core Implementation Module |
| `firmware/core/Cargo.toml` | 31 | Build / Package Manifest |
| `firmware/core/src/broadcast.rs` | 255 | Core Implementation Module |
| `firmware/core/src/crypto.rs` | 334 | Core Implementation Module |
| `firmware/core/src/energy.rs` | 263 | Core Implementation Module |
| `firmware/core/src/integrity.rs` | 334 | Core Implementation Module |
| `firmware/core/src/lib.rs` | 45 | Core Implementation Module |
| `firmware/core/src/memmap.rs` | 47 | Core Implementation Module |
| `firmware/core/src/mesh.rs` | 607 | Core Implementation Module |
| `firmware/core/src/radio.rs` | 303 | Core Implementation Module |
| `firmware/core/src/sensor.rs` | 351 | Core Implementation Module |
| `firmware/core/src/thermal.rs` | 312 | Core Implementation Module |
| `firmware/core/src/types.rs` | 364 | Core Implementation Module |
| `firmware/core/src/wire.rs` | 556 | Core Implementation Module |
| `firmware/core/src/storage/dead_drop.rs` | 383 | Core Implementation Module |
| `firmware/core/src/storage/mod.rs` | 23 | Core Implementation Module |
| `firmware/core/src/storage/wear_level.rs` | 364 | Core Implementation Module |
| `firmware/node/Cargo.toml` | 34 | Build / Package Manifest |
| `firmware/node/build.rs` | 25 | Core Implementation Module |
| `firmware/node/src/host.rs` | 246 | Core Implementation Module |
| `firmware/node/src/main.rs` | 55 | Core Implementation Module |
| `firmware/node/src/node.rs` | 501 | Core Implementation Module |
| `firmware/node/src/platform.rs` | 184 | Core Implementation Module |
| `mesh/Cargo.toml` | 13 | Build / Package Manifest |
| `mesh/src/lib.rs` | 103 | Core Implementation Module |
| `mesh/tests/integration_test.rs` | 298 | Verification & Test Suite |
| `storage/Cargo.toml` | 10 | Build / Package Manifest |
| `storage/src/lib.rs` | 152 | Core Implementation Module |
| `tests/integration_test.rs` | 293 | Verification & Test Suite |
| `tools/flasher/Cargo.toml` | 11 | Build / Package Manifest |
| `tools/flasher/src/main.rs` | 220 | Core Implementation Module |
| `tools/network-simulator/Cargo.toml` | 15 | Build / Package Manifest |
| `tools/network-simulator/src/main.rs` | 87 | Core Implementation Module |
| `tools/network-simulator/src/sim.rs` | 361 | Core Implementation Module |

---

## 5. Gap Analysis & Production-Readiness Audit

- **Piezoelectric Kinetic Extraction & Brownout Invariant (`firmware/core/src/energy.rs`):** Nodes harvest milliwatt-seconds from vehicular traffic vibrations. The energy state machine must prove that a node never executes a flash-write or LoRa/sub-GHz radio burst unless supercapacitor reserve $E_{\text{cap}} > E_{\text{burst}} + E_{\text{margin}}$, guaranteeing zero torn-flash corruptions.
- **Ad-Hoc Wireless Mesh Routing without Central Directory (`firmware/core/src/mesh.rs` & `mesh/src/lib.rs`):** Flooding and epidemic routing through concrete must be bounded by hop-count TTL, bloom-filter message deduplication, and transmission power attenuation to avoid packet storms.
- **Amnesic Flash Dead-Drop Storage (`firmware/core/src/storage/dead_drop.rs` & `wear_level.rs`):** Encrypted multi-hop messages stored in local NOR flash must enforce wear-leveling and automatic epoch-based shredding.

---

## 7. Actionable Development & Verification Queue

Each task below is synchronized with `DEV/ROADMAP.md` and `DEV/TASKS.json` for automated or manual execution:

| Task ID | Title | Target Files | Verification Command |
| :---: | :--- | :--- | :--- |
| **TASK-01** | Enforce flash-write energy margin and power-fail interrupt protection in energy.rs | `firmware/core/src/energy.rs`, `firmware/core/src/storage/dead_drop.rs` | `cargo test -p smartmatter-core --quiet` |
| **TASK-02** | Implement Bloom-filter deduplication and TTL decay in ad-hoc mesh routing | `firmware/core/src/mesh.rs`, `mesh/src/lib.rs` | `cargo test -p smartmatter-mesh --quiet` |
| **TASK-03** | Implement cryptographic dead-drop epoch shredding and anti-tamper zeroization | `firmware/core/src/storage/dead_drop.rs`, `firmware/core/src/crypto.rs` | `cargo test -p smartmatter-core --quiet` |
