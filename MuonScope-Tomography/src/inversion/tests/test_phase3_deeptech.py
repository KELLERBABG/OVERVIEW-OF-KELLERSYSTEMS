import os
import numpy as np
import pytest

from muonscope.reconstructors.joint_grav import JointMuonGravSolver
from muonscope.export.bim_ifc import export_to_ifc4
from muonscope.export.mesh_vtk import (
    extract_isosurface_mesh,
    export_to_obj,
    export_to_ply,
    is_watertight,
    mesh_volume_mm3,
    read_obj,
)
from muonscope.export.dicom_export import export_to_dicom_series, read_dicom_series

def test_joint_muon_grav_solver():
    grid = (16, 16, 16)
    solver = JointMuonGravSolver(grid_size=grid, alpha_grav=0.1, max_iterations=5)

    n_voxels = grid[0] * grid[1] * grid[2]
    w_mat = np.eye(n_voxels, dtype=np.float32)
    angles_sq = np.full(n_voxels, 0.05, dtype=np.float32)
    grav_measurements = np.zeros(20, dtype=np.float32)

    recon = solver.solve(w_mat, angles_sq, grav_measurements)
    assert recon.shape == grid
    assert np.all(recon >= 0.0)

def test_cad_and_dicom_exporters(tmp_path):
    volume = np.zeros((16, 16, 16), dtype=np.float32)
    volume[4:12, 4:12, 4:12] = 0.5 # central inclusion

    # Isosurface extraction: the inclusion is enclosed, so the mesh must close.
    verts, faces = extract_isosurface_mesh(volume, isovalue=0.25)
    assert len(verts) > 0
    assert len(faces) > 0
    assert is_watertight(faces), "enclosed inclusion must give a closed surface"
    # Euler characteristic 2 for a single closed surface.
    edges = set()
    for face in faces:
        for u, v in ((face[0], face[1]), (face[1], face[2]), (face[2], face[0])):
            edges.add((int(min(u, v)), int(max(u, v))))
    assert len(verts) - len(edges) + len(faces) == 2
    # The extracted volume should match the 8x8x8 voxel block.
    assert mesh_volume_mm3(verts, faces) > 0.5 * 8 ** 3

    obj_path = str(tmp_path / "model.obj")
    ply_path = str(tmp_path / "model.ply")
    export_to_obj(verts, faces, obj_path)
    export_to_ply(verts, faces, ply_path)
    assert os.path.exists(obj_path)
    assert os.path.exists(ply_path)
    read_verts, read_faces = read_obj(obj_path)
    assert len(read_verts) == len(verts)
    assert len(read_faces) == len(faces)

    # DICOM series: must actually be DICOM, and must round-trip the physics.
    dcm_dir = str(tmp_path / "dicom")
    spacing = (2.0, 2.0, 1.0)
    count = export_to_dicom_series(volume, dcm_dir, pixel_spacing_mm=spacing)
    assert count == 16
    first = os.path.join(dcm_dir, "slice_0000.dcm")
    assert os.path.exists(first)

    with open(first, "rb") as handle:
        raw = handle.read()
    assert raw[128:132] == b"DICM", "must be a Part 10 file, not a raw array"

    slices = read_dicom_series(dcm_dir)
    assert len(slices) == 16
    assert [int(s.get((0x0020, 0x0013))) for s in slices] == list(range(1, 17))
    recovered = slices[0].as_physical()
    np.testing.assert_allclose(recovered, volume[:, :, 0].T, atol=1e-6)


def test_ifc4_bim_feature_generation(tmp_path):
    concrete = 1.0 / 11.55       # cm^-1
    steel = 1.0 / 1.76

    volume = np.full((16, 16, 16), concrete, dtype=np.float32)
    volume[2:5, 2:5, 2:5] = 0.005      # air void, enclosed
    volume[10:14, 10:14, 10:14] = steel

    ifc_path = str(tmp_path / "bim_model.ifc")
    stats = export_to_ifc4(volume, (-100, -100, -100), (100, 100, 100), ifc_path)
    assert stats["void_elements"] == 1
    assert stats["anomaly_elements"] == 1
    assert os.path.exists(ifc_path)

    text = open(ifc_path, encoding="utf-8").read()
    # Real geometry, not two comments.
    assert "IFCTRIANGULATEDFACESET" in text
    assert "IFCCARTESIANPOINTLIST3D" in text
    assert "IFCRELCONTAINEDINSPATIALSTRUCTURE" in text
    # No reference may point at an entity that does not exist.
    import re
    defined = {int(m.group(1)) for m in re.finditer(r"#(\d+)=", text)}
    referenced = {int(m.group(1)) for m in re.finditer(r"#(\d+)", text)}
    assert not (referenced - defined), f"dangling references {sorted(referenced - defined)}"
