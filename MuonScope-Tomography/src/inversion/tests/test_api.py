"""Tests for the API layer.

These guard the properties that were wrong before: a fabricated metric, a
conclusion drawn from the simulator's own truth, and telemetry invented in the
browser. They exercise the real endpoint against the real system.

Skipped when FastAPI is not installed, since it is a server dependency rather
than a library one.
"""
import os
import sys

import numpy as np
import pytest

fastapi = pytest.importorskip("fastapi")
from fastapi.testclient import TestClient  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "src", "server"))
sys.path.insert(0, os.path.join(ROOT, "src", "inversion"))

import api_server  # noqa: E402


@pytest.fixture(scope="module")
def client():
    return TestClient(api_server.app)


@pytest.fixture(scope="module")
def completed_run(client):
    """One real reconstruction, reused by the tests that inspect its output."""
    resp = client.post("/api/v1/run-reconstruction", json={
        "num_muons": 4000,
        "preset": "heavy_inclusion",
        "grid_res": 10,
        "map_em_iterations": 5,
        "use_map_em": True,
    })
    assert resp.status_code == 200, resp.text
    return resp.json()


def test_status_is_reachable(client):
    body = client.get("/api/v1/status").json()
    assert body["status"] == "READY"


def test_telemetry_is_empty_before_any_run():
    """A fresh process must not report stations it has never heard from."""
    fresh = api_server.STATE.copy()
    api_server.STATE.pop("telemetry", None)
    try:
        body = TestClient(api_server.app).get("/api/v1/telemetry").json()
        assert body["connected"] is False
        assert body["stations"] == []
        assert body["log"] == []
    finally:
        api_server.STATE.update(fresh)


def test_telemetry_reports_measured_values_after_a_run(client, completed_run):
    body = client.get("/api/v1/telemetry").json()
    assert body["connected"] is True
    assert len(body["stations"]) == 1
    station = body["stations"][0]

    # The TDC bin is recovered from the simulator's 75 ps, not asserted.
    assert station["tdc_bin_ps"] == pytest.approx(75.0, rel=0.05)
    assert station["channels_calibrated"] == station["channel_count"] == 512
    assert station["tracks"] > 0
    assert station["tracks"] == completed_run["metrics"]["accepted_tracks"]
    # The lever arm is what the geometry file specifies, and the resolution follows.
    assert station["lever_arm_mm"] == pytest.approx(140.0)
    assert station["sigma_theta_mrad"] == pytest.approx(
        np.sqrt(8.0) * 0.75 / 140.0 * 1e3, rel=0.01)
    # Time of flight across a 600 mm baseline.
    assert station["tof_median_ns"] == pytest.approx(2.0, abs=0.5)
    assert body["note"].startswith("Single station")

    # The log must not claim a chi-squared cut this geometry cannot compute.
    joined = " ".join(body["log"])
    assert "chi-squared quality cut is unavailable" in joined


def test_metrics_are_computed_not_fabricated(completed_run):
    metrics = completed_run["metrics"]
    assert metrics["cnr_lead"] is not None
    assert isinstance(metrics["cnr_lead"], float)

    # Every reported CNR must be one the endpoint actually computed from a
    # region, so it cannot be a constant multiple of another.
    for key in ("cnr_lead", "cnr_metal", "cnr_void"):
        assert key in metrics
    assert metrics["cnr_lead"] != pytest.approx((metrics["cnr_metal"] or 0) * 0.82, rel=1e-6)

    assert "prior-marginalised" in metrics["momentum_treatment"]
    assert metrics["tdc_bin_ps"] == pytest.approx(75.0, rel=0.05)
    assert 0.0 < metrics["track_efficiency"] <= 1.0


def test_reported_points_carry_no_unmeasurable_quantity(client, completed_run):
    cloud = client.get("/api/v1/poca-cloud").json()
    assert cloud["count"] == completed_run["poca_points_count"]
    assert cloud["count"] > 0
    for point in cloud["points"][:20]:
        # A hodoscope measures direction and time, not momentum or energy.
        assert "energy_gev" not in point
        assert "momentum" not in point
        assert set(point) == {"x", "y", "z", "angle_mrad", "sigma_angle_mrad", "tof_ns"}
        assert point["sigma_angle_mrad"] > 0


def test_volume_slice_matches_the_requested_grid(client, completed_run):
    body = client.get("/api/v1/volume-slice?axis=z&index=5").json()
    assert body["shape"] == [10, 10]
    assert len(body["data"]) == 10
    assert body["shape"] == list(completed_run["volume_shape"][:2])


def test_volume_raw_is_finite_and_bounded(client, completed_run):
    body = client.get("/api/v1/volume-raw").json()
    values = np.asarray(body["values"], dtype=float)
    assert values.size == int(np.prod(body["grid_size"]))
    assert np.all(np.isfinite(values))
    # No material scatters more than uranium, so nothing may exceed it.
    assert values.min() >= 0.0
    assert values.max() <= 1.0 / 0.32 + 1e-9


def test_api_specification_matches_the_real_routes():
    """The committed API document must describe the API that exists.

    It previously documented `/api/v1/system/status`, four `/api/v1/survey/*`
    endpoints, a WebSocket on port 8080 and a gRPC `SamTMeshService` -- none of
    which were ever implemented. Regenerate with `py tools/generate_api_doc.py`.
    """
    spec_path = os.path.join(ROOT, "docs", "API_SPECIFICATION.md")
    assert os.path.exists(spec_path), "API specification is missing"
    text = open(spec_path, encoding="utf-8").read()

    # The "Not implemented" section deliberately names the endpoints that were
    # never built, so that it can say so. Only the spec proper is checked.
    marker = "## Not implemented"
    assert marker in text, "the specification must record what was never built"
    spec_body = text.split(marker)[0]

    real_routes = sorted(
        r.path for r in api_server.app.routes
        if hasattr(r, "path") and r.path.startswith("/api/")
    )
    for route in real_routes:
        assert route in spec_body, f"{route} is not documented"

    # Nothing documented may be absent from the application.
    import re
    documented = set(re.findall(r"`(?:GET|POST|PUT|DELETE|WS)\s+(/[A-Za-z0-9_\-/]+)`", spec_body))
    for path in documented:
        if path.startswith("/ws"):
            continue
        assert path in real_routes, f"{path} is documented but does not exist"


def test_ipc_specification_matches_the_wire_format():
    """The committed wire document must describe the frame the code implements.

    It previously specified a fixed 32-byte packet whose own header fields filled
    all 32 bytes, so a four-plane event could not fit. Regenerate with
    `py tools/generate_contract_docs.py`.
    """
    from muonscope import wire

    spec_path = os.path.join(ROOT, "docs", "IPC_COMMUNICATION_SPEC.md")
    assert os.path.exists(spec_path), "IPC specification is missing"
    text = open(spec_path, encoding="utf-8").read()

    assert f"`0x{wire.MAGIC:08X}`" in text
    assert "| 21 | 2 | `n_hits` |" in text
    assert "23+9n" in text.replace(" ", ""), "the variable length must be stated"
    assert str(wire.MIN_PACKET_SIZE) in text, "the enforced minimum must be stated"

    # The history note names the old broken format; no *specification* of a
    # fixed-size packet may survive outside it.
    body = text.split("## Frame layout")[1] if "## Frame layout" in text else text
    assert "fixed-size" not in body.lower(), "the frame is not fixed-size"
