# HoloForge (Tomo-H-VAM) — Comprehensive Architecture & Planning Documentation

> **Project Directory:** `HoloForge/`  
> **Primary Language / Toolchain:** `Python`  
> **Verification Command:** `pytest tests -q`  
> **Reference Sources:** `KELLERSYSTEMS Spine.md` · `PaySlip.canvas` · `Life Work.canvas`

---

## 1. Executive Spine & System Overview

**HoloForge (Tomo-H-VAM)** — A physics simulation and hardware control suite for Holographic Tomographic Volumetric Additive Manufacturing that cures entire 3D resin geometries simultaneously from dynamic projection angles without layers.

### Current Engineering Baseline
A Python Volumetric Additive Manufacturing suite (`holoforge/optics.py`, `optimize.py`, `kinetics.py`, `metrology.py`, `dose_compensate.py`, `volume.py`, `hardware/stage.py`, `laser.py`, `display.py`, `runner.py`, `web/server.py`) achieving 92.3% simulated Cure IoU and <0.02% overcure via Multi-Plane Gerchberg-Saxton and coupled free-radical O2-inhibition kinetics.

---

## 2. Canonical Specifications & Mathematical Invariants (`PaySlip.canvas` / `Spine`)

The following 12 architectural pillars and formal invariants define the target engineering state of the system:

# HoloForge (Tomo-H-VAM)

> **Kategorie:** Keller Systems Engineering Architecture  
> **Grundlage:** HoloForge Lernzentrum Arbeitsbuch (November 2026, Lukas Negenborn)  
> **Index-Links:** [[Life Work.canvas|Master Hub (Life Work)]] · [[Life Work/PaySlip.canvas|PaySlip Directory]]

---

## 🛠️ Technische Spezifikationen & Physikalische Invarianten

- **Elektrodynamik & Komplexe Felder ($\lambda_0 = 405\,\text{nm}$):** Skalare Helmholtz-Wellengleichung im dielektrischen Medium mit $k = \frac{2\pi n}{\lambda_0}$ und effektiver Wellenlänge $\lambda_{\text{resin}} \approx 266{,}4\,\text{nm}$ ($n = 1{,}52$); das Acrylatharz reagiert quadratisch auf die lokale Bestrahlungsstärke $I(r) = |E(r)|^2$.
    
- **Analytische Zylinderglas-Phasenkonjugation (Sagitta $s(x)$):** Exakte geometrische Berechnung der Pfeilhöhe $s(x) = R - \sqrt{R^2 - x^2}$ von Glasinnen- und Außenwand und inverse Aufprägung des optischen Gangunterschieds $\Delta\Phi_{\text{vat}}(x)$ auf den SLM; eliminiert Zylinderlinsen-Astigmatismus trocken an Luft ohne toxisches Index-Matching-Ölbad.
    
- **Winkelspektrum-Methode (ASM) & Split-Step-Operator:** Lichtwellenausbreitung über die Fourier-Transferfunktion des freien Raumes $H(f_x, f_y; \Delta z)$ kombiniert mit dem Split-Step-Operator $\mathcal{P} = \mathcal{D}_{\Delta z/2} \cdot \mathcal{A}_{\mu_{\text{abs}}} \cdot \mathcal{D}_{\Delta z/2}$ für inhomogene Harzmedien und exponentielle Beer-Lambert-Absorption.
    
- **Axiales Tiefen-Multiplexing ($Z_k$-Tiefenbänder):** Überwindung des Beugungslimits der Rayleigh-Länge ($z_R \approx 1{,}5\,\text{mm}$ bei $w_0 \approx 14\,\mu\text{m}$) durch schnelles zeitliches Multiplexen des SLM auf $N$ diskrete Z-Fokusebenen, sodass das gesamte Bauvolumen beugungsbegrenzt scharf belichtet wird.
    
- **Multi-Plane Gerchberg-Saxton (MP-GS) mit Guard Slices:** Iterative Phasenrekonstruktion, die für jeden Rotationswinkel $\theta_k$ simultan an mehreren $Z$-Ebenen die Zielamplituden formt und Streulicht über Guard Slices in unkritische Randbereiche abdrängt.
    
- **Nichtlineare Kinetik der Sauerstoff-Inhibition:** Gelöster Luftsauerstoff $[\text{O}_2]$ fängt Radikale rund $10^6$-fach schneller ab ($k_{\text{inh}} \approx 10^9\,\text{M}^{-1}\text{s}^{-1}$) als das Kettenwachstum der Acrylat-Monomere ($k_p \approx 10^3\,\text{M}^{-1}\text{s}^{-1}$); Kettenwachstum setzt erst nach vollständiger lokaler Sauerstoffzehrung ein.
    
- **Heaviside-Schwellenwert-Polymerisation ($D_{\text{gel}}$):** Einzelne Durchstrahlungsachsen verbleiben unterhalb der kritischen Schwellendosis ($D_{\text{gel}} \approx 10\text{--}18\,\text{mJ/cm}^2$); nur im dreidimensionalen Schnittvolumen aller rotierenden Projektionen akkumuliert die Dosis über $D_{\text{gel}}$, was zu schlagartiger, randscharfer Erstarrung führt.
    
- **Speckle-Unterdrückung durch Rotationsmittelung ($1/\sqrt{K}$):** Überlagerung von $K=64$ statistisch unkorrelierten Phasenmasken reduziert den Speckle-Kontrast $C = 1/\sqrt{K}$ auf unter 12,5 %; thermische Radikaldiffusion glättet verbleibende Mikrofluktuationen auf eine Oberflächenrauheit von $R_a < 100\,\text{nm}$.
    
- **In-situ 635-nm-Schlieren-Metrologie ($\Delta n$):** Nicht-aktinischer Rotlaser erfasst den Brechungsindexsprung $\Delta n$ (Lorentz-Lorenz-Dichteanstieg bei der Monomer-Polymer-Phasenumwandlung) über optische Schattenflüsse in Echtzeit zur automatischen Endpunkt-Abschaltung.
    
- **Coupled Free-Radical ODE-Solver (Digital Twin):** Rigorose Modellierung der gekoppelten Kinetik aus Photolyse ($\frac{d[\text{PI}]}{dt}$), Sauerstoffzehrung ($\frac{d[\text{O}_2]}{dt}$) und Monomerumsatz ($\frac{d[M]}{dt}$) zur exakten Vorhersage von Härtungstiefe und Voxel-Profilometrie.
    
- **Cylindrical Rotary Stage G-Code Interlock:** Sub-Millisekunden-Synchronisation zwischen Schrittmotor-Rotationswinkel $\theta$, SLM-Frame-Buffer und Laser-Sicherheitsblende zur Vermeidung von kinetischen Verzerrungen und Schlupf.
    
- **Trustless Volumetric Curing Gate:** Mathematischer Nachweis der Dosis-Invarianten vor der physischen Laser-Freigabe garantiert ein Target-Cure IoU von $>92\,\%$ bei $<0{,}02\,\%$ Überhärtung im Gesamtharz.

---

## 🔗 Verknüpfungen & Canvases
- [[Life Work.canvas|Master Architecture Canvas (Life Work)]]
- [[Life Work/PaySlip.canvas|PaySlip Canvas (Index)]]


---

## 4. Codebase Architecture & Module Inventory

Total tracked source and configuration files in `HoloForge/`: **42**

| File Path | Lines | Architectural Role |
| :--- | :---: | :--- |
| `pyproject.toml` | 87 | Build / Package Manifest |
| `holoforge/__init__.py` | 8 | Core Implementation Module |
| `holoforge/backend.py` | 232 | Core Implementation Module |
| `holoforge/benchmark.py` | 217 | Core Implementation Module |
| `holoforge/calibration.py` | 212 | Core Implementation Module |
| `holoforge/cli.py` | 534 | Core Implementation Module |
| `holoforge/config.py` | 170 | Core Implementation Module |
| `holoforge/dose_compensate.py` | 273 | Core Implementation Module |
| `holoforge/io_hfp.py` | 46 | Core Implementation Module |
| `holoforge/kinetics.py` | 132 | Core Implementation Module |
| `holoforge/metrology.py` | 200 | Core Implementation Module |
| `holoforge/optics.py` | 310 | Core Implementation Module |
| `holoforge/optimize.py` | 436 | Core Implementation Module |
| `holoforge/player.py` | 140 | Core Implementation Module |
| `holoforge/render.py` | 105 | Core Implementation Module |
| `holoforge/simulate.py` | 87 | Core Implementation Module |
| `holoforge/verticals.py` | 134 | Core Implementation Module |
| `holoforge/volume.py` | 279 | Core Implementation Module |
| `holoforge/hardware/__init__.py` | 25 | Core Implementation Module |
| `holoforge/hardware/camera.py` | 141 | Core Implementation Module |
| `holoforge/hardware/display.py` | 104 | Core Implementation Module |
| `holoforge/hardware/laser.py` | 158 | Core Implementation Module |
| `holoforge/hardware/runner.py` | 153 | Core Implementation Module |
| `holoforge/hardware/stage.py` | 113 | Core Implementation Module |
| `holoforge/web/__init__.py` | 4 | Core Implementation Module |
| `holoforge/web/server.py` | 589 | Core Implementation Module |
| `tests/__init__.py` | 1 | Verification & Test Suite |
| `tests/conftest.py` | 60 | Verification & Test Suite |
| `tests/test_aberration.py` | 37 | Verification & Test Suite |
| `tests/test_backend.py` | 64 | Verification & Test Suite |
| `tests/test_camera_and_stl_ui.py` | 118 | Verification & Test Suite |
| `tests/test_cli.py` | 49 | Verification & Test Suite |
| `tests/test_hardware.py` | 129 | Verification & Test Suite |
| `tests/test_io.py` | 42 | Verification & Test Suite |
| `tests/test_optics.py` | 71 | Verification & Test Suite |
| `tests/test_optimize.py` | 62 | Verification & Test Suite |
| `tests/test_safety.py` | 228 | Verification & Test Suite |
| `tests/test_simulate.py` | 52 | Verification & Test Suite |
| `tests/test_stage1_2_3.py` | 184 | Verification & Test Suite |
| `tests/test_tomographic.py` | 135 | Verification & Test Suite |
| `tests/test_volume.py` | 82 | Verification & Test Suite |
| `tests/test_web.py` | 91 | Verification & Test Suite |

---

## 5. Gap Analysis & Production-Readiness Audit

- **Trustless Volumetric Curing Gate (`holoforge/optimize.py` & `holoforge/hardware/runner.py`):** Before arming the 405 nm laser shutter in `runner.py`, enforce a pre-flight mathematical gate verifying that the synthesized hologram stack achieves target Cure IoU $\ge 0.90$ and out-of-part overcure fraction $< 0.0005$ ($0.05\%$).
- **In-Situ 635 nm Schlieren Endpoint Detection (`holoforge/hardware/camera.py` & `holoforge/kinetics.py`):** Connect real-time refractive index jump $\Delta n$ (Lorentz-Lorenz density shift) from the Schlieren camera feed to trigger automatic laser cut-off the moment gelation threshold $D_{\text{gel}}$ is reached, preventing over-polymerization.

---

## 7. Actionable Development & Verification Queue

Each task below is synchronized with `DEV/ROADMAP.md` and `DEV/TASKS.json` for automated or manual execution:

| Task ID | Title | Target Files | Verification Command |
| :---: | :--- | :--- | :--- |
| **TASK-01** | Enforce pre-flight volumetric dose invariant gate in hardware print runner | `holoforge/hardware/runner.py`, `tests/test_safety.py` | `pytest tests/test_safety.py tests/test_hardware.py -q` |
| **TASK-02** | Implement Lorentz-Lorenz refractive index Δn endpoint trigger in Schlieren metrology | `holoforge/metrology.py`, `holoforge/hardware/camera.py`, `tests/test_optics.py` | `pytest tests/test_optics.py -q` |
