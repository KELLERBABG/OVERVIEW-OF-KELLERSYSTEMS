"""Run the MuonScope system.

The same command runs the instrument from hardware, from a recording, or from the
instrument simulator. Nothing about the processing chain changes between them;
only the byte source does.

    # instrument simulator
    py src/inversion/run_system.py --source sim --events 20000 --grid 32

    # replay a recorded capture
    py src/inversion/run_system.py --source file --capture data/captures/run.msc

    # live hardware
    py src/inversion/run_system.py --source serial --port COM3 --baud 3000000

The simulator writes what the hardware would write -- the same bytes through the
same encoder -- so a green run here is evidence about the software, not about a
scene that the reconstruction was also allowed to read.
"""
from __future__ import annotations

import argparse
import json
import os
import sys

sys.path.insert(0, os.path.abspath(os.path.dirname(__file__)))

import numpy as np  # noqa: E402

from muonscope.acquisition import (  # noqa: E402
    FileSource,
    MemorySource,
    SerialSource,
    SimulatedSource,
    SimulatorConfig,
)
from muonscope.simulation.detector import DetectorResponse  # noqa: E402
from muonscope.simulation.geometry import load_detector_geometry  # noqa: E402
from muonscope.simulation.synthetic_scene import (  # noqa: E402
    create_heavy_inclusion_phantom,
    create_rebar_phantom,
    create_void_phantom,
)
from muonscope.system import MuonScopeSystem, SystemConfig, compare_to_truth  # noqa: E402
from muonscope.wire import describe_layout  # noqa: E402

PHANTOMS = {
    "void": create_void_phantom,
    "heavy_inclusion": create_heavy_inclusion_phantom,
    "rebar": create_rebar_phantom,
}


def parse_args(argv=None):
    p = argparse.ArgumentParser(
        description="Run the MuonScope acquisition, tracking and reconstruction system")
    p.add_argument("--source", choices=["sim", "file", "serial"], default="sim")
    p.add_argument("--events", type=int, default=20000,
                   help="simulator: number of incident muons to generate")
    p.add_argument("--grid", type=int, default=32, help="cubic voxel grid resolution")
    p.add_argument("--phantom", choices=sorted(PHANTOMS), default="heavy_inclusion")
    p.add_argument("--iterations", type=int, default=30, help="marginalised MLEM iterations")
    p.add_argument("--beta", type=float, default=0.0, help="Huber prior weight")
    p.add_argument("--seed", type=int, default=12345)
    p.add_argument("--sigma-pos-mm", type=float, default=0.75,
                   help="per-layer barycentre position resolution")
    p.add_argument("--capture", type=str, default=None,
                   help="file source to read, or recording path to write")
    p.add_argument("--port", type=str, default=None, help="serial port for live hardware")
    p.add_argument("--baud", type=int, default=3_000_000)
    p.add_argument("--out", type=str, default="output_reconstruction",
                   help="directory for volumes and the JSON report")
    p.add_argument("--print-layout", action="store_true",
                   help="print the wire format and exit")
    return p.parse_args(argv)


def _build_source(args, geometry):
    if args.source == "sim":
        scene = PHANTOMS[args.phantom](grid_size=(args.grid,) * 3)
        cfg = SimulatorConfig(num_events=args.events, seed=args.seed)
        source = SimulatedSource(
            scene, geometry,
            detector_response=DetectorResponse(sigma_pos_mm=args.sigma_pos_mm),
            config=cfg)
        return source, scene
    if args.source == "file":
        if not args.capture:
            raise SystemExit("--source file requires --capture <path>")
        return FileSource(args.capture), None
    if not args.port:
        raise SystemExit("--source serial requires --port <device>")
    return SerialSource(args.port, baudrate=args.baud), None


def main(argv=None) -> int:
    args = parse_args(argv)

    if args.print_layout:
        print(describe_layout())
        return 0

    geometry = load_detector_geometry()
    source, scene = _build_source(args, geometry)

    if scene is not None:
        bounds_min = tuple(float(v) for v in scene.bounds_min_mm)
        bounds_max = tuple(float(v) for v in scene.bounds_max_mm)
    else:
        # Live or replayed data: the survey volume has to be declared, because it
        # is a property of the deployment, not of the detector.
        bounds_min = (-250.0, -250.0, -200.0)
        bounds_max = (250.0, 250.0, 200.0)

    config = SystemConfig(
        grid_size=args.grid,
        bounds_min_mm=bounds_min,
        bounds_max_mm=bounds_max,
        iterations=args.iterations,
        beta=args.beta,
        sigma_pos_mm=args.sigma_pos_mm,
        capture_path=(args.capture if args.source == "sim" else None),
    )
    system = MuonScopeSystem(config=config, geometry=geometry)
    run = system.run_from_source(source)

    truth_report = None
    if isinstance(source, SimulatedSource):
        truth_report = compare_to_truth(run, source.truth_by_sequence)

    os.makedirs(args.out, exist_ok=True)
    for name, volume in run.volumes.items():
        np.save(os.path.join(args.out, f"system_{name}.npy"), volume)
    report = run.summary()
    report["source"] = args.source
    report["truth_validation"] = truth_report
    with open(os.path.join(args.out, "system_run.json"), "w", encoding="utf-8") as fh:
        json.dump(report, fh, indent=2, default=str)

    _print_report(args, run, truth_report)
    return 0


def _print_report(args, run, truth_report) -> None:
    bar = "=" * 74
    print(bar)
    print("  MUONSCOPE SYSTEM RUN")
    print(bar)
    geo = run.geometry_summary
    print(f"  source                          {args.source}")
    print(f"  detektor channels               {geo['total_channels']}")
    print(f"  station lever arm (X)           {geo['top_lever_arm_x_mm']} mm")
    print(f"  station centre separation       {geo['panel_centre_separation_mm']} mm")
    print()
    print("  -- acquisition --")
    print(f"  bytes in                        {run.reader_stats['bytes_in']}")
    print(f"  packets decoded                 {run.reader_stats['packets_ok']}")
    print(f"  packets rejected                {run.reader_stats['packets_bad']}")
    if run.reader_stats.get("last_error"):
        print(f"  last error                      {run.reader_stats['last_error']}")
    print()
    print("  -- calibration (measured, not assumed) --")
    cal = run.calibration_summary
    print(f"  fine bin width                  {cal['fine_bin_ps']:.2f} ps")
    print(f"  channels calibrated             {cal['channels_calibrated']}")
    print(f"  residual after deskew           {cal['residual_ps']:.1f} ps")
    print()
    print("  -- tracking --")
    for k, v in run.tracking_stats.items():
        print(f"  {k:<32}{v}")
    print()
    print("  -- measured performance --")
    m = run.metrics

    def fmt(key, suffix="", digits=2):
        value = m.get(key)
        return "n/a" if value is None else f"{value:.{digits}f}{suffix}"

    print(f"  tracks                          {m['n_tracks']}")
    eff = m["record_to_track_efficiency"]
    print(f"  record->track efficiency        {'n/a' if eff is None else f'{eff:.3f}'}")
    print(f"  theta RMS (measured)            {fmt('theta_rms_mrad', ' mrad')}")
    print(f"  sigma_theta from geometry       {fmt('sigma_theta_from_geometry_mrad', ' mrad')}")
    print(f"  theta^2 resolution floor        {fmt('theta_sq_offset_rad2', ' rad^2', 4)}")
    frac = m.get("fraction_theta_sq_below_offset")
    print(f"  events below the floor          {'n/a' if frac is None else f'{frac*100:.1f} %'}")
    print(f"  ToF median                      {fmt('tof_median_ns', ' ns', 3)}")
    print(f"  chi2/ndf median                 {fmt('chi2_ndf_median', '', 3)}")
    print()
    print("  -- reconstruction --")
    info = run.recon_info
    if "backprojection" in info:
        bp = info["backprojection"]
        print(f"  Siddon nonzeros                 {info['matrix_nonzeros']}")
        print(f"  <K> over the prior              {bp['mean_highland_factor']:.4e}")
        print(f"  offset subtracted               {bp['theta_sq_offset_applied']:.4e} rad^2")
    if truth_report and truth_report.get("matched"):
        print()
        print("  -- validation against simulator truth (not used by the reconstruction) --")
        print(f"  matched tracks                  {truth_report['matched']}")
        print(f"  theta true RMS                  {truth_report['theta_true_rms_mrad']:.2f} mrad")
        print(f"  theta meas RMS                  {truth_report['theta_meas_rms_mrad']:.2f} mrad")
        print(f"  predicted floor                 "
              f"{truth_report['theta_sq_offset_predicted_rad2']:.4e} rad^2")
        print(f"  measured mean excess            "
              f"{truth_report['mean_theta_sq_excess_rad2']:.4e} rad^2")
        print(f"  ratio measured/predicted        {truth_report['resolution_ratio']:.3f}"
              "   (1.000 = resolution model is right)")
        print(f"  median E                        {truth_report['energy_median_gev']:.2f} GeV")
    print()
    print("  -- timings --")
    for k, v in run.timings_s.items():
        print(f"  {k:<32}{v:.2f} s")
    print(bar)


if __name__ == "__main__":
    raise SystemExit(main())
