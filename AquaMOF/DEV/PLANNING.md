# AquaMOF Harvester — Comprehensive Architecture & Planning Documentation

> **Project Directory:** `AquaMOF/`  
> **Primary Language / Toolchain:** `Rust`  
> **Verification Command:** `cargo test --quiet`  
> **Reference Sources:** `KELLERSYSTEMS Spine.md` · `PaySlip.canvas` · `Life Work.canvas`

---

## 1. Executive Spine & System Overview

**AquaMOF Harvester** — A passive atmospheric water generator panel using MOF-303 (metal-organic frameworks) that extracts potable water from desert air (as low as 20% relative humidity) powered entirely by ambient daytime solar heat.

### Current Engineering Baseline
A Rust simulation and sizing engine (`src/isotherm.rs`, `src/kinetics.rs`, `src/condenser.rs`, `src/economics.rs`, `src/telemetry.rs`) modeling passive desert atmospheric water harvesting using MOF-303 (Al-3,5-pyrazoledicarboxylate). Needs full diurnal 24-hour thermodynamic state-machine coupling, dust-storm cut-off gate logic, and WHO potability mineralization modeling.

---

## 2. Canonical Specifications & Mathematical Invariants (`PaySlip.canvas` / `Spine`)

The following 12 architectural pillars and formal invariants define the target engineering state of the system:

- **Metal-Organic Framework Adsorption (MOF-303):** Nutzung mikroporöser Aluminium-Pyrazoldicarboxylat-Gitterstrukturen mit maßgeschneiderter Porengröße zur reversiblen Physisorption von Wasserdampfmolekülen selbst in extrem trockener Wüstenluft bei relativer Feuchte von unter 20 % RH.
    
- **Low-Grade Solar-Thermal Desorption Kinetics:** Vollständig passiver Desorptionszyklus, der durch natürliche diffuse und direkte Sonneneinstrahlung bei moderaten Temperaturen von 75–85 °C das gebundene Wasser ohne externe Hilfsenergie freisetzt.
    
- **Thermodynamic Counter-Flow Condensation Chamber:** Integrierter Passivkühler mit optimierten Kühlrippen, der die freigesetzte feuchte Luft durch Wärmeaustausch mit der kühleren Umgebungstemperatur kondensiert und reines Flüssigwasser abscheidet.
    
- **Diurnal Adsorption-Desorption State Machine:** Streng deterministische Abbildung des 24-Stunden-Umgebungszyklus (nächtliche Dampfaufnahme bei kühlen Nachttemperaturen und hoher Adsorptionsenthalpie, tägliche thermische Freisetzung durch Solarwärme).
    
- **Substrate Press-Coating Adhesion Dynamics:** Dünnschichtige, mechanisch hochbelastbare Beschichtung des Aluminium-Wärmetauschers mit MOF-Kristallpulver zur Minimierung des thermischen Übergangswiderstands und Maximierung des Stofftransports.
    
- **Natural Convective Thermosiphon Aerodynamics:** Strömungsmechanische Auslegung des Gehäuses zur passiven Luftumwälzung über Kamineffekte während der Nachtphase, wodurch Ventilatoren und bewegliche Bauteile komplett entfallen.
    
- **Hydrochemical Potability & Mineralization Invariants:** Definition physikochemischer Reinheitsgrenzwerte, die garantieren, dass das erzeugte Kondensat frei von biologischen Keimen, Staubpartikeln und Schwermetallen ist und den WHO-Trinkwasserstandards entspricht.
    
- **Ambient Relative Humidity Cut-Off Gates:** Automatische mechanische Schutzklappen, die das Paneel bei Staubstürmen oder extremer Witterung versiegeln, um eine Vergiftung oder Verstopfung der Nanoporen des MOF-Gitters zu verhindern.
    
- **Modular Balcony & Off-Grid Array Kaskadierung:** Skalierbare Paneel-Geometrie, die sich modular zu Großanlagen zusammenschließen lässt, um Siedlungen, mobile Forschungsstationen oder landwirtschaftliche Sonderkulturen dezentral mit Trinkwasser zu versorgen.
    
- **Zero-Electricity Autonomous Operation:** Vollständiger Verzicht auf elektrische Anschlüsse, Inverter oder Batterien; die gesamte Förderleistung basiert ausschließlich auf solar-thermodynamischen Phasenumwandlungen.
    
- **Gravity-Feed Hydraulic Storage Interface:** Das kondensierte Wasser sammelt sich am tiefsten Punkt der Kondensationskammer und fließt über ein Schwerkraftventil direkt in versiegelte Trinkwasserkanister ab.
    
- **Trustless Environmental Yield Certification:** Mathematische Modellierung der theoretischen Wasserausbeute (ca. 1 Liter pro Kilogramm MOF pro Tag) auf Basis lokaler Wetter- und Einstrahlungsdaten zur verlässlichen Auslegungsberechnung humanitärer Hilfsprojekte.

---

## 4. Codebase Architecture & Module Inventory

Total tracked source and configuration files in `AquaMOF/`: **8**

| File Path | Lines | Architectural Role |
| :--- | :---: | :--- |
| `Cargo.toml` | 21 | Build / Package Manifest |
| `src/condenser.rs` | 49 | Core Implementation Module |
| `src/economics.rs` | 55 | Core Implementation Module |
| `src/isotherm.rs` | 104 | Core Implementation Module |
| `src/kinetics.rs` | 71 | Core Implementation Module |
| `src/lib.rs` | 81 | Core Implementation Module |
| `src/main.rs` | 40 | Core Implementation Module |
| `src/telemetry.rs` | 32 | Core Implementation Module |

---

## 5. Gap Analysis & Production-Readiness Audit

- **S-Shaped Step Isotherm & Temperature Dependence (`src/isotherm.rs`):** Needs Clausius-Clapeyron isosteric heat of adsorption ($\Delta H_{\text{ads}} \approx -52\text{ kJ/mol}$) shifting the inflection point $P/P_0 \approx 0.15$ between $25\,^\circ\text{C}$ night adsorption and $85\,^\circ\text{C}$ solar desorption.
- **Diurnal 24-Hour Coupled State Machine (`src/lib.rs` & `src/kinetics.rs`):** Needs an hourly diurnal integrator coupling solar irradiance, fin temperature, Fickian intracrystalline diffusion, and natural thermosiphon air mass flow.
- **Condenser Dew-Point & Dust Cut-Off Gate (`src/condenser.rs` & `src/telemetry.rs`):** Needs psychrometric Magnus-Tetens dew-point verification and mechanical shutter cut-off logic when ambient dust/PM10 or RH falls outside operational bounds.

---

## 7. Actionable Development & Verification Queue

Each task below is synchronized with `DEV/ROADMAP.md` and `DEV/TASKS.json` for automated or manual execution:

| Task ID | Title | Target Files | Verification Command |
| :---: | :--- | :--- | :--- |
| **TASK-01** | Implement Clausius-Clapeyron temperature-dependent MOF-303 step isotherm | `src/isotherm.rs` | `cargo test --quiet` |
| **TASK-02** | Implement 24-hour diurnal adsorption-desorption-condensation state machine | `src/kinetics.rs`, `src/condenser.rs`, `src/lib.rs` | `cargo test --quiet` |
| **TASK-03** | Implement dust-storm shutter protection gate and WHO potability audit in telemetry | `src/telemetry.rs`, `src/economics.rs` | `cargo test --quiet` |
