"""Tests for Stage 1, Stage 2, and Stage 3 modules & capabilities."""
import os

import numpy as np

from holoforge.calibration import (
    SLMCalibrationLUT,
    export_slm_images,
    flat_field_correction,
    generate_dye_cell_target,
    make_scurve_lut,
)
from holoforge.cli import main
from holoforge.config import GridConfig, OpticsConfig
from holoforge.kinetics import simulate_kinetics
from holoforge.metrology import (
    analyze_channel_patency,
    calculate_roughness,
    extract_topography_height_map,
)
from holoforge.player import play_sequence_simulated, prepare_hardware_bundle
from holoforge.verticals import aspheric_lens, dental_crown, microfluidic_chip


class TestStage1Calibration:
    def test_linear_lut_roundtrip(self):
        lut = SLMCalibrationLUT()
        phases = np.linspace(0.0, 2.0 * np.pi, 100, endpoint=False)
        drive = lut.phase_to_drive(phases)
        assert drive.dtype == np.uint8
        assert drive.min() >= 0 and drive.max() <= 255
        reconstructed_phase = lut.drive_to_phase(drive)
        np.testing.assert_allclose(phases, reconstructed_phase, atol=0.05)

    def test_scurve_lut(self):
        lut = make_scurve_lut(gamma=1.5)
        drive = lut.phase_to_drive(np.array([np.pi]))
        assert 0 <= drive[0] <= 255

    def test_flat_field_correction(self):
        beam = np.random.uniform(0.5, 1.0, (32, 32))
        corr = flat_field_correction(beam)
        assert corr.shape == (32, 32)
        assert np.all(corr > 0)

    def test_dye_cell_targets(self):
        optics = OpticsConfig(slm_pixels=64)
        grid = GridConfig(n_xy=64)
        
        # Spot grid
        p_spots = generate_dye_cell_target("spot_grid", optics, grid, spots_xy=(2, 2))
        assert p_spots.shape == (64, 64)
        
        # Bessel beam
        p_bessel = generate_dye_cell_target("bessel_beam", optics, grid)
        assert p_bessel.shape == (64, 64)
        
        # Crosshair
        p_cross = generate_dye_cell_target("crosshair", optics, grid)
        assert p_cross.shape == (64, 64)

    def test_export_slm_images(self, tmp_path):
        phases = np.zeros((3, 32, 32))
        out_dir = str(tmp_path / "slm_out")
        paths = export_slm_images(phases, out_dir, prefix="test_frame")
        assert len(paths) == 3
        for p in paths:
            assert os.path.exists(p)


class TestStage2KineticsAndMetrology:
    def test_kinetics_simulation(self):
        # 50 mW/cm^2 intensity over 15 seconds
        res = simulate_kinetics(intensity_mW_cm2=50.0, total_time_s=15.0, n_steps=100)
        assert len(res.time_s) == 101
        assert res.induction_time_s < 15.0
        assert res.gelled is True

    def test_roughness_calculation_flat_surface(self):
        height_map = np.full((32, 32), 10.0) # completely flat plane
        report = calculate_roughness(height_map, pitch_um=2.0)
        assert np.isclose(report.ra_um, 0.0, atol=1e-5)
        assert report.pass_optical_grade is True

    def test_roughness_calculation_stepped_surface(self):
        # Synthetic step texture
        height_map = np.zeros((32, 32))
        height_map[16:, :] = 0.5 # 500 nm step
        report = calculate_roughness(height_map, pitch_um=2.0)
        assert report.ra_um > 0.1 # Should exceed optical grade
        assert report.pass_optical_grade is False

    def test_topography_extraction(self, small_optics, small_grid, small_target):
        height_map, valid = extract_topography_height_map(small_target, small_optics, small_grid)
        assert height_map.shape == (small_grid.n_xy, small_grid.n_xy)
        assert np.any(valid)

    def test_channel_patency_analysis(self, small_optics):
        cured = np.ones((16, 16, 16), dtype=bool)
        void_target = np.zeros((16, 16, 16), dtype=bool)
        void_target[7:9, 7:9, :] = True # Internal channel
        
        # Test occluded channel
        report_occluded = analyze_channel_patency(cured, void_target, small_optics)
        assert report_occluded.is_patent is False
        assert report_occluded.channel_cured_fraction > 0.9
        
        # Test patent channel
        cured_with_hole = cured.copy()
        cured_with_hole[void_target] = False
        report_patent = analyze_channel_patency(cured_with_hole, void_target, small_optics)
        assert report_patent.is_patent is True
        assert report_patent.channel_cured_fraction < 0.01


class TestStage3CommercialVerticals:
    def test_dental_crown(self):
        vol = dental_crown(shape=(16, 32, 32))
        assert vol.shape == (16, 32, 32)
        assert vol.voxel_count > 0

    def test_microfluidic_chip(self):
        solid, void = microfluidic_chip(shape=(16, 32, 32))
        assert solid.shape == (16, 32, 32)
        assert void.shape == (16, 32, 32)
        assert solid.voxel_count > 0
        assert void.voxel_count > 0
        # Channels must be mutually exclusive with solid block
        assert not np.any(solid.mask & void.mask)

    def test_aspheric_lens(self):
        vol = aspheric_lens(shape=(16, 32, 32))
        assert vol.shape == (16, 32, 32)
        assert vol.voxel_count > 0


class TestHardwarePlayer:
    def test_prepare_hardware_bundle(self, small_job, tmp_path):
        out_dir = str(tmp_path / "bundle")
        manifest = prepare_hardware_bundle(small_job, out_dir=out_dir)
        assert manifest.num_patterns == len(small_job.pattern_weights)
        assert os.path.exists(os.path.join(out_dir, "manifest.json"))
        assert os.path.exists(os.path.join(out_dir, "frame_0000.png"))

        # Test simulated playback
        play_sequence_simulated(manifest, speed_multiplier=100.0, verbose=False)


class TestCLIStage123Commands:
    def test_cli_calibrate(self, tmp_path):
        out_dir = str(tmp_path / "calib")
        ret = main(["calibrate", "--type", "spot_grid", "--slm-pixels", "64", "--nxy", "64", "--out", out_dir])
        assert ret == 0
        assert os.path.exists(os.path.join(out_dir, "calib_spot_grid_000.png"))

    def test_cli_stage2_benchmark(self, tmp_path):
        out_dir = str(tmp_path / "s2_bench")
        ret = main([
            "stage2-benchmark",
            "--nxy", "32", "--nz", "16", "--slm-pixels", "64",
            "--patterns", "4", "--iters", "3", "--out", out_dir,
        ])
        assert ret == 0
        assert os.path.exists(os.path.join(out_dir, "stage2_sphere.hfp"))

    def test_cli_vertical_dental(self, tmp_path):
        out_dir = str(tmp_path / "vert_dental")
        ret = main([
            "vertical", "--type", "dental",
            "--nxy", "32", "--nz", "16", "--slm-pixels", "64",
            "--patterns", "4", "--iters", "3", "--out", out_dir, "--quiet",
        ])
        assert ret == 0
        assert os.path.exists(os.path.join(out_dir, "dental_crown.hfp"))

    def test_cli_export_slm(self, small_job, tmp_path):
        job_file = str(tmp_path / "export_test.hfp")
        from holoforge.io_hfp import save_job
        save_job(small_job, job_file)
        
        out_frames = str(tmp_path / "slm_exported_frames")
        ret = main(["export-slm", job_file, "--out", out_frames])
        assert ret == 0
        assert os.path.exists(os.path.join(out_frames, "manifest.json"))
