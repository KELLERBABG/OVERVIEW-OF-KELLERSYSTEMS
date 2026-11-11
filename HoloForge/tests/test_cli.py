"""Tests for holoforge.cli — subcommand smoke tests."""
import os

from holoforge.cli import main


class TestSelftest:
    def test_selftest_passes(self):
        ret = main(["selftest"])
        assert ret == 0


class TestOptimize:
    def test_optimize_small(self, tmp_path):
        """Small-grid optimization produces output files."""
        out_dir = str(tmp_path / "out")
        ret = main([
            "optimize",
            "--shape", "sphere",
            "--nxy", "32",
            "--nz", "16",
            "--slm-pixels", "64",
            "--patterns", "4",
            "--iters", "5",
            "--out", out_dir,
            "--name", "test_sphere",
            "--quiet",
        ])
        assert ret == 0
        assert os.path.exists(os.path.join(out_dir, "test_sphere.hfp"))
        assert os.path.exists(os.path.join(out_dir, "test_sphere_report.png"))


class TestBenchmark:
    def test_benchmark_small(self, tmp_path):
        """Benchmark on tiny grid produces summary file."""
        out_dir = str(tmp_path / "bench")
        ret = main([
            "benchmark",
            "--nxy", "32",
            "--nz", "16",
            "--slm-pixels", "64",
            "--patterns", "4",
            "--iters", "5",
            "--out", out_dir,
            "--quiet",
        ])
        assert ret == 0
        assert os.path.exists(os.path.join(out_dir, "BENCHMARK_RESULTS.md"))
