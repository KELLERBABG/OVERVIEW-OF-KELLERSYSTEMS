import numpy as np
import pytest

from muonscope.simulation.synthetic_scene import create_void_phantom, create_heavy_inclusion_phantom
from muonscope.simulation.ray_tracer import MonteCarloRayTracer
from muonscope.physics.gaisser import sample_cosmic_muons
from muonscope.reconstructors.siddon import compute_siddon_ray_voxel_intersections, build_sparse_system_matrix
from muonscope.reconstructors.map_em import MAPEMReconstructor
from muonscope.reconstructors.momentum_est import estimate_momentum_from_deflection
from muonscope.export.bim_ifc import export_to_ifc4

def test_siddon_ray_intersection():
    p1 = np.array([0.0, 0.0, 100.0])
    p2 = np.array([0.0, 0.0, -100.0])
    grid = (10, 10, 10)
    b_min = (-50.0, -50.0, -50.0)
    b_max = (50.0, 50.0, 50.0)

    indices, lengths = compute_siddon_ray_voxel_intersections(p1, p2, grid, b_min, b_max)
    assert len(indices) == 10
    total_len_cm = sum(lengths)
    np.testing.assert_allclose(total_len_cm, 10.0, rtol=1e-3)

def test_map_em_reconstruction_convergence():
    grid = (16, 16, 16)
    scene = create_heavy_inclusion_phantom(grid_size=grid)
    tracer = MonteCarloRayTracer(scene, top_panel_z_mm=250.0, bottom_panel_z_mm=-250.0)

    energies, dirs, charges = sample_cosmic_muons(100)
    events = tracer.trace_batch(100, energies, dirs, charges)

    p_in = np.array([e.p_in for e in events])
    p_out = np.array([e.p_out for e in events])
    angles_sq = np.array([e.scattering_angle_rad**2 for e in events], dtype=np.float32)

    w_mat = build_sparse_system_matrix(p_in, p_out, grid, scene.bounds_min_mm, scene.bounds_max_mm)
    reconstructor = MAPEMReconstructor(grid_size=grid, max_iterations=5, use_torch=False)
    recon = reconstructor.reconstruct(w_mat, angles_sq)

    assert recon.shape == grid
    assert np.all(recon >= 0.0)

def test_momentum_estimation():
    p_est = estimate_momentum_from_deflection(0.01) # 10 mrad
    assert 1000.0 < p_est < 20000.0

def test_ifc_export(tmp_path):
    """A concrete block with an enclosed void and a steel inclusion.

    The previous version of this test used a volume that was zero everywhere
    except two single voxels, and asserted on ``void_elements`` / ``rebar_elements``
    -- which were then *voxel counts*, not IFC elements. There was no material for
    a void to be inside of, so nothing meaningful was being tested.
    """
    out_file = str(tmp_path / "test.ifc")

    concrete = 1.0 / 11.55          # lambda for concrete, cm^-1
    steel = 1.0 / 1.76              # lambda for steel

    grid = np.full((16, 16, 16), concrete, dtype=np.float32)
    grid[3:6, 3:6, 3:6] = 0.0005   # enclosed air void
    grid[10:13, 10:13, 10:13] = steel

    stats = export_to_ifc4(grid, (-100, -100, -100), (100, 100, 100), out_file)

    assert stats["void_elements"] == 1
    assert stats["anomaly_elements"] == 1
    assert stats["void_triangles"] > 0
    assert stats["anomaly_triangles"] > 0
    assert stats["bulk_triangles"] > 0

    text = open(out_file, encoding="utf-8").read()
    assert text.startswith("ISO-10303-21;")
    assert "IFCTRIANGULATEDFACESET" in text
    assert "IFCVOIDINGFEATURE" in text
    assert "IFCRELVOIDSELEMENT" in text
    assert text.count("(") == text.count(")")


def test_ifc_void_must_be_enclosed(tmp_path):
    """A low-density region touching the survey boundary is not a void.

    Meshing it would emit the outer shell of the whole domain as an
    IfcVoidingFeature instead of a feature inside the structure.
    """
    out_file = str(tmp_path / "boundary.ifc")
    grid = np.full((16, 16, 16), 1.0 / 11.55, dtype=np.float32)
    grid[0:3, 0:3, 0:3] = 0.0005          # open to the boundary

    stats = export_to_ifc4(grid, (-100, -100, -100), (100, 100, 100), out_file)
    assert stats["void_elements"] == 0
    assert stats["enclosed_void_voxels"] == 0
