# Why a Backpack-Scale Muon Scattering Tomograph Does Not Compose

**A composition analysis of a portable cosmic-ray muon tomography specification**

MuonScope research note · revised 2026-10

---

## Abstract

A specification was drawn up for a backpack-portable cosmic-ray muon scattering
tomograph: two tracking stations, sub-2 mrad angular resolution, 10 mm voxels at
1 m, greater than 24 h endurance from a 150 Wh pack, at under 9.5 kg per station,
imaging metre-scale concrete structures in a few hours.

Instrument software was then built to evaluate that specification by simulation,
taking care that the reconstruction consumes only quantities a two-station
hodoscope can actually measure. Under that model:

* the angular resolution is **15.15 mrad**, not 2.1 mrad, because the lever arm
  that sets it is the intra-station plane separation (140 mm), not the
  station-to-station separation the error budget used;
* detector acceptance is **18.0 %**, against a claimed 23.8–28.8 %;
* lead contrast-to-noise ratio is **1.41** detector-level and marginalised,
  against claimed values of 5.92 and 27.60;
* coincidence rate falls as the inverse square of standoff, so the specification's
  own 500,000-track requirement for a bridge pier takes **~29 days at 3 m**, not
  6–8 hours;
* the specified timing architecture needs **4,096 carry primitives** against
  roughly 200 available in the chosen device;
* the claimed endurances and masses are **mutually exclusive** with the
  momentum spectrometer the same design requires.

No individual requirement is unreasonable. The finding is that they were
specified independently and never composed: the failures are factors of 2 to 85,
and each is traceable to one of three methodological errors — a lever arm drawn
from the wrong frame, a rate model extrapolated across two orders of magnitude in
standoff, and budgets written as though the others did not exist.

---

## 1. The specification

| parameter | specified |
|---|---|
| Topology | Two stations, 4 strip planes each, 64 X + 64 Y strips per panel |
| Plane separation | 150 mm baseline, extendable to 450 mm |
| Station separation | 600 mm (cited for geometric acceptance) |
| Position resolution | 0.75 mm via triangular-strip barycentric centroiding |
| Angular resolution | 2.1 mrad, from an error budget of 1.77 / 0.85 / 0.16 / 0.87 mrad |
| Timing | 75 ps bin, 128 channels, tapped-delay-line TDC on Artix-7 |
| Momentum | passive 5 mm lead absorber, deflection inversion |
| Power / endurance | 12.5 W, > 24 h on 150 Wh LiFePO4 |
| Mass | ≤ 9.5 kg per station |
| Survey time | 6–8 h for a 2–4 m bridge pier |

The intended novelty was scale: scattering tomography — as opposed to
transmission radiography — at backpack mass, with sub-2 mrad resolution and
hours-scale surveys. Larger scattering systems exist commercially (portal-scale
for cargo, borehole arrays for mineral exploration); the portable end of the
market is dominated by single-tracker absorption radiography precisely because
scattering needs two trackers and a momentum measurement.

---

## 2. Method

The software is not a mock-up of the instrument. Three properties make its output
usable as evidence about a *design*:

**A defined wire protocol.** Events are encoded into a big-endian frame of
`27 + 9n` bytes with a CRC-32, and the reconstruction consumes only decoded
frames. The simulator encodes through the same encoder a front end would have to
match, so a simulated run exercises the real decoder, framing, calibration,
tracking and reconstruction.

**No truth in the reconstruction path.** The ground-truth scattering angle and
per-event momentum are available only through a side channel that
`MuonScopeSystem.run()` cannot reach, used solely for post-hoc validation. The
per-event momentum, in particular, is *not* passed to the solver: a two-station
hodoscope cannot measure it, so it is marginalised over the known cosmic-ray
spectrum using the Gaisser parameterisation. An oracle variant is computed
separately and labelled as an unachievable upper bound.

**Measured, not asserted.** The harness reports only values it computes;
`tests/validation_scripts/accuracy_gate.py` fails if the reconstruction is not in
absolute units, and `automated_system_audit.py` evaluates 20 checks before
printing them. Both exist because an earlier version of this project reported
PASS on a reconstruction that was ~8000× wrong: CNR is scale-invariant, so a
normalisation error is invisible to it.

Detector model: strip charge sharing with per-layer position resolution
`sigma_pos`, 95 % plane efficiency, TDC binning and jitter. Geometry from
`config/detector_geometry.yaml` — X planes at z = +375 and +235 mm, Y planes at
+365 and +225 mm for the top station, mirrored below.

---

## 3. Results

### 3.1 The lever arm is 140 mm, not 600 mm

Angular resolution is set by how precisely a *direction* can be fitted, and a
direction is fitted within a station, from the two planes of one coordinate.

Fitting a slope through two measurements a distance `L` apart, each with
position error `sigma_pos`, gives a per-component slope error

```
sigma_slope = sqrt(2) * sigma_pos / L
```

A scattering angle is the *difference* of two such directions, each measured
independently, so the three-dimensional angle error is

```
sigma_theta = sqrt(2) * sqrt(2) * sigma_pos / L * sqrt(2)
            = sqrt(8) * sigma_pos / L
```

With `sigma_pos = 0.75 mm` and, from the geometry file, `L = 140 mm`:

```
sigma_theta = sqrt(8) * 0.75 / 140 = 15.15 mrad
```

The specified 2.1 mrad is wrong in two independent ways. It uses a 600 mm lever
arm — the *station-to-station* separation, which is not the distance between the
two planes that measure the same coordinate — and it uses `sqrt(2) sigma/L`, the
error on a single direction, rather than the `sqrt(8) sigma/L` a difference of
two directions requires. At the documented baseline the correct figure is 15.15
mrad: **7.2× worse than specified**.

This is the root failure. Everything downstream that depends on angle —
transverse resolution, voxel size at a given standoff, the ability to resolve
rebar spacing — is off by the same factor.

### 3.2 The detector resolution floor is a real systematic

Detector smearing biases the *squared* angle high by the summed direction
variance:

```
E[theta_meas^2] = theta_true^2 + 8 * sigma_pos^2 / L^2
                = theta_true^2 + 2.296e-4 rad^2
```

Verified against measurement: over 5,000 events the observed mean excess was
`2.226e-4 rad^2` against a predicted `2.296e-4`, a ratio of 0.97.

Against the concrete signal in the same marginalised model, with the
prior-averaged Highland factor `<K> = 3.616e-4` and a 50 cm path,

```
<K> * lambda_concrete * L = 3.616e-4 * 0.0866 * 50 = 1.566e-3 rad^2
floor / signal                                      = 14.7 %
```

So the floor is a ~15 % additive bias that must be calibrated out — significant,
but not the contrast limit. Contrast is limited by accumulated statistics: the
lead/concrete scattering-density ratio is 20.6×, strongly imageable, but at 16³
with 9,442 tracks there are only a few tracks per voxel, and the per-voxel
variance swamps the difference.

The floor matters because it *compounds* with §3.3: it must be removed for
absolute values to be right, and the noise can only be beaten with exposure that
the rate law makes unaffordable at standoff.

### 3.3 Rate falls as 1/D², and the survey-time table ignored it

For two stations straddling a target at separation `D`, the solid angle subtended
falls as `1/D²`. Calibrating on the measured end-to-end yield — 9,442 usable
tracks from 80,000 incident muons, 11.8 %, through a 0.25 m² aperture at the
harness geometry — gives approximately 4.9 usable tracks/s at `D = 0.6 m`. Then:

| standoff `D` | usable rate | 500,000 tracks |
|---|---|---|
| 0.6 m | 4.92 /s | 1.2 days |
| 1 m | 1.77 /s | 3.3 days |
| 2 m | 0.44 /s | 13 days |
| 3 m | 0.20 /s | **29 days** |
| 5 m | 0.07 /s | 82 days |

The specification's survey-time table quotes 6–8 hours for a bridge pier at
2–4 m. That is off by **~85× at 3 m**. The table was evidently derived from the
benchtop geometry — where the figure of 1.2 days for 500,000 tracks is at least
the right order — and then applied unchanged to objects metres away.

To survey a 3 m pier in 6 hours requires ~23 usable tracks/s, i.e. roughly **100
m² of active area**. That is not a design iteration; it is a different class of
machine.

### 3.4 The timing architecture does not fit the device

A tapped-delay-line TDC spends carry primitives for resolution. With 128 taps per
channel and four taps per `CARRY4`:

```
128 channels × 128 taps / 4 taps per CARRY4 = 4,096 CARRY4
```

An XC7A35T has approximately 200; an XC7A50T approximately 250. The shortfall is
**16–20×**, and even the largest Artix-7 (~740) is 5.5× short. The sampling
registers alone — 128 taps × dual-rank, per channel — exceed 32,000 flip-flops
against a 41,600-flop device.

This is not a tight fit to be optimised. The fabric does not contain the delay
lines. It is also, for this application, unnecessary: sub-nanosecond timing is
sufficient for direction rejection over a 600 mm baseline, where the flight time
is 2 ns.

### 3.5 Three budgets that cannot hold at once

**Power and endurance.** 150 Wh ÷ 12.5 W = **12.0 h**, against a claimed > 24 h.
Reaching 24 h needs ~300 Wh, which at ~10 Wh/kg is a ~3.5 kg pack.

**Mass.** The design requires two 5 mm lead plates of 500 × 500 mm for its
passive momentum spectrometer:

```
2 × (50 × 50 × 0.5 cm³) × 11.34 g/cm³ = 28.35 kg
```

against a total station mass budget of 9.5 kg. The published mass breakdown does
not mention lead at all.

**Momentum.** A single multiple-scattering deflection is one sample of a Gaussian
whose width is the quantity being inverted. The relative uncertainty on the
inferred momentum is O(1), and the inversion is non-linear in a noisy quantity.
Measuring momentum requires redundant tracking planes and a kink fit — a
different instrument from the one specified.

The 24 h, 9.5 kg and lead-absorber requirements are pairwise incompatible. Any
two can be met; not all three.

---

## 4. Why the errors are systematic

The failures are of a consistent character. Each component, taken alone, is
within a factor of about two of plausible: 75 ps TDCs exist, 10 kg instruments
exist, 12 h battery life is ordinary, lead absorber momentum spectrometry is
textbook. The composite is wrong by 2× to 85×.

Three mechanisms account for all of them:

**A lever arm drawn from the wrong frame.** The error budget took the 600 mm
station separation. The measurement uses the 140 mm intra-station plane spacing.
Both numbers appear in the same documents; neither document connects them to the
quantity being computed. A dimensional-analysis pass over the error budget would
have caught it.

**A regime extrapolated without re-derivation.** The survey-time table was built
at benchtop scale and applied at metre scale, where the governing variable is
standoff squared. The table's internal structure — resolution traded against
time — is sound; its inputs are not. Scaling laws have to be re-evaluated at
every regime crossing.

**Budgets written independently.** Mass, power and endurance were each specified
as if the others did not exist, and the momentum spectrometer was specified
without being entered into the mass budget that would have excluded it.

The generalisable lesson is narrow and unglamorous: **individual plausibility is
not composition.** Every number here would survive a review on its own. The
review that matters is arithmetic — multiply, divide and add the requirements
together before committing to them, and re-derive rather than carry over whenever
a quantity changes regime.

---

## 5. Limits of this analysis

* **All figures are simulated.** No photon, fibre or detector was involved. The
  numbers bound what *this specification* can achieve under a model of it; they
  do not bound muon tomography.
* **`sigma_pos = 0.75 mm` is an input, not a measurement.** It is taken from the
  design documents. Real barycentric resolution depends on light yield, fibre
  coupling, SiPM photon detection efficiency and electronics noise, none of which
  are modelled. If the achievable figure is 1.5 mm, angular resolution halves
  again.
* **The detector model omits** optical crosstalk, SiPM afterpulsing and dark
  count rates, scintillator self-attenuation, and thermal drift. A real system
  would be worse, not better, on all of these.
* **Timing behaviour is modelled, not measured.** Calibration recovers an
  injected 75 ps bin to 74.63 ps and a deskew residual of 82.6 ps against 80 ps
  of jitter — which validates the calibration procedure against a simulator that
  injects a known answer, not against a real TDC's differential and integral
  non-linearity.
* **Muon tomography itself is not in question.** It is a working technique, sold
  commercially for nuclear waste characterisation, mineral exploration,
  volcanology and cargo screening. The finding is about a portable
  *scattering* instrument at this scale.

---

## 6. What would compose

Re-deriving rather than carrying over, the physics is kinder at short standoff —
and this is the direction a continuation should take.

At `D = 0.5 m` the rate is ~7.1 usable tracks/s, so 150,000 tracks accumulate in
about **6 hours**. With the measured 15.15 mrad angular resolution, transverse
resolution at that standoff is ~7.6 mm — which resolves 10 mm features. Mass,
power and timing fall into place once the lead absorber is removed, because
without it the momentum problem does not need solving: absorption-mode and
short-standoff scattering work at the singles rate and on angle alone.

A specification that composes:

| parameter | value | why it holds |
|---|---|---|
| Target class | Sub-metre: columns, pier caps, precast QA | avoids the 1/D² penalty |
| Standoff | 0.3–0.7 m | rate 4–20 tracks/s |
| Angular resolution | ~15 mrad | from `sqrt(8) sigma/L` at the achievable lever arm |
| Voxel resolution | ~8–10 mm | follows from standoff and angle |
| Channels | 32–64 | fits the fabric, ~1,500 `CARRY4` even at 128 taps |
| Timing | ~1 ns | sufficient for direction rejection over 0.6 m |
| Mass | ~12 kg | no lead absorber |
| Endurance | ~12 h | 150 Wh at 12.5 W, stated honestly |
| Survey time | 3–6 h | at sub-metre standoff |

Nothing here is novel at the scale originally intended. It is a smaller product
whose numbers compose — and composition is the property the original
specification lacked.

**The next step is a measurement, not a document.** One SiPM, one scintillator
tile, a bias supply, an amplifier and an oscilloscope, to obtain a
single-photoelectron spectrum. That fixes `sigma_pos`, the largest unmeasured
input in every calculation above.

---

## 7. Reproducing every number

```bash
# the instrument software end to end on simulated detector bytes
py src/inversion/run_system.py --source sim --events 20000 --grid 16

# angular resolution, CNR and RMSE against phantom truth, per momentum treatment
py benchmarks/accuracy_harness.py --events 80000 --grid 16

# the same with the detector model bypassed, to separate tracking error
py benchmarks/accuracy_harness.py --events 80000 --grid 16 --ideal-tracking

# absolute-units gate; 20-check system audit
py tests/validation_scripts/accuracy_gate.py
py tests/validation_scripts/automated_system_audit.py
```

Measured values are recorded in [`ACCURACY_MEASURED.md`](ACCURACY_MEASURED.md);
the full gap analysis, including the certification and mechanical discrepancies
not covered here, is in
[`HARDWARE_READINESS_ASSESSMENT.md`](HARDWARE_READINESS_ASSESSMENT.md).

### Key derivations, for checking

```
sigma_theta   = sqrt(8) * sigma_pos / L            = 15.15 mrad   (0.75 mm, 140 mm)
floor         = 8 * sigma_pos^2 / L^2              = 2.296e-4 rad^2
signal        = <K> * lambda_concrete * L          = 1.566e-3 rad^2
floor/signal                                       = 14.7 %
CARRY4 needed = 128 ch * 128 taps / 4              = 4,096
endurance     = 150 Wh / 12.5 W                    = 12.0 h
lead mass     = 2 * 1250 cm^3 * 11.34 g/cm^3       = 28.35 kg
rate(D)       = 4.92 * (0.6 / D)^2  tracks/s
```
