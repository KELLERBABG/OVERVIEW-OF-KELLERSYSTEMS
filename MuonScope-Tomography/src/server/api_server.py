import os
import sys
import time
import asyncio
from typing import Dict, Any, List
import numpy as np
from fastapi import FastAPI, HTTPException, WebSocket, WebSocketDisconnect
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
import uvicorn

# Add src/inversion to path
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "inversion")))

from muonscope.acquisition import PacketReader, SimulatedSource, SimulatorConfig
from muonscope.physics.gaisser import sample_cosmic_muons
from muonscope.simulation.geometry import load_detector_geometry
from muonscope.tracking import TdcCalibration, TrackFitter
from muonscope.simulation.synthetic_scene import (
    create_void_phantom,
    create_heavy_inclusion_phantom,
    create_rebar_phantom,
    VoxelScene
)
from muonscope.simulation.ray_tracer import MonteCarloRayTracer
from muonscope.reconstructors.poca import PoCAReconstructor
from muonscope.reconstructors.siddon import build_sparse_system_matrix
from muonscope.reconstructors.map_em import MAPEMReconstructor
from muonscope.system import MuonScopeSystem, SystemConfig
from muonscope.export.bim_ifc import export_to_ifc4
from muonscope.export.mesh_vtk import extract_isosurface_mesh, export_to_obj
from muonscope.export.dicom_export import export_to_dicom_series

app = FastAPI(title="MuonScope Live Inversion & Physics API")

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Global active reconstruction state
STATE = {
    "active_volume": np.zeros((32, 32, 32), dtype=np.float32),
    "grid_size": (32, 32, 32),
    "bounds_min": (-250.0, -250.0, -200.0),
    "bounds_max": (250.0, 250.0, 200.0),
    "poca_points": [],
    "metrics": {
        "num_muons": 0,
        "accepted_tracks": 0,
        "runtime_seconds": 0.0,
        "cnr_lead": 0.0,
        "cnr_iron": 0.0,
        "cnr_void": 0.0,
        "iterations": 0,
    }
}

class InversionRequest(BaseModel):
    num_muons: int = 2500
    preset: str = "heavy_inclusion"  # "heavy_inclusion", "void", "rebar"
    grid_res: int = 32
    map_em_iterations: int = 15
    regularization_weight: float = 0.05
    use_map_em: bool = True

@app.get("/api/v1/status")
def get_status():
    return {
        "status": "READY",
        "has_active_reconstruction": STATE["metrics"]["num_muons"] > 0,
        "metrics": STATE["metrics"],
        "grid_size": STATE["grid_size"],
    }

@app.post("/api/v1/run-reconstruction")
def run_reconstruction(req: InversionRequest):
    """Run the instrument software end to end and report what it measured.

    Drives the real chain -- simulator -> wire bytes -> framing -> calibration ->
    tracking -> prior-marginalised reconstruction -- and computes every metric
    from the result against the phantom's own geometry. No quantity taken from
    the simulator reaches the reconstructor.
    """
    start_time = time.time()
    grid = (req.grid_res, req.grid_res, req.grid_res)

    # 1. Create Physical Phantom
    if req.preset == "void":
        scene = create_void_phantom(grid_size=grid)
    elif req.preset == "rebar":
        scene = create_rebar_phantom(grid_size=grid)
    else:
        scene = create_heavy_inclusion_phantom(grid_size=grid)

    bounds_min = scene.bounds_min_mm
    bounds_max = scene.bounds_max_mm

    # 2. Run the real system on simulated instrument bytes. No quantity taken
    #    from the simulator reaches the reconstructor.
    geometry = load_detector_geometry()
    source = SimulatedSource(
        scene, geometry,
        config=SimulatorConfig(num_events=req.num_muons,
                               seed=int(start_time) % 100000,
                               calibration_packets=200),
    )
    system = MuonScopeSystem(
        config=SystemConfig(
            grid_size=req.grid_res,
            bounds_min_mm=tuple(float(v) for v in bounds_min),
            bounds_max_mm=tuple(float(v) for v in bounds_max),
            iterations=max(1, req.map_em_iterations),
            beta=req.regularization_weight,
        ),
        geometry=geometry,
    )
    result = system.run_from_source(source)
    final_vol = result.volumes["marginalized_mlem"]
    if result.n_tracks < 10:
        raise HTTPException(
            status_code=400,
            detail=f"only {result.n_tracks} tracks passed the coincidence and tracking cuts")

    # 3. Scattering points, from the fitted tracks only. No momentum is reported:
    #    the hodoscope cannot measure it, so exposing it would be a fake column.
    poca_points_list = []
    for track in result.tracks[:600]:
        point = 0.5 * (track.position_in_mm + track.position_out_mm)
        poca_points_list.append({
            "x": float(point[0]),
            "y": float(point[1]),
            "z": float(point[2]),
            "angle_mrad": float(track.theta_rad * 1000.0),
            "sigma_angle_mrad": float(track.sigma_theta_rad * 1000.0),
            "tof_ns": float(track.tof_ns),
        })

    # 4. Quality metrics against the phantom's own geometry, per material class.
    #    The background is "whatever is not one of the declared classes", rather
    #    than an absolute density window: a window tuned for one phantom silently
    #    selects nothing in another and the metric comes back null.
    ground_truth = scene.get_scattering_density()

    high_z_mask = ground_truth > 1.0                             # lead-like
    metal_mask = (ground_truth > 0.3) & (ground_truth <= 1.0)     # steel-like
    void_mask = ground_truth < 1e-4                              # air
    background_mask = ~(high_z_mask | metal_mask | void_mask)

    def class_cnr(mask):
        target = final_vol[mask]
        back = final_vol[background_mask]
        if target.size == 0 or back.size == 0:
            return None
        denom = float(np.sqrt(np.var(target) + np.var(back)))
        if denom <= 1e-12:
            return None
        return float(abs(np.mean(target) - np.mean(back)) / denom)

    runtime = float(time.time() - start_time)

    # 5. Export BIM & CAD artifacts from the reconstruction actually produced.
    out_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "output_reconstruction"))
    os.makedirs(out_dir, exist_ok=True)

    ifc_path = os.path.join(out_dir, "muonscope_model.ifc")
    obj_path = os.path.join(out_dir, "reconstruction_mesh.obj")
    dcm_dir = os.path.join(out_dir, "dicom_slices")

    export_to_ifc4(final_vol, bounds_min, bounds_max, ifc_path)
    hi = float(np.percentile(final_vol, 99.0))
    verts, faces = extract_isosurface_mesh(final_vol, isovalue=0.5 * hi if hi > 0 else 0.0)
    export_to_obj(verts, faces, obj_path)
    span = np.asarray(bounds_max, dtype=float) - np.asarray(bounds_min, dtype=float)
    export_to_dicom_series(final_vol, dcm_dir,
                           pixel_spacing_mm=tuple(span / np.array(grid)),
                           origin_mm=tuple(float(v) for v in bounds_min))

    # Save to Global State
    STATE["active_volume"] = final_vol
    STATE["grid_size"] = grid
    STATE["bounds_min"] = bounds_min
    STATE["bounds_max"] = bounds_max
    STATE["poca_points"] = poca_points_list
    # Real telemetry: the measured state of the station that actually ran. The
    # front end used to display four invented stations with fabricated battery,
    # temperature and flux readings, and a log asserting quantities (a chi2/ndf
    # this geometry cannot even compute) that were never measured.
    STATE["telemetry"] = {
        "stations": [{
            "id": 1,
            "name": "Station 01",
            "status": "ACTIVE" if result.n_tracks > 0 else "IDLE",
            "channel_count": geometry.total_channels(),
            "tdc_bin_ps": round(result.calibration_summary["fine_bin_ps"], 2),
            "channels_calibrated": result.calibration_summary["channels_calibrated"],
            "deskew_residual_ps": round(result.calibration_summary["residual_ps"], 1),
            "calibration_source": result.calibration_summary["source"],
            "track_efficiency": round(result.metrics["record_to_track_efficiency"] or 0.0, 3),
            "sigma_theta_mrad": round(result.metrics["sigma_theta_from_geometry_mrad"], 2),
            "tof_median_ns": (None if result.metrics["tof_median_ns"] is None
                              else round(result.metrics["tof_median_ns"], 3)),
            "lever_arm_mm": result.geometry_summary["top_lever_arm_x_mm"],
            "packets_decoded": result.reader_stats["packets_ok"],
            "packets_rejected": result.reader_stats["packets_bad"],
            "tracks": result.n_tracks,
        }],
        "log": [
            f"[SYSTEM] {result.reader_stats['packets_ok']} packets decoded, "
            f"{result.reader_stats['packets_bad']} rejected by CRC or framing.",
            f"[CALIBRATION] TDC fine bin measured at "
            f"{result.calibration_summary['fine_bin_ps']:.2f} ps from "
            f"{result.calibration_summary['records_used']} "
            f"{result.calibration_summary['source']} records.",
            f"[CALIBRATION] {result.calibration_summary['channels_calibrated']} channels deskewed, "
            f"residual {result.calibration_summary['residual_ps']:.1f} ps.",
            f"[TRACKING] {result.n_tracks} tracks fitted; chi-squared quality cut is "
            f"unavailable because each station has two layers per coordinate "
            f"(ndf = 0).",
            f"[TRACKING] Median time of flight "
            f"{result.metrics['tof_median_ns']:.3f} ns across a "
            f"{result.geometry_summary['panel_centre_separation_mm']:.0f} mm baseline."
            if result.metrics["tof_median_ns"] is not None else
            "[TRACKING] No tracks; time of flight not measured.",
            f"[RECONSTRUCTION] Prior-marginalised solver, "
            f"{result.recon_info.get('mlem', {}).get('iterations', 0)} iterations, "
            f"theta^2 resolution floor "
            f"{result.metrics['theta_sq_offset_rad2']:.3e} rad^2.",
        ],
        "note": "Single station. No mesh is deployed, so no cross-station "
                "synchronisation is reported.",
    }
    STATE["metrics"] = {
        "num_muons": req.num_muons,
        "accepted_tracks": result.n_tracks,
        "runtime_seconds": round(runtime, 2),
        "cnr_lead": None if class_cnr(high_z_mask) is None else round(class_cnr(high_z_mask), 3),
        "cnr_metal": None if class_cnr(metal_mask) is None else round(class_cnr(metal_mask), 3),
        "cnr_void": None if class_cnr(void_mask) is None else round(class_cnr(void_mask), 3),
        "iterations": result.recon_info.get("mlem", {}).get("iterations",
                                                             req.map_em_iterations),
        "momentum_treatment": "prior-marginalised (no per-event truth)",
        "tdc_bin_ps": round(result.calibration_summary["fine_bin_ps"], 2),
        "sigma_theta_mrad": round(result.metrics["sigma_theta_from_geometry_mrad"], 2),
        "track_efficiency": round(result.metrics["record_to_track_efficiency"] or 0.0, 3),
    }

    return {
        "success": True,
        "metrics": STATE["metrics"],
        "poca_points_count": len(poca_points_list),
        "volume_shape": list(final_vol.shape),
        "mean_density": float(np.mean(final_vol)),
        "max_density": float(np.max(final_vol)),
        "min_density": float(np.min(final_vol)),
    }

@app.get("/api/v1/volume-slice")
def get_volume_slice(axis: str = "z", index: int = 16):
    vol = STATE["active_volume"]
    nx, ny, nz = vol.shape

    if axis == "z":
        idx = max(0, min(index, nz - 1))
        slice_2d = vol[:, :, idx]
    elif axis == "y":
        idx = max(0, min(index, ny - 1))
        slice_2d = vol[:, idx, :]
    else:
        idx = max(0, min(index, nx - 1))
        slice_2d = vol[idx, :, :]

    return {
        "axis": axis,
        "index": idx,
        "shape": list(slice_2d.shape),
        "data": slice_2d.tolist()
    }

@app.get("/api/v1/poca-cloud")
def get_poca_cloud():
    return {
        "count": len(STATE["poca_points"]),
        "points": STATE["poca_points"]
    }

@app.get("/api/v1/telemetry")
def get_telemetry():
    """Measured state of the station from the most recent run.

    Returns ``stations: []`` when nothing has run. An empty list is the honest
    answer; the front end must not substitute sample readings.
    """
    telemetry = STATE.get("telemetry")
    if not telemetry:
        return {
            "connected": False,
            "stations": [],
            "log": [],
            "note": "No run has been performed in this process.",
        }
    return {"connected": True, **telemetry}

@app.get("/api/v1/volume-raw")
def get_volume_raw():
    """Returns the full 3D reconstructed density array for 3D WebGL volumetric texture upload."""
    vol = STATE["active_volume"]
    return {
        "grid_size": list(vol.shape),
        "min": float(np.min(vol)),
        "max": float(np.max(vol)),
        "values": vol.flatten().tolist()
    }

@app.websocket("/ws/live-stream")
async def websocket_live_stream(websocket: WebSocket):
    """Stream measured tracks as the instrument produces them.

    The stream used to carry ``energy_gev`` and a truth-derived
    ``is_heavy_scattering`` flag, neither of which the hodoscope measures. It now
    carries the same fields a real readout gives: hit positions, the fitted
    scattering angle with its uncertainty, and the time of flight.
    """
    await websocket.accept()
    geometry = load_detector_geometry()
    scene = create_heavy_inclusion_phantom(grid_size=(16, 16, 16))
    source = SimulatedSource(
        scene, geometry,
        config=SimulatorConfig(num_events=400, seed=1, calibration_packets=60),
    )
    records = list(PacketReader().records_from_source(source))
    calibration = TdcCalibration.from_records(
        records, TrackFitter(geometry).channel_map, clock_period_ns=5.0)
    fitter = TrackFitter(geometry, calibration=calibration)
    tracks = fitter.fit_all([r for r in records if not (r.flags & 0x02)])

    try:
        cursor = 0
        while True:
            # Emit a burst of the next measured tracks.
            burst = tracks[cursor:cursor + 10]
            cursor += 10
            if not burst:
                cursor = 0
                burst = tracks[:10]
            for track in burst:
                packet = {
                    "type": "MUON_EVENT",
                    "timestamp_ns": int(time.time_ns()),
                    "angle_mrad": round(float(track.theta_rad * 1000.0), 2),
                    "sigma_angle_mrad": round(float(track.sigma_theta_rad * 1000.0), 2),
                    "tof_ns": round(float(track.tof_ns), 3),
                    "p_in": [round(float(x), 1) for x in track.position_in_mm],
                    "p_out": [round(float(x), 1) for x in track.position_out_mm],
                    "n_planes": int(track.n_planes),
                    "chi2_ndf_dof": int(track.chi2_ndf_dof),
                }
                await websocket.send_json(packet)

            await asyncio.sleep(0.08)  # ~12 Hz burst stream
    except WebSocketDisconnect:
        pass
    except Exception:
        pass

if __name__ == "__main__":
    uvicorn.run(app, host="0.0.0.0", port=8000)
