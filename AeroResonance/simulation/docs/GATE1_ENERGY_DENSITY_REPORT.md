# Gate-1 Energy-Density Report — Energy Harvesting via Atmospheric Resonance

**Status:** PRE-Gate-1 — the Phase-1 gate threshold is **NOT met** on current physics.
**Date:** 2026-08-13 (simulation) / plan Phase-1 gate criteria.
**Gate criterion (Project_Plan §7):** theoretical energy density > **1 mW/m²**.

---

## 1. Summary verdict

| Harvester | Avg density [mW/m²] * | Gate? |
|-----------|----------------------|-------|
| Ionospheric Plasma Coupling | ≈ 0.0000 (sub-µW) | **FAIL** |
| Near-Vertical Skywave Inductor (NVIS) | ≈ 0.0000 (sub-µW) | **FAIL** |
| Maxwellian Field Relaxation | ≈ 0.0001 | **FAIL** |
| Acoustic-Atmospheric Transduction | ≈ 0.0000 (infrasound) | **FAIL** |
| Non-Linear Wave Modulation | ≈ 0.0000 | **FAIL** |
| Cyclostationary Feature Filtering | ≈ 0.0000 | **FAIL** |
| Plasma-Wake Buffering | storage (n/a directly) | FAIL |
| Resonant Substrate Adaptation | negative (control sink) | n/a (sink) |
| Thermodynamic Capacity Gradient (TEG) | ≈ 0.0016 | **FAIL** (borderline) |
| Static Invariant Discharge | negative (safety sink) | n/a (sink) |
| **Solar Photovoltaic (microcell)** | ≈ 970 (mu-cell) | **PASS** — but this is a plain PV cell, not an atmospheric-resonance harvester |

> \* Density = simulated average power ÷ nominal aperture. Values are **order-of-magnitude screening estimates**, not measured data.

**Net:** all *resonance-based* harvesters fail the 1 mW/m² gate. The only PASS comes from a conventional photovoltaic cell, which is outside the atmospheric-resonance concept.

---

## 2. Method & assumptions

- Reproduced by `cargo run -- --gate-report` over a representative run.
- Apertures per subsystem are nominal stand-ins (loop antenna footprint, piezo tile array, TEG footprint, PV micro-cell area); exact surface areas are design-dependent.
- Formula per subsystem: `density = avg_power / nominal_aperture` (W → mW).
- The gate report is a **screening** tool; the plan's own numbers (below) are the reference physics.

## 3. Cross-check against the plan's own physics figures

The plan and readiness report already anticipate this result (§3.7, §4.4):

- **Infrasound / acoustic:** plan §4.4 states energy density ~0.01–1 µW/m² — three orders of magnitude below the 1 mW/m² gate.
- **TEG:** plan §4.9 + report note a localized ΔT ≈ 0.2 °C at a 30 m tower — too low for meaningful power; the model "solves" this only via an **UNVALIDATED** ground-coupling assumption (see §4).
- **Schumann/ionosphere:** the 250–300 kV potential and ~2 pA/m² fair-weather current give tiny extractable power densities.

## 4. The unvalidated assumption (now made explicit & configurable)

Previously the code hardcoded `dt.max(2.5)` — a "guaranteed minimum 2.5 °C" TEG ΔT from ground inertia. This had no citation and masked the borderline TEG figure.

**Resolution:** the assumption is now a config knob `teg_min_dt_degc` (default 2.5 °C) with a doc comment marking it **UNVALIDATED**, pending field measurement. The gate report prints the borderline TEG result so the assumption's sensitivity is visible.

## 5. Recommended next action (risk R1)

- Gate is **NOT met**. Per project plan risk R1 mitigation, the decision is: **pivot to a hybrid solar + atmospheric design**, or **close the concept** and record it as a negative/educational result.
- Before any Phase-2 hardware spend (~10 k€) the physics gate must be revisited with validated inputs (M1.3 local spectrum campaign, M1.4 thermal model).
- Do **not** proceed to benchtop/field prototype on current physics numbers.
