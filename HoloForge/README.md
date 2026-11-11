# HoloForge — Holographic Tomographic Volumetric 3D Printing Suite (Tomo-H-VAM)

[![CI](https://github.com/KELLERBABG/HoloForge/actions/workflows/ci.yml/badge.svg)](https://github.com/KELLERBABG/HoloForge/actions/workflows/ci.yml)
[![Python 3.10–3.14](https://img.shields.io/badge/python-3.10%E2%80%933.14-1F6FEB)](https://www.python.org/downloads/)
[![PyPI](https://img.shields.io/badge/PyPI-holoforge--opt-1F6FEB)](https://pypi.org/project/holoforge-opt/)
[![License: BUSL-1.1](https://img.shields.io/badge/license-BUSL--1.1-1F6FEB)](LICENSE)

**HoloForge** is an end-to-end software, physics simulation, and hardware control platform for **Holographic Tomographic Volumetric Additive Manufacturing (Tomo-H-VAM)**.

Unlike conventional layer-by-layer SLA/DLP 3D printers that take hours and leave stair-stepping striations, HoloForge shapes coherent 405 nm wavefronts across multiple rotary angles into a cylindrical photopolymer resin vat—solidifying an entire 3D object simultaneously in **under 15 seconds** (simulated design target), completely support-free.

> **No Physical Hardware Required to Run:** HoloForge includes a complete **Digital Twin Optical & Photochemical Simulator** and **Mock Hardware Emulation Layer**. You can synthesize holograms, upload `.stl` CAD meshes, inspect 3D optical dose & surface profilometry, and run simulated rotary prints with virtual 635 nm Schlieren camera feedback entirely on any standard laptop/PC.

> **Claims provenance:** Every performance figure in this repository (cure IoU, overcure fraction, surface roughness, print times) comes from the Digital Twin simulator unless explicitly labeled *measured*. Hardware validation is a Stage 2–3 roadmap goal, not a shipped capability.

---

## ✨ Complete Architecture (Tiers 1–5)

| Layer | Module | Description |
|---|---|---|
| **Core Wave-Optics & Tomo-GS** | `holoforge/optics.py`, `holoforge/optimize.py` | Split-step angular spectrum propagation, $4f$ telecentric spatial filtering, double-phase hologram synthesis, and closed-loop multi-angle tomographic optimization (**92.3% Cure IoU**, **<0.02% overcure** — both simulated; regenerate with `python -m holoforge.cli verify`, pinned seed 7, 16 patterns, 20 iterations). |
| **Photochemistry & Metrology** | `holoforge/kinetics.py`, `holoforge/metrology.py` | Coupled radical polymerization + dissolved $\text{O}_2$ inhibition ODE solver and virtual white-light interferometric optical profilometer ($R_a, R_q, R_z$). |
| **Commercial B2B Verticals** | `holoforge/verticals.py` | Parametric generators for **Dental Crowns**, **Microfluidic Chips**, and **Aspheric Optical Lenses**, plus arbitrary `.stl` voxelization (`holoforge/volume.py`). |
| **Tier 1: Hardware Drivers** | `holoforge/hardware/` (`stage.py`, `display.py`, `laser.py`, `runner.py`) | G-code USB-serial rotary stage controller, fullscreen secondary-monitor phase SLM presenter, 405 nm laser shutter safety state machine, and synchronous print runner. |
| **Tier 2: Vat Aberration Conjugate** | `holoforge/optics.py` (`cylindrical_vat_phase_correction`) | Analytical inverse phase conjugate mask ($\Delta\Phi_{\text{vat}}$) pre-canceling cylindrical glass vial astigmatism directly on the SLM (`--vat-radius-mm`) without requiring an index-matching bath. |
| **Tier 3: GPU/CPU Compute Engine** | `holoforge/backend.py` | Automatic `cupy` / `torch.cuda` GPU acceleration with transparent fallback to CPU (`numpy`/`scipy`). |
| **Tier 4: Web & Touchscreen UI** | `holoforge/web/` (`server.py`, `static/index.html`) | Zero-dependency interactive dark-mode industrial dashboard (`holoforge ui`) with 3D rotating vat & voxel viewer, `.stl` mesh upload, and live telemetry. |
| **Tier 5: Schlieren Camera Vision** | `holoforge/hardware/camera.py` | In-situ 635 nm optical shadowgraphy / Schlieren refractive-index ($\Delta n$) gelation monitor and closed-loop laser endpoint auto-stop. |

---

## 🚀 Quickstart (100% Software / Digital Twin Mode)

### 1. Launch the Interactive Web Control Center
```powershell
python -m holoforge.cli ui --port 8080
```
Open `http://127.0.0.1:8080` in your browser to:
- Choose built-in primitives, commercial B2B verticals (`dental`, `microfluidics`, `lens`), or **upload your own `.stl` file**.
- Configure the cylindrical glass vat radius (`Vat Radius (mm)`) for automatic aberration pre-compensation.
- Click **Synthesize Tomographic Holograms** to view the 3D cured voxel cloud, SLM phase hologram ($\phi_0$), mid-volume optical dose slice, and **635 nm Schlieren camera shadowgram**.
- Click **Execute Rotary Print Sequence** to run the simulated rotary stage, SLM frame presenter, and 405 nm laser shutter in real time.

### 2. Command-Line Workflow
```powershell
# Run physics self-test (energy conservation + Fresnel lens focusing)
python -m holoforge.cli selftest

# Evidence harness: regenerate the published (simulated) claims from a
# pinned seed; exits non-zero if a claim has drifted from the code
python -m holoforge.cli verify

# Optimize a 3D target with cylindrical vat aberration pre-compensation
python -m holoforge.cli optimize --mode tomographic --shape sphere --patterns 16 --iters 6 --vat-radius-mm 10.0 --out output

# Run Stage 2 15-second solidification & optical profilometry benchmark
python -m holoforge.cli stage2-benchmark --out output/stage2_benchmark

# Optimize a B2B commercial vertical (dental | microfluidics | lens)
python -m holoforge.cli vertical --type dental --out output/verticals

# Export 8-bit calibrated SLM phase bitmaps + JSON hardware timing manifest
python -m holoforge.cli export-slm output/stage2_benchmark/stage2_sphere.hfp --out output/slm_frames

# Execute a simulated hardware print run
python -m holoforge.cli print output/stage2_benchmark/stage2_sphere.hfp --mock --speed 5.0
```

### 3. Run the Automated Test Suite
```powershell
python -m pytest -v
```

---

## 🛠️ Future Hardware Paths (When You Are Ready to Build)

You do **not** need expensive lab hardware right now. When you eventually decide to build a physical prototype, there are two tiers depending on budget:

### Path A: Ultra-Budget DIY Tomographic Prototype (~$250–$350)
Instead of a phase LCoS SLM, you can feed HoloForge's exported projections into a modified consumer UV DLP projector or collimated monochrome LCD engine:
1. **Light Engine (~$120):** Repurposed 405 nm UV DLP mini-projector (or 405 nm LED array + telecentric Fresnel lens + monochrome LCD mask) driven over HDMI as Monitor 2 (`SLMDisplayManager`).
2. **Rotary Stage (~$45):** NEMA-17 stepper motor + 10:1 worm gear + TMC2209 driver wired to an Arduino Nano running [`firmware/holoforge_controller/holoforge_controller.ino`](firmware/holoforge_controller/holoforge_controller.ino).
3. **Resin Vat (~$15):** 20 mm borosilicate glass scintillation vial suspended from the top rotary chuck (using HoloForge's `--vat-radius-mm 10.0` software phase/refraction correction).
4. **In-Situ Camera (~$30):** 635 nm red LED + $20 USB monochrome webcam connected to `SchlierenCameraMonitor`.

### Path B: Research-Grade Holographic LCoS Setup (~$2,500–$4,000)
1. **Laser:** 405 nm single-mode fiber-coupled laser diode (120–200 mW, $\text{TEM}_{00}$).
2. **Phase Modulator:** 1920×1080 reflective LCoS phase-only SLM calibrated for $2\pi$ retardance at 405 nm.
3. **4f Telecentric Relay:** $f_1 = 150\text{ mm}, f_2 = 75\text{ mm}$ achromatic doublets with a Fourier-plane spatial pinhole (blocks zero-order DC light and converts HoloForge's double-phase carrier holograms into full complex wavefronts inside the vat).
4. **Resin Formulation:** High-viscosity UDMA/PEGDA base monomer + **0.15 wt% TPO** photoinitiator + **0.04 wt% TEMPO** radical inhibitor (sharpens the non-linear gel threshold).

---

## 📜 License

HoloForge is **source-available, not open source**. It is licensed under the [Business Source License 1.1 (BUSL-1.1)](LICENSE):

- **Free for** evaluation, research, education, and internal R&D — including full use of the Digital Twin simulator and mock hardware layer.
- **Commercial license required** to manufacture goods for sale, to offer commercial services powered by HoloForge, or to control physical hardware in a commercial process.
- On the Change Date (**October 10, 2030**), each released version automatically converts to the Apache License 2.0.

For commercial licensing: **contact@kellersystems.dev**
