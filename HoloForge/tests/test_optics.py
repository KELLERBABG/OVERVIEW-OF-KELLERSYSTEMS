"""Tests for holoforge.optics — VatPropagator physics validation."""
import numpy as np

from holoforge.optics import VatPropagator


class TestEnergyConservation:
    """Without absorption/scatter, total intensity should be conserved."""

    def test_flat_phase_conserves_energy(self, small_optics, small_grid, clear_resin):
        prop = VatPropagator(small_optics, small_grid)
        phases = np.zeros((2, small_grid.n_xy, small_grid.n_xy))
        dose, _ = prop.forward(phases, clear_resin, keep_fields=False)

        e_in = float((prop.aperture() ** 2).sum())
        e_out_per_slice = float(dose.sum(axis=(1, 2, 3)).mean()) / small_grid.n_z
        # per-slice energy should match input energy within 2%
        rel_err = abs(e_out_per_slice - e_in) / max(e_in, 1e-12)
        assert rel_err < 0.02, f"Energy conservation error: {rel_err:.4f}"


class TestFocusing:
    """A converging lens phase must produce a focused spot."""

    def test_quadratic_phase_focuses(self, small_optics, small_grid, clear_resin):
        prop = VatPropagator(small_optics, small_grid)
        n = small_grid.n_xy
        x = (np.arange(n) + 0.5) / n - 0.5
        X, Y = np.meshgrid(x, x)

        # Small aperture to stay within Nyquist
        a_px = 5.0
        mask = (np.hypot(X, Y) <= a_px / n).astype(float)

        f_slices = small_grid.n_z // 2
        f_mm = small_grid.slice_step_um * 1e-3 * f_slices
        lam_mm = small_optics.wavelength_vat_um * 1e-3
        p_mm = small_optics.pitch_um * 1e-3

        lens = -np.pi * ((X * n * p_mm)**2 + (Y * n * p_mm)**2) / (lam_mm * f_mm) * mask
        dose, _ = prop.forward(lens[None], clear_resin, keep_fields=False)

        # Peak should be brighter than the mean
        mid_slice = dose[0, f_slices]
        peak_ratio = float(mid_slice.max() / max(mid_slice.mean(), 1e-12))
        assert peak_ratio > 1.5, f"Focus peak ratio too low: {peak_ratio:.1f}"


class TestScatterKernel:
    def test_scatter_kernel_normalized(self, small_optics, small_grid, small_resin):
        prop = VatPropagator(small_optics, small_grid)
        S = prop._scatter_kernel(small_resin)
        if S is not None:
            # The kernel in spatial domain should sum to ~1 (normalized)
            s_spatial = np.fft.ifft2(S[0])
            total = float(np.abs(s_spatial).sum())
            assert abs(total - 1.0) < 0.1, f"Scatter kernel not normalized: sum={total:.4f}"

    def test_no_scatter_returns_none(self, small_optics, small_grid, clear_resin):
        prop = VatPropagator(small_optics, small_grid)
        assert prop._scatter_kernel(clear_resin) is None


class TestAperture:
    def test_aperture_shape(self, small_optics, small_grid):
        prop = VatPropagator(small_optics, small_grid)
        ap = prop.aperture()
        assert ap.shape == (small_grid.n_xy, small_grid.n_xy)
        assert ap.dtype == np.float64
        assert ap.max() == 1.0
        assert ap.min() == 0.0
