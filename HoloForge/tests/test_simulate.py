"""Tests for holoforge.simulate — dose simulation and cure prediction."""
import numpy as np

from holoforge.simulate import (
    evaluate_job,
    gel_level_simulation_units,
    predict_cure,
    simulate_dose,
)


class TestSimulateDose:
    def test_output_shape(self, small_optics, small_grid, small_resin):
        """simulate_dose returns (nz, ny, nx) volume."""
        rng = np.random.default_rng(42)
        phases = rng.uniform(-np.pi, np.pi, (4, small_grid.n_xy, small_grid.n_xy))
        weights = [0.25, 0.25, 0.25, 0.25]
        dose = simulate_dose(phases, weights, small_optics, small_grid, small_resin)
        assert dose.shape == (small_grid.n_z, small_grid.n_xy, small_grid.n_xy)

    def test_dose_nonnegative(self, small_optics, small_grid, small_resin):
        phases = np.zeros((2, small_grid.n_xy, small_grid.n_xy))
        weights = [0.5, 0.5]
        dose = simulate_dose(phases, weights, small_optics, small_grid, small_resin)
        assert np.all(dose >= 0)


class TestPredictCure:
    def test_threshold(self):
        dose = np.array([0.5, 1.0, 1.5, 2.0])
        cured = predict_cure(dose, gel_level_sim=1.0)
        expected = np.array([False, True, True, True])
        np.testing.assert_array_equal(cured, expected)


class TestGelLevel:
    def test_physically_reasonable(self, small_optics, small_grid, small_resin):
        """Gel level should be positive and finite."""
        gel = gel_level_simulation_units(small_resin, small_optics, small_grid,
                                         exposure_s=1.0)
        assert gel > 0
        assert np.isfinite(gel)


class TestEvaluateJob:
    def test_returns_expected_keys(self, small_job):
        result = evaluate_job(small_job)
        assert "dose" in result
        assert "cured" in result
        assert "metrics" in result
        assert "cure_iou" in result["metrics"]
        assert "uniformity_error" in result["metrics"]
