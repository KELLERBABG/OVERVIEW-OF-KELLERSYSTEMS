"""Voxel volumes: primitives, mesh voxelization, helpers."""
from __future__ import annotations

import struct

import numpy as np


class VoxelVolume:
    """Boolean target mask on the simulation grid."""

    def __init__(self, mask: np.ndarray):
        self.mask = np.ascontiguousarray(mask.astype(bool))
        if self.mask.ndim != 3:
            raise ValueError("mask must be 3-D (nz, ny, nx)")

    @property
    def shape(self) -> tuple[int, int, int]:
        return self.mask.shape  # type: ignore[return-value]

    @property
    def voxel_count(self) -> int:
        return int(self.mask.sum())

    def coordinates(self) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
        nz, ny, nx = self.shape
        z = (np.arange(nz) + 0.5) / nz - 0.5
        y = (np.arange(ny) + 0.5) / ny - 0.5
        x = (np.arange(nx) + 0.5) / nx - 0.5
        return z[:, None, None], y[None, :, None], x[None, None, :]


def grid_of(shape: tuple[int, int, int]) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """Normalized coords in [-0.5, 0.5), shaped for broadcasting."""
    nz, ny, nx = shape
    z = ((np.arange(nz) + 0.5) / nz - 0.5)[:, None, None]
    y = ((np.arange(ny) + 0.5) / ny - 0.5)[None, :, None]
    x = ((np.arange(nx) + 0.5) / nx - 0.5)[None, None, :]
    return z, y, x


# ---------------------------------------------------------------------------
# Primitives (SDF-style, resolution independent)

def sphere(radius: float = 0.28, shape: tuple[int, int, int] = (96, 128, 128),
           center: tuple[float, float, float] = (0.0, 0.0, 0.0)) -> VoxelVolume:
    z, y, x = grid_of(shape)
    cz, cy, cx = center
    r2 = (z - cz) ** 2 + (y - cy) ** 2 + (x - cx) ** 2
    return VoxelVolume(r2 <= radius * radius)


def torus_knot(p: int = 2, q: int = 3, tube_radius: float = 0.055,
               shape: tuple[int, int, int] = (96, 128, 128)) -> VoxelVolume:
    """(p,q)-torus knot tube - the classic volumetric benchmark print.

    Knot curve on a torus of R=0.22, r=0.09 in normalized [-0.5,0.5] space.
    """
    t = np.linspace(0.0, 2.0 * np.pi, 4000, endpoint=False)
    R, r = 0.22, 0.09
    kx = (R + r * np.cos(q * t)) * np.cos(p * t)
    ky = (R + r * np.cos(q * t)) * np.sin(p * t)
    kz = r * np.sin(q * t) * 1.4          # stretch z so the knot fills depth
    pts = np.stack([kz, ky, kx], axis=1).astype(np.float64)  # (M, 3) as z,y,x

    nz, ny, nx = shape
    zz, yy, xx = grid_of(shape)
    # distance to knot polyline, chunked over points to bound memory
    d2_min = np.full(shape, np.inf)
    step = 500
    for i in range(0, len(t), step):
        pz, py, px = pts[i:i + step, 0, None, None, None], pts[i:i + step, 1, None, None, None], pts[i:i + step, 2, None, None, None]
        d2 = (zz - pz) ** 2 + (yy - py) ** 2 + (xx - px) ** 2
        np.minimum(d2_min, d2.min(axis=0), out=d2_min)
    return VoxelVolume(d2_min <= tube_radius * tube_radius)


def lattice_cube(cells: int = 3, strut: float = 0.03,
                 shape: tuple[int, int, int] = (96, 128, 128)) -> VoxelVolume:
    """Cubic lattice scaffold (microfluidics/heat-exchanger style geometry)."""
    z, y, x = grid_of(shape)
    span = 0.36
    def near_grid(u: np.ndarray) -> np.ndarray:
        g = (u + span / 2) % (span / max(cells - 1, 1))
        return np.minimum(g, span / max(cells - 1, 1) - g)
    dz, dy, dx = near_grid(z), near_grid(y), near_grid(x)
    inside = (np.abs(z) < span / 2) & (np.abs(y) < span / 2) & (np.abs(x) < span / 2)
    strut_mask = (
        ((dz < strut) & (dy < strut))
        | ((dy < strut) & (dx < strut))
        | ((dx < strut) & (dz < strut))
    )
    return VoxelVolume(inside & strut_mask)


def _microfluidic_chip_primitive(shape: tuple[int, int, int] = (96, 128, 128), **kwargs) -> VoxelVolume:
    from .verticals import microfluidic_chip
    solid, _ = microfluidic_chip(shape=shape, **kwargs)
    return solid


def _dental_crown_primitive(shape: tuple[int, int, int] = (96, 128, 128), **kwargs) -> VoxelVolume:
    from .verticals import dental_crown
    return dental_crown(shape=shape, **kwargs)


def _aspheric_lens_primitive(shape: tuple[int, int, int] = (96, 128, 128), **kwargs) -> VoxelVolume:
    from .verticals import aspheric_lens
    return aspheric_lens(shape=shape, **kwargs)


PRIMITIVES = {
    "sphere": sphere,
    "torus_knot": torus_knot,
    "lattice_cube": lattice_cube,
    "dental_crown": _dental_crown_primitive,
    "microfluidic_chip": _microfluidic_chip_primitive,
    "aspheric_lens": _aspheric_lens_primitive,
}


# ---------------------------------------------------------------------------
# STL voxelization (binary/ASCII), ray-parity along z, vectorized per triangle

def load_stl_triangles(path: str) -> np.ndarray:
    """Return (T, 3, 3) float64 triangle vertices in file units."""
    with open(path, "rb") as f:
        head = f.read(84)
        if len(head) >= 84 and head[:5].lower() != b"solid":
            n = struct.unpack("<I", head[80:84])[0]
            data = np.frombuffer(f.read(n * 50), dtype=np.uint8).reshape(n, 50)
            tris = data[:, 12:48].copy().view("<f4").reshape(n, 3, 3)
            return tris.astype(np.float64)
    # ASCII fallback
    verts: list[list[tuple[float, float, float]]] = []
    cur: list[tuple[float, float, float]] = []
    with open(path, errors="ignore") as f:
        for line in f:
            s = line.split()
            if s and s[0] == "vertex":
                cur.append((float(s[1]), float(s[2]), float(s[3])))
                if len(cur) == 3:
                    verts.append(cur)
                    cur = []
    if not verts:
        raise ValueError(f"no triangles parsed from {path}")
    return np.asarray(verts, dtype=np.float64)


def voxelize_stl(path: str, shape: tuple[int, int, int],
                 fit_margin: float = 0.05) -> VoxelVolume:
    """Ray-parity voxelization along z. Mesh is auto-fit to the grid."""
    tris = load_stl_triangles(path)
    lo = tris.reshape(-1, 3).min(axis=0)
    hi = tris.reshape(-1, 3).max(axis=0)
    scale = (hi - lo).max()
    scale = scale if scale > 0 else 1.0
    mid = (lo + hi) / 2
    tris = (tris - mid) / scale  # centered in [-0.5, 0.5]
    tris = tris * (1.0 - 2 * fit_margin)

    nz, ny, nx = shape
    xs = ((np.arange(nx) + 0.5) / nx - 0.5)
    ys = ((np.arange(ny) + 0.5) / ny - 0.5)
    zs = ((np.arange(nz) + 0.5) / nz - 0.5)

    parity = np.zeros(shape, dtype=np.int32)
    v0, v1, v2 = tris[:, 0], tris[:, 1], tris[:, 2]

    # only triangles not parallel to z rays contribute
    e1 = v1 - v0
    e2 = v2 - v0
    cross_z = e1[:, 0] * e2[:, 1] - e1[:, 1] * e2[:, 0]
    keep = np.abs(cross_z) > 1e-12
    v0, v1, v2, cross_z = v0[keep], v1[keep], v2[keep], cross_z[keep]
    # orient consistently upward-facing normal sign irrelevant for parity
    flip = cross_z < 0
    v1f = v1.copy()
    v2f = v2.copy()
    v1f[flip], v2f[flip] = v2[flip], v1[flip]

    for ti in range(len(v0)):
        ax, ay, _ = v0[ti]
        bx, by, _ = v1f[ti]
        cx, cy, _ = v2f[ti]
        xmin = min(ax, bx, cx)
        xmax = max(ax, bx, cx)
        ymin = min(ay, by, cy)
        ymax = max(ay, by, cy)
        ix0 = int(np.searchsorted(xs, xmin))
        ix1 = int(np.searchsorted(xs, xmax, side="right"))
        iy0 = int(np.searchsorted(ys, ymin))
        iy1 = int(np.searchsorted(ys, ymax, side="right"))
        if ix0 >= ix1 or iy0 >= iy1:
            continue
        X, Y = np.meshgrid(xs[ix0:ix1], ys[iy0:iy1])
        # barycentric containment in xy
        den = (by - cy) * (ax - cx) + (cx - bx) * (ay - cy)
        if abs(den) < 1e-15:
            continue
        w0 = ((by - cy) * (X - cx) + (cx - bx) * (Y - cy)) / den
        w1 = ((cy - ay) * (X - cx) + (ax - cx) * (Y - cy)) / den
        w2 = 1 - w0 - w1
        hit = (w0 >= 0) & (w1 >= 0) & (w2 >= 0)
        if not hit.any():
            continue
        zx = v0[ti][2] + w1 * (v1f[ti][2] - v0[ti][2]) + w2 * (v2f[ti][2] - v0[ti][2])
        iz = np.searchsorted(zs, zx[hit])
        valid = (iz > 0) & (iz <= nz)
        if not valid.any():
            continue
        # entering/exiting: for ray parity we only need crossing counts
        ihit = iz[valid]
        iy, ix = np.nonzero(hit)
        iy = iy[valid]
        ix = ix[valid]
        parity[ihit - 1, iy0 + iy, ix0 + ix] += 1

    solid = (np.cumsum(parity, axis=0) % 2) == 1
    return VoxelVolume(solid)


def export_stl(volume: VoxelVolume, path: str) -> None:
    """Dependency-free voxel-surface STL export: two triangles per exposed
    voxel face (boundary cells between solid and empty neighbours)."""
    solid = volume.mask
    nz, ny, nx = solid.shape
    out_tris: list[tuple[tuple[float, float, float], ...]] = []

    # exposed-face masks per direction
    zsl = solid & ~np.roll(solid, 1, axis=0)
    zsr = solid & ~np.roll(solid, -1, axis=0)
    ysf = solid & ~np.roll(solid, 1, axis=1)
    ysb = solid & ~np.roll(solid, -1, axis=1)
    xsl = solid & ~np.roll(solid, 1, axis=2)
    xsr = solid & ~np.roll(solid, -1, axis=2)

    def mk(zi: int, yi: int, xi: int, dx: float = 0.0, dy: float = 0.0, dzv: float = 0.0):
        return ((xi + dx) / nx - 0.5, (yi + dy) / ny - 0.5, (zi + dzv) / nz - 0.5)

    quads = (
        (zsl, lambda z, y, x: [mk(z, y, x, 0, 0, 0), mk(z, y, x, 1, 0, 0), mk(z, y, x, 1, 1, 0), mk(z, y, x, 0, 1, 0)]),
        (zsr, lambda z, y, x: [mk(z, y, x, 0, 0, 1), mk(z, y, x, 0, 1, 1), mk(z, y, x, 1, 1, 1), mk(z, y, x, 1, 0, 1)]),
        (ysf, lambda z, y, x: [mk(z, y, x, 0, 0, 0), mk(z, y, x, 0, 0, 1), mk(z, y, x, 1, 0, 1), mk(z, y, x, 1, 0, 0)]),
        (ysb, lambda z, y, x: [mk(z, y, x, 0, 1, 0), mk(z, y, x, 1, 1, 0), mk(z, y, x, 1, 1, 1), mk(z, y, x, 0, 1, 1)]),
        (xsl, lambda z, y, x: [mk(z, y, x, 0, 0, 0), mk(z, y, x, 0, 1, 0), mk(z, y, x, 0, 1, 1), mk(z, y, x, 0, 0, 1)]),
        (xsr, lambda z, y, x: [mk(z, y, x, 1, 0, 0), mk(z, y, x, 1, 0, 1), mk(z, y, x, 1, 1, 1), mk(z, y, x, 1, 1, 0)]),
    )

    for mask3d, corners in quads:
        for zi, yi, xi in np.argwhere(mask3d):
            p = corners(int(zi), int(yi), int(xi))
            out_tris.append((p[0], p[1], p[2]))
            out_tris.append((p[0], p[2], p[3]))

    with open(path, "wb") as f:
        header = b"holoforge voxel export".ljust(80, b"\0")
        f.write(header)
        f.write(struct.pack("<I", len(out_tris)))
        for t in out_tris:
            # Binary STL: normal (3f) + 3 vertices (9f) + attribute (H)
            f.write(struct.pack("<3f", 0.0, 0.0, 0.0))  # face normal (auto-compute by viewer)
            f.write(struct.pack("<9f", *(c for pt in t for c in pt)))
            f.write(struct.pack("<H", 0))


def dilate(mask: np.ndarray, iterations: int = 1) -> np.ndarray:
    """6-connected dilation without scipy."""
    out = mask.copy()
    for _ in range(iterations):
        grown = out.copy()
        grown[:-1] |= out[1:]
        grown[1:] |= out[:-1]
        grown[:, :-1] |= out[:, 1:]
        grown[:, 1:] |= out[:, :-1]
        grown[..., :-1] |= out[..., 1:]
        grown[..., 1:] |= out[..., :-1]
        out = grown
    return out
