"""Tests for holoforge.io_hfp — save/load round-trip."""
import json

import numpy as np

from holoforge.io_hfp import load_job, save_job


class TestSaveLoadRoundTrip:
    def test_roundtrip_preserves_data(self, small_job, tmp_path):
        path = str(tmp_path / "test_job.hfp")
        save_job(small_job, path)

        loaded = load_job(path)
        assert loaded.optics.slm_pixels == small_job.optics.slm_pixels
        assert loaded.optics.wavelength_nm == small_job.optics.wavelength_nm
        assert loaded.grid.n_xy == small_job.grid.n_xy
        assert loaded.grid.n_z == small_job.grid.n_z
        assert loaded.resin.name == small_job.resin.name
        assert loaded.exposure_s == small_job.exposure_s
        np.testing.assert_array_equal(loaded.target_mask, small_job.target_mask)
        np.testing.assert_allclose(loaded.phases_rad, small_job.phases_rad, atol=1e-6)
        assert loaded.pattern_weights == small_job.pattern_weights

    def test_json_sidecar_valid(self, small_job, tmp_path):
        path = str(tmp_path / "test_job.hfp")
        save_job(small_job, path)

        with open(path + ".json", encoding="utf-8") as f:
            data = json.load(f)
        assert "optics" in data
        assert "grid" in data
        assert "resin" in data
        assert "pattern_weights" in data
        assert "exposure_s" in data
        assert "metrics" in data

    def test_metrics_preserved(self, small_job, tmp_path):
        path = str(tmp_path / "test_job.hfp")
        save_job(small_job, path)
        loaded = load_job(path)
        assert loaded.metrics.get("cure_iou") == small_job.metrics.get("cure_iou")
