# Energy Harvesting via Atmospheric Resonance — Technical Whitepaper

**Version 1.0 — August 2026**

> A concept-and-planning-phase project to prototype a system that passively extracts usable electrical energy from natural atmospheric electromagnetic and thermodynamic phenomena through resonance-based coupling.

---

## Abstract

This project explores the feasibility of harvesting ambient electromagnetic and thermodynamic energy from Earth's atmosphere. By mathematically tuning receiver circuits to ionospheric resonance frequencies, capturing reflected skywave energy, and converting atmospheric pressure/temperature gradients into electrical current, the system aims to produce a continuous, passively-sourced energy stream. The concept bridges **ionospheric physics**, **electrodynamics (Maxwell's equations)**, **signal processing**, **piezoelectric transduction**, **thermodynamics**, and **real-time embedded control systems**. The document is currently in **Concept & Planning Phase** (v1.0, 2026-07-27) with a 2-year, ~€30,000 prototype roadmap and a hardware bill of materials spanning six categories (~€22k–55k total).

---

## 1. Core Physics Principles

| Principle | Description |
|---|---|
| **Ionospheric Plasma Resonance** | The ionosphere is a conductive plasma layer at ~60–1000 km. A ~250–300 kV potential difference exists between ionosphere and Earth's surface (the "global electric circuit"). Schumann resonances (7.83 Hz fundamental; 14.3, 20.8, 27.3, 33.8 Hz harmonics) and ionospheric critical frequency (f₀F2: 2–15 MHz) enable resonant coupling. |
| **NVIS Propagation** | Near-Vertical Incidence Skywave (70–90° elevation, 2–10 MHz) reflects energy straight back down — a planetary-scale transformer / closed inductive path with Earth. |
| **Maxwellian Matching** | Real-time solution of ∇×E = −∂B/∂t and ∇×H = J + ∂D/∂t computes the time-varying complex atmospheric impedance for continuous impedance matching (maximum power transfer). |
| **Piezoelectric Transduction** | Barometric pressure waves (infrasound 0.001–20 Hz; microbaroms ~0.2 Hz) deform PZT crystals → charge via the direct piezoelectric effect. Energy density: ~0.01–1 µW/m². |
| **Cyclostationary Processing** | Atmospheric EM noise has periodic statistics: solar cycle (~11 yr), diurnal ionospheric variation, ~44,000 thunderstorms/day, Schumann modes. Cyclostationary filters separate periodic components from stochastic noise. |
| **Thermodynamic Gradient** | Tropospheric lapse rate ~6.5°C/km. TEGs (Bi₂Te₃, ZT ≈ 1) exploit the Seebeck effect across temperature differentials (V = S·ΔT, S ≈ 200 µV/K). |

## 2. System Architecture (10 Subsystems)

```
Atmospheric sources (ionosphere 60–1000 km, troposphere 0–11 km)
  │  ┌───────────┐ ┌───────────┐ ┌────────────┐ ┌────────────┐
  │  │ NVIS      │ │ Ionos-    │ │ Acoustic   │ │ Thermo-    │
  │  │ Antenna   │ │ pheric    │ │ Piezo Grid │ │ dynamic    │
  │  │ Array     │ │ Probe     │ │ (Infra-    │ │ TEG Array  │
  │  │ (1–10 MHz)│ │ (ELF/VLF) │ │ sound)     │ │ (Seebeck)  │
  │  └────┬──────┘ └────┬──────┘ └─────┬──────┘ └─────┬──────┘
  │       └─────────────┼──────────────┼──────────────┘
  │                     ▼              ▼
  │   Real-Time Maxwellian Field Solver (FPGA + ARM/RISC-V)
  │   Adaptive Impedance Matching Network (L/C + varactor array)
  │   Cyclostationary Feature Filtering (FFT → CAF → PSD)
  │   Non-Linear Wave Modulation Rectifier (Schottky + resonant tank)
  │   Plasma-Wake Energy Buffer (supercap bank + LiFePO₄)
  │   Resonant Substrate RTOS Kernel (deterministic, jitter-free)
  │   Static Invariant Discharge (safety: GDT + MOV + crowbar)
  └──▶ DC Power Output (regulated bus)
```

### Subsystem summary
1. **Ionospheric Plasma Coupling** — ELF/VLF magnetic loop + digital PLL locked to 7.83 Hz; correlated with TEC data; expected harvest μW/m².
2. **NVIS Skywave Inductor** — horizontal dipole array + phased beamformer; closed-loop resonant cavity; ~3–6 dB round-trip path loss.
3. **Maxwellian Field Relaxation** — FPGA-accelerated FDTD (Yee grid) computing Z(ω,t) = E/H; PID-driven matching network.
4. **Acoustic-Atmospheric Transduction** — PZT-5H tile array + ultra-low-noise charge amps + Helmholtz resonators; Goertzel detection of microbaroms.
5. **Non-Linear Wave Modulation** — broadband (30 MHz–3 GHz) antenna + LNA + zero-bias Schottky detector array + NLTL pulse compressor; stochastic resonance algorithm.
6. **Cyclostationary Feature Filtering** — FFT Accumulation Method (FAM) for cyclic autocorrelation; adaptive notch bank; ML classifier; GPSDO (<100 ns phase).
7. **Plasma-Wake Energy Buffering** — SiC diode surge front-end, supercap bank (1000–3000 F), LiFePO₄ 48V/100Ah, <100 ns transient detection.
8. **Resonant Substrate Adaptation** — RTOS (FreeRTOS tickless/RTEMS) + bare-metal PLL on STM32H7/PRU; **jitter < 1 ns RMS** target; WCET analysis (aiT/OTAWA).
9. **Thermodynamic Capacity Gradient** — Bi₂Te₃ TEG array + heat pipes + DC-DC (LTC3108/BQ25504, 20 mV startup); MPPT perturb & observe.
10. **Static Invariant Discharge** — multi-stage protection: GDT → MOV → TVS → crowbar → HV relay; IEC 62305 / IEC 60364 compliance; independent watchdog MCU (fail-open); <100 ns FPGA threshold response.

## 3. Key Algorithms & Real-Time Requirements

| Algorithm | Implementation | Platform | Timing |
|---|---|---|---|
| FDTD Maxwell Solver | Yee grid, 2nd-order central differences | FPGA (Artix-7) | < 1 ms/update |
| Schumann PLL | 2nd-order digital + Costas | STM32H7 | lock < 10 s |
| Cyclic Autocorrelation (FAM) | O(N log N) | GPU (Jetson Orin) | < 5 min per 1-h block |
| Impedance Matching | Gradient descent on \|Γ\| | MCU | 10 Hz loop |
| MPPT (TEG) | Perturb & observe | MCU | converge < 60 s |
| Transient Detection | Threshold comparator | FPGA | < 100 ns |
| Safety Shutdown | Redundant voting, fail-open | Independent MCU | < 1 ms |

## 4. Hardware Budget (€)

| Category | Low | Mid | High |
|---|---|---|---|
| RF & Antenna | 2,390 | 2,390 | 5,000 |
| Sensors & Transducers | 7,500* | 13,500 | 25,000 |
| Computing & Control | 3,015 | 3,015 | 8,000 |
| Power Electronics | 3,960 | 3,960 | 7,500 |
| Safety & Protection | 2,330 | 2,330 | 5,000 |
| Infrastructure | 3,200 | 3,200 | 5,000 |
| **Total** | **~22,395** | **~28,395** | **~55,500** |

*Low-end assumes custom search-coil magnetometers. **Recommended initial prototype budget: 25,000–30,000 €.**

## 5. Safety & Compliance

- **Galvanic isolation > 10 kV** (IEC 60950/62368) for HV front-end; single-point ground < 5 Ω (IEEE 80).
- **Lightning:** air-terminal 45° cone; GDT → MOV → TVS cascade on every feedline; grounded bulkhead (IEC 62305-4).
- **EMI/EMC:** FCC Part 15 / CISPR 22 Class B; shielded enclosures (40 dB, 30 MHz–3 GHz); twisted shielded pair.
- **Environmental:** PZT lead sealing (WEEE/RoHS); LiFePO₄ fireproof enclosure w/ thermal runaway detection; supercap short-circuit protection (>1000 A capability).
- **Electrical:** arc fault detection on buses > 48V DC (IEC 60364).

## 6. Implementation Roadmap (2 Years, ~€30k)

| Phase | Months | Budget | Gate Review |
|---|---|---|---|
| 1: Research & Simulation | 1–6 | ~€3,000 | Energy density threshold: > 1 mW/m²? |
| 2: Benchtop Prototype | 7–12 | ~€10,000 | Sustained > 10 mW from any single source? |
| 3: Field Prototype | 13–18 | ~€12,000 | Sustained > 1 W total? |
| 4: Iteration & Publication | 19–24 | ~€5,000 | Closed-source release + arXiv/IEEE Access preprint |

## 7. Risk Register (Top)

| Risk | Probability | Impact | Mitigation |
|---|---|---|---|
| Energy density < 1 mW/m² (impractical) | High | Critical | Phase-1 simulation gate; pivot to hybrid solar+atmospheric |
| Lightning destroys outdoor gear | Medium | High | Multi-stage surge protection; sacrificial front-end; insurance |
| FPGA Maxwell solver too complex | Medium | Medium | Start 1D; GPU fallback for PoC |
| TEG ΔT insufficient (< 1 mW/module) | Medium | Medium | Heat-pipe enhancement; larger thermal mass |
| Real-time jitter prevents resonance lock | Medium | High | Bare-metal PLL on dedicated core; GPSDO; WCET analysis |

## 8. Honest Assessment

This is an **exploratory research project** (Concept & Planning Phase). The physics is real (Earth-ionosphere cavity, Schumann resonances, Seebeck effect), but the *harvestable energy density at ambient levels is the fundamental open question* — the Phase-1 gate review (simulation must show > 1 mW/m²) is a deliberate kill-switch if the numbers don't support feasibility. Lightning safety and multi-stage surge protection are non-negotiable hardware requirements, and atmospheric/rf regulatory compliance must be confirmed before any transmission-based probing. The project documents explicit data sources (IRI2016, TEC maps, Blitzortung, NOAA SWPC, DWD) and references (Schumann 1952, Yee 1966, Gardner 1994, Snyder & Toberer 2008) for reproducibility.