"""Tests for holoforge.volume — primitives, STL I/O, dilate, grid_of."""
import numpy as np

from holoforge.volume import (
    PRIMITIVES,
    dilate,
    export_stl,
    grid_of,
    lattice_cube,
    sphere,
    torus_knot,
    voxelize_stl,
)


class TestPrimitives:
    """All primitives produce valid non-empty volumes with correct shape."""

    def test_sphere(self):
        vol = sphere(radius=0.28, shape=(16, 32, 32))
        assert vol.shape == (16, 32, 32)
        assert vol.voxel_count > 0

    def test_torus_knot(self):
        vol = torus_knot(p=2, q=3, shape=(16, 32, 32))
        assert vol.shape == (16, 32, 32)
        assert vol.voxel_count > 0

    def test_lattice_cube(self):
        vol = lattice_cube(cells=3, shape=(16, 32, 32))
        assert vol.shape == (16, 32, 32)
        assert vol.voxel_count > 0

    def test_primitives_dict_complete(self):
        expected = {"sphere", "torus_knot", "lattice_cube", "dental_crown", "microfluidic_chip", "aspheric_lens"}
        assert set(PRIMITIVES.keys()) == expected


class TestGridOf:
    def test_shape_and_range(self):
        z, y, x = grid_of((16, 32, 32))
        assert z.shape == (16, 1, 1)
        assert y.shape == (1, 32, 1)
        assert x.shape == (1, 1, 32)
        # all coords in [-0.5, 0.5)
        for arr in (z, y, x):
            assert arr.min() >= -0.5
            assert arr.max() < 0.5


class TestDilate:
    def test_grows_volume(self):
        vol = sphere(radius=0.15, shape=(16, 32, 32))
        original_count = int(vol.mask.sum())
        dilated = dilate(vol.mask, iterations=2)
        assert int(dilated.sum()) > original_count

    def test_preserves_shape(self):
        mask = np.zeros((8, 8, 8), dtype=bool)
        mask[4, 4, 4] = True
        result = dilate(mask, iterations=1)
        assert result.shape == (8, 8, 8)
        # single voxel dilated once -> 7 voxels (center + 6 neighbors)
        assert int(result.sum()) == 7


class TestStlRoundTrip:
    def test_export_and_reimport(self, tmp_path):
        vol = sphere(radius=0.25, shape=(16, 32, 32))
        stl_path = str(tmp_path / "sphere.stl")
        export_stl(vol, stl_path)

        # Reimport and check shape
        reimported = voxelize_stl(stl_path, (16, 32, 32))
        assert reimported.shape == (16, 32, 32)
        assert reimported.voxel_count > 0

        # IoU between original and reimported should be reasonable
        inter = np.logical_and(vol.mask, reimported.mask).sum()
        union = np.logical_or(vol.mask, reimported.mask).sum()
        iou = inter / union if union > 0 else 0
        assert iou > 0.1, f"STL round-trip IoU too low: {iou:.3f}"
