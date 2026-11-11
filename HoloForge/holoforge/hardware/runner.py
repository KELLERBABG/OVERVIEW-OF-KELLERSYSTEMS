"""Synchronous Physical Print Orchestrator.

Orchestrates real-time hardware execution across the rotary stage, SLM micro-display,
and laser shutter to print a HoloForge job.
"""
from __future__ import annotations

import time
from collections.abc import Callable
from dataclasses import dataclass

import numpy as np

from ..config import PrintJob
from .camera import SchlierenCameraMonitor
from .display import SLMDisplayManager
from .laser import LaserController
from .stage import RotaryStageController


@dataclass
class PrintExecutionSummary:
    job_name: str
    num_patterns: int
    total_exposure_s: float
    elapsed_wall_time_s: float
    success: bool
    final_angle_deg: float
    gelation_contrast_index: float = 0.0
    auto_stopped_on_gel: bool = False


class HardwarePrintRunner:
    """Master controller that synchronizes all physical sub-systems during a 3D print run."""

    def __init__(
        self,
        stage: RotaryStageController | None = None,
        display: SLMDisplayManager | None = None,
        laser: LaserController | None = None,
        camera: SchlierenCameraMonitor | None = None,
        mock: bool = False,
    ):
        self.stage = stage or RotaryStageController(mock=mock)
        self.display = display or SLMDisplayManager(mock=mock)
        self.laser = laser or LaserController(mock=mock)
        self.camera = camera or SchlierenCameraMonitor(mock=mock)
        self.mock = mock

    def execute_print(
        self,
        job: PrintJob,
        progress_callback: Callable[[int, int, float, float], None] | None = None,
        speed_factor: float = 1.0,
        auto_stop_on_gel: bool = False,
        precomputed_dose: np.ndarray | None = None,
        gel_dose: float | None = None,
    ) -> PrintExecutionSummary:
        """Execute complete automated volumetric print sequence.

        Parameters
        ----------
        job : PrintJob
            The compiled print job containing phase patterns, rotation angles, and exposure timings.
        progress_callback : callable, optional
            Function called on every pattern: callback(frame_idx, total_frames, angle_deg, exposure_s).
        speed_factor : float
            Simulation speed scaling factor (1.0 = real physical timing).
        auto_stop_on_gel : bool
            When True, uses the in-situ Schlieren camera monitor to terminate exposure
            automatically once the Gelation Contrast Index crosses threshold.
        """
        t_start = time.time()
        job_name = job.meta.get("name", "unnamed_job")
        K = len(job.pattern_weights)
        weights = np.asarray(job.pattern_weights, dtype=np.float64)
        weights /= max(weights.sum(), 1e-12)

        angles = job.rotation_angles_deg
        if angles is None:
            angles = [float(a) for a in np.linspace(0.0, 360.0, K, endpoint=False)]

        total_exposure_s = job.exposure_s
        patterns_done = 0
        last_gci = 0.0
        auto_stopped = False

        try:
            # 1. Pre-flight & Homing
            self.display.blank()
            self.stage.home()
            self.laser.arm()

            # 2. Sequential Multi-Angle Exposure Loop
            for k in range(K):
                deg = float(angles[k])
                frame_exposure_s = float(total_exposure_s * weights[k]) / max(speed_factor, 1e-3)

                # A. Step rotary vat to target angle
                self.stage.rotate_to(deg, wait=True)

                # B. Present optimized phase pattern onto SLM
                self.display.display_phase(job.phases_rad[k])

                # Small settling delay for liquid crystal transition (e.g. 5ms)
                time.sleep(0.005)

                # C. Pulse laser shutter for exact duration
                self.laser.pulse(frame_exposure_s)
                patterns_done += 1

                # D. In-situ Schlieren Camera Monitoring
                if self.camera is not None and precomputed_dose is not None and gel_dose is not None:
                    frac = float(weights[: k + 1].sum())
                    _, cam_metrics = self.camera.synthesize_schlieren_frame(
                        precomputed_dose, gel_dose, exposure_fraction=frac
                    )
                    last_gci = cam_metrics.gelation_contrast_index
                    if auto_stop_on_gel and cam_metrics.gelation_detected and (k + 1) >= max(K // 2, 1):
                        auto_stopped = True
                        if progress_callback:
                            progress_callback(k + 1, K, deg, frame_exposure_s)
                        break

                if progress_callback:
                    progress_callback(k + 1, K, deg, frame_exposure_s)

            # 3. Post-Exposure Wrap-up
            self.display.blank()
            self.laser.disarm()
            self.stage.home()

            elapsed = time.time() - t_start
            return PrintExecutionSummary(
                job_name=job_name,
                num_patterns=patterns_done,
                total_exposure_s=total_exposure_s,
                elapsed_wall_time_s=elapsed,
                success=True,
                final_angle_deg=self.stage.status.current_angle_deg,
                gelation_contrast_index=last_gci,
                auto_stopped_on_gel=auto_stopped,
            )

        except Exception as e:
            self.emergency_stop()
            raise RuntimeError(f"Print execution failed during layer processing: {e}") from e

    def emergency_stop(self) -> None:
        """Trigger emergency stop across all connected hardware."""
        self.laser.emergency_stop()
        self.display.blank()
        self.stage.disconnect()
