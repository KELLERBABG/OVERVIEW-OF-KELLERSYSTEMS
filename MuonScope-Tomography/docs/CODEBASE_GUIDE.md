# Codebase & Architecture Guide

A map of what is in this repository and how the pieces connect. Line counts are
approximate and include comments.

## The one thing to understand first

The instrument software is a **pipeline over bytes**. The same code path runs
whether the bytes come from a serial port, a recorded capture, or the built-in
simulator — so a simulated run exercises the real decoder, the real calibration,
the real tracker and the real reconstructor.

```
EventSource ──bytes──> PacketReader ──EventRecord──> TdcCalibration
 sim │ file │ serial      CRC + framing                data-driven deskew
        ──> TrackFitter ──Track──> MarginalizedMLEM ──volume──> OBJ / IFC / DICOM
              fitted angles          prior-marginalised
```

Nothing downstream of `PacketReader` knows where the bytes came from. Nothing
downstream of `TrackFitter` knows the detector. Nothing anywhere reads a truth
value from the simulator: the simulator's ground truth is exposed on a side
channel that `MuonScopeSystem.run()` cannot reach, and is used only by
`compare_to_truth()` for validation.

## Instrument software — `src/inversion/muonscope/`

| module | lines | responsibility |
|---|---|---|
| `wire.py` | 224 | The frame: big-endian, `27 + 9n` bytes, CRC-32/ISO-HDLC, resynchronising scanner. Deliberately cannot carry momentum or energy. |
| `acquisition.py` | 455 | `EventSource` implementations (memory, file, serial), the incremental framing reader, capture writer, and `SimulatedSource` — the instrument simulator, which emits real wire bytes. |
| `tracking.py` | 498 | Global channel map; `TdcCalibration` (bin width by code density on a pulser run, per-channel deskew); `TrackFitter` (charge-weighted centroiding, per-station line fits). |
| `system.py` | 281 | `MuonScopeSystem`: acquire → calibrate → track → reconstruct, plus `compare_to_truth()` kept outside the processing path. |
| `physics/momentum_prior.py` | 142 | The Gaisser momentum spectrum as quadrature nodes — the prior the reconstruction marginalises over, replacing the per-event momentum oracle. |
| `physics/gaisser.py` | 101 | Gaisser flux and rejection sampling of the muon spectrum. |
| `physics/scattering.py`, `energy_loss.py`, `capture.py` | ~110 | Highland MCS, Bethe-Bloch, μ⁻ capture lifetimes. |
| `physics/environmental_calibration.py` | 116 | 1-D Kalman barometric/thermal flux normalisation. |
| `simulation/geometry.py` | 235 | Loads and validates `config/detector_geometry.yaml`; the single source of truth for plane positions and channel banking. |
| `simulation/detector.py` | 390 | `HodoscopeSimulator`: strip charge sharing, position resolution, efficiency, TDC timing. Produces `MeasuredTrack` — observables only. |
| `simulation/ray_tracer.py`, `synthetic_scene.py` | ~260 | Forward Monte Carlo through voxel phantoms. |
| `simulation/detector_sim.py` | 111 | **Deprecated** — a second, contradictory detector model. Not exported; see its docstring. |
| `reconstructors/marginalized.py` | 234 | **The primary solver.** Iteratively reweighted non-negative solve with momentum marginalised; physical λ bound at uranium. |
| `reconstructors/normalized.py`, `map_em.py`, `poca.py`, `siddon.py` | ~390 | Baselines and the system matrix: path-normalised one-shot, MLEM/MAP-EM, PoCA, Siddon ray-voxel intersection. |
| `reconstructors/momentum_est.py` | 52 | Single-sample momentum estimate. Documented as **not a measurement**; retained as a diagnostic. |
| `reconstructors/joint_grav.py` | 57 | Muon + microgravimetry joint solve (frontier, lightly used). |
| `export/mesh_vtk.py` | 271 | Marching tetrahedra (watertight, no optional dependency), OBJ/PLY. |
| `export/dicom_export.py` | 315 | DICOM Part 10 writer and a minimal reader, so exports can be verified without a third-party parser. |
| `export/bim_ifc.py` | 313 | IFC4 with `IfcTriangulatedFaceSet` geometry, `IfcVoidingFeature`, spatial structure. |

### Entry points

| file | what it does |
|---|---|
| `run_system.py` (190) | The instrument: `--source sim\|file\|serial`, acquires, calibrates, tracks, reconstructs, reports measured values. |
| `run_pipeline.py` (193) | Shipped pipeline: routes through the system and writes OBJ / IFC / DICOM. |
| `src/server/api_server.py` (336) | FastAPI surface over the same chain. Contract generated into `docs/API_SPECIFICATION.md`. |

## Supporting trees

| path | state |
|---|---|
| `src/client/` | React + Three.js viewer. Every panel reads from the API or shows an explicit empty state; no fixture data. `dist/` is gitignored — build it. |
| `src/daq/` | Rust edge-DAQ numerics (Kalman, PoCA, ring buffer, barycentre). Real maths, **no hardware I/O**. Superseded in the pipeline by the Python chain. |
| `firmware/` | **Stubs.** `tdc_tdl_carry.sv` is never instantiated and `carry_chain_out` is never assigned; there is no `CARRY4`, no `generate`. The MCU readout interface is `spi_miso <= spi_mosi`. The firmware emits none of the wire frame. |
| `hardware/` | Two prose specifications. No schematic, no PCB, no CAD, no BOM spreadsheet. |
| `config/` | `detector_geometry.yaml` is loaded and validated by the code. `inversion_params.yaml` and `materials_database.yaml` are **not read by any code**. |
| `benchmarks/` | `accuracy_harness.py` — CNR/RMSE against phantom truth for several momentum treatments, with the oracle labelled as unachievable. |
| `tests/validation_scripts/` | `automated_system_audit.py` (20 evaluated gates), `accuracy_gate.py` (absolute units, oracle absence), `run_all_validations.py`. |
| `tools/` | `generate_contract_docs.py` (regenerates the API and wire-protocol docs from code), `verify_true_simulation.py`. |
| `src/inversion/tests/` | 56 tests: wire round-trip and framing, geometry validation, calibration recovery, tracking, detector model, end-to-end, API. |

## Conventions worth knowing

- **Generated output is not committed.** `output_reconstruction/` is gitignored;
  the numbers are reproduced by re-running the commands.
- **Contract documents are generated.** `docs/API_SPECIFICATION.md` and
  `docs/IPC_COMMUNICATION_SPEC.md` come from `tools/generate_contract_docs.py`,
  and tests fail if they drift from the code.
- **Validation is not a claim.** `docs/ACCURACY_MEASURED.md` records measured
  values and notes where earlier documents contradict them.
- **Superseded documents live in `docs/archive/`** and are not results.

## Running it

```bash
py src/inversion/run_system.py --source sim --events 20000 --grid 16
py benchmarks/accuracy_harness.py --events 80000 --grid 16
py tests/validation_scripts/run_all_validations.py
cd src/inversion && py -m pytest tests -q
```
