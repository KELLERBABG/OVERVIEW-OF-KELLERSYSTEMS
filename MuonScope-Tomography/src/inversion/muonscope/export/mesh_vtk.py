"""Isosurface extraction and mesh export.

The previous implementation used ``skimage.measure.marching_cubes`` and, when
scikit-image was absent -- which it is, it appears in no dependency list --
silently returned a hardcoded 8-vertex unit cube. The shipped
``isosurface.obj`` was that cube, and the test suite passed it because the test
only asserted ``len(vertices) > 0``.

This module implements the extraction itself so the result never depends on an
undeclared optional import, and never falls back to a shape that has nothing to
do with the data.

Method: marching tetrahedra
---------------------------
Each cube of the voxel grid is split into six tetrahedra about a body diagonal.
A tetrahedron has only three topological cases -- no crossing, one triangle, or a
quad -- so the surface can be built without a 256-entry case table, and the
result is watertight by construction because every crossing edge is shared by
exactly one triangle per incident tetrahedron. It produces more triangles than
marching cubes for the same grid, which is a fair trade for an implementation
whose correctness can be read rather than trusted.
"""
from typing import Dict, List, Optional, Sequence, Tuple

import numpy as np

try:  # pragma: no cover - exercised only when scikit-image is installed
    from skimage.measure import marching_cubes as _skimage_marching_cubes
    HAVE_SKIMAGE = True
except ImportError:  # pragma: no cover
    _skimage_marching_cubes = None
    HAVE_SKIMAGE = False


# Cube corner offsets, indexed 0..7 by (i + 2j + 4k).
_CUBE_CORNERS = np.array([
    [0, 0, 0], [1, 0, 0], [1, 1, 0], [0, 1, 0],
    [0, 0, 1], [1, 0, 1], [1, 1, 1], [0, 1, 1],
], dtype=np.int64)

# Six tetrahedra filling the cube, all sharing the 0-6 body diagonal.
_TETRA_INDICES = np.array([
    [0, 5, 1, 6],
    [0, 1, 2, 6],
    [0, 2, 3, 6],
    [0, 3, 7, 6],
    [0, 7, 4, 6],
    [0, 4, 5, 6],
], dtype=np.int64)

# The six edges of a tetrahedron, as local corner pairs.
_TETRA_EDGES = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]

# Global edge identity for deduplication: (voxel ijk, cube-corner pair).
_CUBE_EDGE_PAIRS = [
    (0, 1), (1, 2), (2, 3), (3, 0),
    (4, 5), (5, 6), (6, 7), (7, 4),
    (0, 4), (1, 5), (2, 6), (3, 7),
]
_EDGE_LOOKUP: Dict[Tuple[int, int], int] = {}
for _e, (_a, _b) in enumerate(_CUBE_EDGE_PAIRS):
    _EDGE_LOOKUP[(_a, _b)] = _e
    _EDGE_LOOKUP[(_b, _a)] = _e


def _edge_key(cell: Tuple[int, int, int], ca: int, cb: int):
    """Identity of an edge, for vertex welding.

    Every edge the split can produce -- the twelve cube edges, the twelve face
    diagonals, and the body diagonal -- is a pair of grid corners, so keying by
    those two global corner coordinates is unambiguous and welds all of them.
    Keying face diagonals per-cell instead leaves two coincident vertices on each
    shared face, which still looks like a closed surface but is a mesh of
    disconnected shells: the divergence-theorem volume then comes out wrong and
    no downstream CAD tool can heal it.
    """
    a = np.array(cell, dtype=np.int64) + _CUBE_CORNERS[ca]
    b = np.array(cell, dtype=np.int64) + _CUBE_CORNERS[cb]
    ta = (int(a[0]), int(a[1]), int(a[2]))
    tb = (int(b[0]), int(b[1]), int(b[2]))
    return (ta, tb) if ta <= tb else (tb, ta)


def marching_tetrahedra(
    volume: np.ndarray,
    level: float,
    spacing: Sequence[float] = (1.0, 1.0, 1.0),
    origin: Sequence[float] = (0.0, 0.0, 0.0),
) -> Tuple[np.ndarray, np.ndarray]:
    """Extract the ``volume == level`` isosurface as a triangle mesh.

    Parameters
    ----------
    volume : (nx, ny, nz) array
    level : float
        Iso-level. Vertices are placed by linear interpolation along each
        crossing edge, so the surface is smooth in position, not staircased.
    spacing, origin : physical scaling and offset of the vertex coordinates.

    Returns
    -------
    vertices : (V, 3) float64
    faces : (F, 3) int64, indexing into ``vertices``.

    Raises
    ------
    ValueError if the volume is too small to contain a cell.
    """
    vol = np.asarray(volume, dtype=np.float64)
    if vol.ndim != 3:
        raise ValueError(f"expected a 3-D volume, got shape {vol.shape}")
    nx, ny, nz = vol.shape
    if nx < 2 or ny < 2 or nz < 2:
        raise ValueError(f"need at least a 2x2x2 volume, got {vol.shape}")

    spacing = np.asarray(spacing, dtype=np.float64)
    origin = np.asarray(origin, dtype=np.float64)

    # Pad by one voxel so the surface closes at the volume boundary instead of
    # being cut open. The pad value is below the level, so the surface is capped.
    pad_value = float(np.min(vol)) - abs(float(np.max(vol) - np.min(vol))) - 1.0
    padded = np.full((nx + 2, ny + 2, nz + 2), pad_value, dtype=np.float64)
    padded[1:-1, 1:-1, 1:-1] = vol

    inside = padded > level

    verts: List[np.ndarray] = []
    faces: List[List[int]] = []
    vertex_cache: Dict[Tuple[int, int, int, int], int] = {}

    def vertex_for(cell: Tuple[int, int, int], corner_a: int, corner_b: int) -> int:
        """Interpolated vertex on a voxel edge, deduplicated across cells."""
        key = _edge_key(cell, corner_a, corner_b)
        cached = vertex_cache.get(key)
        if cached is not None:
            return cached

        ca, cb = (corner_a, corner_b) if corner_a < corner_b else (corner_b, corner_a)
        pa = np.array(cell) + _CUBE_CORNERS[ca]
        pb = np.array(cell) + _CUBE_CORNERS[cb]
        va = padded[pa[0], pa[1], pa[2]]
        vb = padded[pb[0], pb[1], pb[2]]
        denom = vb - va
        t = 0.5 if abs(denom) < 1e-15 else (level - va) / denom
        t = float(np.clip(t, 0.0, 1.0))
        point = (pa + t * (pb - pa)).astype(np.float64)
        # Undo the one-voxel pad, then scale into physical coordinates.
        point -= 1.0
        point = origin + point * spacing

        index = len(verts)
        verts.append(point)
        vertex_cache[key] = index
        return index

    # Iterate over cells of the padded grid so boundary cells are included.
    for i in range(padded.shape[0] - 1):
        for j in range(padded.shape[1] - 1):
            for k in range(padded.shape[2] - 1):
                cell = (i, j, k)
                corner_pos = np.array([
                    [i, j, k], [i + 1, j, k], [i + 1, j + 1, k], [i, j + 1, k],
                    [i, j, k + 1], [i + 1, j, k + 1], [i + 1, j + 1, k + 1],
                    [i, j + 1, k + 1],
                ], dtype=np.int64)
                corner_vals = np.array([
                    padded[p[0], p[1], p[2]] for p in corner_pos], dtype=np.float64)
                corner_in = corner_vals > level
                if corner_in.all() or not corner_in.any():
                    continue

                for tet in _TETRA_INDICES:
                    tet_in = corner_in[tet]
                    n_in = int(np.count_nonzero(tet_in))
                    if n_in == 0 or n_in == 4:
                        continue

                    crossing = [
                        (a, b) for (a, b) in _TETRA_EDGES
                        if tet_in[a] != tet_in[b]
                    ]
                    if len(crossing) == 3:
                        tri = [vertex_for(cell, tet[a], tet[b]) for a, b in crossing]
                        faces.append(tri)
                    elif len(crossing) == 4:
                        # K(2,2): the crossing edges join each inside corner to
                        # each outside corner, which gives a proper quad cycle.
                        inside_local = [n for n in range(4) if tet_in[n]]
                        outside_local = [n for n in range(4) if not tet_in[n]]
                        a0, a1 = inside_local
                        b0, b1 = outside_local
                        quad = [
                            (a0, b0), (a0, b1), (a1, b1), (a1, b0),
                        ]
                        pts = [vertex_for(cell, tet[u], tet[v]) for u, v in quad]
                        faces.append([pts[0], pts[1], pts[2]])
                        faces.append([pts[0], pts[2], pts[3]])

    if not faces:
        return np.zeros((0, 3), dtype=np.float64), np.zeros((0, 3), dtype=np.int64)

    vertices = np.asarray(verts, dtype=np.float64)
    faces_arr = np.asarray(faces, dtype=np.int64)
    faces_arr = _orient_outward(vertices, faces_arr, level)
    return vertices, faces_arr


def _orient_outward(vertices: np.ndarray, faces: np.ndarray, level: float) -> np.ndarray:
    """Flip triangles whose normal points into the high-density region.

    Marching tetrahedra does not by itself impose a consistent winding, and an
    inconsistent mesh is not a valid boundary representation. Orientation is
    fixed per triangle from the local gradient of the field, which is available
    implicitly because the surface separates high from low.
    """
    tri = vertices[faces]
    normals = np.cross(tri[:, 1] - tri[:, 0], tri[:, 2] - tri[:, 0])
    centroid = tri.mean(axis=1)
    # Outward means "towards decreasing density"; approximate the gradient by
    # the direction from the triangle centroid to the volume centre of mass.
    direction = centroid - centroid.mean(axis=0)
    flip = np.einsum("ij,ij->i", normals, direction) < 0.0
    faces = faces.copy()
    faces[flip] = faces[flip][:, [0, 2, 1]]
    return faces


def mesh_volume_mm3(vertices: np.ndarray, faces: np.ndarray) -> float:
    """Signed volume enclosed by a closed triangle mesh, via the divergence theorem."""
    if len(faces) == 0:
        return 0.0
    tri = vertices[faces]
    a, b, c = tri[:, 0], tri[:, 1], tri[:, 2]
    return float(abs(np.sum(np.einsum("ij,ij->i", a, np.cross(b, c))) / 6.0))


def is_watertight(faces: np.ndarray) -> bool:
    """True when every edge is shared by exactly two triangles."""
    if len(faces) == 0:
        return False
    edges: Dict[Tuple[int, int], int] = {}
    for face in faces:
        for u, v in ((face[0], face[1]), (face[1], face[2]), (face[2], face[0])):
            key = (int(min(u, v)), int(max(u, v)))
            edges[key] = edges.get(key, 0) + 1
    return all(count == 2 for count in edges.values())


def extract_isosurface_mesh(
    volume: np.ndarray,
    isovalue: float,
    spacing: Tuple[float, float, float] = (1.0, 1.0, 1.0),
    origin: Tuple[float, float, float] = (0.0, 0.0, 0.0),
    prefer_skimage: bool = False,
) -> Tuple[np.ndarray, np.ndarray]:
    """Extract an isosurface.

    ``prefer_skimage`` opts into scikit-image's marching cubes when it is
    installed. The default is the built-in marching tetrahedra so that the result
    does not depend on an optional import. There is no fallback shape: if the
    volume is too small to mesh, this raises.
    """
    if prefer_skimage and HAVE_SKIMAGE:  # pragma: no cover - optional path
        verts, faces, _normals, _values = _skimage_marching_cubes(
            np.asarray(volume, dtype=np.float64), level=isovalue, spacing=spacing)
        verts = verts + np.asarray(origin, dtype=np.float64)
        return verts, faces.astype(np.int64)
    return marching_tetrahedra(volume, isovalue, spacing=spacing, origin=origin)


def export_to_obj(
    vertices: np.ndarray,
    faces: np.ndarray,
    output_filepath: str,
) -> None:
    """Write a Wavefront OBJ. Vertices are emitted in millimetres."""
    with open(output_filepath, "w", encoding="utf-8") as f:
        f.write("# MuonScope Tomography isosurface mesh\n")
        f.write(f"# vertices {len(vertices)}  faces {len(faces)}\n")
        f.write("# units: millimetre\n")
        for v in vertices:
            f.write(f"v {v[0]:.4f} {v[1]:.4f} {v[2]:.4f}\n")
        for face in faces:
            f.write(f"f {face[0] + 1} {face[1] + 1} {face[2] + 1}\n")


def export_to_ply(
    vertices: np.ndarray,
    faces: np.ndarray,
    output_filepath: str,
) -> None:
    """Write an ASCII PLY mesh."""
    with open(output_filepath, "w", encoding="utf-8") as f:
        f.write("ply\nformat ascii 1.0\n")
        f.write("comment MuonScope Tomography isosurface, units millimetre\n")
        f.write(f"element vertex {len(vertices)}\n")
        f.write("property float x\nproperty float y\nproperty float z\n")
        f.write(f"element face {len(faces)}\n")
        f.write("property list uchar int vertex_indices\n")
        f.write("end_header\n")
        for v in vertices:
            f.write(f"{v[0]:.4f} {v[1]:.4f} {v[2]:.4f}\n")
        for face in faces:
            f.write(f"3 {face[0]} {face[1]} {face[2]}\n")


def read_obj(path: str) -> Tuple[np.ndarray, np.ndarray]:
    """Read back an OBJ written by :func:`export_to_obj` (used by tests)."""
    verts: List[List[float]] = []
    faces: List[List[int]] = []
    with open(path, "r", encoding="utf-8") as f:
        for line in f:
            if line.startswith("v "):
                verts.append([float(x) for x in line.split()[1:4]])
            elif line.startswith("f "):
                faces.append([int(x.split("/")[0]) - 1 for x in line.split()[1:4]])
    return np.asarray(verts, dtype=np.float64), np.asarray(faces, dtype=np.int64)
