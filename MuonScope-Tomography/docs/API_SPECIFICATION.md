# MuonScope HTTP & WebSocket API

> **Generated from the running application's OpenAPI schema by
> `tools/generate_contract_docs.py`. Do not edit by hand -- regenerate it.**

A single FastAPI application. It runs the instrument software end to end on
simulated detector bytes: forward physics, the detector model, the wire
encoder, the framing reader, TDC calibration, track fitting and a
prior-marginalised reconstruction.

**No per-event momentum is accepted or reported.** A two-station hodoscope
measures directions and time, not momentum, so the API has no field for it.
The reconstruction marginalises over the cosmic-ray momentum spectrum.

Default bind: `0.0.0.0:8000`.

## Endpoints

### `GET /api/v1/poca-cloud`

Get Poca Cloud

**Responses**

* `200` -- Successful Response (`object`)

### `POST /api/v1/run-reconstruction`

Run Reconstruction

Run the instrument software end to end and report what it measured.

Drives the real chain -- simulator -> wire bytes -> framing -> calibration ->
tracking -> prior-marginalised reconstruction -- and computes every metric
from the result against the phantom's own geometry. No quantity taken from
the simulator reaches the reconstructor.

**Request body:** `InversionRequest`

| field | type | default |
|---|---|---|
| `num_muons` | integer | `2500` |
| `preset` | string | `heavy_inclusion` |
| `grid_res` | integer | `32` |
| `map_em_iterations` | integer | `15` |
| `regularization_weight` | number | `0.05` |
| `use_map_em` | boolean | `True` |

**Responses**

* `200` -- Successful Response (`object`)
* `422` -- Validation Error (`HTTPValidationError`)

### `GET /api/v1/status`

Get Status

**Responses**

* `200` -- Successful Response (`object`)

### `GET /api/v1/telemetry`

Get Telemetry

Measured state of the station from the most recent run.

Returns ``stations: []`` when nothing has run. An empty list is the honest
answer; the front end must not substitute sample readings.

**Responses**

* `200` -- Successful Response (`object`)

### `GET /api/v1/volume-raw`

Get Volume Raw

Returns the full 3D reconstructed density array for 3D WebGL volumetric texture upload.

**Responses**

* `200` -- Successful Response (`object`)

### `GET /api/v1/volume-slice`

Get Volume Slice

**Responses**

* `200` -- Successful Response (`object`)
* `422` -- Validation Error (`HTTPValidationError`)

## WebSocket

### `WS /ws/live-stream`

Streams measured tracks as the instrument produces them, roughly 10 per
80 ms. Each message is a JSON object:

| field | meaning |
|---|---|
| `type` | always `MUON_EVENT` |
| `timestamp_ns` | wall-clock time of transmission |
| `angle_mrad` | fitted scattering angle, milliradians |
| `sigma_angle_mrad` | uncertainty on that angle, from the geometry |
| `tof_ns` | time of flight between the stations |
| `p_in`, `p_out` | fitted entry and exit points, millimetres |
| `n_planes` | planes that contributed to the fit |
| `chi2_ndf_dof` | degrees of freedom of the fit; see below |

There is deliberately no `energy_gev` and no truth-derived quality flag:
the hodoscope measures neither.

## A note on the chi-squared cut

The design documents specify a track-quality cut at `chi2/ndf <= 2.5`. With
the shipped geometry each station fits a straight line through two layers per
coordinate, so the fit has **zero degrees of freedom** and no chi-squared
exists. The API reports `chi2_ndf_dof: 0` rather than inventing a figure.
Adding a third layer per coordinate would make the cut possible.

## Not implemented

Appearing in an earlier version of this document and never built:
`/api/v1/system/status`, `/api/v1/survey/start`, `/api/v1/survey/stop`,
`/api/v1/survey/export`, a WebSocket at `/ws/v1/live_stream` on port 8080, and
a gRPC `SamTMeshService` for multi-station coordination. The instrument is
modelled as a single station; there is no mesh.
