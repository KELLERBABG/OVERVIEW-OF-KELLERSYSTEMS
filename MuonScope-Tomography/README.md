# MuonScope Tomography

**An archived research project on why portable cosmic-ray muon scattering
tomography does not compose at backpack scale — with the simulation and
reconstruction software that established it.**

> **Status: research archive.** No instrument was built. No muon was ever
> detected. The software works and is tested; the hardware specification does
> not survive its own arithmetic.

---

## What this is

I set out to design an ultra-portable cosmic-ray muon scattering tomograph: two
tracking stations, backpack-portable, sub-2 mrad angular resolution, 10 mm
voxels, 24-hour endurance, imaging metre-scale concrete in a few hours.

The design was specified in detail — error budgets, a bill of materials,
regulatory plan, firmware architecture, mechanical layout.

Then I wrote the simulation, and it said no.

This repository is the record of that. It contains:

- **a working instrument software stack** — wire protocol, acquisition,
  data-driven calibration, tracking, reconstruction, and CAD/BIM/DICOM export,
  with 56 tests and a validation pipeline;
- **a quantified analysis** of why the specified design fails, with the
  arithmetic in [`docs/HARDWARE_READINESS_ASSESSMENT.md`](docs/HARDWARE_READINESS_ASSESSMENT.md);
- **the original design documents**, archived in [`docs/archive/`](docs/archive/)
  for provenance.

The negative result is the contribution. Portable scattering tomography is hard
for specific, quantifiable reasons, and those reasons are worth writing down.

---

## The finding

Every requirement in the specification is individually plausible. None of them
compose. That is the whole result.

| requirement | why it fails |
|---|---|
| **2.1 mrad angular resolution** | Directions are fitted *per station*, so the lever arm is the intra-station plane separation — **140 mm**, not the 600 mm station-to-station figure the error budget uses. Measured: **σ_θ = 15.15 mrad**, 7.2× worse. |
| **10 mm voxels at 1 m** | Follows from the above. At 15 mrad and 1 m standoff the transverse resolution is ~15 mm before any reconstruction loss. |
| **hours-scale surveys of metre-scale structures** | Coincidence rate falls as 1/D². Calibrated on the measured 11.8 % end-to-end yield, a 0.25 m² aperture gives **0.20 usable tracks/s at 3 m**; a 500,000-track pier survey takes **~29 days**, not 6–8 hours. Matching the claim needs a ~100 m² detector. |
| **128 channels × 75 ps TDC** | 128 taps × 128 channels needs **4,096 `CARRY4`** primitives. The specified Artix-7 has ~200; even the largest Artix-7 is 5.5× short. |
| **> 24 h on a 150 Wh pack** | 150 Wh ÷ 12.5 W = **12 h**. Reaching 24 h needs ~300 Wh ≈ 3.5 kg of cells, which breaks the mass budget. |
| **< 9.5 kg per station** | The design's own momentum spectrometer needs **two 5 mm lead plates = 28.4 kg**. The mass budget omits them entirely. |
| **passive lead momentum measurement** | One multiple-scattering sample gives O(1) relative error. Measuring momentum needs redundant tracking planes and a kink fit — a different detector. |

The pattern matters more than any single row: each number was designed in
isolation and none were ever multiplied together.

---

## What the software establishes

The instrument software was built to test the design rather than to illustrate
it, and it is the reason the table above has numbers in it.

```
EventSource ──bytes──> PacketReader ──EventRecord──> TdcCalibration
 sim │ file │ serial      CRC + framing                data-driven deskew
        ──> TrackFitter ──Track──> MarginalizedMLEM ──volume──> OBJ / IFC / DICOM
              fitted angles          prior-marginalised
```

Two properties make it a real test rather than a simulation of one:

**The reconstruction never sees truth.** An earlier version was handed each
event's true momentum by the simulator. A two-station hodoscope cannot measure
momentum, so that was an oracle, and the reported accuracy was partly a property
of the simulator. Momentum is now marginalised over the known cosmic-ray
spectrum. The simulator's ground truth is exposed on a side channel that
`MuonScopeSystem.run()` cannot reach.

**The same code runs on any byte source.** The simulator encodes through the
same wire encoder a real front end would have to match, so a passing run
exercises the real decoder, calibration, tracker and reconstructor.

### Measured, not asserted

Through the detector model, at 16³ and 80,000 incident muons
([`docs/ACCURACY_MEASURED.md`](docs/ACCURACY_MEASURED.md)):

| quantity | value |
|---|---|
| Detector acceptance | **18.0 %** (specification claimed 23.8–28.8 %) |
| Lead CNR, prior-marginalised | **1.41** (specification claimed 5.92 and 27.60) |
| Lead CNR, with an oracle | 3.41 — and that is an upper bound, not achievable |
| Angular resolution | **15.15 mrad** |
| TDC bin width, recovered from a pulser run | **74.63 ps** against an injected 75 ps |
| Deskew residual | **82.6 ps** against 80 ps jitter |
| Median time of flight | **2.09 ns** across a 600 mm baseline |

The calibration and timing numbers are the software working correctly. The
resolution and contrast numbers are the specification failing.

---

## What exists, and what does not

| | |
|---|---|
| Simulation, detector model, tracking, reconstruction, export | **works, tested** |
| Wire protocol and API contracts | **implemented**, generated into docs |
| Client viewer | **works**, reads only real data |
| FPGA firmware | **stub.** The TDC is never instantiated; `carry_chain_out` is never assigned; no `CARRY4`, no `generate`. The MCU readout interface is `spi_miso <= spi_mosi`. |
| Analog front end | **does not exist.** No schematic, no PCB, no layout. |
| Mechanical design | **does not exist.** No CAD, no drawings, no toleranced parts. |
| Bill of materials | **prose only.** No orderable part numbers, no quotes, no lead times. |
| Detectors, scintillator, SiPMs | **none.** |
| Muons detected | **zero.** |

**Readiness: TRL 2–3.** Critical function demonstrated in simulation only.

---

## Reproduce it

```bash
# the instrument software end to end, on simulated detector bytes
py src/inversion/run_system.py --source sim --events 20000 --grid 16

# accuracy against phantom truth, all momentum treatments
py benchmarks/accuracy_harness.py --events 80000 --grid 16

# the absolute-units gate and the 20-check system audit
py tests/validation_scripts/accuracy_gate.py
py tests/validation_scripts/automated_system_audit.py

# everything
py tests/validation_scripts/run_all_validations.py
cd src/inversion && py -m pytest tests -q
```

Python 3.10+. `numpy`, `scipy`, `pyyaml` are required; `fastapi` for the server,
`pydicom` to cross-check the DICOM export, `node` to build the viewer.

---

## Repository map

| path | what |
|---|---|
| [`docs/HARDWARE_READINESS_ASSESSMENT.md`](docs/HARDWARE_READINESS_ASSESSMENT.md) | **Start here.** The gap analysis with arithmetic. |
| [`docs/ACCURACY_MEASURED.md`](docs/ACCURACY_MEASURED.md) | What the code computes, and where earlier documents contradict it. |
| [`docs/CODEBASE_GUIDE.md`](docs/CODEBASE_GUIDE.md) | Module-by-module map. |
| [`docs/API_SPECIFICATION.md`](docs/API_SPECIFICATION.md), [`docs/IPC_COMMUNICATION_SPEC.md`](docs/IPC_COMMUNICATION_SPEC.md) | Generated from code; tests fail if they drift. |
| `src/inversion/`, `src/server/`, `src/client/` | The software. |
| `src/daq/`, `firmware/`, `hardware/`, `config/` | Rust numerics, firmware stubs, two hardware specs, geometry. |
| [`docs/archive/`](docs/archive/) | Superseded design and business documents. **Not results.** |

---

## If you want to continue this

The software is the asset. It will tell you in minutes whether a proposed
geometry composes, before you spend money building it.

The first real step is not another document. It is **one SiPM, one scintillator
tile, one bias supply, one amplifier and an oscilloscope** — a few thousand euros
and a few months — producing a single-photoelectron spectrum. That measurement
resolves the largest unknown in the entire model (`σ_pos`, currently an input
taken from the documents rather than measured) and everything downstream is
guesswork until it exists.

What the physics plausibly allows, if the design is re-derived rather than
carried over: **sub-metre targets, hours-scale surveys** — rebar mapping in a
column, voids in a pier cap, precast QA — at roughly 10 mm voxels and 15 mrad,
with ~12 kg, ~12 h endurance, 1 ns timing and 32–64 channels. A smaller product
than the original specification, and one whose numbers compose.

---

## Licence and provenance

Research code, provided as-is. The design documents in `docs/archive/` are
retained as design reasoning, not as a description of anything that exists.
Where they state performance as fact, treat it as a target that has since been
measured and not met.

Muon tomography itself is a real, working technique — it is sold commercially for
nuclear waste characterisation, mineral exploration, volcanology and cargo
screening. What this project established is that *this particular* specification
— backpack-portable, sub-2 mrad, hours-scale, metre-scale — does not close.
