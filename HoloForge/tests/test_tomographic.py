"""Tests for Holographic Tomographic Volumetric Additive Manufacturing (Tomo-H-VAM).

Validates rotary 3D projection, TomographicHolographicOptimizer, high Cure IoU (>0.80),
and near-zero outside cure fraction (<0.05).
"""
import os

import numpy as np

from holoforge.cli import main
from holoforge.config import RESINS, GridConfig, OpticsConfig, PrintJob
from holoforge.io_hfp import load_job, save_job
from holoforge.optics import rotate_volume_y
from holoforge.optimize import TomographicHolographicOptimizer
from holoforge.simulate import evaluate_job
from holoforge.volume import sphere


class TestVolumeRotation:
    def test_zero_deg_identity(self):
        v = np.random.uniform(0.0, 1.0, (16, 16, 16))
        rot = rotate_volume_y(v, 0.0)
        np.testing.assert_allclose(v, rot, atol=1e-6)

    def test_360_deg_identity(self):
        v = np.zeros((16, 16, 16))
        v[4:8, 4:8, 4:8] = 1.0
        rot = rotate_volume_y(v, 360.0)
        np.testing.assert_allclose(v, rot, atol=1e-3)

    def test_180_deg_symmetry(self):
        v = np.zeros((16, 16, 16))
        v[2:5, 4:8, 2:5] = 1.0
        rot180 = rotate_volume_y(v, 180.0)
        rot360 = rotate_volume_y(rot180, 180.0)
        np.testing.assert_allclose(v, rot360, atol=0.05)

    def test_non_cubic_shape_rotation(self):
        # nz=12, ny=16, nx=16
        v = np.zeros((12, 16, 16))
        v[4:8, 4:12, 4:8] = 1.0
        rot = rotate_volume_y(v, 45.0)
        assert rot.shape == (12, 16, 16)
        assert rot.sum() > 0


class TestTomographicOptimizer:
    def test_high_cure_iou_and_low_overcure(self):
        """Tomo-GS must achieve Cure IoU >= 0.80 and outside cure <= 0.05 on sphere."""
        optics = OpticsConfig(slm_pixels=32)
        grid = GridConfig(n_xy=32, n_z=32, slice_step_um=4.0)
        resin = RESINS["hf-standard-405"]
        vol = sphere(radius=0.28, shape=(grid.n_z, grid.n_xy, grid.n_xy))

        opt = TomographicHolographicOptimizer(
            optics, grid, resin, vol.mask,
            n_patterns=16, seed=42,
        )
        res = opt.optimize(iterations=5, beta=0.3, verbose=False)

        assert res.final_metrics["cure_iou"] >= 0.80, (
            f"Expected Cure IoU >= 0.80, got {res.final_metrics['cure_iou']:.3f}"
        )
        assert res.final_metrics["outside_cure_fraction"] <= 0.05, (
            f"Expected Outside Cure <= 0.05, got {res.final_metrics['outside_cure_fraction']:.4f}"
        )
        assert res.final_metrics["bleed_ratio"] < 0.65

    def test_phases_are_pure_phase(self):
        """Optimizer output must be pure real-valued phase in radians."""
        optics = OpticsConfig(slm_pixels=32)
        grid = GridConfig(n_xy=32, n_z=32, slice_step_um=4.0)
        resin = RESINS["hf-standard-405"]
        vol = sphere(radius=0.28, shape=(grid.n_z, grid.n_xy, grid.n_xy))

        opt = TomographicHolographicOptimizer(
            optics, grid, resin, vol.mask, n_patterns=8, seed=7,
        )
        assert opt.phases.shape == (8, 32, 32)
        assert opt.phases.dtype in (np.float64, np.float32)
        assert opt.phases.min() >= -np.pi - 1e-4
        assert opt.phases.max() <= np.pi + 1e-4

    def test_job_serialization_with_angles(self, tmp_path):
        optics = OpticsConfig(slm_pixels=32)
        grid = GridConfig(n_xy=32, n_z=32, slice_step_um=4.0)
        resin = RESINS["hf-standard-405"]
        vol = sphere(radius=0.28, shape=(grid.n_z, grid.n_xy, grid.n_xy))

        opt = TomographicHolographicOptimizer(optics, grid, resin, vol.mask, n_patterns=8)
        res = opt.optimize(iterations=2, verbose=False)

        job = PrintJob(
            optics=optics, grid=grid, resin=resin, target_mask=vol.mask,
            phases_rad=res.phases_rad, pattern_weights=res.pattern_weights,
            exposure_s=res.exposure_s, rotation_angles_deg=opt.angles_deg,
            metrics=res.final_metrics, meta={"mode": "tomographic"},
        )
        p = str(tmp_path / "tomo_job.hfp")
        save_job(job, p)
        loaded = load_job(p)

        assert loaded.rotation_angles_deg is not None
        assert len(loaded.rotation_angles_deg) == 8
        np.testing.assert_allclose(loaded.rotation_angles_deg, opt.angles_deg)

        # evaluate_job recomputing with angles
        ev = evaluate_job(loaded)
        assert "cure_iou" in ev["metrics"]
        assert ev["metrics"]["cure_iou"] >= 0.70


class TestCLITomographic:
    def test_cli_optimize_tomographic(self, tmp_path):
        out_dir = str(tmp_path / "cli_tomo")
        ret = main([
            "optimize",
            "--mode", "tomographic",
            "--shape", "sphere",
            "--nxy", "32",
            "--nz", "32",
            "--slm-pixels", "32",
            "--patterns", "8",
            "--iters", "2",
            "--out", out_dir,
            "--name", "test_sphere_tomo",
            "--quiet",
        ])
        assert ret == 0
        hfp_file = os.path.join(out_dir, "test_sphere_tomo.hfp")
        assert os.path.exists(hfp_file)
        loaded = load_job(hfp_file)
        assert loaded.rotation_angles_deg is not None
        assert len(loaded.rotation_angles_deg) == 8
        assert loaded.metrics["cure_iou"] >= 0.75
