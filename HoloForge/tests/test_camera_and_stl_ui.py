"""Unit tests for Tier 5 Schlieren Camera Monitor and Web UI STL Upload."""
from __future__ import annotations

import base64
import json
import os
import urllib.request

import numpy as np

from holoforge.config import RESINS, GridConfig, OpticsConfig, PrintJob
from holoforge.hardware import HardwarePrintRunner, SchlierenCameraMonitor
from holoforge.volume import export_stl, sphere
from holoforge.web import HoloForgeDashboardServer


def _post_json(url: str, payload: dict) -> dict:
    data = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(
        url, data=data, headers={"Content-Type": "application/json"}, method="POST"
    )
    with urllib.request.urlopen(req, timeout=30) as resp:
        assert resp.status == 200
        return json.loads(resp.read().decode("utf-8"))


def test_schlieren_camera_monitor_synthesis():
    cam = SchlierenCameraMonitor(mock=True, gel_contrast_threshold=0.55)
    vol = sphere(shape=(24, 24, 24), radius=0.3).mask.astype(np.float64)
    dose = vol * 1.4  # above gel_dose=1.0 inside sphere

    # Early exposure (20%): below oxygen inhibition / gel threshold
    rgb_early, m_early = cam.synthesize_schlieren_frame(dose, gel_dose=1.0, exposure_fraction=0.2)
    assert rgb_early.shape == (24, 24, 3)
    assert rgb_early.dtype == np.uint8
    assert m_early.gelation_detected is False
    assert m_early.gelation_contrast_index < 0.3

    # Full exposure (100%): cured core -> strong delta_n Schlieren contrast
    rgb_full, m_full = cam.synthesize_schlieren_frame(dose, gel_dose=1.0, exposure_fraction=1.0)
    assert m_full.gelation_detected is True
    assert m_full.gelation_contrast_index >= 0.55
    assert m_full.delta_n_max > 0.02


def test_hardware_runner_auto_stop_on_gel():
    optics = OpticsConfig(slm_pixels=64)
    grid = GridConfig(n_xy=24, n_z=24)
    resin = RESINS["hf-standard-405"]
    target = sphere(shape=(24, 24, 24), radius=0.28).mask
    K = 10
    job = PrintJob(
        optics=optics,
        grid=grid,
        resin=resin,
        target_mask=target,
        phases_rad=np.zeros((K, 24, 24)),
        pattern_weights=[1.0 / K] * K,
        exposure_s=0.05,
        rotation_angles_deg=list(np.linspace(0.0, 360.0, K, endpoint=False)),
    )
    # High dose so gelation triggers right at 50% of patterns (frame 5 of 10)
    dose = target.astype(np.float64) * 2.5
    runner = HardwarePrintRunner(mock=True)
    summary = runner.execute_print(
        job,
        speed_factor=100.0,
        auto_stop_on_gel=True,
        precomputed_dose=dose,
        gel_dose=1.0,
    )
    assert summary.success is True
    assert summary.auto_stopped_on_gel is True
    assert summary.num_patterns < K
    assert summary.gelation_contrast_index >= 0.55


def test_web_stl_upload_and_schlieren_preview(tmp_path):
    # Export a test binary STL from a sphere volume
    stl_file = os.path.join(str(tmp_path), "sample_part.stl")
    vol = sphere(shape=(20, 20, 20), radius=0.3)
    export_stl(vol, stl_file)
    with open(stl_file, "rb") as fh:
        b64_stl = base64.b64encode(fh.read()).decode("ascii")

    srv = HoloForgeDashboardServer(
        host="127.0.0.1", port=0, mock_hardware=True, device="cpu", out_dir=str(tmp_path)
    )
    host, port = srv.start_background()
    base_url = f"http://{host}:{port}"

    try:
        up_res = _post_json(
            f"{base_url}/api/upload-stl",
            {"filename": "sample_part.stl", "data_b64": b64_stl},
        )
        assert up_res["status"] == "ok"
        assert up_res["filename"] == "sample_part.stl"

        opt_res = _post_json(
            f"{base_url}/api/optimize",
            {
                "shape": "custom_stl",
                "resin": "hf-standard-405",
                "nxy": 24,
                "nz": 24,
                "patterns": 8,
                "iters": 1,
                "vat_radius_mm": 10.0,
            },
        )
        assert opt_res["status"] == "ok"
        job_info = opt_res["job"]
        assert job_info["name"] == "sample_part"
        assert job_info["schlieren_preview"].startswith("data:image/png;base64,")
        assert job_info["gelation_contrast_index"] > 0.4
    finally:
        srv.stop()
