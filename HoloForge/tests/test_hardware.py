"""Tests for Hardware Interface Layer: Stage, Display, Laser, and Print Runner."""
import os

import numpy as np
import pytest

from holoforge.cli import main
from holoforge.config import RESINS, GridConfig, OpticsConfig, PrintJob
from holoforge.hardware.display import SLMDisplayManager
from holoforge.hardware.laser import LaserController, LaserState
from holoforge.hardware.runner import HardwarePrintRunner
from holoforge.hardware.stage import RotaryStageController
from holoforge.volume import sphere


class TestRotaryStage:
    def test_mock_stage_homing_and_rotation(self):
        stage = RotaryStageController(mock=True, settle_time_s=0.001)
        assert stage.status.is_connected
        stage.home()
        assert stage.status.is_homed
        assert stage.status.current_angle_deg == 0.0

        stage.rotate_to(45.0)
        assert stage.status.current_angle_deg == 45.0

        stage.rotate_by(90.0)
        assert stage.status.current_angle_deg == 135.0

        stage.rotate_to(390.0)  # Modulo 360
        assert stage.status.current_angle_deg == 30.0


class TestSLMDisplay:
    def test_mock_display_phase_and_blank(self):
        disp = SLMDisplayManager(mock=True, resolution=(64, 64))
        assert disp.is_open

        phase = np.random.uniform(-np.pi, np.pi, (64, 64))
        drive = disp.display_phase(phase)
        assert drive.shape == (64, 64)
        assert drive.dtype == np.uint8
        assert disp.frame_count == 1

        disp.blank()
        assert disp.current_frame.max() == 0
        disp.close()
        assert not disp.is_open


class TestLaserController:
    def test_laser_state_machine_and_pulse(self):
        laser = LaserController(mock=True, power_mw=150.0)
        assert laser.telemetry.state == LaserState.DISARMED

        laser.arm()
        assert laser.telemetry.state == LaserState.ARMED

        laser.pulse(0.005)
        assert laser.telemetry.pulse_count == 1
        assert laser.telemetry.total_emission_s > 0.0
        assert laser.telemetry.state == LaserState.ARMED

        laser.disarm()
        assert laser.telemetry.state == LaserState.DISARMED

        # Pulse while disarmed should fail
        with pytest.raises(RuntimeError):
            laser.shutter_open()


class TestHardwarePrintRunner:
    def test_end_to_end_mock_print(self):
        optics = OpticsConfig(slm_pixels=32)
        grid = GridConfig(n_xy=32, n_z=32)
        resin = RESINS["hf-standard-405"]
        vol = sphere(radius=0.28, shape=(32, 32, 32))

        job = PrintJob(
            optics=optics, grid=grid, resin=resin, target_mask=vol.mask,
            phases_rad=np.zeros((4, 32, 32)),
            pattern_weights=[0.25, 0.25, 0.25, 0.25],
            exposure_s=0.01,
            rotation_angles_deg=[0.0, 90.0, 180.0, 270.0],
            meta={"name": "test_hw_print"},
        )

        runner = HardwarePrintRunner(mock=True)
        callbacks = []

        def on_prog(f, tot, ang, dur):
            callbacks.append((f, ang))

        summary = runner.execute_print(job, progress_callback=on_prog, speed_factor=10.0)
        assert summary.success
        assert summary.num_patterns == 4
        assert len(callbacks) == 4
        assert callbacks[-1][1] == 270.0
        assert runner.laser.telemetry.state == LaserState.DISARMED
        assert runner.stage.status.current_angle_deg == 0.0


class TestCLIPrintCommand:
    def test_cli_print_mock(self, tmp_path):
        out_dir = str(tmp_path / "cli_print")
        ret_opt = main([
            "optimize",
            "--mode", "tomographic",
            "--shape", "sphere",
            "--nxy", "32",
            "--nz", "32",
            "--patterns", "4",
            "--iters", "1",
            "--vat-radius-mm", "10.0",
            "--out", out_dir,
            "--name", "print_target",
            "--quiet",
        ])
        assert ret_opt == 0
        hfp_file = os.path.join(out_dir, "print_target.hfp")
        assert os.path.exists(hfp_file)

        ret_print = main([
            "print",
            hfp_file,
            "--mock",
            "--speed", "100.0",
        ])
        assert ret_print == 0
