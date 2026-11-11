r"""Stage 1: SLM Calibration, Beam Conditioning, and Optical Alignment Suite.

This module provides tools for:
1. SLM Phase Look-Up Table (LUT) calibration ($[0, 255] \leftrightarrow [0, 2\pi]$).
2. Flat-field laser intensity profile correction.
3. Test target holograms for first-light validation in dye-doped quartz cells.
4. Direct export of 8-bit SLM driver bitmap frames.
"""
from __future__ import annotations

import os
from dataclasses import dataclass

import numpy as np
from PIL import Image

from .config import GridConfig, OpticsConfig


@dataclass
class SLMCalibrationLUT:
    """Phase response Look-Up Table for reflective phase-only LCoS SLMs.
    
    Maps digital grayscale drive values (0-255) to optical phase retardation (0 to 2pi radians).
    """
    name: str = "linear-405nm"
    drive_levels: np.ndarray = None  # (256,) array of uint8 [0..255]
    phase_response_rad: np.ndarray = None  # (256,) array of float [0..2pi]

    def __post_init__(self):
        if self.drive_levels is None:
            self.drive_levels = np.arange(256, dtype=np.uint8)
        if self.phase_response_rad is None:
            # Default linear 0 to 2pi response
            self.phase_response_rad = np.linspace(0.0, 2.0 * np.pi, 256, endpoint=False)

    def phase_to_drive(self, phase_rad: np.ndarray) -> np.ndarray:
        """Map phase in radians (modulo 2pi) to 8-bit SLM drive levels [0..255]."""
        wrapped = np.mod(phase_rad, 2.0 * np.pi)
        # Interpolate into 8-bit drive index
        indices = np.interp(wrapped, self.phase_response_rad, self.drive_levels)
        return np.clip(np.round(indices), 0, 255).astype(np.uint8)

    def drive_to_phase(self, drive_uint8: np.ndarray) -> np.ndarray:
        """Map 8-bit SLM drive levels [0..255] back to optical phase retardation in radians."""
        drive = np.asarray(drive_uint8, dtype=np.float64)
        return np.interp(drive, self.drive_levels, self.phase_response_rad)


# Pre-calibrated default profile
DEFAULT_LUT = SLMCalibrationLUT()

# Typical non-linear S-curve response often measured on uncalibrated LCoS at 405nm
def make_scurve_lut(gamma: float = 1.6) -> SLMCalibrationLUT:
    """Create a calibrated LUT for non-linear liquid crystal electro-optic response."""
    x = np.linspace(0, 1, 256)
    # S-shaped response
    s_curve = 1.0 / (1.0 + np.exp(-gamma * (x - 0.5) * 10.0))
    s_curve = (s_curve - s_curve.min()) / (s_curve.max() - s_curve.min())
    phases = s_curve * 2.0 * np.pi
    return SLMCalibrationLUT(
        name=f"scurve-gamma{gamma}",
        drive_levels=np.arange(256, dtype=np.uint8),
        phase_response_rad=phases,
    )


def flat_field_correction(
    measured_beam: np.ndarray,
    target_profile: str = "uniform",
    clip_percentile: float = 99.0,
) -> np.ndarray:
    """Compute an amplitude/phase weighting mask to compensate for non-uniform laser beam illumination.
    
    Parameters
    ----------
    measured_beam : (ny, nx) float
        Measured 2D intensity profile of the laser spot across the SLM pupil.
    target_profile : str
        Desired field profile ('uniform' or 'gaussian').
    """
    beam = np.maximum(measured_beam, 1e-12)
    max_val = np.percentile(beam, clip_percentile)
    beam_norm = np.clip(beam / max(max_val, 1e-12), 1e-3, 1.0)
    
    if target_profile == "uniform":
        # Inverse intensity correction weights
        correction_weight = 1.0 / np.sqrt(beam_norm)
        correction_weight /= correction_weight.max()
        return correction_weight
    return np.ones_like(measured_beam)


def generate_dye_cell_target(
    target_type: str = "spot_grid",
    optics: OpticsConfig | None = None,
    grid: GridConfig | None = None,
    spots_xy: tuple[int, int] = (3, 3),
    focal_depth_mm: float = 0.5,
) -> np.ndarray:
    """Generate analytical phase patterns for Stage 1 dye-cell alignment verification.
    
    Parameters
    ----------
    target_type : str
        'spot_grid' : Multi-focal 3D array of high-intensity focus spots.
        'bessel_beam' : Axially extended non-diffracting Bessel beam (conical phase / axicon).
        'crosshair' : Alignment crosshairs with concentric radius rings.
    
    Returns
    -------
    phase_rad : (ny, nx) float64
        Phase map in radians in [-pi, pi].
    """
    optics = optics or OpticsConfig()
    grid = grid or GridConfig(n_xy=optics.slm_pixels)
    n = grid.n_xy
    
    x = (np.arange(n) + 0.5) / n - 0.5
    X, Y = np.meshgrid(x, x)
    R = np.hypot(X, Y)
    
    p_mm = optics.pitch_um * 1e-3
    lam_mm = optics.wavelength_vat_um * 1e-3
    
    if target_type == "spot_grid":
        # Superposition of multiple quadratic lenses steering to (xi, yi) with focal distance f
        nx_s, ny_s = spots_xy
        xs = np.linspace(-0.25, 0.25, nx_s)
        ys = np.linspace(-0.25, 0.25, ny_s)
        f_mm = max(focal_depth_mm, 1e-3)
        
        field = np.zeros((n, n), dtype=np.complex128)
        for xi in xs:
            for yi in ys:
                # Quadratic focusing phase + linear tilt to position (xi, yi)
                r2 = (X * n * p_mm - xi * n * p_mm)**2 + (Y * n * p_mm - yi * n * p_mm)**2
                lens_phase = -np.pi * r2 / (lam_mm * f_mm)
                field += np.exp(1j * lens_phase)
        
        # Aperture mask
        aperture = (R <= 0.48).astype(float)
        return np.angle(field) * aperture

    elif target_type == "bessel_beam":
        # Conical axicon phase: phi(r) = -2*pi*r / period
        cone_period_px = 8.0
        cone_phase = -2.0 * np.pi * (R * n) / cone_period_px
        aperture = (R <= 0.48).astype(float)
        return np.angle(np.exp(1j * cone_phase)) * aperture

    elif target_type == "crosshair":
        # Crosshair with concentric rings
        rings = np.cos(2.0 * np.pi * R * 16.0) > 0.0
        cross = (np.abs(X) < 1.5 / n) | (np.abs(Y) < 1.5 / n)
        binary_pattern = (rings | cross) & (R <= 0.48)
        # Binary phase grating (0 or pi)
        return np.where(binary_pattern, np.pi, 0.0)

    else:
        raise ValueError(f"Unknown target_type: {target_type}. Choices: 'spot_grid', 'bessel_beam', 'crosshair'")


def export_slm_images(
    phases_rad: np.ndarray,
    out_dir: str,
    prefix: str = "slm_pattern",
    lut: SLMCalibrationLUT | None = None,
    image_format: str = "png",
) -> list[str]:
    """Convert phase maps to 8-bit SLM drive images and save to disk.
    
    Parameters
    ----------
    phases_rad : (K, ny, nx) or (ny, nx) float
        Phase array in radians.
    out_dir : str
        Target output directory.
    prefix : str
        Filename prefix.
    lut : SLMCalibrationLUT, optional
        Calibration LUT mapping phase to 8-bit pixel values (defaults to linear).
    image_format : str
        'png' or 'bmp'.
    
    Returns
    -------
    saved_paths : list[str]
        Paths of written images.
    """
    os.makedirs(out_dir, exist_ok=True)
    lut = lut or DEFAULT_LUT
    
    phases = np.atleast_3d(phases_rad)
    if phases.ndim == 2:
        phases = phases[None, ...]
    elif phases_rad.ndim == 2:
        phases = phases_rad[None, ...]
    else:
        phases = phases_rad

    saved_paths = []
    for k in range(phases.shape[0]):
        drive_img = lut.phase_to_drive(phases[k])
        img = Image.fromarray(drive_img, mode="L")
        
        filename = f"{prefix}_{k:03d}.{image_format.lower()}"
        filepath = os.path.join(out_dir, filename)
        img.save(filepath)
        saved_paths.append(filepath)

    return saved_paths
