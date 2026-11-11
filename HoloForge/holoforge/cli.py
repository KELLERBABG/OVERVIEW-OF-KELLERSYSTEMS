"""HoloForge command line interface."""
from __future__ import annotations

import argparse
import os
import sys

import numpy as np

from .config import RESINS, GridConfig, OpticsConfig, PrintJob
from .io_hfp import load_job, save_job
from .optimize import WavefrontMatchedCGH
from .render import render_report
from .simulate import gel_level_simulation_units, predict_cure, simulate_dose
from .volume import PRIMITIVES


def _build_target(args) -> np.ndarray:
    if args.stl:
        from .volume import voxelize_stl
        shape = (args.nz, args.nxy, args.nxy)
        vol = voxelize_stl(args.stl, shape)
    else:
        shape = (args.nz, args.nxy, args.nxy)
        kwargs = {}
        if args.shape == "torus_knot":
            kwargs = {"p": args.knot_p, "q": args.knot_q}
        if args.shape == "sphere":
            kwargs = {"radius": 0.28}
        vol = PRIMITIVES[args.shape](shape=shape, **kwargs)
    return vol.mask


def cmd_optimize(args) -> int:
    optics = OpticsConfig(slm_pixels=args.slm_pixels)
    grid = GridConfig(n_xy=args.nxy, n_z=args.nz,
                      slice_step_um=args.slice_step)
    resin = RESINS[args.resin]
    target = _build_target(args)

    mode = getattr(args, "mode", "tomographic")
    print(f"HoloForge optimizer: mode={mode} target={args.stl or args.shape} "
          f"voxels={int(target.sum())} grid={target.shape} resin={resin.name}")

    if mode == "tomographic":
        from .optimize import TomographicHolographicOptimizer
        vat_radius = getattr(args, "vat_radius_mm", None)
        dev = getattr(args, "device", "auto")
        cgh = TomographicHolographicOptimizer(optics, grid, resin, target,
                                              n_patterns=args.patterns, seed=args.seed,
                                              vat_radius_mm=vat_radius, device=dev)
        result = cgh.optimize(iterations=args.iters, beta=args.beta,
                              verbose=not args.quiet)
        rotation_angles = cgh.angles_deg
    elif args.compensate:
        from .dose_compensate import DoseCompensator
        print(f"  dose compensation enabled ({args.comp_iters} outer iters)")
        comp = DoseCompensator(optics, grid, resin, target,
                               n_patterns=args.patterns, seed=args.seed)
        comp_result = comp.run(
            gs_iterations=args.iters, comp_iterations=args.comp_iters,
            beta=args.beta, verbose=not args.quiet,
        )
        result = comp_result.final_result
        rotation_angles = None
    else:
        cgh = WavefrontMatchedCGH(optics, grid, resin, target,
                                  n_patterns=args.patterns, seed=args.seed)
        result = cgh.optimize(iterations=args.iters, beta=args.beta,
                              verbose=not args.quiet)
        rotation_angles = None

    dose = simulate_dose(result.phases_rad, result.pattern_weights,
                         optics, grid, resin, angles_deg=rotation_angles)
    exposure_s = result.exposure_s
    gel = gel_level_simulation_units(resin, optics, grid, exposure_s)
    cured = predict_cure(dose, gel)

    job = PrintJob(
        optics=optics, grid=grid, resin=resin, target_mask=target,
        phases_rad=result.phases_rad, pattern_weights=result.pattern_weights,
        exposure_s=exposure_s, rotation_angles_deg=rotation_angles,
        metrics=dict(result.final_metrics),
        meta={"name": os.path.basename(args.stl or args.shape),
              "mode": mode,
              "iterations": result.iterations,
              "optimizer_elapsed_s": round(result.elapsed_s, 2),
              "compensated": args.compensate},
    )
    os.makedirs(args.out, exist_ok=True)
    base = os.path.join(args.out, args.name)
    save_job(job, base + ".hfp")
    render_report(job, dose, cured, base + "_report.png")
    print(job.summary())
    print(f"optimizer wall time: {result.elapsed_s:.1f}s "
          f"({result.iterations} iters)")
    print(f"wrote {base}.hfp and {base}_report.png")
    return 0


def cmd_render(args) -> int:
    job = load_job(args.job)
    dose = simulate_dose(job.phases_rad, job.pattern_weights,
                         job.optics, job.grid, job.resin)
    gel = gel_level_simulation_units(job.resin, job.optics, job.grid,
                                     job.exposure_s)
    cured = predict_cure(dose, gel)
    out = render_report(job, dose, cured, args.out)
    print(f"rendered {out}")
    return 0


def cmd_selftest(args) -> int:
    """Fast sanity checks: propagation energy conservation + spot focusing."""
    optics = OpticsConfig(slm_pixels=256)            # pitch_vat = 2 um
    grid = GridConfig(n_xy=128, n_z=24, slice_step_um=32.0)
    resin = RESINS["hf-standard-405"]
    prop = __import__("holoforge.optics", fromlist=["VatPropagator"]).VatPropagator(optics, grid)

    # 1) energy conservation without absorption/scatter
    clear = type(resin)("clear", gel_dose_mJ_cm2=1.0, absorption_per_mm=0.0,
                        scatter_length_um=0.0, index=1.46)
    ph = np.zeros((2, grid.n_xy, grid.n_xy))
    dose, _ = prop.forward(ph, clear, keep_fields=False)
    e_in = float((prop.aperture() ** 2).sum())
    e_out = float(dose.sum(axis=(1, 2, 3)).mean())
    ok_energy = abs(e_out - e_in * grid.n_z) / (e_in * grid.n_z) < 0.02

    # 2) converging quadratic phase on a reduced aperture must focus
    #    mid-volume (Nyquist fringe limit: f >= 2 p a / lambda).
    n = grid.n_xy
    x = (np.arange(n) + 0.5) / n - 0.5
    X, Y = np.meshgrid(x, x)
    R = np.hypot(X, Y)
    a_px = 13.0                                       # aperture radius (pixels)
    mask = (R <= a_px / n).astype(float)
    f_slices = grid.n_z // 2
    f_mm = grid.slice_step_um * 1e-3 * f_slices       # focus mid-volume
    lam_mm = optics.wavelength_vat_um * 1e-3
    p_mm = optics.pitch_um * 1e-3
    lens = -np.pi * ((X * n * p_mm) ** 2 + (Y * n * p_mm) ** 2) / (lam_mm * f_mm) * mask
    dose_f, _ = prop.forward(lens[None], clear, keep_fields=False)
    zpk = int(np.argmax(dose_f[0].max(axis=(1, 2))))
    mid = dose_f[0, f_slices]
    peak_ratio = float(mid.max() / max(mid.mean(), 1e-12))
    ok_focus = peak_ratio > 8.0 and abs(zpk - f_slices) <= 2

    print(f"selftest.energy_conservation : {'PASS' if ok_energy else 'FAIL'} "
          f"(per-slice sum {e_out / max(grid.n_z, 1):.2f} vs pupil {e_in:.2f})")
    print(f"selftest.focus_peakedness    : {'PASS' if ok_focus else 'FAIL'} "
          f"(peak/mean {peak_ratio:.1f}, peak slice {zpk} vs {f_slices})")
    return 0 if (ok_energy and ok_focus) else 1


def cmd_verify(args) -> int:
    """Evidence harness: regenerate published (simulated) claims from a pinned seed.

    Pins seed 7, a sphere target, 64x32 grid, hf-pegda-405 resin, 16
    patterns and 20 iterations to regenerate the figures cited in README.md
    and docs/; changing these values changes published claims, so update the
    docs in the same commit. Prints machine-checkable `name=value ... PASS/FAIL`
    lines and exits non-zero if any claim has drifted from the code.
    """
    from .metrology import calculate_roughness, extract_topography_height_map
    from .optimize import TomographicHolographicOptimizer

    optics = OpticsConfig(slm_pixels=128)
    grid = GridConfig(n_xy=64, n_z=32, slice_step_um=2.0)
    resin = RESINS["hf-pegda-405"]
    vol = PRIMITIVES["sphere"](shape=(grid.n_z, grid.n_xy, grid.n_xy), radius=0.28)

    print("HoloForge evidence harness (Digital Twin, all values simulated)")
    print(
        "config: seed=7 patterns=16 iters=20 shape=sphere "
        f"resin={resin.name} grid={grid.n_xy}x{grid.n_z} slm_pixels={optics.slm_pixels}"
    )

    cgh = TomographicHolographicOptimizer(
        optics, grid, resin, vol.mask, n_patterns=16, seed=7
    )
    res = cgh.optimize(iterations=20, verbose=False)
    dose = simulate_dose(
        res.phases_rad, res.pattern_weights, optics, grid, resin,
        angles_deg=cgh.angles_deg,
    )
    gel = gel_level_simulation_units(resin, optics, grid, res.exposure_s)
    cured = predict_cure(dose, gel)
    height_map, valid = extract_topography_height_map(
        cured, optics, grid, axis=0, direction=1
    )
    roughness = calculate_roughness(height_map, valid, pitch_um=optics.pitch_um)

    # Published claims; docs cite ~92.3% IoU, <0.02% overcure, ~3.4 um Ra.
    cure_iou = res.final_metrics["cure_iou"]
    overcure = res.final_metrics["outside_cure_fraction"]
    ra_um = roughness.ra_um
    checks = [
        ("cure_iou", cure_iou, cure_iou >= 0.920, ">=0.92"),
        ("outside_cure_fraction", overcure, overcure <= 0.0002, "<=0.0002"),
        ("surface_roughness_ra_um", ra_um, ra_um <= 5.0, "<=5.0"),
    ]
    for name, value, passed, claim in checks:
        print(f"{name}={value:.6f} claim:{claim} {'PASS' if passed else 'FAIL'}")
    ok = all(passed for _, _, passed, _ in checks)
    print("VERIFY PASS" if ok else "VERIFY FAIL")
    return 0 if ok else 1


def cmd_benchmark(args) -> int:
    """Run the full benchmark suite on all standard geometries."""
    from .benchmark import run_benchmark
    result = run_benchmark(
        n_xy=args.nxy, n_z=args.nz, slm_pixels=args.slm_pixels,
        slice_step=args.slice_step, resin_name=args.resin,
        n_patterns=args.patterns, iterations=args.iters,
        beta=args.beta, seed=args.seed, out_dir=args.out,
        verbose=not args.quiet,
    )
    print(result.to_markdown())
    return 0


def cmd_calibrate(args) -> int:
    """Stage 1: Generate SLM alignment and dye-cell test holograms."""
    from .calibration import export_slm_images, generate_dye_cell_target
    optics = OpticsConfig(slm_pixels=args.slm_pixels)
    grid = GridConfig(n_xy=args.nxy)
    
    print(f"[Stage 1 Calibration] Generating '{args.type}' target (SLM pixels: {args.slm_pixels})...")
    phase = generate_dye_cell_target(
        target_type=args.type, optics=optics, grid=grid,
        spots_xy=(args.spots_x, args.spots_y),
        focal_depth_mm=args.focal_depth,
    )
    
    os.makedirs(args.out, exist_ok=True)
    saved = export_slm_images(phase, args.out, prefix=f"calib_{args.type}", image_format=args.format)
    print(f"  [OK] Saved {len(saved)} SLM bitmap(s) to '{args.out}'")
    for s in saved:
        print(f"    - {s}")
    return 0


def cmd_stage2_benchmark(args) -> int:
    """Stage 2: Run 15-second solidification benchmark on 10mm sphere and lattice with profilometry."""
    from .kinetics import simulate_kinetics
    from .metrology import calculate_roughness, extract_topography_height_map
    
    print(f"\n{'='*70}")
    print("Stage 2: Photochemical Solidification & Optical Metrology Benchmark")
    print(f"Resin: {args.resin} (Target cure window: <= 15.0s, Target Ra: < 100 nm)")
    print(f"{'='*70}\n")
    
    optics = OpticsConfig(slm_pixels=args.slm_pixels)
    grid = GridConfig(n_xy=args.nxy, n_z=args.nz, slice_step_um=args.slice_step)
    resin = RESINS[args.resin]
    
    # 1. Chemical Kinetics Simulation
    print("[1/3] Simulating dissolved O2 depletion and radical polymerization kinetics...")
    # Reference full-pupil intensity ~ 50 mW/cm^2
    kin_res = simulate_kinetics(intensity_mW_cm2=45.0, total_time_s=15.0)
    print(f"  - Induction time (O2 depletion) : {kin_res.induction_time_s:.2f} s")
    print(f"  - Gel threshold reached at      : {kin_res.gel_time_s:.2f} s")
    print(f"  - Status in 15s window          : {'PASS (Crosslinked)' if kin_res.gelled else 'FAIL'}")
    
    # 2. Optimize 10mm Sphere Benchmark
    print("\n[2/3] Optimizing 10mm Sphere target (Holographic Tomographic VAM)...")
    vol_sphere = PRIMITIVES["sphere"](shape=(grid.n_z, grid.n_xy, grid.n_xy), radius=0.28)
    from .optimize import TomographicHolographicOptimizer
    cgh = TomographicHolographicOptimizer(optics, grid, resin, vol_sphere.mask, n_patterns=args.patterns, seed=7)
    opt_res = cgh.optimize(iterations=args.iters, verbose=False)
    dose = simulate_dose(opt_res.phases_rad, opt_res.pattern_weights, optics, grid, resin, angles_deg=cgh.angles_deg)
    gel = gel_level_simulation_units(resin, optics, grid, opt_res.exposure_s)
    cured = predict_cure(dose, gel)
    
    # 3. Profilometry Roughness Extraction
    print("\n[3/3] Running Virtual Optical Profilometer on cured surface...")
    height_map, valid = extract_topography_height_map(cured, optics, grid, axis=0, direction=1)
    roughness = calculate_roughness(height_map, valid, pitch_um=optics.pitch_um)
    print(roughness.summary())
    
    # Save Report & Job
    os.makedirs(args.out, exist_ok=True)
    job = PrintJob(
        optics=optics, grid=grid, resin=resin, target_mask=vol_sphere.mask,
        phases_rad=opt_res.phases_rad, pattern_weights=opt_res.pattern_weights,
        exposure_s=opt_res.exposure_s, rotation_angles_deg=cgh.angles_deg,
        metrics=dict(opt_res.final_metrics),
        meta={"name": "stage2_sphere_benchmark", "mode": "tomographic", "profilometry_ra_um": roughness.ra_um},
    )
    base = os.path.join(args.out, "stage2_sphere")
    save_job(job, base + ".hfp")
    render_report(job, dose, cured, base + "_report.png")
    print(f"\n[OK] Stage 2 Benchmark Complete. Wrote {base}.hfp and {base}_report.png\n")
    return 0


def cmd_vertical(args) -> int:
    """Stage 3: Optimize specialized high-margin B2B commercial models."""
    from .optimize import TomographicHolographicOptimizer
    from .verticals import aspheric_lens, dental_crown, microfluidic_chip
    
    optics = OpticsConfig(slm_pixels=args.slm_pixels)
    grid = GridConfig(n_xy=args.nxy, n_z=args.nz, slice_step_um=args.slice_step)
    resin = RESINS[args.resin]
    
    shape = (grid.n_z, grid.n_xy, grid.n_xy)
    print(f"\n[Stage 3 Commercial Vertical] Generating '{args.type}' (Resin: {resin.name})...")
    
    if args.type == "dental":
        vol = dental_crown(shape=shape)
        name = "dental_crown"
    elif args.type == "microfluidics":
        vol, void = microfluidic_chip(shape=shape)
        name = "microfluidic_chip"
    elif args.type == "lens":
        vol = aspheric_lens(shape=shape)
        name = "aspheric_lens"
    else:
        raise ValueError(f"Unknown vertical type: {args.type}")
        
    target = vol.mask
    print(f"  Target solid voxels: {int(target.sum())} on grid {shape}")
    
    cgh = TomographicHolographicOptimizer(optics, grid, resin, target, n_patterns=args.patterns, seed=args.seed)
    opt_res = cgh.optimize(iterations=args.iters, verbose=not args.quiet)
    
    dose = simulate_dose(opt_res.phases_rad, opt_res.pattern_weights, optics, grid, resin, angles_deg=cgh.angles_deg)
    gel = gel_level_simulation_units(resin, optics, grid, opt_res.exposure_s)
    cured = predict_cure(dose, gel)
    
    os.makedirs(args.out, exist_ok=True)
    base = os.path.join(args.out, name)
    job = PrintJob(
        optics=optics, grid=grid, resin=resin, target_mask=target,
        phases_rad=opt_res.phases_rad, pattern_weights=opt_res.pattern_weights,
        exposure_s=opt_res.exposure_s, rotation_angles_deg=cgh.angles_deg,
        metrics=dict(opt_res.final_metrics),
        meta={"name": name, "vertical": args.type, "mode": "tomographic", "iterations": opt_res.iterations},
    )
    save_job(job, base + ".hfp")
    render_report(job, dose, cured, base + "_report.png")
    print(job.summary())
    print(f"[OK] Wrote {base}.hfp and {base}_report.png")
    return 0


def cmd_export_slm(args) -> int:
    """Export .hfp job into 8-bit SLM driver bitmaps + timing manifest."""
    from .player import prepare_hardware_bundle
    job = load_job(args.job)
    print(f"[Hardware Exporter] Processing '{args.job}'...")
    manifest = prepare_hardware_bundle(job, out_dir=args.out, image_format=args.format)
    print(f"[OK] Generated hardware bundle in '{args.out}' ({manifest.num_patterns} frames, total {manifest.total_exposure_s:.2f}s)")
    return 0


def cmd_print(args) -> int:
    """Execute physical hardware print run from a compiled .hfp job file."""
    from .hardware import (
        HardwarePrintRunner,
        LaserController,
        RotaryStageController,
        SLMDisplayManager,
    )
    job = load_job(args.job)
    print(f"\n{'='*70}")
    print(f"HoloForge Hardware Print Orchestrator: '{job.meta.get('name', 'unnamed')}'")
    print(f"Patterns: {len(job.pattern_weights)} | Total Exposure: {job.exposure_s:.4f} s | Resin: {job.resin.name}")
    print(f"Mode: {'MOCK/DRY-RUN' if args.mock else 'LIVE PHYSICAL HARDWARE'}")
    print(f"{'='*70}\n")

    stage = RotaryStageController(port=args.stage_port, mock=args.mock)
    display = SLMDisplayManager(monitor_index=args.display, mock=args.mock)
    laser = LaserController(port=args.laser_port, mock=args.mock)

    runner = HardwarePrintRunner(stage=stage, display=display, laser=laser, mock=args.mock)

    def on_progress(frame, total, angle, dur_s):
        print(f"  [{frame:02d}/{total:02d}] Angle: {angle:6.1f}° | SLM Pattern Active | Laser Pulse: {dur_s*1000.0:6.1f} ms")

    summary = runner.execute_print(job, progress_callback=on_progress, speed_factor=args.speed)
    print(f"\n[OK] Hardware print complete in {summary.elapsed_wall_time_s:.2f} s wall time.")
    return 0


def cmd_ui(args) -> int:
    """Launch the interactive HoloForge Web & Touchscreen Control Center."""
    from .web import start_dashboard_server
    return start_dashboard_server(
        host=args.host,
        port=args.port,
        mock_hardware=args.mock,
        device=args.device,
        open_browser=not args.no_browser,
    )


def main(argv=None) -> int:
    p = argparse.ArgumentParser(prog="holoforge",
                                description="Holographic volumetric printing core")
    sub = p.add_subparsers(dest="cmd", required=True)

    po = sub.add_parser("optimize", help="optimize SLM phases for a target")
    po.add_argument("--mode", default="tomographic", choices=["tomographic", "static"],
                    help="optical projection mode: tomographic (rotary) or static (1-axis)")
    po.add_argument("--stl", help="path to STL mesh")
    po.add_argument("--shape", default="torus_knot", choices=sorted(PRIMITIVES))
    po.add_argument("--knot-p", type=int, default=2)
    po.add_argument("--knot-q", type=int, default=3)
    po.add_argument("--resin", default="hf-standard-405", choices=sorted(RESINS))
    po.add_argument("--nxy", type=int, default=128)
    po.add_argument("--nz", type=int, default=96)
    po.add_argument("--slm-pixels", type=int, default=256)
    po.add_argument("--slice-step", type=float, default=2.0)
    po.add_argument("--patterns", type=int, default=8)
    po.add_argument("--iters", type=int, default=40)
    po.add_argument("--beta", type=float, default=0.65)
    po.add_argument("--seed", type=int, default=7)
    po.add_argument("--out", default="output")
    po.add_argument("--name", default="job")
    po.add_argument("--quiet", action="store_true")
    po.add_argument("--compensate", action="store_true", default=False,
                    help="enable iterative dose compensation loop")
    po.add_argument("--no-compensate", dest="compensate", action="store_false")
    po.add_argument("--comp-iters", type=int, default=5,
                    help="number of dose compensation outer iterations")
    po.add_argument("--vat-radius-mm", type=float, default=None,
                    help="cylindrical vat outer radius (mm) for in-situ aberration pre-compensation")
    po.add_argument("--device", default="auto", choices=["auto", "cpu", "cuda"],
                    help="compute backend device: auto, cpu, or cuda")
    po.set_defaults(func=cmd_optimize)

    pr = sub.add_parser("render", help="re-simulate a saved job and render report")
    pr.add_argument("job")
    pr.add_argument("--out", default="report.png")
    pr.set_defaults(func=cmd_render)

    ps = sub.add_parser("selftest", help="physics sanity checks")
    ps.set_defaults(func=cmd_selftest)

    p_ver = sub.add_parser(
        "verify",
        help="evidence harness: regenerate published (simulated) claims from a pinned seed",
    )
    p_ver.set_defaults(func=cmd_verify)

    pb = sub.add_parser("benchmark",
                        help="run optimizer on all standard test geometries")
    pb.add_argument("--resin", default="hf-standard-405", choices=sorted(RESINS))
    pb.add_argument("--nxy", type=int, default=64)
    pb.add_argument("--nz", type=int, default=32)
    pb.add_argument("--slm-pixels", type=int, default=128)
    pb.add_argument("--slice-step", type=float, default=2.0)
    pb.add_argument("--patterns", type=int, default=8)
    pb.add_argument("--iters", type=int, default=30)
    pb.add_argument("--beta", type=float, default=0.65)
    pb.add_argument("--seed", type=int, default=7)
    pb.add_argument("--out", default="output/benchmark")
    pb.add_argument("--quiet", action="store_true")
    pb.set_defaults(func=cmd_benchmark)

    # Stage 1: Calibration
    p_cal = sub.add_parser("calibrate", help="Stage 1: generate SLM alignment & dye-cell test targets")
    p_cal.add_argument("--type", default="spot_grid", choices=["spot_grid", "bessel_beam", "crosshair"])
    p_cal.add_argument("--slm-pixels", type=int, default=256)
    p_cal.add_argument("--nxy", type=int, default=256)
    p_cal.add_argument("--spots-x", type=int, default=3)
    p_cal.add_argument("--spots-y", type=int, default=3)
    p_cal.add_argument("--focal-depth", type=float, default=0.5)
    p_cal.add_argument("--format", default="png", choices=["png", "bmp"])
    p_cal.add_argument("--out", default="output/calibration")
    p_cal.set_defaults(func=cmd_calibrate)

    # Stage 2: 15-second Solidification & Metrology Benchmark
    p_s2 = sub.add_parser("stage2-benchmark", help="Stage 2: run 15s solidification & optical profilometry benchmark")
    p_s2.add_argument("--resin", default="hf-pegda-405", choices=sorted(RESINS))
    p_s2.add_argument("--nxy", type=int, default=64)
    p_s2.add_argument("--nz", type=int, default=32)
    p_s2.add_argument("--slm-pixels", type=int, default=128)
    p_s2.add_argument("--slice-step", type=float, default=2.0)
    p_s2.add_argument("--patterns", type=int, default=6)
    p_s2.add_argument("--iters", type=int, default=20)
    p_s2.add_argument("--out", default="output/stage2_benchmark")
    p_s2.set_defaults(func=cmd_stage2_benchmark)

    # Stage 3: Commercial Verticals
    p_vert = sub.add_parser("vertical", help="Stage 3: optimize high-margin B2B commercial models")
    p_vert.add_argument("--type", required=True, choices=["dental", "microfluidics", "lens"])
    p_vert.add_argument("--resin", default="hf-clear-405", choices=sorted(RESINS))
    p_vert.add_argument("--nxy", type=int, default=64)
    p_vert.add_argument("--nz", type=int, default=32)
    p_vert.add_argument("--slm-pixels", type=int, default=128)
    p_vert.add_argument("--slice-step", type=float, default=2.0)
    p_vert.add_argument("--patterns", type=int, default=6)
    p_vert.add_argument("--iters", type=int, default=20)
    p_vert.add_argument("--seed", type=int, default=7)
    p_vert.add_argument("--out", default="output/verticals")
    p_vert.add_argument("--quiet", action="store_true")
    p_vert.set_defaults(func=cmd_vertical)

    # Hardware Bundle Exporter
    p_exp = sub.add_parser("export-slm", help="export .hfp job into 8-bit SLM driver bitmaps + timing manifest")
    p_exp.add_argument("job", help="path to .hfp job file")
    p_exp.add_argument("--out", default="output/slm_frames")
    p_exp.add_argument("--format", default="png", choices=["png", "bmp"])
    p_exp.set_defaults(func=cmd_export_slm)

    # Physical Print Runner
    p_pr = sub.add_parser("print", help="execute physical hardware print run from a .hfp job")
    p_pr.add_argument("job", help="path to .hfp job file")
    p_pr.add_argument("--stage-port", default=None, help="serial COM port for rotary stage controller")
    p_pr.add_argument("--laser-port", default=None, help="serial COM port for laser shutter controller")
    p_pr.add_argument("--display", type=int, default=1, help="target monitor index for phase SLM")
    p_pr.add_argument("--speed", type=float, default=1.0, help="playback speed multiplier")
    p_pr.add_argument("--mock", action="store_true", default=False, help="run in dry-run/mock hardware mode")
    p_pr.set_defaults(func=cmd_print)

    # Web & Touchscreen Control Dashboard
    p_ui = sub.add_parser("ui", help="launch interactive Web & Touchscreen Control Center")
    p_ui.add_argument("--host", default="127.0.0.1", help="HTTP bind address")
    p_ui.add_argument("--port", type=int, default=8080, help="HTTP port")
    p_ui.add_argument("--device", default="auto", choices=["auto", "cpu", "cuda"], help="compute backend device")
    p_ui.add_argument("--mock", action="store_true", default=True, help="use mock hardware simulator")
    p_ui.add_argument("--live-hardware", dest="mock", action="store_false", help="connect to live serial hardware")
    p_ui.add_argument("--no-browser", action="store_true", default=False, help="do not auto-open browser tab")
    p_ui.set_defaults(func=cmd_ui)

    args = p.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
