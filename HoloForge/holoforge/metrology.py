"""Stage 2: Metrology & Virtual Optical Profilometry Engine.

Calculates quantitative surface roughness and feature metrics from simulated
3D volumetric dose and cured solid profiles:
1. Surface height map extraction (virtual optical profilometer).
2. Arithmetic Average Roughness (Ra).
3. Root Mean Square Roughness (Rq).
4. Peak-to-Valley Roughness (Rz).
5. Internal void / microchannel patency and hydraulic diameter.
"""
from __future__ import annotations

from dataclasses import dataclass

import numpy as np

from .config import GridConfig, OpticsConfig


@dataclass
class RoughnessReport:
    """Quantitative surface roughness metrics (ISO 4287 / ASME B46.1)."""
    ra_um: float           # Arithmetic average roughness (microns)
    rq_um: float           # Root-mean-square roughness (microns)
    rz_um: float           # Ten-point height / peak-to-valley roughness (microns)
    height_map_um: np.ndarray # 2D topography height map (um)
    sampling_pitch_um: float  # Transverse sampling resolution (um)
    pass_optical_grade: bool  # True if Ra < 0.1 um (100 nm)

    def summary(self) -> str:
        status = "PASS (Optical Grade)" if self.pass_optical_grade else "FAIL (Exceeds 100nm target)"
        return (
            f"Surface Profilometry Report:\n"
            f"  Ra (Average Roughness)      : {self.ra_um * 1e3:.1f} nm ({self.ra_um:.4f} um)\n"
            f"  Rq (RMS Roughness)          : {self.rq_um * 1e3:.1f} nm ({self.rq_um:.4f} um)\n"
            f"  Rz (Peak-to-Valley)         : {self.rz_um * 1e3:.1f} nm ({self.rz_um:.4f} um)\n"
            f"  Status (Target < 100nm)     : {status}"
        )


@dataclass
class ChannelPatencyReport:
    """Internal microfluidic channel integrity metrics."""
    channel_cured_fraction: float  # Fraction of channel that accidentally gelled (must be 0.0)
    is_patent: bool                # True if internal channel is completely clear and open
    hydraulic_diameter_um: float   # Effective open hydraulic diameter (um)
    target_diameter_um: float      # Desired CAD channel diameter (um)


def extract_topography_height_map(
    cured_mask: np.ndarray,
    optics: OpticsConfig,
    grid: GridConfig,
    axis: int = 0,
    direction: int = 1,
) -> tuple[np.ndarray, np.ndarray]:
    """Simulate an optical profilometer scan across an exposed surface.
    
    Parameters
    ----------
    cured_mask : (nz, ny, nx) bool
        Binary solid mask.
    optics, grid : OpticsConfig, GridConfig
        Physical coordinate mapping.
    axis : int
        Axis along which height is measured (0=z, 1=y, 2=x).
    direction : int
        1 (from positive side downwards) or -1 (from negative side upwards).
        
    Returns
    -------
    height_map_um : 2D float
        Surface elevation in micrometers.
    valid_mask : 2D bool
        Mask of detected surface coordinates.
    """
    dz_um = grid.slice_step_um
    nz, ny, nx = cured_mask.shape
    
    if axis == 0:
        # Looking down Z axis -> scanning XY plane
        ny_out, nx_out = ny, nx
        pitch_um = optics.pitch_um
        step_um = dz_um
    else:
        ny_out, nx_out = nz, nx
        pitch_um = optics.pitch_um
        step_um = pitch_um

    height_map = np.full((ny_out, nx_out), np.nan)
    
    if axis == 0:
        for yi in range(ny):
            for xi in range(nx):
                z_indices = np.where(cured_mask[:, yi, xi])[0]
                if len(z_indices) > 0:
                    idx = z_indices[-1] if direction == 1 else z_indices[0]
                    height_map[yi, xi] = idx * step_um
    elif axis == 1:
        for zi in range(nz):
            for xi in range(nx):
                y_indices = np.where(cured_mask[zi, :, xi])[0]
                if len(y_indices) > 0:
                    idx = y_indices[-1] if direction == 1 else y_indices[0]
                    height_map[zi, xi] = idx * step_um

    valid = ~np.isnan(height_map)
    return height_map, valid


def calculate_roughness(
    height_map_um: np.ndarray,
    valid_mask: np.ndarray | None = None,
    pitch_um: float = 2.0,
    remove_tilt: bool = True,
) -> RoughnessReport:
    """Calculate ISO 4287 roughness parameters (Ra, Rq, Rz) from surface topography.
    
    Parameters
    ----------
    height_map_um : (H, W) float
        Elevation map in micrometers.
    valid_mask : (H, W) bool, optional
        Valid region of interest.
    pitch_um : float
        Transverse pixel spacing in micrometers.
    remove_tilt : bool
        Subtract best-fit plane before computing roughness (standard profilometer practice).
    """
    valid = valid_mask if valid_mask is not None else ~np.isnan(height_map_um)
    if not np.any(valid):
        return RoughnessReport(0.0, 0.0, 0.0, height_map_um, pitch_um, False)

    z = height_map_um[valid]
    
    if remove_tilt and np.sum(valid) >= 3:
        # Fit a least-squares 2D plane: z = ax + by + c
        yy, xx = np.where(valid)
        A = np.stack([xx * pitch_um, yy * pitch_um, np.ones_like(xx)], axis=1)
        plane_coeffs, _, _, _ = np.linalg.lstsq(A, z, rcond=None)
        z_plane = A @ plane_coeffs
        z_residual = z - z_plane
    else:
        z_residual = z - np.mean(z)

    # 1. Ra: Arithmetic Average Roughness
    ra = float(np.mean(np.abs(z_residual)))
    # 2. Rq: Root-Mean-Square Roughness
    rq = float(np.sqrt(np.mean(z_residual ** 2)))
    # 3. Rz: Peak-to-Valley Roughness
    rz = float(np.ptp(z_residual))
    
    # Target: Ra < 100 nm (0.100 um)
    pass_optical = ra < 0.100

    return RoughnessReport(
        ra_um=ra,
        rq_um=rq,
        rz_um=rz,
        height_map_um=height_map_um,
        sampling_pitch_um=pitch_um,
        pass_optical_grade=pass_optical,
    )


def analyze_channel_patency(
    cured_mask: np.ndarray,
    channel_void_target: np.ndarray,
    optics: OpticsConfig,
) -> ChannelPatencyReport:
    """Verify that internal microfluidic channels are clear of solidified resin.
    
    Parameters
    ----------
    cured_mask : (nz, ny, nx) bool
        Achieved solidified part.
    channel_void_target : (nz, ny, nx) bool
        Boolean mask where the internal channel MUST be empty/uncured liquid.
    optics : OpticsConfig
        Optical parameters.
    """
    channel_voxels = np.sum(channel_void_target)
    if channel_voxels == 0:
        return ChannelPatencyReport(0.0, True, 0.0, 0.0)

    # Cured voxels inside the designated channel void
    occluded_voxels = np.sum(cured_mask & channel_void_target)
    cured_frac = float(occluded_voxels / channel_voxels)
    
    is_patent = cured_frac < 0.02 # < 2% occlusion
    
    # Estimate hydraulic diameter based on volume
    est_diameter_um = 2.0 * np.sqrt(channel_voxels * (optics.pitch_um ** 2) / np.pi)
    
    return ChannelPatencyReport(
        channel_cured_fraction=cured_frac,
        is_patent=is_patent,
        hydraulic_diameter_um=float(est_diameter_um * (1.0 - cured_frac)),
        target_diameter_um=float(est_diameter_um),
    )
