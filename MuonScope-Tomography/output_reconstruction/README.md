# output_reconstruction

Everything in this directory is **generated**. Nothing here is committed, and
nothing here is evidence — the numbers it contains can be reproduced from
source, which is the point.

## What lands here

| pattern | written by |
|---|---|
| `accuracy_report*.json`, `accuracy_bands.json` | `benchmarks/accuracy_harness.py` |
| `accuracy_gate.json` | `tests/validation_scripts/accuracy_gate.py` |
| `system_run.json`, `system_*.npy` | `src/inversion/run_system.py` |
| `ground_truth.npy`, `*_reconstruction.npy` | `src/inversion/run_pipeline.py` |
| `isosurface.obj`, `reconstruction_mesh.obj` | isosurface export from the reconstructed volume |
| `structural_model.ifc`, `muonscope_model.ifc` | IFC4 export |
| `dicom_slices/*.dcm` | DICOM Part 10 series export |
| `*.msc` | raw packet captures from the acquisition chain |

## Reproduce

```bash
# the instrument software end to end, on simulator bytes
py src/inversion/run_system.py --source sim --events 20000 --grid 16

# the accuracy harness: CNR and RMSE against phantom truth
py benchmarks/accuracy_harness.py --events 80000 --grid 16

# the absolute-units gate
py tests/validation_scripts/accuracy_gate.py

# the full pipeline including IFC / OBJ / DICOM exports
py src/inversion/run_pipeline.py --phantom heavy_inclusion --events 20000 --grid-size 16
```

The measured values these produce are recorded in
[`docs/ACCURACY_MEASURED.md`](../docs/ACCURACY_MEASURED.md).
