"""Zero-dependency HTTP & REST API server for the HoloForge Control Center.

Provides endpoints for:
  - System & compute backend telemetry (`GET /api/status`)
  - Available photopolymer resins & 3D primitives (`GET /api/resins`)
  - Saved .hfp print jobs (`GET /api/jobs`)
  - Interactive Tomo-H-VAM hologram optimization (`POST /api/optimize`)
  - Hardware print execution (`POST /api/print`)
  - Direct hardware jogging & laser shutter control (`POST /api/hardware/stage`, `POST /api/hardware/laser`)
"""
from __future__ import annotations

import base64
import hmac
import io
import json
import os
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Any
from urllib.parse import urlparse

import numpy as np

from ..backend import detect_available_backends, get_backend
from ..config import RESINS, GridConfig, OpticsConfig, PrintJob
from ..hardware import (
    HardwarePrintRunner,
    LaserController,
    RotaryStageController,
    SLMDisplayManager,
)
from ..io_hfp import load_job, save_job
from ..optimize import TomographicHolographicOptimizer
from ..simulate import gel_level_simulation_units, predict_cure, simulate_dose
from ..volume import PRIMITIVES


def _array_to_png_b64(arr_uint8: np.ndarray) -> str:
    """Encode a 2D grayscale or 3D RGB uint8 array into a base64 PNG data URI."""
    try:
        from PIL import Image
        img = Image.fromarray(arr_uint8)
        buf = io.BytesIO()
        img.save(buf, format="PNG")
        b64 = base64.b64encode(buf.getvalue()).decode("ascii")
        return f"data:image/png;base64,{b64}"
    except Exception:
        return ""


def _render_phase_preview_b64(phase_rad: np.ndarray) -> str:
    norm = ((phase_rad + np.pi) / (2.0 * np.pi) * 255.0).clip(0, 255).astype(np.uint8)
    return _array_to_png_b64(norm)


def _render_dose_preview_b64(dose_slice: np.ndarray, target_slice: np.ndarray) -> str:
    mx = float(dose_slice.max()) if dose_slice.size else 1.0
    norm = (dose_slice / max(mx, 1e-12)).clip(0.0, 1.0)
    h, w = norm.shape
    rgb = np.zeros((h, w, 3), dtype=np.uint8)
    # Cyan/emerald thermal colormap for dose + subtle outline for target
    rgb[..., 0] = (norm ** 1.5 * 80).astype(np.uint8)
    rgb[..., 1] = (norm * 230).astype(np.uint8)
    rgb[..., 2] = (np.sqrt(norm) * 255).astype(np.uint8)
    edge = target_slice.astype(bool)
    rgb[edge, 0] = np.clip(rgb[edge, 0].astype(int) + 60, 0, 255).astype(np.uint8)
    return _array_to_png_b64(rgb)


def _hardware_denial_reason(headers: Any, api_token: str | None, bound_to_loopback: bool) -> str | None:
    """Return the error a hardware command must be refused with, or None to allow it.

    Policy (docs/SAFETY.md): browser-style cross-origin requests are refused;
    when HOLOFORGE_API_TOKEN is configured every hardware command must
    present it as a bearer token; without one, hardware commands are
    refused whenever the server is bound beyond 127.0.0.1.
    """
    origin = headers.get("Origin")
    if origin and urlparse(origin).netloc != headers.get("Host"):
        return "Cross-origin hardware command rejected."
    auth = headers.get("Authorization", "")
    presented = auth[len("Bearer "):] if auth.startswith("Bearer ") else ""
    if api_token:
        if not (presented and hmac.compare_digest(presented, api_token)):
            return "Hardware commands require a valid bearer token."
    elif not bound_to_loopback:
        return (
            "Hardware commands are disabled when the server is bound "
            "beyond 127.0.0.1 without HOLOFORGE_API_TOKEN configured."
        )
    return None


class DashboardState:
    """Thread-safe shared state for the HoloForge web control center."""

    def __init__(self, mock_hardware: bool = True, device: str = "auto", out_dir: str = "output"):
        self.lock = threading.Lock()
        self.mock_hardware = mock_hardware
        self.backend = get_backend(device)
        self.out_dir = out_dir

        self.stage = RotaryStageController(mock=mock_hardware)
        self.display = SLMDisplayManager(mock=mock_hardware)
        self.laser = LaserController(mock=mock_hardware)
        from ..hardware import SchlierenCameraMonitor
        self.camera = SchlierenCameraMonitor(mock=mock_hardware)
        self.runner = HardwarePrintRunner(
            stage=self.stage, display=self.display, laser=self.laser,
            camera=self.camera, mock=mock_hardware
        )

        self.uploaded_stl_path: str | None = None
        self.active_job: PrintJob | None = None
        self.active_job_path: str | None = None
        self.last_optimize_summary: dict[str, Any] | None = None
        self.print_status: dict[str, Any] = {
            "is_printing": False,
            "current_frame": 0,
            "total_frames": 0,
            "current_angle_deg": 0.0,
            "last_pulse_ms": 0.0,
            "completed_prints": 0,
            "log": ["HoloForge Control Center initialized."],
        }

    def log(self, msg: str) -> None:
        ts = time.strftime("%H:%M:%S")
        with self.lock:
            self.print_status["log"].append(f"[{ts}] {msg}")
            if len(self.print_status["log"]) > 50:
                self.print_status["log"] = self.print_status["log"][-50:]

    def get_status(self) -> dict[str, Any]:
        info = self.backend.info()
        with self.lock:
            gci = self.camera.history[-1].gelation_contrast_index if self.camera.history else 0.0
            return {
                "status": "ok",
                "compute": {
                    "device_type": info.device_type.value,
                    "backend_name": info.backend_name,
                    "device_name": info.device_name,
                    "is_gpu": info.is_gpu,
                    "available": detect_available_backends(),
                },
                "hardware": {
                    "mock": self.mock_hardware,
                    "stage_angle_deg": round(self.stage.status.current_angle_deg, 2),
                    "stage_homed": self.stage.status.is_homed,
                    "laser_armed": self.laser.telemetry.state.value in ("ARMED", "EMITTING"),
                    "laser_shutter_open": self.laser.telemetry.state.value == "EMITTING",
                    "estop_latched": self.laser.telemetry.estop_latched,
                    "laser_total_on_s": round(self.laser.telemetry.total_emission_s, 4),
                    "slm_frames_displayed": self.display.frame_count,
                    "schlieren_frames": self.camera.frames_captured,
                    "gelation_contrast_index": gci,
                },
                "print_status": dict(self.print_status),
                "active_job": self.last_optimize_summary,
                "uploaded_stl": os.path.basename(self.uploaded_stl_path) if self.uploaded_stl_path else None,
            }


class HoloForgeDashboardServer:
    """Manageable HTTP server wrapper for HoloForge UI."""

    def __init__(self, host: str = "127.0.0.1", port: int = 8080,
                 mock_hardware: bool = True, device: str = "auto",
                 out_dir: str = "output"):
        self.host = host
        self.port = port
        # Safety policy lives in _hardware_denial_reason; see docs/SAFETY.md.
        self.api_token: str | None = os.environ.get("HOLOFORGE_API_TOKEN", "").strip() or None
        self.bound_to_loopback = host in ("127.0.0.1", "localhost", "::1")
        self.state = DashboardState(mock_hardware=mock_hardware, device=device, out_dir=out_dir)
        self._httpd: ThreadingHTTPServer | None = None
        self._thread: threading.Thread | None = None

    def _make_handler(self):
        state = self.state
        api_token = self.api_token
        bound_to_loopback = self.bound_to_loopback
        static_dir = os.path.join(os.path.dirname(__file__), "static")

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, format: str, *args: Any) -> None:
                # Suppress noisy console logging during unit tests / polling
                pass

            def _send_json(self, payload: dict[str, Any], status_code: int = 200) -> None:
                raw = json.dumps(payload).encode("utf-8")
                self.send_response(status_code)
                self.send_header("Content-Type", "application/json; charset=utf-8")
                self.send_header("Content-Length", str(len(raw)))
                self.send_header("Cache-Control", "no-store")
                self.end_headers()
                self.wfile.write(raw)

            def _read_json(self) -> dict[str, Any]:
                length = int(self.headers.get("Content-Length", "0"))
                if length <= 0:
                    return {}
                raw = self.rfile.read(length).decode("utf-8")
                return json.loads(raw) if raw.strip() else {}

            def do_GET(self) -> None:
                parsed = urlparse(self.path)
                path = parsed.path

                if path == "/api/status":
                    self._send_json(state.get_status())
                    return

                if path == "/api/resins":
                    resins_dict = {
                        k: {
                            "name": r.name,
                            "gel_dose_mJ_cm2": r.gel_dose_mJ_cm2,
                            "absorption_per_mm": r.absorption_per_mm,
                            "scatter_length_um": r.scatter_length_um,
                            "index": r.index,
                        }
                        for k, r in RESINS.items()
                    }
                    self._send_json({
                        "resins": resins_dict,
                        "shapes": sorted(PRIMITIVES) + ["dental", "microfluidics", "lens"],
                    })
                    return

                if path == "/api/jobs":
                    jobs = []
                    if os.path.isdir(state.out_dir):
                        for root, _, files in os.walk(state.out_dir):
                            for f in sorted(files):
                                if f.endswith(".hfp"):
                                    full = os.path.join(root, f)
                                    jobs.append({
                                        "name": f,
                                        "path": full,
                                        "size_kb": round(os.path.getsize(full) / 1024.0, 1),
                                    })
                    self._send_json({"jobs": jobs})
                    return

                # Serve static index.html
                if path in ("/", "/index.html"):
                    index_path = os.path.join(static_dir, "index.html")
                    if os.path.isfile(index_path):
                        with open(index_path, "rb") as fh:
                            content = fh.read()
                        self.send_response(200)
                        self.send_header("Content-Type", "text/html; charset=utf-8")
                        self.send_header("Content-Length", str(len(content)))
                        self.end_headers()
                        self.wfile.write(content)
                        return

                self._send_json({"error": f"Not found: {path}"}, status_code=404)

            def do_POST(self) -> None:
                parsed = urlparse(self.path)
                path = parsed.path
                try:
                    body = self._read_json()
                except Exception as exc:
                    self._send_json({"error": f"Invalid JSON: {exc}"}, status_code=400)
                    return

                if path in ("/api/hardware/laser", "/api/hardware/stage", "/api/print"):
                    reason = _hardware_denial_reason(self.headers, api_token, bound_to_loopback)
                    if reason is not None:
                        self._send_json({"error": reason}, status_code=403)
                        return

                if path == "/api/upload-stl":
                    self._handle_upload_stl(body)
                    return

                if path == "/api/optimize":
                    self._handle_optimize(body)
                    return

                if path == "/api/print":
                    self._handle_print(body)
                    return

                if path == "/api/hardware/stage":
                    self._handle_stage(body)
                    return

                if path == "/api/hardware/laser":
                    self._handle_laser(body)
                    return

                self._send_json({"error": f"Not found: {path}"}, status_code=404)

            def _handle_upload_stl(self, body: dict[str, Any]) -> None:
                filename = os.path.basename(str(body.get("filename", "uploaded.stl")))
                if not filename.lower().endswith(".stl"):
                    filename += ".stl"
                data_b64 = str(body.get("data_b64", ""))
                if not data_b64:
                    self._send_json({"error": "Missing data_b64 payload"}, status_code=400)
                    return
                if "," in data_b64:
                    data_b64 = data_b64.split(",", 1)[1]
                raw_bytes = base64.b64decode(data_b64)
                upload_dir = os.path.join(state.out_dir, "uploads")
                os.makedirs(upload_dir, exist_ok=True)
                target_path = os.path.join(upload_dir, filename)
                with open(target_path, "wb") as fh:
                    fh.write(raw_bytes)
                with state.lock:
                    state.uploaded_stl_path = target_path
                state.log(f"Uploaded custom STL mesh: '{filename}' ({len(raw_bytes)} bytes).")
                self._send_json({
                    "status": "ok",
                    "filename": filename,
                    "stl_path": target_path,
                    "bytes": len(raw_bytes),
                })

            def _handle_optimize(self, body: dict[str, Any]) -> None:
                shape_name = str(body.get("shape", "sphere"))
                stl_path = body.get("stl_path") or (state.uploaded_stl_path if shape_name == "custom_stl" else None)
                resin_name = str(body.get("resin", "hf-standard-405"))
                nxy = int(body.get("nxy", 32))
                nz = int(body.get("nz", 32))
                patterns = int(body.get("patterns", 12))
                iters = int(body.get("iters", 4))
                vat_radius = body.get("vat_radius_mm", 10.0)
                vat_radius_f = float(vat_radius) if vat_radius is not None else None

                if resin_name not in RESINS:
                    self._send_json({"error": f"Unknown resin '{resin_name}'"}, status_code=400)
                    return

                optics = OpticsConfig(slm_pixels=max(nxy * 2, 64))
                grid = GridConfig(n_xy=nxy, n_z=nz, slice_step_um=2.0)
                resin = RESINS[resin_name]
                shape_dims = (nz, nxy, nxy)

                state.log(f"Starting Tomo-H-VAM optimization: {shape_name} ({nz}x{nxy}x{nxy}, K={patterns})...")

                if stl_path and os.path.isfile(stl_path):
                    from ..volume import voxelize_stl
                    vol = voxelize_stl(stl_path, shape_dims)
                    shape_name = os.path.splitext(os.path.basename(stl_path))[0]
                elif shape_name == "dental":
                    from ..verticals import dental_crown
                    vol = dental_crown(shape=shape_dims)
                elif shape_name == "microfluidics":
                    from ..verticals import microfluidic_chip
                    vol, _ = microfluidic_chip(shape=shape_dims)
                elif shape_name == "lens":
                    from ..verticals import aspheric_lens
                    vol = aspheric_lens(shape=shape_dims)
                elif shape_name in PRIMITIVES:
                    kwargs = {"radius": 0.28} if shape_name == "sphere" else {}
                    vol = PRIMITIVES[shape_name](shape=shape_dims, **kwargs)
                else:
                    self._send_json({"error": f"Unknown shape '{shape_name}'"}, status_code=400)
                    return

                cgh = TomographicHolographicOptimizer(
                    optics, grid, resin, vol.mask,
                    n_patterns=patterns, seed=7,
                    vat_radius_mm=vat_radius_f,
                    device=state.backend.requested_device,
                )
                opt_res = cgh.optimize(iterations=iters, verbose=False)
                dose = simulate_dose(
                    opt_res.phases_rad, opt_res.pattern_weights,
                    optics, grid, resin, angles_deg=cgh.angles_deg
                )
                gel = gel_level_simulation_units(resin, optics, grid, opt_res.exposure_s)
                cured = predict_cure(dose, gel)

                os.makedirs(state.out_dir, exist_ok=True)
                job_path = os.path.join(state.out_dir, f"ui_{shape_name}.hfp")
                job = PrintJob(
                    optics=optics, grid=grid, resin=resin, target_mask=vol.mask,
                    phases_rad=opt_res.phases_rad, pattern_weights=opt_res.pattern_weights,
                    exposure_s=opt_res.exposure_s, rotation_angles_deg=cgh.angles_deg,
                    metrics=dict(opt_res.final_metrics),
                    meta={"name": shape_name, "mode": "tomographic", "vat_radius_mm": vat_radius_f},
                )
                save_job(job, job_path)

                mid_z = nz // 2
                phase_b64 = _render_phase_preview_b64(opt_res.phases_rad[0])
                dose_b64 = _render_dose_preview_b64(dose[mid_z], vol.mask[mid_z])
                schlieren_rgb, sch_metrics = state.camera.synthesize_schlieren_frame(dose, gel, exposure_fraction=1.0)
                schlieren_b64 = _array_to_png_b64(schlieren_rgb)

                # Extract decimated 3D voxel point cloud of cured object for 3D viewer
                pts_z, pts_y, pts_x = np.nonzero(cured)
                max_pts = 1500
                if len(pts_z) > max_pts:
                    idx = np.linspace(0, len(pts_z) - 1, max_pts).astype(int)
                    pts_z, pts_y, pts_x = pts_z[idx], pts_y[idx], pts_x[idx]
                voxels_norm = np.stack([
                    (pts_x / max(nxy - 1, 1)) - 0.5,
                    (pts_y / max(nxy - 1, 1)) - 0.5,
                    (pts_z / max(nz - 1, 1)) - 0.5,
                ], axis=-1).round(3).tolist()

                summary = {
                    "name": shape_name,
                    "resin": resin_name,
                    "job_path": job_path,
                    "grid": [nz, nxy, nxy],
                    "patterns": patterns,
                    "exposure_s": round(opt_res.exposure_s, 4),
                    "elapsed_s": round(opt_res.elapsed_s, 3),
                    "vat_radius_mm": vat_radius_f,
                    "metrics": {k: round(float(v), 4) for k, v in opt_res.final_metrics.items()},
                    "angles_deg": [round(a, 1) for a in cgh.angles_deg],
                    "phase_preview": phase_b64,
                    "dose_preview": dose_b64,
                    "schlieren_preview": schlieren_b64,
                    "gelation_contrast_index": sch_metrics.gelation_contrast_index,
                    "voxels": voxels_norm,
                }
                with state.lock:
                    state.active_job = job
                    state.active_job_path = job_path
                    state.last_optimize_summary = summary

                state.log(
                    f"Optimization complete: IoU={summary['metrics']['cure_iou']*100:.1f}%, "
                    f"Bleed={summary['metrics']['bleed_ratio']:.3f}, Exposure={summary['exposure_s']*1000:.1f}ms"
                )
                self._send_json({"status": "ok", "job": summary})

            def _handle_print(self, body: dict[str, Any]) -> None:
                job_path = body.get("job_path")
                speed = float(body.get("speed", 20.0))

                with state.lock:
                    job = state.active_job
                if job_path and os.path.isfile(job_path):
                    job = load_job(job_path)

                if job is None:
                    self._send_json({"error": "No active PrintJob loaded. Run optimization first."}, status_code=400)
                    return

                state.log(f"Executing hardware print sequence for '{job.meta.get('name', 'job')}'...")
                with state.lock:
                    state.print_status["is_printing"] = True
                    state.print_status["total_frames"] = len(job.pattern_weights)

                def on_prog(frame: int, total: int, angle: float, dur_s: float) -> None:
                    with state.lock:
                        state.print_status["current_frame"] = frame
                        state.print_status["total_frames"] = total
                        state.print_status["current_angle_deg"] = round(angle, 1)
                        state.print_status["last_pulse_ms"] = round(dur_s * 1000.0, 2)

                try:
                    run_summary = state.runner.execute_print(job, progress_callback=on_prog, speed_factor=speed)
                except RuntimeError as exc:
                    with state.lock:
                        state.print_status["is_printing"] = False
                    self._send_json({"error": str(exc)}, status_code=409)
                    return
                with state.lock:
                    state.print_status["is_printing"] = False
                    state.print_status["completed_prints"] += 1

                state.log(
                    f"Print finished: {run_summary.num_patterns} angles exposed in "
                    f"{run_summary.elapsed_wall_time_s:.2f}s wall time."
                )
                self._send_json({
                    "status": "ok",
                    "print_summary": {
                        "job_name": run_summary.job_name,
                        "patterns_printed": run_summary.num_patterns,
                        "total_exposure_s": round(run_summary.total_exposure_s, 4),
                        "elapsed_wall_time_s": round(run_summary.elapsed_wall_time_s, 3),
                        "final_angle_deg": run_summary.final_angle_deg,
                    },
                })

            def _handle_stage(self, body: dict[str, Any]) -> None:
                action = body.get("action", "home")
                try:
                    if action == "home":
                        state.stage.home()
                        state.log("Rotary stage homed to 0.0°.")
                    elif action == "move":
                        angle = float(body.get("angle_deg", 0.0))
                        state.stage.rotate_to(angle, wait=False)
                        state.log(f"Rotary stage moved to {state.stage.status.current_angle_deg:.1f}°.")
                    else:
                        self._send_json({"error": f"Unknown stage action '{action}'"}, status_code=400)
                        return
                except RuntimeError as exc:
                    self._send_json({"error": str(exc)}, status_code=409)
                    return
                self._send_json({"status": "ok", "angle_deg": state.stage.status.current_angle_deg})

            def _handle_laser(self, body: dict[str, Any]) -> None:
                action = body.get("action", "disarm")
                try:
                    if action == "arm":
                        state.laser.arm()
                        state.log("405 nm Laser ARMED.")
                    elif action == "disarm":
                        state.laser.disarm()
                        state.log("405 nm Laser DISARMED (safe).")
                    elif action == "estop":
                        state.laser.emergency_stop()
                        state.log("EMERGENCY STOP latched: shutter closed, power dropped; "
                                  "requires manual reset_estop().")
                    elif action == "pulse":
                        dur = float(body.get("duration_s", 0.02))
                        if state.laser.telemetry.state.value == "DISARMED":
                            state.laser.arm()
                        state.laser.pulse(dur)
                        state.log(f"Laser test pulse fired ({dur*1000:.1f} ms).")
                    else:
                        self._send_json({"error": f"Unknown laser action '{action}'"}, status_code=400)
                        return
                except RuntimeError as exc:
                    self._send_json({"error": str(exc)}, status_code=409)
                    return
                self._send_json({
                    "status": "ok",
                    "is_armed": state.laser.telemetry.state.value in ("ARMED", "EMITTING"),
                    "estop_latched": state.laser.telemetry.estop_latched,
                    "total_on_time_s": round(state.laser.telemetry.total_emission_s, 4),
                })

        return Handler

    def start_background(self) -> tuple[str, int]:
        """Start the HTTP server on a background daemon thread. Returns (host, bound_port)."""
        handler_cls = self._make_handler()
        self._httpd = ThreadingHTTPServer((self.host, self.port), handler_cls)
        self.port = self._httpd.server_address[1]
        self._thread = threading.Thread(target=self._httpd.serve_forever, daemon=True)
        self._thread.start()
        return self.host, self.port

    def stop(self) -> None:
        if self._httpd is not None:
            self._httpd.shutdown()
            self._httpd.server_close()
            self._httpd = None
        if self._thread is not None:
            self._thread.join(timeout=2.0)
            self._thread = None


def start_dashboard_server(host: str = "127.0.0.1", port: int = 8080,
                           mock_hardware: bool = True, device: str = "auto",
                           open_browser: bool = True) -> int:
    """Run the HoloForge Web Control Center in the foreground until Ctrl+C."""
    srv = HoloForgeDashboardServer(host=host, port=port, mock_hardware=mock_hardware, device=device)
    bound_host, bound_port = srv.start_background()
    url = f"http://{bound_host}:{bound_port}"
    print(f"\n{'='*70}")
    print(f"  HoloForge Web Control Center running at: {url}")
    print(f"  Compute Backend : {srv.state.backend.info().device_name}")
    print(f"  Hardware Mode   : {'MOCK / SIMULATOR' if mock_hardware else 'LIVE HARDWARE'}")
    print("  Press Ctrl+C to stop server.")
    print(f"{'='*70}\n")

    if open_browser:
        try:
            import webbrowser
            webbrowser.open(url)
        except Exception:
            pass

    try:
        while True:
            time.sleep(0.5)
    except KeyboardInterrupt:
        print("\nShutting down HoloForge Control Center...")
        srv.stop()
    return 0
