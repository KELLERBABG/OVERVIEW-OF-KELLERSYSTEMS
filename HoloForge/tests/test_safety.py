"""Tests for the phase-3 safety layer.

Covers the laser interlock / e-stop state machine (holoforge/hardware/laser.py)
and the web API hardware-command policy (holoforge/web/server.py):
authentication via HOLOFORGE_API_TOKEN, origin checks, and safe-by-default
behavior when bound beyond loopback.
"""
from __future__ import annotations

import json
import urllib.error
import urllib.request

import pytest

from holoforge.hardware.laser import LaserController, LaserState
from holoforge.web import HoloForgeDashboardServer

# ---------------------------------------------------------------------------
# Laser interlock / e-stop state machine
# ---------------------------------------------------------------------------

class TestLaserSafetyStateMachine:

    def test_estop_latches_and_gates_all_activation(self):
        laser = LaserController(mock=True)
        laser.arm()
        laser.pulse(0.001)

        laser.emergency_stop()
        assert laser.telemetry.estop_latched is True
        assert laser.telemetry.state == LaserState.DISARMED
        assert laser.telemetry.power_mw == 0.0

        # Every activation path is refused while latched.
        with pytest.raises(RuntimeError):
            laser.arm()
        with pytest.raises(RuntimeError):
            laser.shutter_open()
        with pytest.raises(RuntimeError):
            laser.pulse(0.001)

        # disarm() is a safe action but must NOT clear the latch.
        laser.disarm()
        assert laser.telemetry.estop_latched is True
        with pytest.raises(RuntimeError):
            laser.arm()

        # Only the explicit manual reset clears it, and re-arming is separate.
        laser.reset_estop()
        assert laser.telemetry.estop_latched is False
        laser.arm()
        laser.pulse(0.001)
        assert laser.telemetry.state == LaserState.ARMED

    def test_interlock_open_forces_shutter_closed(self):
        laser = LaserController(mock=True)
        laser.arm()
        laser.shutter_open()
        assert laser.telemetry.state == LaserState.EMITTING

        # Opening the interlock mid-emission forces the shutter closed.
        laser.set_interlock(False)
        assert laser.telemetry.interlock_ok is False
        assert laser.telemetry.state != LaserState.EMITTING
        assert "SHUTTER:CLOSE" in laser.command_log

        # All activation paths are refused while the circuit is open.
        with pytest.raises(RuntimeError):
            laser.arm()
        with pytest.raises(RuntimeError):
            laser.shutter_open()
        with pytest.raises(RuntimeError):
            laser.pulse(0.001)

        # Closing the circuit restores normal operation.
        laser.set_interlock(True)
        laser.arm()
        laser.shutter_open()
        assert laser.telemetry.state == LaserState.EMITTING


# ---------------------------------------------------------------------------
# Web API hardware-command policy
# ---------------------------------------------------------------------------

def _get_json(url: str) -> dict:
    with urllib.request.urlopen(url, timeout=30) as resp:
        return json.loads(resp.read().decode("utf-8"))


def _post(url: str, payload: dict | None = None, headers: dict | None = None,
          expect_status: int = 200) -> dict:
    data = json.dumps(payload or {}).encode("utf-8")
    req = urllib.request.Request(url, data=data, headers=headers or {}, method="POST")
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            status = resp.status
            body = resp.read().decode("utf-8")
    except urllib.error.HTTPError as exc:
        status = exc.code
        body = exc.read().decode("utf-8")
    assert status == expect_status, f"expected {expect_status}, got {status}: {body}"
    return json.loads(body) if body.strip() else {}


class TestHardwareCommandPolicy:

    def test_hardware_commands_refused_when_exposed_without_token(self, tmp_path):
        srv = HoloForgeDashboardServer(
            host="0.0.0.0", port=0, mock_hardware=True, device="cpu", out_dir=str(tmp_path)
        )
        assert srv.api_token is None and srv.bound_to_loopback is False
        host, port = srv.start_background()
        try:
            base = f"http://127.0.0.1:{port}"
            _post(f"{base}/api/hardware/laser", {"action": "arm"}, expect_status=403)
            _post(f"{base}/api/hardware/stage", {"action": "home"}, expect_status=403)
            _post(f"{base}/api/print", {"speed": 100.0}, expect_status=403)
            # Read-only endpoints stay open.
            with urllib.request.urlopen(f"{base}/api/status", timeout=30) as resp:
                assert resp.status == 200
        finally:
            srv.stop()

    def test_token_grants_access_and_wrong_token_rejected(self, tmp_path, monkeypatch):
        monkeypatch.setenv("HOLOFORGE_API_TOKEN", "test-secret-token")
        srv = HoloForgeDashboardServer(
            host="0.0.0.0", port=0, mock_hardware=True, device="cpu", out_dir=str(tmp_path)
        )
        host, port = srv.start_background()
        try:
            base = f"http://127.0.0.1:{port}"
            # No token, wrong token: rejected.
            _post(f"{base}/api/hardware/laser", {"action": "arm"}, expect_status=403)
            _post(
                f"{base}/api/hardware/laser", {"action": "arm"},
                headers={"Authorization": "Bearer wrong-token"}, expect_status=403,
            )
            # Correct token: accepted.
            _post(
                f"{base}/api/hardware/laser", {"action": "arm"},
                headers={"Authorization": "Bearer test-secret-token"}, expect_status=200,
            )
        finally:
            srv.stop()

    def test_cross_origin_hardware_command_rejected(self, tmp_path):
        srv = HoloForgeDashboardServer(
            host="127.0.0.1", port=0, mock_hardware=True, device="cpu", out_dir=str(tmp_path)
        )
        host, port = srv.start_background()
        try:
            base = f"http://{host}:{port}"
            # Foreign origin: rejected even on loopback.
            _post(
                f"{base}/api/hardware/stage", {"action": "home"},
                headers={"Origin": "http://evil.example"}, expect_status=403,
            )
            # Same origin (what the bundled UI sends): accepted.
            _post(
                f"{base}/api/hardware/stage", {"action": "home"},
                headers={"Origin": base}, expect_status=200,
            )
        finally:
            srv.stop()

    def test_laser_safety_refusals_surface_as_409(self, tmp_path):
        srv = HoloForgeDashboardServer(
            host="127.0.0.1", port=0, mock_hardware=True, device="cpu", out_dir=str(tmp_path)
        )
        host, port = srv.start_background()
        try:
            base = f"http://{host}:{port}"
            _post(f"{base}/api/hardware/laser", {"action": "pulse", "duration_s": 0.001},
                  expect_status=200)
            # Latch the e-stop out-of-band, then try to activate over the API.
            srv.state.laser.emergency_stop()
            _post(f"{base}/api/hardware/laser", {"action": "arm"}, expect_status=409)
            _post(f"{base}/api/hardware/laser", {"action": "pulse", "duration_s": 0.001},
                  expect_status=409)
            # Disarm remains available as a safe action; manual reset restores.
            _post(f"{base}/api/hardware/laser", {"action": "disarm"}, expect_status=200)
            srv.state.laser.reset_estop()
            _post(f"{base}/api/hardware/laser", {"action": "arm"}, expect_status=200)
        finally:
            srv.stop()

    def test_estop_action_latches_and_gates_over_api(self, tmp_path):
        srv = HoloForgeDashboardServer(
            host="127.0.0.1", port=0, mock_hardware=True, device="cpu", out_dir=str(tmp_path)
        )
        host, port = srv.start_background()
        try:
            base = f"http://{host}:{port}"
            laser_url = f"{base}/api/hardware/laser"
            r = _post(laser_url, {"action": "arm"})
            assert r["estop_latched"] is False

            # The operator's E-Stop action engages the latch and reports the safety state.
            r = _post(laser_url, {"action": "estop"})
            assert r["estop_latched"] is True
            assert r["is_armed"] is False

            # The operator's status feed reports the latched state.
            st = _get_json(f"{base}/api/status")
            assert st["hardware"]["estop_latched"] is True

            # Activation is refused with clean 409 JSON (connection not dropped) until reset.
            err = _post(laser_url, {"action": "arm"}, expect_status=409)
            assert "latched" in err["error"]
            err = _post(laser_url, {"action": "pulse", "duration_s": 0.001}, expect_status=409)
            assert "latched" in err["error"]

            # disarm stays available but does not clear the latch.
            r = _post(laser_url, {"action": "disarm"})
            assert r["estop_latched"] is True

            # Manual reset (controller-level, deliberately not an HTTP action) restores.
            srv.state.laser.reset_estop()
            r = _post(laser_url, {"action": "arm"})
            assert r["is_armed"] is True
            assert r["estop_latched"] is False

            st = _get_json(f"{base}/api/status")
            assert st["hardware"]["estop_latched"] is False
        finally:
            srv.stop()
