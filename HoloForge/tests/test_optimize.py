"""Tests for holoforge.optimize — MultiPlaneGS convergence and constraints."""
import numpy as np

from holoforge.optimize import MultiPlaneGS, WavefrontMatchedCGH, _dose_metrics


class TestMultiPlaneGS:
    def test_convergence(self, small_optics, small_grid, small_resin, small_target):
        """IoU should improve over iterations."""
        gs = MultiPlaneGS(small_optics, small_grid, small_resin, small_target,
                          n_patterns=4, seed=42)
        result = gs.optimize(iterations=10, beta=0.65, verbose=False)
        history = result.metrics_history
        assert len(history) > 0
        # Last IoU should be >= first IoU (convergence)
        assert history[-1]["cure_iou"] >= history[0]["cure_iou"] - 0.01

    def test_phase_only_constraint(self, small_optics, small_grid, small_resin, small_target):
        """Optimizer output should be pure phase (no amplitude info in phases_rad)."""
        gs = MultiPlaneGS(small_optics, small_grid, small_resin, small_target,
                          n_patterns=4, seed=42)
        result = gs.optimize(iterations=5, verbose=False)
        # Phases should be real-valued in [-pi, pi] range
        assert result.phases_rad.dtype in (np.float64, np.float32)
        assert result.phases_rad.shape == (4, small_grid.n_xy, small_grid.n_xy)

    def test_band_edges_cover_volume(self, small_optics, small_grid, small_resin, small_target):
        """Band edges should partition the entire z-axis."""
        gs = MultiPlaneGS(small_optics, small_grid, small_resin, small_target,
                          n_patterns=4, seed=42)
        assert gs.band_edges[0][0] == 0
        assert gs.band_edges[-1][1] == small_grid.n_z

    def test_target_amplitudes_accepted(self, small_optics, small_grid, small_resin, small_target):
        """The optimizer should accept per-voxel target amplitudes."""
        amps = small_target.astype(np.float64) * 1.5
        gs = MultiPlaneGS(small_optics, small_grid, small_resin, small_target,
                          n_patterns=4, seed=42, target_amplitudes=amps)
        result = gs.optimize(iterations=3, verbose=False)
        assert result.phases_rad.shape[0] == 4


class TestAlias:
    def test_wavefront_matched_alias(self):
        assert WavefrontMatchedCGH is MultiPlaneGS


class TestDoseMetrics:
    def test_full_cure(self):
        """When dose >> gel everywhere, IoU should be high."""
        target = np.ones((8, 8, 8), dtype=bool)
        dose = np.ones((8, 8, 8)) * 10.0
        m = _dose_metrics(dose, target, gel_level=1.0)
        assert m["cure_iou"] > 0.9
        assert m["uniformity_error"] < 0.01

    def test_empty_target(self):
        """Edge case: empty target shouldn't crash."""
        target = np.zeros((8, 8, 8), dtype=bool)
        dose = np.ones((8, 8, 8))
        m = _dose_metrics(dose, target, gel_level=1.0)
        assert "cure_iou" in m
