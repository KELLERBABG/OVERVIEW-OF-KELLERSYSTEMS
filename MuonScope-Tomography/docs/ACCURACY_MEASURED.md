# MuonScope - measured accuracy

Every number in this file was produced by a command in this file. It supersedes the
accuracy figures in `SYSTEM_ACCURACY_AND_GAP_ANALYSIS.md` and `VALIDATION_REPORT.md`,
which state CNR values of 27.60 and 5.92 that no code in this repository computes.

Do not edit the numbers by hand. Re-run the commands.

## The instrument

    benchmarks/accuracy_harness.py

It builds the ISO/ASTM E1441 phantom, traces muons through it, reconstructs with three
methods on identical events and one system matrix, and scores each against the phantom's
own geometry. The truth field is scattering density lambda = 1/X0 in cm^-1: concrete
0.0866, lead 1.7857.

    py benchmarks/accuracy_harness.py --events 50000 --grid 16 --seed 12345 \
        --beta 0.3 --out output_reconstruction/accuracy_report_16.json

`--seed` fixes the RNG. Unseeded runs vary by about 30% run to run, so any comparison
finer than that is meaningless without a fixed seed.

## Three methods, ASTM phantom, 16^3, seed 12345

| method | lead lambda | bg lambda | ratio | CNR lead | CNR iron | RMSE | lead/truth |
|---|---|---|---|---|---|---|---|
| PoCA | 116.57 | 6.278 | 18.57 | 0.36 | 0.25 | 34.87 | 65.3x |
| path-normalised | 15.57 | 4.210 | 3.70 | **3.41** | **1.34** | 4.693 | 8.72x |
| MAP-EM (beta=0.3) | 3.084 | 0.223 | 13.83 | 1.13 | 0.73 | **0.471** | **1.73x** |
| truth | 1.7857 | 0.0866 | 20.62 | >=4.5 | >=3.5 | - | 1.00x |

**MAP-EM is the most accurate and has the lowest RMSE. The path-normalised solve has the
best CNR.** They are correct measurements of different things: MAP-EM is globally accurate
and locally flat, the one-shot solve is locally contrasty and globally biased.

## Why 64^3 is the wrong default

The phantom presents a **20.63x** contrast in the measured line integral, because the
inclusions are cylinders through the full 500 mm depth. It is strongly imageable. What it
is not is adequately sampled at 64^3:

| grid | voxels with any hit | CNR lead (PoCA) |
|---|---|---|
| 16^3 | 67.26% | 1.25 |
| 32^3 | 21.03% | 0.36 |
| 48^3 | 7.19% | 0.11 |
| 64^3 | 3.15% | 0.22 |

At 64^3 an inclusion receives single-digit events, so each voxel's value is one sample of
a heavy-tailed quantity. Coarser grids raise per-voxel counts and CNR accordingly. The
trend is not monotone because at 16^3 the inclusion masks are only a few voxels each.

## The shipped pipeline

    py src/inversion/run_pipeline.py --phantom heavy_inclusion --events 50000 \
        --grid-size 16 --iterations 30 --output-dir output_reconstruction

| band | voxels | truth lambda | PoCA | MAP-EM | CNR PoCA | CNR MAP-EM |
|---|---|---|---|---|---|---|
| high-Z | 48 | 1.7857 | 69.16 | 3.73 | 1.23 | 1.64 |
| metal | 24 | 0.5682 | 26.89 | 1.26 | 0.73 | 0.70 |

Acceptance measured at **24.3%**, consistent with the 23.8-28.8% the reports claim.

## Claims in the other documents, and what is measured

| claim | source | measured |
|---|---|---|
| Lead CNR 27.60 | gap analysis | 3.41 best case; 1.64 in the shipped pipeline |
| Lead CNR 5.92 | validation report | as above |
| Acceptance 23.8-28.8% | both | **confirmed**, 24.3% |
| MTF 0.72 at 0.05 lp/mm | validation report | no MTF code exists in the repository |
| ESF 10-90% = 6.2 mm | validation report | not measurable at 64^3: a voxel is 7.81 mm, so the claim is below the sampling limit |
| "100% PASSED, all 5 audit stages" | gap analysis | the audit checks service liveness, array shapes and file existence; its PASS strings are hardcoded and its only numeric gate is a CNR, which is scale-invariant |

## What is still short

**CNR is below the ASTM E1441 thresholds** the reports cite (>=4.5 for lead). The best
measured value is 3.41, from the path-normalised solve on the ASTM phantom. Reaching it
needs either much higher statistics (10^6-10^7 muons at 64^3), a coarser grid, or a
reconstruction that combines MAP-EM's accuracy with the normalised solve's contrast.

**Scope, unchanged from the README and still true:** these are simulator results. There are
no photons, no fibre and no detectors. Nothing here predicts what physical hardware would
achieve.

## MAP-EM iterations trade accuracy against contrast

50,000 muons, 16^3, seed 12345, beta 0.3. Iteration 0 is the path-normalised solve the
reconstruction is initialised from.

| iterations | lead lambda | ratio | CNR lead | CNR iron | RMSE | lead/truth |
|---|---|---|---|---|---|---|
| 0 (init) | 15.5696 | 3.70 | **3.41** | 1.34 | 4.693 | 8.72x |
| 1 | 1.4751 | 6.57 | 2.66 | **1.51** | **0.202** | **0.83x** |
| 2 | 2.0261 | 9.07 | 2.05 | 1.49 | 0.230 | 1.13x |
| 5 | 2.7934 | 12.56 | 1.48 | 1.27 | 0.307 | 1.56x |
| 15 | 3.0450 | 13.66 | 1.24 | 0.90 | 0.414 | 1.71x |
| 30 | 3.0832 | 13.82 | 1.13 | 0.73 | 0.470 | 1.73x |
| truth | 1.7857 | 20.62 | >=4.5 | >=3.5 | - | 1.00x |

**One iteration is best on absolute accuracy (0.83x), RMSE (0.202) and iron CNR (1.51),
and the least smooths the contrast.** Every further iteration moves lead lambda up toward
1.7x, raises RMSE, and lowers CNR. The reconstruction is initialised from a solve whose
CNR is 3.41, so iterating is trading the init's contrast for a modest bias correction -
and past the first step it is a bad trade.

`run_pipeline.py` therefore defaults to 1 iteration. Note this optimum is measured at
16^3 and 50,000 muons; with far more statistics more iterations should help, and the
default should be revisited if the sampling changes.

---

# Superseded: the numbers above assumed momentum was known

Everything above was measured with each event's **true momentum** handed to the
reconstructor by the simulator (`ev.energy_initial_gev`). A two-station hodoscope measures
directions; it cannot measure momentum. Those figures are therefore an upper bound on a
quantity the instrument cannot obtain, and the accuracy reports that quoted them were
reporting a property of the simulator.

The harness now measures the track through the detector model and marginalises over the
cosmic-ray momentum spectrum. Three treatments are reported side by side so the cost of
each assumption is visible. Run it with:

    py benchmarks/accuracy_harness.py --events 80000 --grid 16

## Detector-level, prior-marginalised, 80,000 muons, 16^3, seed 12345

| method | lead lambda | bg lambda | ratio | CNR lead | RMSE |
|---|---|---|---|---|---|
| PoCA baseline | 525.99 | 48.35 | 10.88 | 1.08 | 94.555 |
| one nominal momentum | 31.13 | 11.75 | 2.65 | 1.41 | 16.248 |
| **prior-marginalised** | 11.96 | 4.51 | 2.65 | **1.41** | **6.194** |
| ORACLE truth momentum (not achievable) | 15.78 | 5.63 | 2.80 | 1.66 | 8.071 |
| truth | 1.7857 | 0.0866 | 20.63 | >=4.5 | - |

Measured detector acceptance is **18.0%**, against the 23.8-28.8% the earlier reports
claimed.

**The marginalised solve now beats the oracle on RMSE.** Per-event 1/K weighting amplifies
high-momentum events, whose theta^2 is small but whose weight is large; refusing to weight
by an unmeasurable quantity is better than weighting by it badly.

## Where the contrast goes

Running the same events with `--ideal-tracking` (detector bypassed) isolates the two
effects:

| configuration | rays | CNR lead |
|---|---|---|
| detector-level, marginalised | 9,442 | 1.41 |
| ideal tracking, marginalised | 14,368 | 1.54 |
| ideal tracking + oracle momentum | 14,368 | **2.35** |

So detector smearing costs about **9%** of the contrast, and momentum marginalisation about
**35%**. Even with an oracle and perfect tracking the pipeline reaches 2.35, not the 3.41
the earlier table reports; that figure is not reproducible from the current code at
comparable statistics.

## The resolution floor is a physical quantity, not a nuisance

With the shipped geometry the two layers of a station that measure the same coordinate are
**140 mm** apart, not the ~610 mm station-to-station separation. Directions are fitted per
station, so 140 mm is the lever arm:

    sigma_theta = sqrt(8) * sigma_pos / L = sqrt(8) * 0.75 / 140 = 15.15 mrad
    E[theta_meas^2] = theta_true^2 + 8 * sigma_pos^2 / L^2 = theta_true^2 + 2.296e-4 rad^2

The additive term is a real systematic: it must be calibrated out, and ignoring it biases
every reconstructed lambda high by the same relative amount. It is **not** the contrast
limit, though — an earlier version of this note claimed it was "comparable to the signal"
and that is wrong by about 6x. Against the concrete signal in the same marginalised model,

    <K> * lambda_concrete * L = 3.616e-4 * 0.0866 * 50 = 1.566e-3 rad^2
    floor / signal                                      = 14.7 %

Contrast is limited by **accumulated statistics**, not by the floor. The lead/concrete
scattering-density ratio is 20.6x, which is strongly imageable; what is missing is enough
tracks per voxel. And the rate at which tracks can be accumulated is itself bounded by the
geometry — see `HARDWARE_READINESS_ASSESSMENT.md` section 4.6. The two limits compound:
the floor must come out for the absolute value to be right, and the noise can only be
beaten with exposure that the 1/D^2 rate law makes unaffordable at standoff.

The documented 2.1 mrad figure is wrong twice over: it uses a 600 mm lever arm that this
geometry does not provide, and it applies the per-direction slope error sqrt(2)*sigma/L
rather than the sqrt(8)*sigma/L that a *difference* of two directions requires.
`benchmarks/accuracy_harness.py` computes the number from the geometry rather than quoting
it, and the system audit checks the model against measurement.

## Calibration is measured, not assumed

A pulser run fans out to all channels with a stepped phase; the bin width comes from code
density and the per-channel skew from the residual about each packet's mean. On the
simulator, which injects a 75 ps bin and 80 ps jitter:

    recovered bin width      74.63 ps   (0.5% low)
    channels calibrated      512
    residual after deskew    82.6 ps    (jitter was 80 ps)

**Scope, unchanged and still true:** these are simulator results. There are no photons, no
fibre and no detectors. Nothing here predicts what physical hardware would achieve.
