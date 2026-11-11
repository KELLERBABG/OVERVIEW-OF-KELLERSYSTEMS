"""Optical propagation engine: split-step angular spectrum through the resin.

The SLM phase is relayed into the vat; the field walks slice-by-slice
through the resin accumulating dose. Each slice applies: half-step Fresnel
drift -> forward-scatter blur -> absorption -> half-step Fresnel drift.
The optimizer uses the exact reverse walk as its adjoint/backpropagation.
"""
from __future__ import annotations

import numpy as np

from .backend import get_backend
from .config import GridConfig, OpticsConfig, ResinModel


class VatPropagator:
    """Batched angular-spectrum propagator for K simultaneous patterns."""

    def __init__(self, optics: OpticsConfig, grid: GridConfig, device: str = "auto"):
        self.optics = optics
        self.grid = grid
        self.backend = get_backend(device)

        n = grid.n_xy
        p_mm = optics.pitch_um * 1e-3              # mm
        lam_mm = optics.wavelength_vat_um * 1e-3   # mm inside the resin
        self.dz_mm = grid.slice_step_um * 1e-3     # mm

        fx = np.fft.fftfreq(n, d=p_mm)             # cycles per mm
        KY, KX = np.meshgrid(fx, fx, indexing="ij")
        k = 2 * np.pi / lam_mm                     # rad/mm
        kz = np.sqrt(np.maximum(k * k - (2 * np.pi * KX) ** 2 - (2 * np.pi * KY) ** 2, 0.0))
        self.H_half = np.exp(1j * kz * self.dz_mm / 2)[None, :, :]

    def _scatter_kernel(self, resin: ResinModel) -> np.ndarray | None:
        """Small-angle forward-scatter blur in Fourier space.

        Model: transport mean free path L -> angular diffusion theta_rms =
        sqrt(pitch/L); lateral broadening per step sigma = theta_rms * dz.
        Implemented as a normalized Gaussian convolution per slice.
        """
        L = float(getattr(resin, "scatter_length_um", 0.0))
        if L <= 0:
            return None
        n = self.grid.n_xy
        p_um = self.optics.pitch_um
        theta_rms = float(np.sqrt(p_um / L))
        sigma_px = max(theta_rms * self.dz_mm * 1e3 / p_um, 1e-6)
        x = np.arange(n) - n // 2
        X, Y = np.meshgrid(x, x, indexing="ij")
        g = np.exp(-(X ** 2 + Y ** 2) / (4 * sigma_px ** 2))
        g /= g.sum()
        return np.fft.fft2(g)[None, :, :]          # real & symmetric

    def aperture(self) -> np.ndarray:
        n = self.grid.n_xy
        x = (np.arange(n) + 0.5) / n - 0.5
        X, Y = np.meshgrid(x, x)
        return (np.hypot(X, Y) <= 0.5).astype(np.float64)

    def field_from_phases(self, phases_rad: np.ndarray, relay_filter: bool = False) -> np.ndarray:
        """SLM-plane complex field for a batch (K, ny, nx) of phases.

        When relay_filter=True, applies a 4f telecentric relay pupil low-pass filter
        to convert carrier/double-phase modulations into true spatial amplitude profiles.
        """
        raw = np.exp(1j * phases_rad) * self.aperture()
        if not relay_filter:
            return raw
        n = self.grid.n_xy
        fx = np.fft.fftshift(np.fft.fftfreq(n))
        FX, FY = np.meshgrid(fx, fx)
        pupil = (np.hypot(FX, FY) <= 0.35)[None, :, :]
        f_field = self.backend.fftshift(self.backend.fft2(raw, axes=(1, 2)), axes=(1, 2)) * pupil
        return self.backend.to_numpy(self.backend.ifft2(self.backend.ifftshift(f_field, axes=(1, 2)), axes=(1, 2)))

    def forward(self, phases_rad: np.ndarray, resin: ResinModel,
                keep_fields: bool = False, relay_filter: bool = False):
        """Propagate batch through the vat.

        Returns
        -------
        dose : (K, nz, ny, nx) accumulated |U|^2 per slice
        fields : list[nz] of (K, ny, nx) complex fields at measurement planes
                 (empty list when keep_fields=False)
        """
        b = self.backend
        K, n = phases_rad.shape[0], self.grid.n_xy
        nz = self.grid.n_z

        U = b.fft2(self.field_from_phases(phases_rad, relay_filter=relay_filter), axes=(1, 2))
        att = float(np.exp(-resin.amplitude_absorption_per_mm * 1e-3 * self.dz_mm))
        S = self._scatter_kernel(resin)
        H_half = b.asarray(self.H_half)
        S_dev = b.asarray(S) if S is not None else None

        dose = np.zeros((K, nz, n, n), dtype=np.float64)
        fields: list[np.ndarray] = []

        for zi in range(nz):
            U = U * H_half                         # half drift (freq)
            U = b.ifft2(U, axes=(1, 2))            # mid-slice (spatial)
            if S_dev is not None:
                U = b.ifft2(b.fft2(U, axes=(1, 2)) * S_dev, axes=(1, 2))
            U = U * att                            # absorption
            U_np = b.to_numpy(U)
            if keep_fields:
                dose[:, zi] = np.abs(U_np) ** 2
                fields.append(U_np.copy())
            else:
                dose[:, zi] = np.abs(U_np) ** 2
            U = b.fft2(U, axes=(1, 2)) * H_half   # exit drift

        return dose, fields

    def forward_band(self, phase_rad: np.ndarray, resin: ResinModel,
                     z_lo: int, z_hi: int):
        """Propagate ONE pattern from the SLM to the far side of its band,
        returning the complex fields at every slice in [z_lo, z_hi)."""
        xp = np
        U = xp.fft.fft2(self.field_from_phases(phase_rad[None]), axes=(1, 2))
        att = float(np.exp(-resin.amplitude_absorption_per_mm * 1e-3 * self.dz_mm))
        S = self._scatter_kernel(resin)

        fields = {}
        for zi in range(z_hi):
            U = U * self.H_half
            U = xp.fft.ifft2(U, axes=(1, 2))
            if S is not None:
                U = xp.fft.ifft2(xp.fft.fft2(U, axes=(1, 2)) * S, axes=(1, 2))
            U *= att
            if zi >= z_lo:
                fields[zi] = U[0].copy()
            U = xp.fft.fft2(U, axes=(1, 2)) * self.H_half
        return fields

    def backward_band(self, fields_by_slice: dict[int, np.ndarray],
                      z_lo: int, z_hi: int, resin: ResinModel) -> np.ndarray:
        """Adjoint walk from the end of a band back to the SLM, carrying the
        amplitude-corrected fields. Returns the corrected SLM field."""
        xp = np
        att = float(np.exp(-resin.amplitude_absorption_per_mm * 1e-3 * self.dz_mm))
        inv_att = 1.0 / att
        S = self._scatter_kernel(resin)

        # state just past the band's last slice (freq domain)
        Cf = None
        for zi in range(z_hi - 1, z_lo - 1, -1):
            if Cf is not None:
                Cf = Cf * xp.conj(self.H_half)
                C = xp.fft.ifft2(Cf, axes=(1, 2))
            else:
                C = np.zeros((1, self.grid.n_xy, self.grid.n_xy), np.complex128)
            C = C + fields_by_slice[zi][None]
            C *= inv_att
            if S is not None:
                C = xp.fft.ifft2(xp.fft.fft2(C, axes=(1, 2)) * S, axes=(1, 2))
            Cf = xp.fft.fft2(C, axes=(1, 2)) * xp.conj(self.H_half)
        return xp.fft.ifft2(Cf, axes=(1, 2))[0]

    def backward(self, sources: list[np.ndarray],
                 resin: ResinModel) -> np.ndarray:
        """Adjoint walk: bring per-slice correction sources back to the SLM.

        sources[z] : (K, ny, nx) real/complex correction injected at the
        measurement plane of slice z (same position where dose was sampled).
        Mirrors forward() exactly: undo exit drift -> add source -> undo
        absorption -> undo scatter -> undo entry drift; final iFFT returns
        to the SLM plane. Returns (K, ny, nx) complex correction.
        """
        xp = np
        att = float(np.exp(-resin.amplitude_absorption_per_mm * 1e-3 * self.dz_mm))
        inv_att = 1.0 / att
        S = self._scatter_kernel(resin)

        Cf = xp.zeros_like(sources[0], dtype=np.complex128)
        Cf = xp.fft.fft2(Cf, axes=(1, 2))

        for zi in range(len(sources) - 1, -1, -1):
            Cf = Cf * xp.conj(self.H_half)         # undo exit half-drift
            C = xp.fft.ifft2(Cf, axes=(1, 2))      # measurement plane
            C = C + sources[zi].astype(np.complex128)
            C *= inv_att                            # undo absorption
            if S is not None:                       # undo scatter (S symmetric)
                C = xp.fft.ifft2(xp.fft.fft2(C, axes=(1, 2)) * S, axes=(1, 2))
            Cf = xp.fft.fft2(C, axes=(1, 2)) * xp.conj(self.H_half)  # undo entry half-drift

        return xp.fft.ifft2(Cf, axes=(1, 2))       # back through SLM FFT

    def forward_tomographic(self, phases_rad: np.ndarray, angles_deg: list[float] | np.ndarray,
                            resin: ResinModel) -> np.ndarray:
        """Propagate K patterns at K rotation angles through the vat.

        For each pattern k at angle theta_k:
          1. Computes 3D beam dose along optical axis z via forward().
          2. Rotates the 3D dose volume by -theta_k around the y-axis to map
             into the stationary vat coordinate frame.
        Returns
        -------
        tomo_dose : (K, nz, ny, nx) accumulated rotated dose volumes in vat frame.
        """
        K = len(angles_deg)
        if phases_rad.shape[0] != K:
            raise ValueError(f"phases_rad has {phases_rad.shape[0]} patterns, expected {K} angles")

        beam_dose, _ = self.forward(phases_rad, resin, keep_fields=False, relay_filter=True)
        tomo_dose = np.zeros_like(beam_dose)
        for k in range(K):
            deg = float(angles_deg[k])
            tomo_dose[k] = self.backend.rotate_volume_y(beam_dose[k], -deg, order=1)
        return tomo_dose


def rotate_volume_y(volume: np.ndarray, angle_deg: float, order: int = 1) -> np.ndarray:
    """Rotate a 3D volume (nz, ny, nx) around the vertical y-axis by angle_deg.

    Rotation is in the (z, x) plane (axes 0 and 2). If nz != nx, volume is padded
    symmetrically to (max_dim, ny, max_dim) before rotation, then cropped back to (nz, ny, nx).
    Uses scipy.ndimage.rotate with bilinear interpolation (order=1).
    """
    if abs(angle_deg % 360.0) < 1e-4:
        return volume.copy()

    nz, ny, nx = volume.shape
    need_pad = (nz != nx)
    if need_pad:
        max_dim = max(nz, nx)
        pad_z = ((max_dim - nz) // 2, max_dim - nz - (max_dim - nz) // 2)
        pad_x = ((max_dim - nx) // 2, max_dim - nx - (max_dim - nx) // 2)
        vol_pad = np.pad(volume, (pad_z, (0, 0), pad_x), mode="constant", constant_values=0.0)
    else:
        vol_pad = volume

    try:
        from scipy.ndimage import rotate as _scipy_rotate
        rot = _scipy_rotate(vol_pad, angle_deg, axes=(0, 2), reshape=False, order=order, mode="constant", cval=0.0)
    except ImportError:
        angle_rad = np.radians(angle_deg)
        cos_a, sin_a = np.cos(angle_rad), np.sin(angle_rad)
        dim_z, _, dim_x = vol_pad.shape
        cz, cx = (dim_z - 1) / 2.0, (dim_x - 1) / 2.0
        z_idx, x_idx = np.indices((dim_z, dim_x))
        z_orig = (cos_a * (z_idx - cz) + sin_a * (x_idx - cx) + cz).round().astype(int)
        x_orig = (-sin_a * (z_idx - cz) + cos_a * (x_idx - cx) + cx).round().astype(int)
        valid = (z_orig >= 0) & (z_orig < dim_z) & (x_orig >= 0) & (x_orig < dim_x)
        rot = np.zeros_like(vol_pad)
        for y in range(ny):
            rot[valid, y] = vol_pad[z_orig[valid], y, x_orig[valid]]

    if need_pad:
        z_start = (max_dim - nz) // 2
        x_start = (max_dim - nx) // 2
        return rot[z_start:z_start + nz, :, x_start:x_start + nx].copy()
    return rot


def cylindrical_vat_phase_correction(
    optics: OpticsConfig,
    grid: GridConfig,
    vat_radius_mm: float = 10.0,
    vat_index: float | None = None,
    glass_index: float = 1.51,
    wall_thickness_mm: float = 1.0,
) -> np.ndarray:
    """Analytical inverse phase conjugate mask to pre-cancel cylindrical vat astigmatism.

    When a beam passes through a round glass cell into resin, the curved boundary
    acts as a strong cylindrical lens focusing along the horizontal x-axis.
    Adding this inverse phase conjugate onto the SLM cancels the vat aberration
    before light reaches the resin volume.

    Returns
    -------
    phase_comp : (ny, nx) float64
        Phase correction map in radians in [-pi, pi] across the SLM aperture.
    """
    n = grid.n_xy
    p_mm = optics.pitch_um * 1e-3
    n_resin = vat_index if vat_index is not None else optics.vat_index
    lam_mm = optics.wavelength_nm * 1e-6  # in mm (in air)

    # Transverse horizontal coordinates x in mm
    x_px = np.arange(n) - (n - 1) / 2.0
    x_mm = x_px * p_mm

    # Outer and inner radius of the cylindrical vat wall
    r_out = max(vat_radius_mm, 1e-3)
    r_in = max(r_out - wall_thickness_mm, 0.5 * r_out)

    # Paraxial circular optical path difference relative to on-axis ray
    x_clamped_out = np.clip(np.abs(x_mm), 0.0, r_out * 0.95)
    x_clamped_in = np.clip(np.abs(x_mm), 0.0, r_in * 0.95)

    sag_out = r_out - np.sqrt(r_out**2 - x_clamped_out**2)
    sag_in = r_in - np.sqrt(r_in**2 - x_clamped_in**2)

    # Total phase retardance added by physical vat
    opd_mm = (glass_index - 1.0) * sag_out - (glass_index - n_resin) * sag_in
    phi_vat_1d = (2.0 * np.pi / lam_mm) * opd_mm

    # Inverse phase conjugate is -phi_vat
    inv_phi_1d = -phi_vat_1d
    inv_phi_2d = np.broadcast_to(inv_phi_1d[None, :], (n, n)).copy()

    # Wrap into [-pi, pi] and mask to aperture
    wrapped = np.angle(np.exp(1j * inv_phi_2d))
    X, Y = np.meshgrid(x_px / n, x_px / n)
    return wrapped * (np.hypot(X, Y) <= 0.5)


