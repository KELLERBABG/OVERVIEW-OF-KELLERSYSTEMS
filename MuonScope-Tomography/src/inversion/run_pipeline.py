"""End-to-end MuonScope reconstruction pipeline.

This is the shipped pipeline. It now runs the real instrument software rather
than a bespoke simulation loop:

    SimulatedSource (wire bytes) -> PacketReader -> TdcCalibration -> TrackFitter
        -> prior-marginalised reconstruction -> OBJ / IFC / DICOM exports

What it used to do
------------------
It handed each reconstructor ``ev.energy_initial_gev`` -- the simulator's true
per-event muon momentum. A two-station hodoscope measures directions only, so
that was an oracle, and the reconstructed contrast was partly a property of the
simulator. The code comment acknowledged it ("a real station would use the
deflection estimate, which is circular here") and used it anyway.

Momentum is now marginalised over the cosmic-ray spectrum, which is a known
a-priori input rather than a per-event truth value. The detector model adds strip
charge sharing, position resolution and TDC quantisation, so the reconstruction
sees a measurement rather than the ray tracer's own state.

To compare against the old behaviour, run ``benchmarks/accuracy_harness.py`` with
``--ideal-tracking``, which bypasses the detector model, and note that the harness
also reports an explicitly-labelled oracle row as an unachievable upper bound.
"""
import argparse
import json
import os
import sys
import time

sys.path.insert(0, os.path.abspath(os.path.dirname(__file__)))

import numpy as np  # noqa: E402

from muonscope.acquisition import SimulatedSource, SimulatorConfig  # noqa: E402
from muonscope.export.bim_ifc import export_to_ifc4  # noqa: E402
from muonscope.export.dicom_export import export_to_dicom_series  # noqa: E402
from muonscope.export.mesh_vtk import extract_isosurface_mesh, export_to_obj  # noqa: E402
from muonscope.simulation.geometry import load_detector_geometry  # noqa: E402
from muonscope.simulation.synthetic_scene import (  # noqa: E402
    create_heavy_inclusion_phantom,
    create_rebar_phantom,
    create_void_phantom,
)
from muonscope.system import MuonScopeSystem, SystemConfig  # noqa: E402

PHANTOMS = {
    "void": create_void_phantom,
    "heavy_inclusion": create_heavy_inclusion_phantom,
    "rebar": create_rebar_phantom,
}


def parse_args():
    parser = argparse.ArgumentParser(
        description="MuonScope forward simulation, reconstruction and export pipeline")
    parser.add_argument("--phantom", type=str, default="heavy_inclusion",
                        choices=sorted(PHANTOMS))
    parser.add_argument("--events", type=int, default=3000,
                        help="number of incident muons to generate")
    parser.add_argument("--grid-size", type=int, default=32,
                        help="cubic voxel grid resolution (N x N x N)")
    parser.add_argument("--iterations", type=int, default=20,
                        help="marginalised MLEM iterations")
    parser.add_argument("--beta", type=float, default=0.0,
                        help="Huber prior regularization weight")
    parser.add_argument("--huber-delta", type=float, default=0.01,
                        help="Huber transition parameter")
    parser.add_argument("--seed", type=int, default=12345)
    parser.add_argument("--sigma-pos-mm", type=float, default=0.75,
                        help="per-layer barycentre position resolution")
    parser.add_argument("--output-dir", type=str, default="output_reconstruction")
    parser.add_argument("--skip-exports", action="store_true",
                        help="skip OBJ/IFC/DICOM writing (they dominate runtime at large grids)")
    return parser.parse_args()


def compute_cnr(target_mask: np.ndarray, background_mask: np.ndarray,
                volume: np.ndarray) -> float:
    target_vals = volume[target_mask]
    bg_vals = volume[background_mask]
    if len(target_vals) == 0 or len(bg_vals) == 0:
        return 0.0
    mu_t, std_t = float(np.mean(target_vals)), float(np.std(target_vals))
    mu_b, std_b = float(np.mean(bg_vals)), float(np.std(bg_vals))
    denom = float(np.sqrt(std_t ** 2 + std_b ** 2))
    return abs(mu_t - mu_b) / denom if denom > 1e-9 else 0.0


def run():
    args = parse_args()
    os.makedirs(args.output_dir, exist_ok=True)
    grid = (args.grid_size,) * 3

    scene = PHANTOMS[args.phantom](grid_size=grid)
    ground_truth = scene.get_scattering_density()

    geometry = load_detector_geometry()
    source = SimulatedSource(
        scene, geometry,
        config=SimulatorConfig(num_events=args.events, seed=args.seed,
                               calibration_packets=400),
    )

    system = MuonScopeSystem(
        config=SystemConfig(
            grid_size=args.grid_size,
            bounds_min_mm=tuple(float(v) for v in scene.bounds_min_mm),
            bounds_max_mm=tuple(float(v) for v in scene.bounds_max_mm),
            iterations=args.iterations,
            beta=args.beta,
            huber_delta=args.huber_delta,
            sigma_pos_mm=args.sigma_pos_mm,
        ),
        geometry=geometry,
    )
    t0 = time.time()
    result = system.run_from_source(source)
    total_s = time.time() - t0

    volumes = result.volumes
    marg = volumes["marginalized_mlem"]
    nominal = volumes["nominal_momentum"]

    bands = {
        "high-Z":   ground_truth > 1.0,
        "metal":    (ground_truth > 0.3) & (ground_truth <= 1.0),
        "light":    ground_truth < 0.05,
        "concrete": (ground_truth > 0.05) & (ground_truth <= 0.15),
    }
    bg_mask = bands["concrete"]

    band_report = {}
    for label, mask in bands.items():
        if label == "concrete" or not mask.any():
            continue
        band_report[label] = {
            "voxels": int(mask.sum()),
            "truth_lambda": float(np.mean(ground_truth[mask])),
            "marginalised_lambda": float(np.mean(marg[mask])),
            "nominal_lambda": float(np.mean(nominal[mask])),
            "cnr_marginalised": compute_cnr(mask, bg_mask, marg),
            "cnr_nominal": compute_cnr(mask, bg_mask, nominal),
        }

    np.save(os.path.join(args.output_dir, "ground_truth.npy"), ground_truth)
    np.save(os.path.join(args.output_dir, "marginalized_reconstruction.npy"), marg)
    np.save(os.path.join(args.output_dir, "nominal_reconstruction.npy"), nominal)

    export_stats = {}
    if not args.skip_exports:
        export_stats["ifc"] = export_to_ifc4(
            marg, scene.bounds_min_mm, scene.bounds_max_mm,
            os.path.join(args.output_dir, "structural_model.ifc"))

        # Isovalue is a fraction of the reconstructed dynamic range rather than a
        # constant: the one-shot estimator is biased high by roughly L/w (total
        # path over voxel path), so a fixed 0.25 would mesh noise in one run and
        # nothing in another.
        hi = float(np.percentile(marg, 99.0))
        iso = 0.5 * hi if hi > 0 else 0.0
        verts, faces = extract_isosurface_mesh(
            marg, isovalue=iso,
            spacing=tuple((np.asarray(scene.bounds_max_mm) - np.asarray(scene.bounds_min_mm))
                          / np.array(grid)),
            origin=scene.bounds_min_mm)
        export_to_obj(verts, faces, os.path.join(args.output_dir, "isosurface.obj"))
        export_stats["mesh"] = {"isovalue": iso, "vertices": int(len(verts)),
                                "faces": int(len(faces))}

        # DICOM stores row/column spacing and slice spacing separately, and the
        # survey is in mm, so pass the physical spacing explicitly.
        spacing_mm = tuple((np.asarray(scene.bounds_max_mm) - np.asarray(scene.bounds_min_mm))
                           / np.array(grid))
        export_stats["dicom_slices"] = export_to_dicom_series(
            marg, os.path.join(args.output_dir, "dicom_slices"),
            pixel_spacing_mm=spacing_mm, origin_mm=scene.bounds_min_mm)

    print("=" * 74)
    print("  MUONSCOPE RECONSTRUCTION PIPELINE SUMMARY")
    print("=" * 74)
    print(f"  Target Phantom:           {args.phantom}")
    print(f"  Simulated Muon Events:    {args.events}")
    print(f"  Detected Tracks:          {result.n_tracks}")
    print(f"  Voxel Grid Resolution:    {grid[0]}x{grid[1]}x{grid[2]} "
          f"({grid[0]*grid[1]*grid[2]} voxels)")
    print(f"  Momentum Treatment:       prior-marginalised (no per-event truth)")
    print(f"  Detector Timing Bin:      {result.calibration_summary['fine_bin_ps']:.2f} ps "
          f"(calibrated from {result.calibration_summary['records_used']} records)")
    print(f"  Angular Resolution:       "
          f"{result.metrics['sigma_theta_from_geometry_mrad']:.2f} mrad (from geometry)")
    print(f"  Total Wall Time:          {total_s:.2f} s")
    print("  " + "-" * 70)
    print(f"  {'band':<12}{'voxels':>8}{'truth L':>11}{'marg L':>11}"
          f"{'nominal L':>11}{'CNR marg':>10}")
    for label, b in band_report.items():
        print(f"  {label:<12}{b['voxels']:>8}{b['truth_lambda']:>11.4f}"
              f"{b['marginalised_lambda']:>11.4f}{b['nominal_lambda']:>11.4f}"
              f"{b['cnr_marginalised']:>10.2f}")
    if export_stats:
        print("  " + "-" * 70)
        for name, stats in export_stats.items():
            print(f"  export {name}: {stats}")

    with open(os.path.join(args.output_dir, "accuracy_bands.json"), "w",
              encoding="utf-8") as fh:
        json.dump({
            "phantom": args.phantom,
            "events": args.events,
            "grid": args.grid_size,
            "beta": args.beta,
            "momentum_treatment": "prior-marginalised",
            "system": result.summary(),
            "bands": band_report,
            "exports": export_stats,
        }, fh, indent=2, default=str)
    print("=" * 74)


if __name__ == "__main__":
    run()
