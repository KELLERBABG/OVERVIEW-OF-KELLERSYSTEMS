# Bill of Materials (BOM) — Stage 1 Optical Alignment Bench

**Hardware Budget Target:** ~€8,000 – €10,000  
**Phase Objective:** Construct an optical alignment bench using lab-grade phase-only spatial light modulation to demonstrate 3D dynamic spot focusing into dye-doped liquid cells, leading directly into Stage 2 volumetric photopolymerization.

---

## 🔬 Component Breakdown & Sourcing Table

| Item | Component / Description | Recommended Model / Sourcing | Key Specification | Estimated Cost (EUR) |
|---|---|---|---|---|
| **1** | **Laser Source** | Thorlabs / Roithner Lasertechnik | $405\text{ nm}$ single-transverse-mode (TEM00), $100\text{–}500\text{ mW}$, CW, narrow linewidth ($<0.5\text{ nm}$) | €600 – €1,200 |
| **2** | **Spatial Light Modulator (SLM)** | Holoeye Pluto-2.1-NIR / Jasper Display JD8554 | Lab-grade phase-only reflective LCoS, $\ge 1920 \times 1080$, $8.0\,\mu\text{m}$ pitch, calibrated $0\text{–}2\pi$ at $405\text{ nm}$ | €5,500 – €6,800 |
| **3** | **Beam Expander & Spatial Filter** | Thorlabs KT310 / BE052-A | $25\,\mu\text{m}$ pinhole, $5\times\text{–}10\times$ variable magnification for flat-top pupil illumination | €400 – €650 |
| **4** | **4f Relay Telecentric Lenses** | Edmund Optics / Thorlabs AC254 series | Achromatic doublets ($f_1 = 200\text{ mm}, f_2 = 50\text{ mm}$ for $M = 0.25\times$ demagnification), AR coating $400\text{–}700\text{ nm}$ | €450 – €800 |
| **5** | **Optomechanical Cage & Rail Rig** | Thorlabs 30 mm / 60 mm Cage System | Rigid cage plates, $\varnothing 1/2''$ and $\varnothing 1''$ precision posts, kinematic mirror mounts (KM100), breadboard base | €800 – €1,200 |
| **6** | **Optical Test Vat & Liquid Cell** | Hellma Analytics / Thorlabs Quartz Cells | Optical-grade fused quartz cuvette/cell with anti-reflective coated windows ($R < 0.2\%$ at $405\text{ nm}$) | €150 – €350 |
| **7** | **In-Situ Metrology & CMOS Camera** | FLIR / Basler / Thorlabs CMOS | USB3 machine vision camera with $405\text{ nm}$ bandpass filter for wavefront & fluorescence PSF imaging | €300 – €600 |
| **8** | **Fluorescent POC Dyes & Solvents** | Sigma-Aldrich | Rhodamine 6G / Sodium Fluorescein in ethanol for 3D focus spot profiling | €100 – €200 |
| **Total** | **Stage 1 Bench Prototype** | **Full Optomechanics + Modulator + Laser** | **Complete static alignment system** | **€8,300 – €11,800** |

---

## 🛠️ Stage 1 Alignment & Proof-of-Concept Workflow

```
[ 405nm Single-Mode Laser ] 
            │
            ▼
[ Spatial Filter (25um Pinhole) & Collimator ] 
            │
            ▼
[ Polarizing Beam Splitter (PBS) ] ──► [ Phase-Only LCoS SLM (Holoeye/Jasper) ]
                                                       │
                                                       ▼
[ Dye-Doped Liquid Cell ] ◄── [ 4f Relay System (M=0.25) ] ◄── [ Modulated Wavefront ]
```

1. **Collimation & Flat-Field Setup**:
   - Align laser beam through the spatial filter to produce a Gaussian TEM00 beam expanded to overfill the active SLM sensor area ($15.36 \times 8.64\text{ mm}$).
2. **SLM Phase Calibration (LUT)**:
   - Perform two-beam interferometry or diffraction grating characterization at $405\text{ nm}$ to establish the linear 8-bit lookup table mapping grayscale values $[0, 255] \to [0, 2\pi]$ phase shift.
3. **Fluorescent Cell Proof-of-Concept**:
   - Fill the quartz cell with low-concentration fluorescein/rhodamine in index-matching liquid ($n \approx 1.46$).
   - Load optimized HoloForge phase holograms to project dynamic 3D focal spots and 2D test slices at variable depths without moving any lenses.
   - Capture cross-sectional slice intensity profiles using the orthogonal CMOS camera to verify peak-to-background contrast ($> 8:1$).

---

## 🚀 Transition from Stage 1 to Stage 2

Once the optical wavefront and focusing peakedness are validated in the dye cell:
1. Swap the liquid dye cell with the **Stage 2 photopolymer resin** (PEGDA / HDDA + 1.0 wt% TPO-L).
2. Execute the 15-second solidification runs on the **10 mm sphere** and **lattice cube**.
3. Measure cured surface roughness ($R_a$) on an optical white-light profilometer.
