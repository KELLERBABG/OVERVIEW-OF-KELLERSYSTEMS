"""Stage 3: High-Value Commercial B2B Vertical Geometries.

Generates parametric solid models for:
1. Dental Labs: Monolithic molar crown & inlay analog with anatomical cusps.
2. Microfluidics: Monolithic lab-on-a-chip with enclosed, leak-free serpentine channels.
3. Custom Optics: Even-asphere optical lens with high-order polynomial surface sag.
"""
from __future__ import annotations

import numpy as np

from .volume import VoxelVolume, grid_of


def dental_crown(
    shape: tuple[int, int, int] = (96, 128, 128),
    cusp_height: float = 0.12,
    margin_radius: float = 0.28,
) -> VoxelVolume:
    """Anatomical dental molar crown analog for high-speed dental lab manufacturing.
    
    Generates a 3D solid with 4 occlusal cusps, central developmental groove,
    and a defined cervical marginal finish line.
    """
    nz, ny, nx = shape
    z, y, x = grid_of(shape)
    
    # Radial coordinate
    r = np.hypot(x, y)
    theta = np.arctan2(y, x)
    
    # 4-lobed occlusal cusp geometry: r_profile(theta)
    lobe_modulation = 1.0 + 0.14 * np.cos(4.0 * theta) + 0.05 * np.cos(2.0 * theta)
    r_outer = margin_radius * lobe_modulation
    
    # Base height and occlusal surface profile
    # z in [-0.5, 0.5]
    z_base = -0.30
    
    # Cusp elevation: higher at four corners (theta = pi/4, 3pi/4, 5pi/4, 7pi/4)
    cusps = cusp_height * np.cos(2.0 * theta)**2 * np.exp(-((r - 0.18) ** 2) / 0.02)
    # Central fissure depression
    fissure = -0.06 * np.exp(-(r ** 2) / 0.015)
    
    z_top = 0.18 + cusps + fissure
    
    # Interior preparation cavity (for cementation onto tooth die)
    r_prep = margin_radius * 0.55
    z_prep_ceiling = 0.05
    is_cavity = (r <= r_prep) & (z <= z_prep_ceiling) & (z >= z_base - 0.05)
    
    # Solid crown envelope
    is_solid = (r <= r_outer) & (z >= z_base) & (z <= z_top) & ~is_cavity
    
    return VoxelVolume(is_solid)


def microfluidic_chip(
    shape: tuple[int, int, int] = (96, 128, 128),
    channel_radius: float = 0.035,
    chip_span: tuple[float, float, float] = (0.28, 0.40, 0.40),
) -> tuple[VoxelVolume, VoxelVolume]:
    """Monolithic microfluidic lab-on-a-chip with internal serpentine channels.
    
    Returns
    -------
    solid_chip : VoxelVolume
        The solid polymer block enclosing the channels.
    channel_void : VoxelVolume
        The internal hollow channel path (must remain unsolidified liquid).
    """
    nz, ny, nx = shape
    z, y, x = grid_of(shape)
    sz, sy, sx = chip_span
    
    # Solid chip outer block
    in_block = (np.abs(z) <= sz / 2.0) & (np.abs(y) <= sy / 2.0) & (np.abs(x) <= sx / 2.0)
    
    # Parametric serpentine channel curve in mid-plane (z = 0)
    t = np.linspace(-0.32, 0.32, 1200)
    # Serpentine wave
    cx = t
    cy = 0.12 * np.sin(4.0 * np.pi * t / 0.32)
    cz = np.zeros_like(t)
    pts = np.stack([cz, cy, cx], axis=1)
    
    # Distance from grid to channel centerline
    d2_min = np.full(shape, np.inf)
    step = 300
    for i in range(0, len(t), step):
        pz = pts[i:i + step, 0, None, None, None]
        py = pts[i:i + step, 1, None, None, None]
        px = pts[i:i + step, 2, None, None, None]
        d2 = (z - pz)**2 + (y - py)**2 + (x - px)**2
        np.minimum(d2_min, d2.min(axis=0), out=d2_min)
        
    is_channel = d2_min <= (channel_radius ** 2)
    
    # The printed object is the solid block MINUS the channel void
    is_solid = in_block & ~is_channel
    
    return VoxelVolume(is_solid), VoxelVolume(is_channel & in_block)


def aspheric_lens(
    shape: tuple[int, int, int] = (96, 128, 128),
    radius: float = 0.36,
    curvature_c: float = 1.8,
    conic_k: float = -0.65,
    alpha4: float = 0.25,
    center_thickness: float = 0.30,
) -> VoxelVolume:
    """Precision even-asphere optical lens geometry.
    
    Surface sag equation:
    z(r) = (c * r^2) / (1 + sqrt(1 - (1 + k) * c^2 * r^2)) + alpha_4 * r^4
    """
    nz, ny, nx = shape
    z, y, x = grid_of(shape)
    
    r = np.hypot(x, y)
    r2 = r ** 2
    
    # Flat base at z_bottom
    z_bottom = - center_thickness / 2.0
    
    # Aspheric top surface sag
    sqrt_arg = np.maximum(1.0 - (1.0 + conic_k) * (curvature_c ** 2) * r2, 0.0)
    sag = (curvature_c * r2) / (1.0 + np.sqrt(sqrt_arg)) + alpha4 * (r2 ** 2)
    
    z_top = z_bottom + center_thickness - sag
    
    is_lens = (r <= radius) & (z >= z_bottom) & (z <= z_top)
    return VoxelVolume(is_lens)
