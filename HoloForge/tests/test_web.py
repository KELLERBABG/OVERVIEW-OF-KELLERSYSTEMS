"""Unit tests for Tier 4: HoloForge Web & Touchscreen Control Center."""
from __future__ import annotations

import json
import urllib.request

from holoforge.web import HoloForgeDashboardServer


def _get_json(url: str) -> dict:
    # Timeouts are generous: CI runners are slow and this suite drives a
    # full hologram optimization on the same machine.
    with urllib.request.urlopen(url, timeout=30) as resp:
        assert resp.status == 200
        return json.loads(resp.read().decode("utf-8"))


def _post_json(url: str, payload: dict) -> dict:
    data = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(
        url, data=data, headers={"Content-Type": "application/json"}, method="POST"
    )
    with urllib.request.urlopen(req, timeout=120) as resp:
        assert resp.status == 200
        return json.loads(resp.read().decode("utf-8"))


def test_web_dashboard_server_endpoints(tmp_path):
    srv = HoloForgeDashboardServer(
        host="127.0.0.1", port=0, mock_hardware=True, device="cpu", out_dir=str(tmp_path)
    )
    host, port = srv.start_background()
    base_url = f"http://{host}:{port}"

    try:
        # 1. Index HTML page loads
        with urllib.request.urlopen(f"{base_url}/", timeout=30) as resp:
            html = resp.read().decode("utf-8")
            assert resp.status == 200
            assert "HoloForge // Tomo-H-VAM Control Center" in html

        # 2. Status endpoint
        status = _get_json(f"{base_url}/api/status")
        assert status["status"] == "ok"
        assert status["compute"]["backend_name"] == "numpy"
        assert status["hardware"]["mock"] is True

        # 3. Resins & shapes endpoint
        resins_info = _get_json(f"{base_url}/api/resins")
        assert "hf-standard-405" in resins_info["resins"]
        assert "sphere" in resins_info["shapes"]
        assert "dental" in resins_info["shapes"]

        # 4. Stage & Laser hardware endpoints
        st_res = _post_json(f"{base_url}/api/hardware/stage", {"action": "move", "angle_deg": 90.0})
        assert st_res["angle_deg"] == 90.0

        ls_res = _post_json(f"{base_url}/api/hardware/laser", {"action": "pulse", "duration_s": 0.01})
        assert ls_res["total_on_time_s"] >= 0.01
        _post_json(f"{base_url}/api/hardware/laser", {"action": "disarm"})

        # 5. Optimize endpoint (synthesizes hologram, previews, and saves .hfp)
        opt_res = _post_json(
            f"{base_url}/api/optimize",
            {
                "shape": "sphere",
                "resin": "hf-standard-405",
                "nxy": 32,
                "nz": 32,
                "patterns": 12,
                "iters": 2,
                "vat_radius_mm": 10.0,
            },
        )
        assert opt_res["status"] == "ok"
        job_info = opt_res["job"]
        assert job_info["metrics"]["cure_iou"] > 0.85
        assert job_info["phase_preview"].startswith("data:image/png;base64,")
        assert job_info["dose_preview"].startswith("data:image/png;base64,")
        assert len(job_info["voxels"]) > 0

        # 6. Jobs listing endpoint
        jobs = _get_json(f"{base_url}/api/jobs")
        assert len(jobs["jobs"]) == 1

        # 7. Hardware print execution endpoint
        pr_res = _post_json(f"{base_url}/api/print", {"speed": 100.0})
        assert pr_res["status"] == "ok"
        assert pr_res["print_summary"]["patterns_printed"] == 12
    finally:
        srv.stop()
