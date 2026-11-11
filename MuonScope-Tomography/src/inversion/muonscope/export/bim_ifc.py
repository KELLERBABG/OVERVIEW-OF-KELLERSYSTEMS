"""IFC4 export with real tessellated geometry.

What this replaced
------------------
The previous exporter wrote a STEP header, then two ``/* comment */`` lines, and
stopped. The output contained no geometry and no IFC entity of any kind beyond the
project stub, while the docstring promised ``IfcVoidingFeature`` and
``IfcReinforcingBar`` entities and the audit reported the file as a pass.

What this writes
----------------
A conformant IFC4 STEP file with:

* a spatial structure -- IfcProject, IfcSite, IfcBuilding, IfcBuildingStorey --
  with the elements contained in the storey through IfcRelContainedInSpatialStructure;
* geometry as ``IfcTriangulatedFaceSet``, whose coordinates come from marching
  tetrahedra over the reconstructed volume, one face set per density class;
* ``IfcVoidingFeature`` for low-density anomalies, tied to the bulk element with
  ``IfcRelVoidsElement`` -- which is what a voiding feature means in IFC.

One deliberate departure from the old docstring: high-density anomalies are
exported as ``IfcBuildingElementProxy``, not ``IfcReinforcingBar``. A muon
scattering reconstruction measures density, not bar diameter, grade or cover, so
labelling an anomaly as a reinforcing bar would assert something the measurement
cannot support. The element is named for what it is.
"""
from __future__ import annotations

import hashlib
import os
from dataclasses import dataclass, field
from datetime import datetime, timezone
from typing import Dict, List, Optional, Sequence, Tuple

import numpy as np

from .mesh_vtk import marching_tetrahedra

# IFC GlobalId alphabet: digits, upper, lower, underscore, dollar.
_GUID_ALPHABET = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$"

IFC_SCHEMA = "IFC4"


class IfcExportError(RuntimeError):
    pass


def ifc_guid(*parts: str) -> str:
    """Deterministic 22-character IFC GlobalId.

    Deterministic so that re-exporting the same reconstruction produces the same
    identity, which is what lets a BIM model be updated in place rather than
    duplicated. A random GUID per run would silently fork the model.
    """
    digest = hashlib.sha256("|".join(parts).encode("utf-8")).digest()
    value = int.from_bytes(digest[:16], "big")
    chars = []
    for _ in range(22):
        chars.append(_GUID_ALPHABET[value & 0x3F])
        value >>= 6
    return "".join(chars)


class StepWriter:
    """Collects STEP entity lines and hands out reference numbers."""

    def __init__(self):
        self._lines: List[str] = []
        self._next = 1

    def add(self, body: str) -> int:
        ref = self._next
        self._next += 1
        self._lines.append(f"#{ref}={body};")
        return ref

    def reserve(self) -> int:
        ref = self._next
        self._next += 1
        self._lines.append("")
        return ref

    def set(self, ref: int, body: str) -> None:
        self._lines[ref - 1] = f"#{ref}={body};"

    def body(self) -> str:
        return "\n".join(line for line in self._lines if line)


def _step_number(value: float) -> str:
    """STEP REAL: must contain a decimal point."""
    text = f"{float(value):.6g}"
    if "." not in text and "e" not in text and "E" not in text:
        text += "."
    return text


def _cartesian_point_list(points: np.ndarray) -> str:
    coords = ",".join(
        f"({_step_number(p[0])},{_step_number(p[1])},{_step_number(p[2])})"
        for p in points)
    return f"IFCCARTESIANPOINTLIST3D(({coords}))"


def _coord_index(vertices: np.ndarray, faces: np.ndarray, max_index: int = 4096) -> str:
    indices = ",".join(f"({int(f[0]) + 1},{int(f[1]) + 1},{int(f[2]) + 1})"
                       for f in faces)
    return f"(({indices}))".replace("((", "((", 1)


@dataclass
class IfcExportSummary:
    void_elements: int
    anomaly_elements: int
    bulk_triangles: int
    void_triangles: int
    anomaly_triangles: int
    total_voxels: int
    vertices: int
    file_bytes: int
    entities: int
    void_threshold: float = 0.0
    anomaly_threshold: float = 0.0
    enclosed_void_voxels: int = 0

    def as_dict(self) -> Dict[str, object]:
        return {
            "void_elements": self.void_elements,
            "anomaly_elements": self.anomaly_elements,
            "bulk_triangles": self.bulk_triangles,
            "void_triangles": self.void_triangles,
            "anomaly_triangles": self.anomaly_triangles,
            "total_voxels": self.total_voxels,
            "vertices": self.vertices,
            "file_bytes": self.file_bytes,
            "entities": self.entities,
            "void_threshold": self.void_threshold,
            "anomaly_threshold": self.anomaly_threshold,
            "enclosed_void_voxels": self.enclosed_void_voxels,
        }


def _class_mesh(mask: np.ndarray, spacing: Sequence[float], origin: Sequence[float]):
    """Tessellate a boolean mask, returning (vertices, faces)."""
    if not mask.any():
        return np.zeros((0, 3)), np.zeros((0, 3), dtype=np.int64)
    # A signed field that is positive inside the mask and negative outside, so
    # the level-0 isosurface is the mask boundary. A light smoothing pass keeps
    # the surface from staircasing along voxel edges.
    from scipy import ndimage
    smooth = ndimage.uniform_filter(mask.astype(np.float64), size=3, mode="nearest")
    field = smooth - 0.5
    if field.max() <= 0.0 or field.min() >= 0.0:
        return np.zeros((0, 3)), np.zeros((0, 3), dtype=np.int64)
    if min(mask.shape) < 2:
        return np.zeros((0, 3)), np.zeros((0, 3), dtype=np.int64)
    return marching_tetrahedra(field, 0.0, spacing=spacing, origin=origin)


def _enclosed_components(mask: np.ndarray) -> np.ndarray:
    """Keep only connected components of ``mask`` that do not touch the boundary.

    A low-density region that reaches the edge of the survey volume is not an
    enclosed void -- it is simply where the measurement stops. Meshing it would
    emit the outer shell of the whole domain as an ``IfcVoidingFeature``, which
    is both wrong and enormous (it dominated the file: 48k triangles against 2k
    for the actual structure).
    """
    from scipy import ndimage
    if not mask.any():
        return np.zeros_like(mask, dtype=bool)
    structure = ndimage.generate_binary_structure(3, 1)
    labels, count = ndimage.label(mask, structure=structure)
    if count == 0:
        return np.zeros_like(mask, dtype=bool)

    border_labels = set()
    for axis in (0, 1, 2):
        for index in (0, -1):
            face = np.take(labels, index, axis=axis).ravel()
            border_labels.update(int(v) for v in np.unique(face) if v != 0)
    keep = np.isin(labels, [n for n in range(1, count + 1) if n not in border_labels])
    return keep


def _auto_thresholds(grid: np.ndarray) -> Tuple[float, float]:
    """Void and anomaly thresholds derived from the volume itself.

    The documented constants were wrong in a way that silently produced nothing:
    ``void_threshold = 0.15`` sits *above* concrete's own scattering density
    (lambda = 1/11.55 = 0.0866), so the entire concrete matrix was classed as void
    and then discarded as boundary-touching; and ``rebar_threshold = 5.5`` is
    above lead (1.79), so no anomaly could ever be emitted.

    The rule used here is a robust one on the distribution of reconstructed
    values: the background is the median, the spread is the interquartile range,
    and the two classes are placed a fixed number of spreads either side. A floor
    on the spread keeps a perfectly uniform synthetic volume from making every
    voxel an outlier.
    """
    flat = grid.ravel()
    background = float(np.median(flat))
    q25, q75 = (float(v) for v in np.percentile(flat, [25, 75]))
    spread = q75 - q25
    if spread <= 0.0:
        spread = max(0.05 * abs(background), 1e-9)
    void_threshold = background - 4.0 * spread
    anomaly_threshold = background + 8.0 * spread
    return void_threshold, anomaly_threshold


def export_to_ifc4(
    density_grid: np.ndarray,
    bounds_min_mm: Tuple[float, float, float],
    bounds_max_mm: Tuple[float, float, float],
    output_filepath: str,
    void_threshold: Optional[float] = None,
    rebar_threshold: Optional[float] = None,
    max_triangles: int = 400_000,
) -> Dict[str, int]:
    """Write a reconstructed volume as an IFC4 model with tessellated geometry.

    Parameters
    ----------
    density_grid : (nx, ny, nz)
        Reconstructed scattering density in cm^-1.
    bounds_min_mm, bounds_max_mm : the survey volume, in millimetres.
    void_threshold : voxels below this are candidates for structural voids. Only
        *enclosed* low-density components become features. ``None`` derives it
        from the volume.
    rebar_threshold : voxels above this are high-density anomalies. ``None``
        derives it from the volume.
    max_triangles : refuse rather than silently decimate.

    Returns
    -------
    Summary counts, including the triangle counts actually written and the
    thresholds used, so a caller can tell what classification was applied.
    """
    grid = np.asarray(density_grid, dtype=np.float64)
    if grid.ndim != 3:
        raise ValueError(f"expected a 3-D density grid, got shape {grid.shape}")

    nx, ny, nz = grid.shape
    span = np.asarray(bounds_max_mm, dtype=np.float64) - np.asarray(bounds_min_mm, dtype=np.float64)
    spacing = span / np.array(grid.shape, dtype=np.float64)
    origin = np.asarray(bounds_min_mm, dtype=np.float64)

    auto_void, auto_anomaly = _auto_thresholds(grid)
    void_cut = auto_void if void_threshold is None else float(void_threshold)
    anomaly_cut = auto_anomaly if rebar_threshold is None else float(rebar_threshold)

    bulk_mask = grid > void_cut
    void_mask = _enclosed_components(~bulk_mask)
    anomaly_mask = grid > anomaly_cut

    bulk_verts, bulk_faces = _class_mesh(bulk_mask, spacing, origin)
    void_verts, void_faces = _class_mesh(void_mask, spacing, origin)
    anom_verts, anom_faces = _class_mesh(anomaly_mask, spacing, origin)

    total_triangles = len(bulk_faces) + len(void_faces) + len(anom_faces)
    if total_triangles > max_triangles:
        raise IfcExportError(
            f"tessellation produced {total_triangles} triangles, above max_triangles="
            f"{max_triangles}. Reduce the grid resolution or raise the limit; "
            "silently decimating would misrepresent the reconstruction.")

    w = StepWriter()
    now = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%S")
    root = hashlib.sha256(np.ascontiguousarray(grid).tobytes()).hexdigest()[:16]

    # -- ownership and units --------------------------------------------------
    person = w.add("IFCPERSON($,'MuonScope','Operator',$,$,$,$,$)")
    org = w.add("IFCORGANIZATION($,'MuonScope Tomography',$,$,$)")
    p_and_o = w.add(f"IFCPERSONANDORGANIZATION(#{person},#{org},$)")
    app = w.add(f"IFCAPPLICATION(#{org},'1.0','MuonScope','MSCOPE')")
    owner = w.add(f"IFCOWNERHISTORY(#{p_and_o},#{app},$,.ADDED.,$,$,$,0)")

    u_len = w.add("IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.)")
    u_area = w.add("IFCSIUNIT(*,.AREAUNIT.,$,.SQUARE_METRE.)")
    u_vol = w.add("IFCSIUNIT(*,.VOLUMEUNIT.,$,.CUBIC_METRE.)")
    u_ang = w.add("IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.)")
    units = w.add(f"IFCUNITASSIGNMENT((#{u_len},#{u_area},#{u_vol},#{u_ang}))")

    origin_pt = w.add("IFCCARTESIANPOINT((0.,0.,0.))")
    axis = w.add(f"IFCAXIS2PLACEMENT3D(#{origin_pt},$,$)")
    ctx = w.add(f"IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#{axis},$)")
    body_ctx = w.add(f"IFCGEOMETRICREPRESENTATIONSUBCONTEXT('Body','Model',*,*,*,*,#{ctx},$,.MODEL_VIEW.,$)")
    world_placement = w.add(f"IFCLOCALPLACEMENT($,#{axis})")

    project = w.add(
        f"IFCPROJECT('{ifc_guid(root, 'project')}',#{owner},"
        f"'MuonScope Structural Inspection',$,$,$,$,(#{ctx}),#{units})")
    site_place = w.add(f"IFCLOCALPLACEMENT(#{world_placement},#{axis})")
    site = w.add(
        f"IFCSITE('{ifc_guid(root, 'site')}',#{owner},'Survey site',$,$,#{site_place},$,$,"
        ".ELEMENT.,$,$,$,$,$)")
    building_place = w.add(f"IFCLOCALPLACEMENT(#{site_place},#{axis})")
    building = w.add(
        f"IFCBUILDING('{ifc_guid(root, 'building')}',#{owner},'Inspected structure',$,$,"
        f"#{building_place},$,$,.ELEMENT.,$,$,$)")
    storey_place = w.add(f"IFCLOCALPLACEMENT(#{building_place},#{axis})")
    storey = w.add(
        f"IFCBUILDINGSTOREY('{ifc_guid(root, 'storey')}',#{owner},'Reconstructed volume',$,$,"
        f"#{storey_place},$,$,.ELEMENT.,0.)")

    w.add(f"IFCRELAGGREGATES('{ifc_guid(root, 'agg1')}',#{owner},$,$,#{project},(#{site}))")
    w.add(f"IFCRELAGGREGATES('{ifc_guid(root, 'agg2')}',#{owner},$,$,#{site},(#{building}))")
    w.add(f"IFCRELAGGREGATES('{ifc_guid(root, 'agg3')}',#{owner},$,$,#{building},(#{storey}))")

    def product(verts: np.ndarray, faces: np.ndarray, name: str,
                kind: str = "proxy") -> Optional[int]:
        if len(faces) == 0:
            return None
        pt_list = w.add(_cartesian_point_list(verts))
        tri_set = w.add(
            f"IFCTRIANGULATEDFACESET(#{pt_list},$,$,{_coord_index(verts, faces)},$)")
        shape = w.add(
            f"IFCSHAPEREPRESENTATION(#{body_ctx},'Body','Tessellation',(#{tri_set}))")
        pds = w.add(f"IFCPRODUCTDEFINITIONSHAPE($,$,(#{shape}))")
        place = w.add(f"IFCLOCALPLACEMENT(#{storey_place},#{axis})")
        guid = ifc_guid(root, name)
        if kind == "void":
            return w.add(
                f"IFCVOIDINGFEATURE('{guid}',#{owner},'{name}',$,$,#{place},#{pds},$)")
        return w.add(
            f"IFCBUILDINGELEMENTPROXY('{guid}',#{owner},'{name}',"
            "'Reconstructed density class from muon scattering',$,$,"
            f"#{place},#{pds},$,.ELEMENT.)")

    bulk = product(bulk_verts, bulk_faces, "Reconstructed bulk material")
    anomaly = product(anom_verts, anom_faces, "High-density anomaly")
    void = product(void_verts, void_faces, "Low-density feature", kind="void")

    contained = [r for r in (bulk, anomaly, void) if r is not None]
    if contained:
        refs = ",".join(f"#{r}" for r in contained)
        w.add(f"IFCRELCONTAINEDINSPATIALSTRUCTURE('{ifc_guid(root, 'contain')}',"
              f"#{owner},$,$,({refs}),#{storey})")
    if bulk is not None and void is not None:
        w.add(f"IFCRELVOIDSELEMENT('{ifc_guid(root, 'voids')}',#{owner},$,$,#{bulk},#{void})")

    header = (
        "ISO-10303-21;\n"
        "HEADER;\n"
        "FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');\n"
        f"FILE_NAME('{os.path.basename(output_filepath)}','{now}',"
        "('MuonScope Tomography'),('MuonScope'),'MuonScope 1.0','MuonScope','');\n"
        f"FILE_SCHEMA(('{IFC_SCHEMA}'));\n"
        "ENDSEC;\n"
        "DATA;\n"
    )
    footer = "ENDSEC;\nEND-ISO-10303-21;\n"

    text = header + w.body() + "\n" + footer
    os.makedirs(os.path.dirname(os.path.abspath(output_filepath)) or ".", exist_ok=True)
    with open(output_filepath, "w", encoding="utf-8") as handle:
        handle.write(text)

    return IfcExportSummary(
        void_elements=1 if void is not None else 0,
        anomaly_elements=1 if anomaly is not None else 0,
        bulk_triangles=int(len(bulk_faces)),
        void_triangles=int(len(void_faces)),
        anomaly_triangles=int(len(anom_faces)),
        total_voxels=int(nx * ny * nz),
        vertices=int(len(bulk_verts) + len(void_verts) + len(anom_verts)),
        file_bytes=len(text.encode("utf-8")),
        entities=w._next - 1,
        void_threshold=void_cut,
        anomaly_threshold=anomaly_cut,
        enclosed_void_voxels=int(void_mask.sum()),
    ).as_dict()
