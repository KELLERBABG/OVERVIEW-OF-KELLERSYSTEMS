"""Shared pytest fixtures for HoloForge tests.

All fixtures use small grids (n_xy=32, n_z=16, slm_pixels=64) for speed.
"""
import numpy as np
import pytest

from holoforge.config import RESINS, GridConfig, OpticsConfig, PrintJob, ResinModel
from holoforge.volume import sphere


@pytest.fixture
def small_optics():
    return OpticsConfig(slm_pixels=64)


@pytest.fixture
def small_grid():
    return GridConfig(n_xy=32, n_z=16, slice_step_um=2.0)


@pytest.fixture
def small_resin():
    return RESINS["hf-standard-405"]


@pytest.fixture
def clear_resin():
    """Resin with no absorption or scatter (for physics tests)."""
    return ResinModel("clear", gel_dose_mJ_cm2=1.0, absorption_per_mm=0.0,
                      scatter_length_um=0.0, index=1.46)


@pytest.fixture
def small_target(small_grid):
    """Small sphere target volume."""
    shape = (small_grid.n_z, small_grid.n_xy, small_grid.n_xy)
    vol = sphere(radius=0.28, shape=shape)
    return vol.mask


@pytest.fixture
def small_job(small_optics, small_grid, small_resin, small_target):
    """A minimal PrintJob for round-trip tests."""
    rng = np.random.default_rng(42)
    K = 4
    n = small_grid.n_xy
    phases = rng.uniform(-np.pi, np.pi, (K, n, n))
    weights = [0.25] * K
    return PrintJob(
        optics=small_optics,
        grid=small_grid,
        resin=small_resin,
        target_mask=small_target,
        phases_rad=phases,
        pattern_weights=weights,
        exposure_s=1.0,
        metrics={"cure_iou": 0.5, "uniformity_error": 0.2},
        meta={"name": "test_sphere"},
    )
