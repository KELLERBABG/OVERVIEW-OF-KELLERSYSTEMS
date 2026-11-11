from dataclasses import dataclass
from typing import Tuple
import numpy as np

@dataclass
class VoxelScene:
    grid_size: Tuple[int, int, int]
    bounds_min_mm: Tuple[float, float, float]
    bounds_max_mm: Tuple[float, float, float]
    density_grid_g_cm3: np.ndarray
    rad_length_grid_cm: np.ndarray
    z_eff_grid: np.ndarray

    @property
    def voxel_size_mm(self) -> Tuple[float, float, float]:
        dx = (self.bounds_max_mm[0] - self.bounds_min_mm[0]) / self.grid_size[0]
        dy = (self.bounds_max_mm[1] - self.bounds_min_mm[1]) / self.grid_size[1]
        dz = (self.bounds_max_mm[2] - self.bounds_min_mm[2]) / self.grid_size[2]
        return dx, dy, dz

    def get_scattering_density(self) -> np.ndarray:
        return 1.0 / np.maximum(self.rad_length_grid_cm, 1e-4)

def create_void_phantom(
    grid_size: Tuple[int, int, int] = (64, 64, 64),
    bounds_mm: Tuple[float, float, float, float, float, float] = (-200, 200, -200, 200, -150, 150),
    void_radius_mm: float = 40.0,
) -> VoxelScene:
    nx, ny, nz = grid_size
    xmin, xmax, ymin, ymax, zmin, zmax = bounds_mm

    density = np.full((nx, ny, nz), 2.35, dtype=np.float32)
    rad_len = np.full((nx, ny, nz), 11.55, dtype=np.float32)
    z_eff = np.full((nx, ny, nz), 11.5, dtype=np.float32)

    xs = np.linspace(xmin, xmax, nx)
    ys = np.linspace(ymin, ymax, ny)
    zs = np.linspace(zmin, zmax, nz)
    xg, yg, zg = np.meshgrid(xs, ys, zs, indexing="ij")

    void_mask = (xg**2 + yg**2 + zg**2) <= (void_radius_mm**2)
    density[void_mask] = 0.0012
    rad_len[void_mask] = 30420.0
    z_eff[void_mask] = 7.2

    return VoxelScene(
        grid_size=grid_size,
        bounds_min_mm=(xmin, ymin, zmin),
        bounds_max_mm=(xmax, ymax, zmax),
        density_grid_g_cm3=density,
        rad_length_grid_cm=rad_len,
        z_eff_grid=z_eff,
    )

def create_heavy_inclusion_phantom(
    grid_size: Tuple[int, int, int] = (64, 64, 64),
    bounds_mm: Tuple[float, float, float, float, float, float] = (-250, 250, -250, 250, -200, 200),
) -> VoxelScene:
    nx, ny, nz = grid_size
    xmin, xmax, ymin, ymax, zmin, zmax = bounds_mm

    density = np.full((nx, ny, nz), 2.35, dtype=np.float32)
    rad_len = np.full((nx, ny, nz), 11.55, dtype=np.float32)
    z_eff = np.full((nx, ny, nz), 11.5, dtype=np.float32)

    xs = np.linspace(xmin, xmax, nx)
    ys = np.linspace(ymin, ymax, ny)
    zs = np.linspace(zmin, zmax, nz)
    xg, yg, zg = np.meshgrid(xs, ys, zs, indexing="ij")

    lead_mask = (
        (xg >= -120.0) & (xg <= -40.0) &
        (yg >= -50.0) & (yg <= 50.0) &
        (zg >= -50.0) & (zg <= 50.0)
    )
    density[lead_mask] = 11.35
    rad_len[lead_mask] = 0.56
    z_eff[lead_mask] = 82.0

    iron_mask = (((xg - 80.0)**2 + yg**2) <= 35.0**2) & (zg >= -80.0) & (zg <= 80.0)
    density[iron_mask] = 7.87
    rad_len[iron_mask] = 1.76
    z_eff[iron_mask] = 26.0

    return VoxelScene(
        grid_size=grid_size,
        bounds_min_mm=(xmin, ymin, zmin),
        bounds_max_mm=(xmax, ymax, zmax),
        density_grid_g_cm3=density,
        rad_length_grid_cm=rad_len,
        z_eff_grid=z_eff,
    )

def create_rebar_phantom(
    grid_size: Tuple[int, int, int] = (64, 64, 64),
    bounds_mm: Tuple[float, float, float, float, float, float] = (-200, 200, -200, 200, -100, 100),
    rebar_radius_mm: float = 6.0,
    spacing_mm: float = 60.0,
) -> VoxelScene:
    nx, ny, nz = grid_size
    xmin, xmax, ymin, ymax, zmin, zmax = bounds_mm

    density = np.full((nx, ny, nz), 2.35, dtype=np.float32)
    rad_len = np.full((nx, ny, nz), 11.55, dtype=np.float32)
    z_eff = np.full((nx, ny, nz), 11.5, dtype=np.float32)

    xs = np.linspace(xmin, xmax, nx)
    ys = np.linspace(ymin, ymax, ny)
    zs = np.linspace(zmin, zmax, nz)
    xg, yg, zg = np.meshgrid(xs, ys, zs, indexing="ij")

    rebar_x = (np.abs(np.remainder(xg + 1000.0, spacing_mm) - spacing_mm / 2.0) <= rebar_radius_mm) & (np.abs(zg) <= 20.0)
    rebar_y = (np.abs(np.remainder(yg + 1000.0, spacing_mm) - spacing_mm / 2.0) <= rebar_radius_mm) & (np.abs(zg) <= 20.0)
    rebar_mask = rebar_x | rebar_y

    density[rebar_mask] = 7.87
    rad_len[rebar_mask] = 1.76
    z_eff[rebar_mask] = 26.0

    return VoxelScene(
        grid_size=grid_size,
        bounds_min_mm=(xmin, ymin, zmin),
        bounds_max_mm=(xmax, ymax, zmax),
        density_grid_g_cm3=density,
        rad_length_grid_cm=rad_len,
        z_eff_grid=z_eff,
    )
