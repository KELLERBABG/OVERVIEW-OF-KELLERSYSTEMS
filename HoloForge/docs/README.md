# HoloForge Documentation Portal

Welcome to the **HoloForge** documentation suite. HoloForge is an experimental holographic volumetric additive manufacturing platform that solidifies complete 3D geometries in 10–30 seconds using phase-only spatial light modulation and tomographic light-field shaping.

---

## 📚 Table of Contents

### 🚀 Getting Started & Architecture
* **[Getting Started Guide](GETTING_STARTED.md)**: Installation, self-tests, first optimization, `.hfp` job files, Stage 1 calibration, Stage 2 solidification benchmarks, Stage 3 commercial verticals, and Python API quickstart.
* **[System Architecture](ARCHITECTURE.md)**: Software pipeline diagrams, module dependency graphs, end-to-end data flow from CAD mesh to SLM hardware, and 5-layer functional breakdown.
* **[Python API Reference](API_REFERENCE.md)**: Complete API documentation for all public modules, functions, classes, and dataclasses across `holoforge` (`config`, `optics`, `optimize`, `dose_compensate`, `calibration`, `kinetics`, `metrology`, `verticals`, `player`, `simulate`, `volume`, `io_hfp`, `benchmark`, `render`, `cli`).

### 🔬 Physics, Chemistry & Optics
* **[Technical Whitepaper](WHITEPAPER.md)**: Comprehensive academic and engineering whitepaper detailing the physics, mathematical formulation, inverse solvers, and blue-ocean commercial applications.
* **[Algorithm Deep Dive](ALGORITHM_DEEP_DIVE.md)**: In-depth mathematical derivations of Multi-Plane Gerchberg-Saxton (MP-GS), least-squares scale-free projection, guard slice suppression, and iterative dose compensation.
* **[Optical System Design Guide](OPTICS_GUIDE.md)**: Physical optical train specifications, 405nm laser requirements, SLM phase response LUT calibration, 4f relay optics, and Nyquist spatial frequency limits.
* **[Resin Chemistry & Formulation Guide](RESIN_GUIDE.md)**: Photopolymer kinetics, oxygen inhibition thresholds, absorption ($\alpha$) and scatter mean free path ($L_{\text{scatter}}$) characterization, and the 5 calibrated resin formulations (including Stage 2 PEGDA and HDDA systems).

### 🛠️ Hardware & Community
* **[Stage 1 Bill of Materials (BOM)](BOM.md)**: Component list, specifications, budget breakdown (~€8k–€10k), and sourcing recommendations for the physical optical alignment bench prototype.
* **[Frequently Asked Questions (FAQ)](FAQ.md)**: Answers to common questions regarding resolution limits, differences from CAL/xolography/SLA, 3-stage roadmap, B2B commercial focus, and hardware pattern export.

---

## ⚡ Quick Command Reference

| Task | CLI Command |
|---|---|
| **Run physics self-test** | `holoforge selftest` |
| **Stage 1: Generate SLM alignment targets** | `holoforge calibrate --type spot_grid --out output/calibration` |
| **Stage 2: Run 15s solidification & metrology** | `holoforge stage2-benchmark --resin hf-pegda-405 --out output/stage2_benchmark` |
| **Stage 3: Optimize digital dental crown** | `holoforge vertical --type dental --resin hf-dental-405 --out output/verticals` |
| **Stage 3: Optimize microfluidic chip** | `holoforge vertical --type microfluidics --resin hf-clear-405 --out output/verticals` |
| **Stage 3: Optimize aspheric lens** | `holoforge vertical --type lens --resin hf-hdda-405 --out output/verticals` |
| **Hardware: Export SLM bitmaps + manifest** | `holoforge export-slm output/verticals/microfluidic_chip.hfp --out output/slm_frames` |
| **Voxelize & optimize custom STL** | `holoforge optimize --stl path/to/model.stl --out output/` |
| **Run full benchmark suite** | `holoforge benchmark --nxy 64 --nz 32 --iters 30` |
| **Re-simulate & render QA report** | `holoforge render output/job.hfp --out output/report.png` |

---

*HoloForge — Holographic Volumetric 3D Printing Core.*
