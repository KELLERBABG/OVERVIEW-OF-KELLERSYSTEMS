# Resin Chemistry, Kinetics & Characterization Guide

HoloForge volumetric additive manufacturing relies on non-linear threshold photochemistry. The entire vat volume is illuminated simultaneously by coherent 3D light fields; crosslinking occurs exclusively in spatial regions where the accumulated optical dose exceeds the critical threshold $D_{\text{gel}}$, governed by dissolved oxygen radical inhibition.

---

## 🧪 Stage 2 Formulation: PEGDA / HDDA + 1.0 wt% TPO-L

For Stage 2 physical volumetric solidification, HoloForge standardizes on a high-reactivity, low-viscosity acrylate system:

### 1. Chemical Composition & Sourcing

```
┌────────────────────────────────────────────────────────────────────────┐
│                   Stage 2 Standard Resin Formulation                   │
├──────────────────────────┬──────────────┬──────────────┬───────────────┤
│ Component                │ Chemical     │ Weight (wt%) │ Function      │
├──────────────────────────┼──────────────┼──────────────┼───────────────┤
│ Monomer Option A (Clear) │ PEGDA 250/575│ 98.0 – 99.0% │ Fast diacrylate network, low viscosity, optical clarity │
│ Monomer Option B (Stiff) │ HDDA         │ 98.0 – 99.0% │ High crosslink density, mechanical rigidity, low shrinkage │
│ Photoinitiator (405nm)   │ TPO-L        │ 1.0 wt%      │ Liquid phosphine oxide, rapid radical generation at 405nm │
│ Dissolved Gas Inhibitor  │ Molecular O₂ │ Ambient sat. │ Radical scavenger enforcing sharp non-linear gel threshold │
└──────────────────────────┴──────────────┴──────────────┴───────────────┘
```

* **PEGDA** (Poly(ethylene glycol) diacrylate, $M_n \approx 250\text{–}575\,\text{g/mol}$): Ideal for microfluidic chips and benchmark spheres due to ultra-low scatter ($L_{\text{scatter}} \approx 3200\,\mu\text{m}$) and water-like viscosity ($\sim 0.05\text{–}0.10\,\text{Pa}\cdot\text{s}$).
* **HDDA** (1,6-Hexanediol diacrylate): Ideal for structural prototypes and aspheric lenses requiring high mechanical hardness and low post-cure shrinkage.
* **TPO-L** (Ethyl (2,4,6-trimethylbenzoyl) phenylphosphinate): Optimized for $405\text{ nm}$ absorption ($\epsilon_{405} \approx 620\,\text{L}\cdot\text{mol}^{-1}\text{cm}^{-1}$) with liquid miscibility and high quantum yield ($\Phi_i \approx 0.6$).

---

## 📊 Calibrated Built-In Resin Profiles

HoloForge ships with five calibrated models in `holoforge.config.RESINS`:

| Profile Key | Base Monomer / System | $D_{\text{gel}}$ ($\text{mJ/cm}^2$) | $\alpha$ ($\text{mm}^{-1}$) | $L_{\text{scatter}}$ ($\mu\text{m}$) | Index ($n_{405}$) | Target Application |
|---|---|---|---|---|---|---|
| **`hf-standard-405`** | Acrylate/Urethane hybrid | $12.0$ | $0.35$ | $900.0$ | $1.460$ | General prototyping |
| **`hf-clear-405`** | Low-viscosity aliphatic diacrylate | $10.0$ | $0.20$ | $4000.0$ | $1.470$ | Optics & microfluidics |
| **`hf-dental-405`** | Ceramic-filled dimethacrylate | $18.0$ | $0.60$ | $350.0$ | $1.520$ | Monolithic dental crowns |
| **`hf-pegda-405`** | PEGDA + 1.0 wt% TPO-L | $11.0$ | $0.28$ | $3200.0$ | $1.465$ | **Stage 2 Benchmark Solidification** |
| **`hf-hdda-405`** | HDDA + 1.0 wt% TPO-L | $14.0$ | $0.32$ | $2800.0$ | $1.456$ | **Stage 2 High-Hardness Benchmark** |

---

## 🔬 Reaction Kinetics & Oxygen Threshold Mechanism

```
                       Exposure Intensity I(r)
                                  │
                                  ▼
               Photoinitiator Photolysis (TPO-L + hν)
                                  │
                   ┌──────────────┴──────────────┐
                   │ Free Radicals Generated R•  │
                   └──────────────┬──────────────┘
                                  │
         ┌────────────────────────┴────────────────────────┐
         │                                                 │
         ▼                                                 ▼
[ Reaction with Dissolved O₂ ]                   [ Chain Polymerization ]
(Rate: k_inh ~ 10⁹ M⁻¹s⁻¹)                       (Rate: k_p ~ 10³ M⁻¹s⁻¹)
R• + O₂ ──► ROO• (Inert Peroxy)                 R• + M ──► RM• ──► Polymer Network
      [ INHIBITION REGIME ]                              [ GELATION REGIME ]
  (Accumulated Dose < D_gel)                         (Accumulated Dose ≥ D_gel)
```

1. **Inhibition Stage ($D < D_{\text{gel}}$)**:
   Dissolved oxygen acts as an ultra-fast radical trap ($k_{\text{inh}} \gg k_p$). Polymer chain growth is completely suppressed while $[O_2] > [O_2]_{\text{crit}}$.
2. **Threshold Crossing ($D = D_{\text{gel}}$)**:
   Once the local photon dose depletes dissolved oxygen to critical levels ($[O_2] < 10^{-6}\text{ M}$), radical propagation takes over abruptly, curing the resin into a solid network within $<1\text{ second}$.
3. **High Spatial Contrast**:
   Because background scattered light outside the target never exceeds $D_{\text{gel}}$, it is quenched continuously by ambient oxygen replenishment, preventing stray curing.

---

## 📐 Stage 2 Solidification & Metrology Protocol

```
                        15-Second Solidification Workflow
┌───────────────────────┐       ┌───────────────────────┐       ┌───────────────────────┐
│ 15s Exposure in Vat   │──────►│ Passive Part Release  │──────►│ Optical Profilometry  │
│ 10mm Sphere / Lattice │       │ Gravity Drain (<0.1s) │       │ Ra < 100 nm (Smooth)  │
└───────────────────────┘       └───────────────────────┘       └───────────────────────┘
```

1. **Target Benchmarks**:
   - **10 mm Solid Sphere**: Validates isotropic sphericity, lack of layer lines, and uniform radial gelation in $\le 15\text{ s}$.
   - **$3 \times 3$ Lattice Cube**: Validates hollow internal strut clearance and zero trapped resin clogging in $\le 15\text{ s}$.
2. **Surface Roughness Characterization ($R_a$)**:
   - Measure cured surfaces using white-light interferometry or optical confocal profilometry.
   - **Target Gate**: Surface roughness $R_a < 100\text{ nm}$ ($0.1\,\mu\text{m}$), verifying true optical smoothness vs. the $1\text{–}10\,\mu\text{m}$ roughness of SLA.
3. **Viscosity & Passive Drainage**:
   - Viscosity must remain $<0.4\text{ Pa}\cdot\text{s}$ at $25^\circ\text{C}$ to allow liquid uncured resin to drain freely from internal cavities within seconds without pressurized solvent rinsing.
