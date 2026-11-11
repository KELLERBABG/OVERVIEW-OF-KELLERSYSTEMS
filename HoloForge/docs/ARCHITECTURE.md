# HoloForge System Architecture

This document describes the software and hardware-control architecture of the HoloForge holographic volumetric 3D printing platform.

---

## 1. Module Dependency Graph

```mermaid
flowchart TD
    cli[holoforge.cli] --> optimize[holoforge.optimize]
    cli --> simulate[holoforge.simulate]
    cli --> volume[holoforge.volume]
    cli --> dose_compensate[holoforge.dose_compensate]
    cli --> io[holoforge.io_hfp]
    cli --> render[holoforge.render]
    cli --> calibration[holoforge.calibration]
    cli --> kinetics[holoforge.kinetics]
    cli --> metrology[holoforge.metrology]
    cli --> verticals[holoforge.verticals]
    cli --> player[holoforge.player]
    
    benchmark[holoforge.benchmark] --> optimize
    benchmark --> simulate
    
    dose_compensate --> optimize
    dose_compensate --> simulate
    dose_compensate --> optics[holoforge.optics]
    
    verticals --> volume
    metrology --> config[holoforge.config]
    kinetics --> config
    calibration --> config
    player --> io
    player --> calibration
    
    optimize --> optics
    simulate --> optics
    
    optics --> config
    volume --> config
```

---

## 2. End-to-End Data Flow Pipeline

From 3D digital CAD model to real-time holographic hardware exposure:

```mermaid
flowchart LR
    subgraph S0 [Input Geometry]
        STL[Custom STL] -->|voxelize_stl| Mask[Binary 3D Mask]
        Primitives[Primitives / Verticals] --> Mask
    end

    subgraph S1 [Phase Optimization]
        Mask --> GS[Multi-Plane GS Optimizer]
        GS --> Phase[SLM Phase Patterns]
    end

    subgraph S2 [Simulation & Metrology]
        Phase --> Sim[Split-Step ASM Propagation]
        Sim --> Dose[3D Dose Map]
        Dose --> Cure[Cure Prediction]
        Cure --> Met[Optical Profilometer Metrology]
        Dose -.->|Error Feedback| Comp[Iterative Dose Compensator]
        Comp -.->|Corrected Amplitudes| GS
    end

    subgraph S3 [Hardware Execution]
        Cure --> Job[Job File .hfp + JSON]
        Job --> Bundle[Calibrated SLM Bitmaps + Timing Manifest]
        Bundle --> SLM[Phase-Only LCoS Modulator]
    end
```

---

## 3. Core Functional Layers

### Layer 1: Configuration & Physics Core (`config.py`, `optics.py`)
- `OpticsConfig`: Defines laser wavelength, SLM pixel count ($N$), pixel pitch ($p_{\text{SLM}}$), relay demagnification ($M$), and resin refractive index ($n_{\text{vat}}$).
- `GridConfig`: Defines the 3D computational discretization ($N_x \times N_y \times N_z$) and axial step size ($\Delta z$).
- `ResinModel`: Physical and photochemical parameters ($D_{\text{gel}}$, absorption $\alpha$, scatter mean free path $L_{\text{scatter}}$, refractive index).
- `VatPropagator`: High-performance batched split-step angular spectrum propagator computing forward walks and adjoint backward walks.

### Layer 2: Inverse Solvers & Compensation (`optimize.py`, `dose_compensate.py`)
- `MultiPlaneGS`: Band-decomposed Gerchberg-Saxton algorithm using scale-free least-squares amplitude projections and guard slice out-of-band suppression.
- `DoseCompensator`: Outer feedback loop updating per-voxel target amplitudes to pre-compensate for depth-dependent Beer-Lambert attenuation and forward scattering.

### Layer 3: Photochemical Kinetics & Metrology (`kinetics.py`, `metrology.py`)
- `simulate_kinetics`: Solves coupled differential equations for dissolved oxygen depletion ($[O_2](t)$) and radical photopolymerization conversion ($\chi(t)$) in PEGDA and HDDA.
- `calculate_roughness`: Extracts topography height maps and computes ISO 4287 roughness parameters ($R_a, R_q, R_z$) to verify the $<100\text{ nm}$ optical surface finish.
- `analyze_channel_patency`: Assesses internal microfluidic void clearance and hydraulic diameter.

### Layer 4: Commercial B2B Verticals (`verticals.py`, `volume.py`)
- Generates parametric CAD primitives for digital dental crowns, monolithic microfluidic chips with serpentine channels, and precision even-asphere lenses.
- Ray-parity STL voxelizer and binary/ASCII STL exporter.

### Layer 5: Hardware Driver & Pattern Player (`calibration.py`, `player.py`, `io_hfp.py`)
- `SLMCalibrationLUT`: Phase response Look-Up Table calibration ($[0, 255] \leftrightarrow [0, 2\pi]$) and flat-field beam profile correction.
- `prepare_hardware_bundle`: Generates calibrated 8-bit bitmaps and JSON exposure timing manifests ($t_k = T_{\text{total}} \cdot w_k$).
- `.hfp` container format (compressed NPZ payload + metadata JSON sidecar).
