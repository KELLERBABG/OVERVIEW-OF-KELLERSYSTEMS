# Getting Started with HoloForge

HoloForge is an experimental holographic volumetric 3D printing engine. This guide walks you through installation, optical self-tests, first hologram optimization, Stage 1 calibration, Stage 2 solidification benchmarks, and Stage 3 commercial vertical printing.

---

## 1. Prerequisites & Installation

### Requirements
- **Python 3.10+**
- Core dependencies: `numpy >= 1.24`, `pillow >= 10.0`
- Development & testing: `pytest >= 7.0`
- Optional GPU acceleration: `scipy >= 1.11`, `cupy-cuda12x`

### Install
Clone the repository and install in editable mode:
```bash
pip install -e '.[dev]'
```

---

## 2. First Run: Physics Self-Test
Verify that optical diffraction, energy conservation, and focal peakedness are functioning correctly:
```bash
holoforge selftest
```
Output:
```
selftest.energy_conservation : PASS (per-slice sum 12892.00 vs pupil 12892.00)
selftest.focus_peakedness    : PASS (peak/mean 128.9, peak slice 12 vs 12)
```

---

## 3. Running Your First Hologram Optimization

Optimize a time-multiplexed SLM phase sequence for a primitive geometry (e.g., a sphere or torus knot):
```bash
holoforge optimize --shape sphere --nxy 64 --nz 32 --iters 20
```

### Understanding the Generated Files
The optimizer writes two files to the `output/` directory:
1. **`job.hfp`**: A compressed container (`npz`) storing the 3D target voxel mask and the optimized SLM phase maps $\phi_k(x, y)$ in radians.
2. **`job.hfp.json`**: A sidecar metadata file storing optical setup, grid discretization, resin properties, exposure durations ($t_k$), and quality metrics (IoU, uniformity error, bleed ratio).
3. **`job_report.png`**: Visual QA report displaying Maximum Intensity Projections (MIP), axial dose cross-sections, cure vs. target overlay, and SLM phase maps.

---

## 4. Stage 1: Optical Alignment & Calibration

Generate test holograms to align the 405 nm laser beam and test focusing in a dye-doped quartz cuvette:
```bash
# Generate 3D multi-focal spot grid (3x3 array)
holoforge calibrate --type spot_grid --out output/calibration

# Generate non-diffracting Bessel beam axicon pattern
holoforge calibrate --type bessel_beam --out output/calibration

# Generate precision optical alignment crosshairs
holoforge calibrate --type crosshair --out output/calibration
```

---

## 5. Stage 2: 15-Second Solidification & Metrology Benchmark

Simulate transient oxygen depletion kinetics and evaluate optical surface roughness ($R_a$) on a 10 mm sphere:
```bash
holoforge stage2-benchmark --resin hf-pegda-405 --out output/stage2_benchmark
```
Output includes:
- Dissolved $O_2$ scavenging induction delay ($t_{\text{ind}}$).
- Radical crosslinking gelation time ($t_{\text{gel}} \le 15\text{ s}$).
- Virtual Optical Profilometer report calculating $R_a$, $R_q$, and $R_z$.

---

## 6. Stage 3: High-Value Commercial Verticals

Optimize dedicated models for high-margin B2B applications:
```bash
# Digital Dentistry (15-second monolithic dental crown)
holoforge vertical --type dental --resin hf-dental-405 --out output/verticals

# Microfluidic Lab-on-a-Chip (enclosed serpentine micro-channels)
holoforge vertical --type microfluidics --resin hf-clear-405 --out output/verticals

# Custom Optics (Precision Aspheric Imaging Lens)
holoforge vertical --type lens --resin hf-hdda-405 --out output/verticals
```

---

## 7. Hardware Export: SLM Driver Frames

Export any `.hfp` job into sequential 8-bit grayscale bitmaps and a millisecond timing manifest ready for hardware projection:
```bash
holoforge export-slm output/verticals/microfluidic_chip.hfp --out output/slm_frames
```
Generated bundle:
- `frame_0000.png`, `frame_0001.png`, ...
- `manifest.json`: Exact exposure durations per frame ($t_k = T_{\text{total}} \cdot w_k$).

---

## 8. Python Programmatic API Quickstart

You can integrate HoloForge into your own scripts and automated pipelines:

```python
import numpy as np
from holoforge.config import OpticsConfig, GridConfig, RESINS
from holoforge.verticals import dental_crown
from holoforge.optimize import MultiPlaneGS
from holoforge.simulate import simulate_dose, gel_level_simulation_units, predict_cure
from holoforge.metrology import extract_topography_height_map, calculate_roughness

# 1. Setup physical configuration
optics = OpticsConfig(slm_pixels=256)
grid = GridConfig(n_xy=128, n_z=96, slice_step_um=2.0)
resin = RESINS["hf-dental-405"]

# 2. Generate target geometry
crown = dental_crown(shape=(grid.n_z, grid.n_xy, grid.n_xy))

# 3. Optimize phase-only holograms
cgh = MultiPlaneGS(optics, grid, resin, crown.mask, n_patterns=8)
result = cgh.optimize(iterations=40, beta=0.65)

# 4. Simulate volumetric dose and cure
dose = simulate_dose(result.phases_rad, result.pattern_weights, optics, grid, resin)
gel = gel_level_simulation_units(resin, optics, grid, result.exposure_s)
cured = predict_cure(dose, gel)

# 5. Measure surface roughness
height_map, valid = extract_topography_height_map(cured, optics, grid)
roughness = calculate_roughness(height_map, valid, pitch_um=optics.pitch_um)
print(roughness.summary())
```
