# MuonScope Tomography — Hardware Product-Readiness Assessment

**Date:** 2026-10-07
**Scope:** full working tree at `8cbe643` plus uncommitted changes
**Method:** read every source, firmware, configuration and documentation file; re-ran the Python and
Rust test suites; re-ran the accuracy harness under a fixed seed to reproduce the reported numbers;
re-derived the mass, energy, carry-chain and certification arithmetic from the documents' own
figures. Claims marked "verified" below were reproduced independently in this assessment, not
relayed from the repository's own summaries.

---

## 1. Verdict

**MuonScope is a strong simulation and inversion research project that has been described, in its
own documents and on its public website, as a finished instrument. No part of the instrument
exists.**

Four independent audits — firmware, hardware engineering, software stack, and commercial/IP/
regulatory — converge on the same picture:

| Subsystem | Exists as | Readiness (0–10) |
|---|---|---|
| Simulation & tomographic inversion (Python) | Working, tested, honestly documented | **6 / 10** |
| Edge DAQ & tracking numerics (Rust) | Working library, no hardware I/O | **4 / 10** |
| Digital-twin UI (React/Three.js) | Working, but simulates its own data | **4 / 10** |
| FPGA firmware (SystemVerilog) | Stubs; the TDC is uninstantiated dead code | **1 / 10** |
| Analog/electrical hardware | Two prose specification files. No schematic. | **1.5 / 10** |
| Mechanical hardware | One prose specification file. No CAD. | **1 / 10** |
| System integration | Never attempted | **0 / 10** |

**Overall: TRL 2–3.** Critical function is proven *in simulation* only. There is no component, no
benchtop demonstration, no detector in the loop, and no engineering design artifact of any kind.

The gap to a hardware launch is **not "finish the code."** It is that roughly 80 % of what a
hardware product consists of — analog front end, PCB, mechanics, calibration, certification,
manufacturing — has not been started, and the specific architecture described in the documents
cannot be built as written (§4.5).

Two findings matter more than the rest. First, the **survey-time budget is wrong by one to three
orders of magnitude** (§4.6): the rate model was taken from a benchtop 150–600 mm hodoscope and then
applied to targets that must sit metres away between panels, where the solid angle falls as 1/*D*².
The commercial thesis — a bridge pier in 6–8 hours, a dam in 24–30 hours — is not achievable with
this architecture. Second, the **live public website at `muon.kellersystems.dev` publishes a
performance datasheet and a "v1.0" version number for a device that has never detected anything**
(§4.8). Both were confirmed directly in this assessment.

The single most important immediate action is to correct the public claims — not because honesty is
a virtue in the abstract, but because `docs/ACCURACY_MEASURED.md` already disproves them, so any
competent reviewer who benchmarks the code will find them.

---

## 2. What is actually in the repository

Measured line counts (excluding blank/comment noise is not attempted; these are file totals):

| Component | Lines | Files | State |
|---|---|---|---|
| Python inversion + physics | 1 714 | 29 | Real numerics, real physics constants |
| Rust edge DAQ | 773 | 15 | Real numerics, **no I/O** |
| Client (TS/TSX/WGSL/CSS) | 1 783 | 8 | Real UI, partly hardcoded fixtures |
| Benchmarks + validation | 698 | 4 | Genuinely useful, honest |
| FastAPI server | 228 | 1 | Real computation, minor unit bugs |
| **FPGA SystemVerilog + XDC** | **399** | **7** | **Stub; TDC never instantiated** |
| **MCU Rust** | **67** | **2** | **Touches no hardware** |
| Markdown documents | 2 519 | 24 | 13 of them describe hardware/business |
| `index.html` (public site) | 436 | 1 | Presents unbuilt specs as fact |

**There are more lines of prose describing the hardware (≈ 2 955) than lines of firmware for it
(466) — a ratio of 6.3 : 1.** There are **zero** files of any of these types anywhere in the
repository, including in git history: `.kicad_sch`, `.kicad_pcb`, `.sch`, `.brd`, `.gbr`,
`.step`, `.stp`, `.iges`, `.f3d`, `.sldprt`, `.dwg`, `.dxf`, `.xlsx`, `.csv`, `.pdf`.

Git history is 5 commits by one author between 2026-09-01 and 2026-10-07.

### The two-projects problem

The repository contains two projects pulling in opposite directions:

**Project A — an honest research tool.** `docs/ACCURACY_MEASURED.md` opens by stating that it
"supersedes the accuracy figures in `SYSTEM_ACCURACY_AND_GAP_ANALYSIS.md` and `VALIDATION_REPORT.md`,
which state CNR values of 27.60 and 5.92 that no code in this repository computes."
`tests/validation_scripts/accuracy_gate.py` exists solely to catch a units bug the project found in
itself. `normalized.py` and `map_em.py` carry docstrings that document previously-fixed errors
("the old form ... diverged ~25x per iteration"). This is exemplary scientific practice.

**Project B — a marketing narrative.** `README.md` marks *every* Phase 0–5 deliverable `[x]`,
including `5.3 Laboratory Demonstration: Physical scan of a lead target embedded inside a concrete
block` and `5.4 Field Pilot Deployments: Real-world testing on historical masonry, tunnels, and
geotechnical sites`. Neither happened. No artifact of either exists.

Project A is the asset. Project B is the liability, and it must be removed first (§8, Stage 0).

---

## 3. What is genuinely good — do not lose this

This is the part worth protecting, and it is more substantial than the rest of the repository
implies.

**The physics constants are correct.** Verified by reading the source:

- Highland formula in `src/inversion/muonscope/physics/scattering.py:31` uses **13.6 MeV** and
  β = p/E with m_μ = 105.6583755 MeV — correct.
- Bethe-Bloch in `energy_loss.py` uses K = 0.307075, a correct W_max with s = m_e/M, the plasma
  energy ħω_p = 28.816 eV·√(ρZ/A), and the Sternheimer high-energy density correction.
- Muon lifetimes in `capture.py`: free lifetime **2196.98 ns** (2.197 µs) and capture lifetimes
  Al 864, Fe 206, Pb 75.4 ns — literature values.
- The Gaisser parameterisation is the canonical form.

**The reconstructors are real algorithms, not placeholders.**

- `siddon.py` implements a genuine Siddon ray-voxel intersection with per-axis boundary-plane
  α collection and cm chord lengths.
- `map_em.py` implements a genuine MLEM/MAP-EM step,
  `λ ← λ·(Wᵀ(y/Wλ)) / (sensitivity − β·prior_grad)`, with the prior in the denominator — the correct
  Green (1990) one-step-late MAP-EM form.
- `normalized.py` is a real closed-form normal-equation solve and makes a fair baseline.

**The tests actually run and pass.** Reproduced in this assessment:

- `pytest src/inversion/tests` → **17 passed** in 5.64 s.
- `cargo test` on `src/daq` → **8 passed** (built with the GNU target; the repo's default MSVC
  target cannot link on this machine because the Windows SDK is absent — an environment issue, not
  a code defect).

**The accuracy harness is honest and reproducible.** Re-run under the documented seed:

```
py benchmarks/accuracy_harness.py --events 50000 --grid 16 --seed 12345 --beta 0.3
```

reproduced the committed numbers exactly: lead CNR **PoCA 0.36 / path-normalised 3.41 /
MAP-EM 1.13**; RMSE MAP-EM 0.471 versus PoCA 34.87; acceptance 19.8 %. A harness that survives
independent re-execution is worth more than any number of green checkmarks.

**`accuracy_gate.py` is the single most valuable file in the repository.** It exists because the
project discovered that CNR is scale-invariant, so its own audit had been reporting PASS on a
reconstruction that was ~8 000× wrong. It says so plainly in its own docstring. It then reports
"Accuracy is gated; contrast is not yet met." That is the voice of a real engineer.

---

## 4. What is wrong — the honest defect list

These are not "unfinished items." They are things that are stated as complete and are not.

### 4.1 The published accuracy figures are false, and the repository says so

| Claim | Source | Measured |
|---|---|---|
| Lead CNR 27.60 | `SYSTEM_ACCURACY_AND_GAP_ANALYSIS.md:21` | **3.41** best case |
| Lead CNR 5.92, iron 4.88, void 4.75 | `VALIDATION_REPORT.md:44–47` | **1.19 / 0.73 / 0.51** (MAP-EM) |
| Ground-truth lead λ = 0.428 cm⁻¹ | `VALIDATION_REPORT.md:44` | **1.7857 cm⁻¹** (4.2× off) |
| MTF 0.72 at 0.05 lp/mm exceeding ISO 0.5 | `VALIDATION_REPORT.md:71` | **no MTF code exists** |
| ESF 10–90 % = 6.2 mm | `VALIDATION_REPORT.md:73` | harness reports `null`; below the 7.81 mm voxel size |
| "Measured gain variation ≤ 0.18 % across full environmental chamber cycling" | `VALIDATION_REPORT.md:103` | no chamber, no data |
| Acceptance 23.8–28.8 % | both reports | **19.8 %** reproduced |

The ASTM E1441 threshold the documents cite is lead CNR ≥ 4.5. **The best measured value is 3.41.**
One further correction, in the project's favour: **ASTM E1441 does not contain that threshold.**
It is the Standard *Guide* for Computed Tomography (CT) Imaging — a guide, with no CNR acceptance
criteria (`VALIDATION_REPORT.md:42–47` and `BUILD_PLAN.md:562` attribute the numbers to it
incorrectly). So the project is not failing a real standard; it is failing a target it invented and
misattributed. That is a smaller failure and an easier fix — but it also means the project currently
has **no external benchmark at all**, which is the thing a funder or a customer will actually ask
for.

### 4.2 Four documents check off work that does not exist

`README.md:322–333` marks as complete: a sub-75 ps carry-chain TDL TDC; a TinyML shower classifier;
an MCU closed-loop thermal PID; a WebGPU volume raymarcher; a physical laboratory scan; and field
pilot deployments. Meanwhile:

- `tdc_tdl_carry.sv` is **never instantiated by anything** — grep for its name returns only its own
  module declaration and its internal signal. `carry_chain_out` (line 32) is declared and read
  (line 42) but **never assigned**. There is **no `CARRY4`, no `MUXCY`, no `generate`, no `genvar`**
  anywhere in the RTL. The module would synthesise to a constant.
- There is **no TinyML model, no lookup table, no MAC** — the file is four popcounts and a hardcoded
  band-pass.
- The MCU "PID" contains **no P, I or D term**. It is a linear temperature-to-DAC-code map. It reads
  a hardcoded `measured_temp_c = 24.5` (`firmware/mcu/src/main.rs:21`) and discards its own output
  (`let _ = dac_code;`).
- `volume_raymarch.wgsl` is a real 67-line WebGPU shader that **no file imports**.
- `data/sample_scans/` and `data/calibration_procedures/` contain **only a README** — the datasets
  and procedures those READMEs list do not exist.

The project's *own* tracker contradicts the README: `BUILD_PLAN.md:432–434` still lists T19/T20/T21
as `[ ] Pending`.

### 4.3 Exports are stubs that the audit reports as PASS

Verified byte-level in this assessment:

- **`*.dcm` files are not DICOM.** `slice_0000.dcm` is 512 bytes, begins `87 13 00 00 CE 08 18 02`,
  and has no `DICM` magic at offset 128. They are raw `uint16` arrays renamed with a `.dcm`
  extension (`dicom_export.py:22`). `pydicom` is declared as a dependency and never imported.
- **The "Marching Cubes isosurface mesh" is a unit cube.** Both `isosurface.obj` and
  `reconstruction_mesh.obj` contain exactly **8 vertices and 12 faces**. `mesh_vtk.py` silently falls
  back to a hardcoded cube when `skimage` is missing, and `skimage` is in no dependency list. The
  test suite blesses it with `len(verts) > 0`.
- **The IFC export contains no geometry** — two comment lines and dangling references
  (`bim_ifc.py:58–83`), while the gap analysis and the UI claim `IfcVoidingFeature` and
  `IfcReinforcingBar` entities.
- **`automated_system_audit.py` prints its PASS strings unconditionally.** Line 53 prints
  `[PASS] Lead Target CNR: {value} (ASTM Standard Target >= 4.5)` without comparing the value to
  4.5; the only numeric gate is `cnr_lead > 3.0`, and CNR is scale-invariant. The script ends by
  printing `100% OPERATIONAL`. `ACCURACY_MEASURED.md:75` documents this about its own audit.

### 4.4 There is no detector anywhere in the loop, and the headline accuracy needs an oracle

- **No serial, USB, socket or file ingestion path exists** in any language. `src/daq/Cargo.toml`
  declares `tokio` and `rayon`; grep finds **zero references** to either in any source file.
- The documented binary wire format is decoded by `packet_decoder.rs`, and its **only producer is
  `create_synthetic_raw_packet` in the same crate** — a function written to feed the decoder. The
  RTL emits none of it: `muonscope_top.sv:92` is literally `spi_miso <= spi_mosi;` — a wire
  loopback. The sync word `0xAA55AA55` appears in the docs and the Rust, and **nowhere in the
  firmware**.
- `run_pipeline.py:76,97` and `accuracy_harness.py` feed the reconstructors the Monte-Carlo
  **true per-event muon momentum** (`ev.energy_initial_gev * 1000.0`). That is the one quantity a
  two-panel hodoscope cannot measure, and it enters the θ² weighting with up to a 10⁴ lever arm.
  The code comment admits it; the harness itself states the θ² spread factor is 26 719×.
- Nothing converts detector hits into the `(p_in, d_in, p_out, d_out, θ², p)` tuple the
  reconstructors consume. `detector_sim.py` — which does model TDC binning, SiPM PDE, dark counts
  and position smearing — **is imported by no entry point**. The inversion never sees a smeared,
  noisy, quantised measurement.
- The accuracy corrections are **uncommitted**. `git show HEAD:map_em.py` reveals the committed
  MAP-EM was *not an EM algorithm* (`ratio = y / pred_var**2`, `denom = Wᵀ(1/pred_var) + β·grad`).
  The correct version exists only in the working tree.

### 4.5 There is no hardware design, and the specification is self-contradictory

`hardware/` contains exactly **two files, 10 442 bytes**: one AFE specification and one mechanical
specification. Both are prose and ASCII art.

**The AFE cannot work as described.** `AFE_SCHEMATIC_SPEC.md:56–63` places a 50 Ω pulldown directly
on the SiPM anode in parallel with the TIA's virtual ground, shunting the ~1.4 pC / ~100 ps pulse
away from the integrator. The inverting input has no DC bias path. The pole-zero network
(100 pF / 500 Ω = 50 ns) is 10× the 500 ps flight time the product exists to measure. And at 1.5 kΩ
transimpedance a single photoelectron produces ≈ 0.4 mV at a comparator whose own hysteresis is
2–7 mV — so the specified "0.5 p.e. threshold" is unreachable with the specified parts.

**Channel count is unreconciled three ways.** The BOM says 128 SiPMs; `config/detector_geometry.yaml`
defines **8 planes × 64 strips = 512**; `muonscope_top.sv:15–22` declares **256 differential LVDS
pairs**; the commercial plan says 256. Four documents, three answers.

**Four different FPGA part numbers** appear across four files (XC7A35T, XC7A50T, Lattice ECP5-85F,
Intel Cyclone 10 GX).

**The TDC does not fit in the FPGA — by 20×.** `tdc_tdl_carry.sv:10` specifies 128 taps. A `CARRY4`
provides 4 taps, so one channel needs 32; 128 channels need **4 096 `CARRY4`**. An XC7A35T has
approximately **200**. This is a shortfall of **20.5×**, and even the largest Artix-7 is ~5.5× short.
The fabric does not contain the delay lines. The 128 sampling registers alone would consume
> 32 000 flip-flops against the device's 41 600.

**Three specifications are mutually exclusive.** Verified arithmetic:

| Spec | Claim | Reality |
|---|---|---|
| Endurance | "> 24 h" at 12.5 W on 153.6 Wh | 153.6 / 12.5 = **12.3 h**; ~11 h realistically. **2.0× over.** |
| Mass | "< 9.5 kg per station" | The mass breakdown omits the lead absorber entirely: one 500×500×5 mm plate is **14.2 kg**, and the geometry config calls for two → **28.4 kg**. |
| Mass (strips) | "3.4 kg strips & fibers" | At the configured 20 mm × 10 mm × 500 mm PVT geometry a strip is ~52 g, so the claimed 3.4 kg implies **~65 strips** — not the 128, 256 or 512 the other documents specify. |

The 24 h and the 9.5 kg claims cannot both be true: 24 h at 12.5 W needs ~300 Wh, which at
~10 Wh/kg is a ~3.5 kg pack, which alone breaks the weight budget. **The headline endurance claim
and the headline weight claim are mutually exclusive by a factor of two.**

**Nothing about certification is costed.** `REGULATORY_COMPLIANCE.md` is 67 lines of standard names
with no test house, no quote, no schedule. The device claims Wi-Fi 6, BLE 5.3, LoRaWAN and 4G/LTE
radios — three transmitters requiring intentional-radiator type approval — but **no LoRa radio, no
4G modem and no sub-GHz transceiver exists anywhere in the BOM** (the `ESP32-C6` is not a LoRa
part). A credible multi-region radio approval programme is **$60–150 k and 6–12 months**; the
budgeted figure is zero. UN38.3 is asserted with tests T.1–T.7 (the standard requires T.8), and the
certification is voided by integrating a different BMS anyway. The document claims "no heavy metal
hazardous substances" for a device containing 14.2 kg of pure lead, and promises an "ISO 9712
Calibration Certificate" — ISO 9712 certifies *personnel*, not equipment.

**The BOM is priced with fiction.** The OPA855 at $1.25 is a ~$6–9 part; the u-blox ZED-F9P at $65
is ~$150–190; 64 GB industrial eMMC at $6.20 is $20–40; a bespoke autoclave-cured carbon monocoque
half at $45 does not exist at any quantity given the tooling; a metrology-grade reference phantom at
$240 is a $2 000–10 000 item. No lead times, MOQs, lifecycle status or vendor quotes appear.

### 4.6 The survey-time budget is wrong by one to three orders of magnitude

This is the most commercially consequential error in the project, because survey time is the number
that decides whether muon tomography is viable for a given job.

The documents quote **15–25 usable tracks/s** (`BUILD_PLAN.md:579`, `WHITEPAPER.md:182`) and derive
survey times from it: 6–8 hours for a bridge pier, 24–30 hours for a dam core, 72–96 hours for deep
overburden. That rate is only attainable in a compact geometry. The project's own numbers contradict
it immediately: a 0.25 m² aperture sees 41.7 muons/s, and the project's own measured acceptance is
**19.8–28.8 %**, giving a ceiling of **8.3–12.0 tracks/s** — *below* the claimed floor of 15/s.

More seriously, the rate collapses with standoff. A target 0.5–15 m thick cannot sit inside a
150 mm panel gap; the panels must straddle the object at separation *D*, and the solid angle falls
as 1/*D*². Calibrating to the project's own 600 mm acceptance test (verified here):

| Panel separation *D* | Usable rate | Claimed application | Required tracks | Actual time | Documented claim |
|---|---|---|---|---|---|
| 0.6 m | 12.0 /s | lab / compact | — | — | — |
| 1 m | 4.3 /s | rebar & honeycomb | 150 k | 9.7 h | 2.0–2.5 h |
| 2 m | 1.08 /s | bridge pier | 500 k | **5.4 days** | 6.0–8.0 h |
| 3 m | 0.48 /s | bridge pier | 500 k | **12 days** | 6.0–8.0 h |
| 5 m | 0.17 /s | dam core | 2 M | **134 days** | 24–30 h |
| 8 m | 0.068 /s | dam core | 2 M | **343 days** | 24–30 h |

The pier case is off by ~20–35×; the dam case by ~130–340×. The project's headline commercial claim
— "*deploy 4-station SAM-T arrays to compress scan times to < 40 minutes*"
(`RISK_REGISTER.md:68`) — is off by three to four orders of magnitude under the same arithmetic.
**A 4-station mesh does not rescue the numbers**: each station sees its own low-coincidence-rate
projection, so four stations buy at most ~4× in total rate (and less in effective angular coverage),
not the ~40× the plan implies.

This does not mean muon tomography of large structures is impossible; real systems do it. It means
they do it with **square-metre-class apertures and multi-day-to-multi-week exposures**, and they
sell *monitoring over time*, not a half-day survey. The stated product thesis is not achievable with
a 0.25 m² detector at metre standoff, and no document in the repository notices.

### 4.7 The angular resolution is 3.4× worse than published

The error budget (`BUILD_PLAN.md:618–629`, `README.md:580`, `WHITEPAPER.md:197–198`) reports
σ_θ ≤ 2.1 mrad, built from a position term of 1.77 mrad. Verified here:

```
sqrt(2) * 0.75 mm / 600 mm  = 1.77 mrad   <- what the budget uses
sqrt(2) * 0.75 mm / 450 mm  = 2.36 mrad
sqrt(2) * 0.75 mm / 150 mm  = 7.07 mrad   <- the documented baseline plane spacing
combined with MS/jitter/IMU = 7.18 mrad
baseline needed for 2.1 mrad at 0.75 mm = 505 mm
```

The published 1.77 mrad term silently assumes a **600 mm** lever arm, but the documented baseline
tracker spacing is **Δz = 150 mm** (`README.md:105`, `FIELD_OPERATIONS.md:13`, and the BOM's
`MEC-03 AL6061-SPACER-150`). `README.md:666` states plainly: "Baseline Δz = 150 mm fixed; extendable
to 450 mm." At the documented baseline the angular resolution is **7.2 mrad, not 2.1 mrad** — a
3.4× degradation, and it propagates directly into every spatial-resolution claim in §5 of the
README and the website's specification table.

### 4.8 The public website presents an unbuilt device as a product

`index.html` is deployed via `CNAME` → **muon.kellersystems.dev**. I fetched it during this
assessment: **HTTP 200, live, byte-equivalent to `index.html`.** Its specification table
(lines 392–404) states, in the present tense:

> Timing **< 100 ps** · Resolution **10 mm @ 1 m** · Power **12.5 W · Over 24 h on 150 Wh** ·
> Mass **< 9.5 kg per station** · Positioning **RTK-GNSS < 1 cm**

and the body text asserts "Two panels, 9.5 kilograms each, carried to the structure and set either
side of it", "**12 mm** steel rebar resolved", "**30 mm** internal void resolved",
"MAP-EM 256³ voxels, 1.8 s", and a footer reading "μ-Scope **v1.0**". There is **no disclaimer
anywhere on the page** that this is a design concept rather than a built, tested device.

Several of these are arithmetically or physically impossible as written: the mass row omits 28.4 kg
of lead (§4.5), "over 24 h on 150 Wh" at 12.5 W is 12.3 h by the page's own numbers, the angular
resolution is 3.4× optimistic (§4.7), and the survey times are off by 1–3 orders of magnitude
(§4.6). The page also contradicts itself: 9.5 kg "per station" in the table versus "9.5 kilograms
each" for two panels in the prose.

The site is otherwise well-made and does include honest caveats ("both remain open engineering
items"). The problem is not tone — it is that a **live public product page asserts measured
specifications for hardware that does not exist**, in present tense, with a version number. That is
the largest single credibility and legal exposure in the project, and it is a five-minute fix.

### 4.9 Commercial, IP and regulatory claims are unsupported

- **No market analysis exists.** Grep finds no TAM/SAM/SOM anywhere. The headline
  "$2.5 Trillion in aging global concrete bridges" (`INVESTOR_PITCH.md:7`) is asset *replacement
  value*, not an addressable market. There are **zero** letters of intent, named pilot partners,
  signed agreements or distributor deals.
- **Cost figures disagree across documents.** The detailed BOM totals **$1,774** and $2,504 with
  accessories (`BILL_OF_MATERIALS_FULL.md:103–104`), while `COMMERCIAL_PLAN.md:99`,
  `INVESTOR_PITCH.md:30` and `BUILD_PLAN.md:667` all claim **$1,650** from a shorter parts list, and
  `BUILD_PLAN.md:667` adds a "$3,800 prototype" figure that appears nowhere else. The SaaS model in
  the commercial plan double-counts the subscriber ramp.
- **Nothing is filed, and prior-art risk is understated.** `PATENT_CLAIMS.md` is drafted in claim
  style — better than bullets — but has **no application number, priority date, inventor, assignee,
  attorney, figures or enablement data**. `RISK_REGISTER.md:39` rates prior-art overlap with Decision
  Sciences and Lynkeos as likelihood "**Low**" and reduces the field to drift-tube and bulk-block
  geometries; no LANL, Decision Sciences, Toshiba or Lynkeos patent family is cited. Mitigation is
  "File defensive provisional patents" — future tense. Muon scattering tomography is a heavily
  patented field; a real freedom-to-operate search must precede any patent spend. Patent claim 3
  (single-sample momentum from one 5 mm lead deflection) is not enabled: multiple Coulomb scattering
  is a random process, so one sample gives O(1) relative momentum error, not the claimed accuracy.
- **The ASTM acceptance criterion is misattributed.** The documents repeatedly gate on "ASTM E1441:
  lead CNR ≥ 4.5, iron ≥ 3.5, void ≥ 2.0" (`SYSTEM_ACCURACY_AND_GAP_ANALYSIS.md:36–38`,
  `VALIDATION_REPORT.md:42–47`, `BUILD_PLAN.md:562`). **ASTM E1441 is the Standard Guide for
  Computed Tomography (CT) Imaging — a guide, with no CNR acceptance thresholds.** The numbers are
  invented and attributed to a standard that does not contain them. More relevant references are
  ASTM E2737 (digital detector array performance) and ISO 15708 (CT); **there is no
  muon-tomography performance standard**, and the documents never say so.
- **Certifications are stated as held.** `README.md:593` lists CE, FCC Part 15 Class B, IEC 60529
  IP67, RoHS 3, UN38.3 and an ECCN as achieved. No test report, certificate or lab correspondence
  exists in the repository — there is not a single PDF in it.
- **`RESEARCH.md:27–31` claims released open datasets**, including a 30-day field dataset below a
  45 m granite outcrop. `data/` contains only README files; a glob for `*.jsonl`, `*.csv`, `*.h5`
  returns nothing.
- **The plain-language German guide repeats the inflated claims**: "auf die Pikosekunde genau"
  (false even against the project's own 75–100 ps spec) and "verborgene Schäden millimetergenau
  orten" (contradicted by a 7.81 mm voxel and CNR 3.41). It does describe the physically necessary
  two-station straddling geometry, which contradicts `FIELD_OPERATIONS.md`'s single 150 mm stack.

### 4.10 The repository contradicts itself in its own status reporting

`BUILD_PLAN.md:569` states "Phase 0 and Phase 1 are completed and verified, and Phase 2 DAQ
execution is operational", while the same document's task table at lines 414–436 lists **T01–T23
all `[ ] Pending`**. `README.md` ticks every Phase 0–5 item `[x]`. The repository's own honest
sub-document (`ACCURACY_MEASURED.md`) explicitly supersedes two of the others. No TRL number appears
anywhere in the repository.

Further internal contradictions, each verified by reading the cited files:

- **Dark-count rejection is off by many orders of magnitude.** `README.md:143` claims 4-fold
  coincidence with an 8 ns window reduces false triggers to **< 10⁻⁶ Hz**. With the BOM's own 128
  SiPMs at ~300 kHz-class DCR, singles are ~38 MHz; a four-plane 8 ns window before spatial cuts
  yields accidentals in the 10²–10⁶ per second range. No derivation exists in the repository. The DCR
  specification is also self-inconsistent: ≤100 kHz/mm² in the spec versus a ≤50 kHz/mm² production
  gate in `MANUFACTURING.md:49`.
- **The material-contrast table is ~3× too high** against the Highland formula printed ten lines
  above it. `WHITEPAPER.md:40` gives the formula; the table at lines 49–57 lists lead at 90.8 mrad for
  10 cm at 2 GeV, where the formula gives **31.9 mrad** (concrete 20.0 vs 6.3; air 0.39 vs 0.086).
  The entire contrast argument is built on the inflated column.
- **The operator decision table would misclassify steel.** `TRAINING.md:53–57` puts steel at
  λ ≈ 0.22–0.35 cm⁻¹; the true value is **0.568** (X₀ = 1.76 cm). It is also calibrated to
  reconstructed values (lead 3.73–69.16) that no fielded reconstruction produces.
- **The IPC packet cannot hold a 4-plane event.** `IPC_COMMUNICATION_SPEC.md:26–49` specifies a
  "fixed-size 32-byte packet" whose listed header fields already consume all 32 bytes, then continues
  "… Additional Plane Hits (N−1)". `BUILD_PLAN.md:287` also writes the sync word as
  `0xAA55MUON`, which is not hexadecimal.
- **Battery capacity is misquoted twice.** A 12 V 12 Ah pack is **144 Wh**, not 153.6 Wh
  (which would be 12.8 V × 12 Ah); either way the 24 h claim fails.
- **Optical index is wrong.** η = 1.465 appears throughout, but the adhesive actually specified —
  Norland NOA 68 at `MANUFACTURING.md:32` — is **1.54**.
- **Training authority exceeds the real scheme.** `TRAINING.md:19–21` lets a vendor-run course
  authorise "Official ISO 9712 report sign-off". ISO 9712 certification is issued by an accredited
  national body under ISO/IEC 17024; no certification body, scheme or syllabus exists here.
- **The SaaS model assumes a 100 % attach rate and double-counts the ramp.** `INVESTOR_PITCH.md:50`
  shows 1 705 subscriptions at Year 5 against cumulative hardware sales of 15+60+180+450+1000 =
  1 705, at the top $1 250 tier for all of them; each year's subscription revenue is billed as if
  December's subscriber had paid for all twelve months. At the $450 tier the subscription line falls
  ~64 %.
- **Patent claims 3 and 5 are not enabled.** Claim 3 asserts momentum from a single 5 mm lead
  deflection — one sample of a random process, giving O(1) relative error. Claim 5's operative
  thresholds (λ < 0.02 void, λ > 0.25 rebar) are contradicted by the project's own measurements
  (concrete 0.0866, lead 1.786, reconstructed high-Z 3.73–69.16). Claim 1 claims a *result*
  (σ_pos ≤ 0.8 mm) with no reduction to practice.

---

## 5. What is left — the critical path

Everything below must exist before "launch on hardware" is a meaningful phrase. None of it does.

**Tier 0 — non-negotiable physical facts (must be established on a bench, not in a document)**

1. A single SiPM + scintillator + bias supply producing a resolved single-photoelectron and
   minimum-ionising-particle spectrum on an oscilloscope.
2. A measured noise and threshold budget proving the comparator can see 1 p.e.
3. A measured timing resolution, including the scintillator's own contribution — not an assumed
   number.
4. A measured light yield and attenuation for the chosen strip geometry and fibre coupling.

**Tier 1 — the signal chain**

5. An actual analog front-end schematic and layout: TIA, pole-zero, discriminator, per-channel bias
   decoupling, crosstalk budget, HV generation that matches the chosen converter's real pin
   behaviour (the current design misuses the LT3482's `CTRL` pin, which is a compensation node, not
   a voltage-programming input).
6. A timing architecture that fits real silicon. Either a small number of TDC channels, an
   external TDC ASIC, or a commercial digitizer — and a redesign of the channel count to match.
7. A firmware readout path that actually emits the documented packet. Today the interface is
   `spi_miso <= spi_mosi`.

**Tier 2 — integration**

8. A converter from detector hits to track state: geometry mapping, per-channel deskew calibration,
   and the momentum estimator that the current pipeline bypasses with ground truth.
9. A detector response model in the reconstruction loop — `detector_sim.py` exists and is unused.
10. A data-ingestion path. There is currently no way to put real events into the pipeline.

**Tier 3 — engineering and product**

11. PCB design, stackup, DFM, connectors, harness drawings, EMC pre-compliance.
12. Mechanical CAD, thermal analysis (carbon fibre is a thermal *insulator*, which inverts the
    current thermal strategy), shock/vibration analysis, seal design that is compatible with the
    claimed IP67 and nitrogen purge.
13. A real BOM with quotes, lead times and lifecycle status.
14. Calibration fixtures, a production test jig, and a bring-up procedure.
15. Certification: EMC, radio type approval, UN38.3, environmental.
16. A design FMEA *with RPN numbers* — the current "FMEA" is five mitigation bullets.

---

## 6. How far, honestly

Assuming a funded team of 4–8 (analog/PCB engineer, FPGA engineer, mechanical engineer, physicist,
software engineer, plus part-time certification and manufacturing support):

| Gate | From now | Cost to reach | TRL |
|---|---|---|---|
| Single-channel bench demonstration | 3–6 months | €10–25 k | 3 |
| Working multi-plane tracker on real cosmic muons | 9–15 months | €60–120 k | 4–5 |
| Imaging prototype, real phantom, real CNR | 18–30 months | €300–700 k | 5–6 |
| Certifiable, manufacture-ready design | 30–48 months | €1.2–3 M | 6–8 |
| Certified, manufactured, first revenue | 4–6 years | +€1–3 M | 9 |

For a single developer learning hardware as they go, the first two gates are realistically 2–3×
longer. The current repository represents roughly **the software brain, built ahead of a body that
has not been conceived.** That is the central asymmetry: the software cannot be validated further
without hardware, and the hardware is the slow, expensive, regulation-bound part.

---

## 7. How to get to the maximum

### Stage 0 — Integrity reset (this week, no cost)

This is prerequisite to everything and is the highest-return work available.

1. **Correct `README.md`.** Remove the `[x]` from every Phase 4 and Phase 5 item. Mark them
   `[ ] planned` or `[~] stub`. Align with `BUILD_PLAN.md`'s own pending list.
2. **Quarantine the false numbers.** In `SYSTEM_ACCURACY_AND_GAP_ANALYSIS.md` and
   `VALIDATION_REPORT.md`, insert the banner that `ACCURACY_MEASURED.md` already implies: these
   figures were not computed by any code in this repository. Better: move both files to
   `docs/archive/` and let `ACCURACY_MEASURED.md` be the single source of accuracy truth.
3. **Fix `automated_system_audit.py`.** Make the CNR line actually compare against 4.5, and stop
   printing `100% OPERATIONAL` unless every assertion is enforced. Point it at `accuracy_gate.py`.
4. **Take down or stamp the live website.** Add a prominent banner: "Design targets. No prototype has
   been built; nothing on this page has been measured." Remove "μ-Scope v1.0", remove the
   "12 mm rebar resolved" and "30 mm void resolved" figures, and recompute the endurance row
   (150 Wh at 12.5 W is 12 h, not "over 24 h"). This is the highest-priority single action in the
   entire assessment.
5. **Recompute the survey-time budget from the rate law.** Replace the "15–25 tracks/s" figure with
   the standoff-dependent rate (§4.6), and re-derive every application's integration time. This will
   change the product thesis, and it is better to discover that now than in front of a customer.
6. **Commit the accuracy corrections.** `normalized.py`, `accuracy_harness.py`,
   `ACCURACY_MEASURED.md`, `accuracy_gate.py`, `map_em.py`, `poca.py`, `run_pipeline.py` are
   currently sitting uncommitted. The most important scientific fix in the repository's history is
   not in version control.
7. **Stop shipping stubs as products.** Either implement the DICOM/IFC/OBJ exporters or rename
   their outputs and label them as placeholders in the UI and the gap analysis.
8. **Stop citing ASTM E1441 thresholds.** They do not exist. Either define your own acceptance
   protocol and publish it, or cite ASTM E2737 / ISO 15708 correctly.

### Stage 1 — One channel on a bench (the single highest-leverage step)

Do **not** design a 128-channel board. Buy one SiPM (S13360-3050CS class), one PVT scintillator
piece, one WLS fibre, a 57 V bias supply and a commercial transimpedance amplifier evaluation board.
Put it on an oscilloscope. The goal is a resolved single-photoelectron spectrum and a MIP peak.

This one experiment either validates or destroys the entire AFE specification, and it costs a few
thousand euros. It also produces the calibration constants the simulation currently invents:
light yield, PDE, DCR, timing jitter, temperature coefficient. Every number in the current documents
that is currently assumed becomes measured.

### Stage 2 — A real muon tracker (buy the timing, don't build it)

Build a small hodoscope — 4 planes, 8–16 strips each, 32–64 channels — and **use a commercial TDC or
digitizer rather than a 75 ps FPGA carry-chain design.** For muon scattering tomography, timing
matters only for direction rejection: with 150 mm plane separation the flight time is 500 ps, so
sub-nanosecond timing is entirely sufficient. The 75 ps target buys nothing for tomography and costs
a 20× silicon shortfall.

Deliverable: real muon tracks, an angular distribution matching cos²θ, a rate matching the known
flux. **Then feed real data into the existing inversion.** This is the moment the project's strongest
asset — the reconstructor — becomes real, and it is the first evidence that any of the accuracy
claims mean anything.

### Stage 3 — Pick a defensible product, and cut the rest

The current specification tries to be three products at once, and the three requirements are
mutually exclusive (§4.5). The rate law (§4.6) removes a fourth option. The maximum achievable
outcome requires choosing:

- **Transmission radiography (recommended near-term).** Targets 1–50 m: dams, dykes, tunnels,
  geotechnical overburden, blast furnaces. This is the mode where muon tomography genuinely beats
  GPR and where real commercial demand exists. It needs **flux counting and angular resolution, not
  momentum** — which removes the entire unsolved momentum problem and the 28 kg of lead with it.
  Critically, it must be sold honestly: at these standoffs the deliverable is **opacity monitoring
  over days to weeks**, not a same-day survey. That is how the existing commercial players position
  it, and it is a real business.
- **A larger aperture, if survey speed is the selling point.** The 1/*D*² law means speed comes from
  detector *area*, not from better timing or more channels. A 1 m² aperture at 3 m standoff buys
  ~8× the rate of the specified 0.25 m². If "hours, not days" is the value proposition, the design
  must grow in area and accept the mass and cost consequences — and the 9.5 kg backpack target dies
  with it.
- **Scattering tomography (roadmap).** High-Z detection and rebar imaging. Requires solving the
  momentum ambiguity. The right answer is **redundant tracking planes and a kink fit**, as real MST
  systems use — not a passive lead absorber bolted to a portable instrument. Note that a
  single-sample momentum estimate from one 5 mm lead layer cannot work (the scattering is a random
  process), so the current patent claim 3 is not enabled.
- **The software as its own product.** The inversion toolkit, published or licensed, monetises the
  asset that actually exists today. This is the only path that can generate revenue or credibility
  before any hardware exists, and it is the fastest route to a defensible position.

### Stage 4 — The maximum, stated honestly

The realistic maximum for this project is **not** a 9.2 kg, 24 h, 128-channel, 75 ps momentum
spectrometer. That specification is internally contradictory and physically unbuildable. The
achievable maximum is:

> A credible, honestly documented inversion and simulation toolkit — already close to existing —
> plus a validated instrument in **one** well-chosen application, built on commercial timing
> electronics, with a real measured accuracy figure and a peer-reviewed acceptance protocol.

That is a fundable, defensible position. The shortcut to it is not more simulation; it is one
oscilloscope trace of one muon.

---

## 8. Decisions required now

| # | Decision | Recommendation |
|---|---|---|
| 1 | Correct the public claims, or defend them | **Correct them, and take the site down first.** The repository's own audit already disproves them; anyone who benchmarks the code will find it. |
| 2 | Keep the 75 ps / 128-channel TDC | **Drop it.** 20× over silicon resources, and unnecessary for tomography. |
| 3 | Keep the lead-plate momentum spectrometer | **Drop it for the portable product.** 28.4 kg of lead is incompatible with a 9.5 kg backpack. |
| 4 | Keep the 24 h / 9.5 kg pair | **Pick one.** At 12.5 W, 9.5 kg buys ~12 h; 24 h buys ~13 kg. |
| 5 | **Is the product "hours per survey" or "monitoring over time"?** | **Monitoring over time.** The 1/*D*² rate law makes sub-day surveys of metre-scale structures unachievable at 0.25 m². This single decision reshapes the product, the market and the business model. |
| 6 | 128 / 256 / 512 channels, and 500 mm or 1280 mm aperture | **Decide once and propagate.** Four documents currently disagree; 64 strips at 20 mm pitch is 1280 mm, not 500 mm. |
| 7 | Build hardware, software, or services | **Software/services first.** Hardware needs a funded team; the toolkit needs a release. |

---

## 9. What would change this assessment

The assessment is pessimistic because the evidence is. It would change materially on any of:

- One oscilloscope screenshot of a resolved single-photoelectron spectrum from a real SiPM.
- One real muon track reconstructed from detector hits rather than simulated ones.
- A committed accuracy correction and a truthful README.

All three are achievable within a few months and a few thousand euros. The scientific core is
already good enough to deserve them.

---

## Appendix — measurements reproduced in this assessment

```
pytest src/inversion/tests          ->  17 passed in 5.64s
cargo test (src/daq, GNU target)    ->   8 passed
accuracy_harness --events 50000 --grid 16 --seed 12345 --beta 0.3
    acceptance ratio                19.8 %
    lead CNR   PoCA 0.36 | path-normalised 3.41 | MAP-EM 1.13
    RMSE       PoCA 34.87 | path-normalised 4.693 | MAP-EM 0.471
    edge width  not measurable (validation report claims 6.2 mm)
    theta^2 spread across momentum spectrum   26 719x
verified independently:
    tdc_tdl_carry instantiated anywhere        NO (own file only)
    CARRY4 / generate / genvar in RTL          NONE
    muonscope_top.sv:92                        spi_miso <= spi_mosi;
    slice_0000.dcm 512 B, DICM magic at 128    ABSENT
    isosurface.obj / reconstruction_mesh.obj   8 vertices, 12 faces
    HEAD map_em.py EM step                     ratio = y/pred_var**2  (not EM)
    working-tree map_em.py EM step             ratio = y/pred_var     (correct MLEM)
    mass breakdown 3.4+1.6+1.8+2.4             = 9.2 kg (sum is correct; lead omitted)
    153.6 Wh / 12.5 W                          = 12.29 h (claim: > 24 h)
    one 500x500x5 mm lead plate                = 14.2 kg (two required = 28.4 kg)
    128 ch x 128 taps / 4 per CARRY4           = 4096 CARRY4 vs ~200 available (20.5x)
    detector_geometry.yaml planes             8 planes x 64 strips = 512 strips
    sqrt(2)*0.75mm/600mm                       = 1.77 mrad  (the published term)
    sqrt(2)*0.75mm/150mm                       = 7.07 mrad  (documented baseline) -> total 7.18 mrad
    dz needed for 2.1 mrad                     = 505 mm
    max rate at 600 mm acceptance              = 12.0 tracks/s (docs claim 15-25/s)
    rate at 2 m / 3 m / 5 m / 8 m              = 1.08 / 0.48 / 0.17 / 0.068 tracks/s
    bridge pier, 500k tracks at 2-3 m          = 5.4-12 days (claim: 6-8 h)
    dam core, 2M tracks at 5-8 m               = 134-343 days (claim: 24-30 h)
    muon.kellersystems.dev                     HTTP 200, live, no disclaimer, footer "v1.0"
    *.kicad_sch/.sch/.brd/.step/.dwg/.xlsx     NONE (also absent from all git history)
    BOM total vs commercial plan               $1,774 vs $1,650 (different parts lists)
    12 V 12 Ah LiFePO4                         = 144 Wh, not 153.6 Wh
```

---

## Conclusion

The repository contains a genuinely competent simulation and inversion codebase — correctly
implemented physics, a working MLEM/MAP-EM with a correct Green one-step-late prior, a real Siddon
matrix, running tests, and an accuracy harness that reproduces exactly and that already caught and
documented the project's own units bug. That is a real asset and it is worth about two years of
software effort.

Around it, every claim about a physical instrument is either unsupported, arithmetically impossible,
or contradicted by another document in the same repository. The hardware does not exist at any
level: no schematic, no PCB, no CAD, no BOM spreadsheet, no measured photon, no detected muon. The
live public website asserts a measured performance envelope and a version number for that
non-existent device.

**The distance to a hardware launch is not measured in commits. It is measured in the number of
things that must be built from zero, and it is 3–5 years and €1.5–4 M of funded engineering.** The
fastest way to shorten it is not to write more simulation. It is to buy one SiPM, one scintillator
and one amplifier, put them on a bench, and look at a single muon.
