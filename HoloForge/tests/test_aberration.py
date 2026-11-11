"""Tests for Cylindrical Vat Optical Aberration Pre-Compensation."""
import numpy as np

from holoforge.config import GridConfig, OpticsConfig
from holoforge.optics import cylindrical_vat_phase_correction


class TestCylindricalVatAberration:
    def test_phase_correction_shape_and_bounds(self):
        optics = OpticsConfig(slm_pixels=64)
        grid = GridConfig(n_xy=64)
        comp = cylindrical_vat_phase_correction(optics, grid, vat_radius_mm=10.0)

        assert comp.shape == (64, 64)
        assert comp.dtype == np.float64
        assert comp.min() >= -np.pi - 1e-4
        assert comp.max() <= np.pi + 1e-4

    def test_horizontal_astigmatic_symmetry(self):
        """Cylindrical vat aberration should be symmetric along x and uniform along vertical y."""
        optics = OpticsConfig(slm_pixels=64)
        grid = GridConfig(n_xy=64)
        comp = cylindrical_vat_phase_correction(optics, grid, vat_radius_mm=12.0)

        # Symmetry along horizontal center (x-axis)
        mid_row = comp[32, :]
        np.testing.assert_allclose(mid_row, mid_row[::-1], atol=1e-5)

    def test_larger_radius_produces_smaller_sag(self):
        """A larger radius vat has lower curvature and less OPD per millimeter."""
        optics = OpticsConfig(slm_pixels=64)
        grid = GridConfig(n_xy=64)
        comp_small = cylindrical_vat_phase_correction(optics, grid, vat_radius_mm=5.0)
        comp_large = cylindrical_vat_phase_correction(optics, grid, vat_radius_mm=25.0)

        # Standard deviation of phase variation across aperture should be lower for larger radius
        assert comp_large[comp_large != 0].std() < comp_small[comp_small != 0].std()
