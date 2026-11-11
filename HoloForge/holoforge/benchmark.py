"""Automated benchmark suite for HoloForge optimizer evaluation.

Runs optimization on all standard test geometries (sphere, torus_knot,
lattice_cube) and collects quality metrics for comparison.  Produces a
markdown summary table and per-geometry PNG reports.

This implements PLAN.md Phase 0, item 3:
  "Reconstruct standard test volumes (torus knot, lattice) in simulation;
   metric = dose uniformity error < 10%, print-time estimate."
"""
from __future__ import annotations

import os
import time
from dataclasses import dataclass, field

from .config import RESINS, GridConfig, OpticsConfig, PrintJob
from .io_hfp import save_job
from .optimize import WavefrontMatchedCGH
from .render import render_report
from .simulate import gel_level_simulation_units, predict_cure, simulate_dose
from .volume import PRIMITIVES


@dataclass
class BenchmarkEntry:
    """Results for one test geometry."""
    name: str
    voxel_count: int
    grid_shape: tuple[int, int, int]
    resin: str
    optimizer_elapsed_s: float
    iterations: int
    exposure_s: float
    uniformity_error: float
    cure_iou: float
    bleed_ratio: float
    peak_to_mean: float
    outside_cure_fraction: float


@dataclass
class BenchmarkResult:
    """Complete benchmark run results."""
    entries: list[BenchmarkEntry] = field(default_factory=list)
    total_elapsed_s: float = 0.0

    def to_markdown(self) -> str:
        """Render results as a markdown table."""
        lines = [
            "# HoloForge Benchmark Results\n",
            f"Total wall time: {self.total_elapsed_s:.1f}s\n",
            "| Geometry | Voxels | Grid | Resin | Time (s) | Iters "
            "| Exposure (s) | Uniformity Err | Cure IoU | Bleed | Peak/Mean |",
            "|----------|--------|------|-------|----------|-------"
            "|--------------|----------------|----------|-------|-----------|",
        ]
        for e in self.entries:
            shape_str = "×".join(str(s) for s in e.grid_shape)
            lines.append(
                f"| {e.name} | {e.voxel_count} | {shape_str} | {e.resin} "
                f"| {e.optimizer_elapsed_s:.1f} | {e.iterations} "
                f"| {e.exposure_s:.2f} | {e.uniformity_error:.4f} "
                f"| {e.cure_iou:.4f} | {e.bleed_ratio:.4f} "
                f"| {e.peak_to_mean:.2f} |"
            )
        lines.append("")

        # Pass/fail summary
        lines.append("## Pass/Fail Summary\n")
        for e in self.entries:
            ue_ok = "✅" if e.uniformity_error < 0.10 else "⚠️"
            iou_ok = "✅" if e.cure_iou > 0.70 else "⚠️"
            lines.append(
                f"- **{e.name}**: uniformity {ue_ok} ({e.uniformity_error:.3f}), "
                f"IoU {iou_ok} ({e.cure_iou:.3f})"
            )
        lines.append("")
        return "\n".join(lines)


# Standard benchmark configurations
BENCHMARK_GEOMETRIES = [
    {"name": "sphere", "shape": "sphere", "kwargs": {"radius": 0.28}},
    {"name": "torus_knot_2_3", "shape": "torus_knot", "kwargs": {"p": 2, "q": 3}},
    {"name": "lattice_cube_3", "shape": "lattice_cube", "kwargs": {"cells": 3}},
]


def run_benchmark(
    n_xy: int = 64,
    n_z: int = 32,
    slm_pixels: int = 128,
    slice_step: float = 2.0,
    resin_name: str = "hf-standard-405",
    n_patterns: int = 8,
    iterations: int = 30,
    beta: float = 0.65,
    seed: int = 7,
    out_dir: str = "output/benchmark",
    verbose: bool = True,
) -> BenchmarkResult:
    """Run optimization on all standard test geometries and collect metrics.

    Parameters
    ----------
    n_xy, n_z : int
        Grid dimensions. Use smaller values (64/32) for quick runs.
    slm_pixels : int
        SLM pixel count.
    iterations : int
        GS iterations per geometry.
    out_dir : str
        Output directory for reports and job files.
    verbose : bool
        Print progress.

    Returns
    -------
    BenchmarkResult
        Contains per-geometry metrics and a markdown summary.
    """
    t_total = time.time()
    os.makedirs(out_dir, exist_ok=True)

    optics = OpticsConfig(slm_pixels=slm_pixels)
    grid = GridConfig(n_xy=n_xy, n_z=n_z, slice_step_um=slice_step)
    resin = RESINS[resin_name]

    result = BenchmarkResult()

    for geom in BENCHMARK_GEOMETRIES:
        name = geom["name"]
        shape_fn = PRIMITIVES[geom["shape"]]
        kwargs = dict(geom["kwargs"])
        kwargs["shape"] = (n_z, n_xy, n_xy)

        if verbose:
            print(f"\n{'='*60}")
            print(f"Benchmarking: {name}")
            print(f"{'='*60}")

        vol = shape_fn(**kwargs)
        target = vol.mask

        if verbose:
            print(f"  voxels: {int(target.sum())}, "
                  f"grid: {target.shape}, resin: {resin.name}")

        cgh = WavefrontMatchedCGH(
            optics, grid, resin, target,
            n_patterns=n_patterns, seed=seed,
        )
        opt_result = cgh.optimize(
            iterations=iterations, beta=beta,
            verbose=verbose, log_every=10,
        )

        dose = simulate_dose(
            opt_result.phases_rad, opt_result.pattern_weights,
            optics, grid, resin,
        )
        exposure_s = opt_result.exposure_s
        gel = gel_level_simulation_units(resin, optics, grid, exposure_s)
        cured = predict_cure(dose, gel)

        metrics = opt_result.final_metrics

        entry = BenchmarkEntry(
            name=name,
            voxel_count=int(target.sum()),
            grid_shape=target.shape,
            resin=resin.name,
            optimizer_elapsed_s=round(opt_result.elapsed_s, 2),
            iterations=opt_result.iterations,
            exposure_s=round(exposure_s, 2),
            uniformity_error=metrics["uniformity_error"],
            cure_iou=metrics["cure_iou"],
            bleed_ratio=metrics["bleed_ratio"],
            peak_to_mean=metrics["peak_to_mean"],
            outside_cure_fraction=metrics.get("outside_cure_fraction", 0.0),
        )
        result.entries.append(entry)

        # Save job + report
        job = PrintJob(
            optics=optics, grid=grid, resin=resin, target_mask=target,
            phases_rad=opt_result.phases_rad,
            pattern_weights=opt_result.pattern_weights,
            exposure_s=exposure_s, metrics=dict(metrics),
            meta={"name": name, "iterations": opt_result.iterations,
                  "optimizer_elapsed_s": round(opt_result.elapsed_s, 2),
                  "benchmark": True},
        )
        base = os.path.join(out_dir, name)
        save_job(job, base + ".hfp")
        render_report(job, dose, cured, base + "_report.png")

        if verbose:
            print(f"  → uniformity_error: {metrics['uniformity_error']:.4f}")
            print(f"  → cure_iou: {metrics['cure_iou']:.4f}")
            print(f"  → exposure: {exposure_s:.2f}s")

    result.total_elapsed_s = time.time() - t_total

    # Write markdown summary
    summary_path = os.path.join(out_dir, "BENCHMARK_RESULTS.md")
    with open(summary_path, "w", encoding="utf-8") as f:
        f.write(result.to_markdown())

    if verbose:
        print(f"\n{'='*60}")
        print(f"Benchmark complete: {result.total_elapsed_s:.1f}s total")
        print(f"Summary written to {summary_path}")
        print(f"{'='*60}")

    return result
