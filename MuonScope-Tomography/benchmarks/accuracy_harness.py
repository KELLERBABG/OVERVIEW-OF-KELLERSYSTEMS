"""MuonScope accuracy harness.

Measures a reconstruction against the phantom it was generated from, using the
phantom's own geometry as the mask. Reports only numbers it computes.

    py benchmarks/accuracy_harness.py [--events 5000] [--grid 64]

What changed, and why
---------------------
This harness used to feed every reconstructor ``ev.energy_initial_gev`` -- the
simulator's true per-event muon momentum. That is an oracle: a two-station
hodoscope measures directions and cannot measure momentum, so the reported
accuracy was partly a property of the simulator rather than of the instrument.
The code comment admitted as much while still using it.

The default path now measures the track through the detector model (strip
splitting, position resolution, TDC quantisation) and marginalises over the
cosmic-ray momentum spectrum, which is a known a-priori input rather than a
per-event truth value. Three variants are reported side by side so the cost of
each assumption is visible:

    oracle        truth momentum per event   -- NOT ACHIEVABLE, kept as an upper bound
    marginalised  prior over the spectrum    -- what the instrument can actually do
    nominal       one momentum for all events -- the naive treatment

Why geometry masks rather than thresholding the ground truth: run_pipeline.py
selects regions with `ground_truth > 0.3`, which lumps lead and iron into one
"high-Z" class and cannot produce a per-inclusion figure. Every inclusion in the
phantom is a cylinder at a known position with a known radius, so the mask can be
built exactly.

The truth field is 1 / radiation_length_cm, i.e. scattering density lambda in
cm^-1. Concrete is 1/11.55 = 0.0866, lead is 1/0.56 = 1.786.
"""
import argparse
import json
import os
import sys
import time

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, ".."))
sys.path.insert(0, os.path.join(ROOT, "src", "inversion"))
sys.path.insert(0, HERE)

from ground_truth_rig import build_iso_astm_calibration_phantom          # noqa: E402
from muonscope.physics.gaisser import sample_cosmic_muons                # noqa: E402
from muonscope.physics.momentum_prior import gaisser_momentum_prior      # noqa: E402
from muonscope.simulation.detector import DetectorResponse, HodoscopeSimulator  # noqa: E402
from muonscope.simulation.geometry import load_detector_geometry         # noqa: E402
from muonscope.simulation.ray_tracer import MonteCarloRayTracer          # noqa: E402
from muonscope.reconstructors.poca import PoCAReconstructor              # noqa: E402
from muonscope.reconstructors.siddon import build_sparse_system_matrix    # noqa: E402
from muonscope.reconstructors.normalized import momentum_factor           # noqa: E402
from muonscope.reconstructors.marginalized import (                       # noqa: E402
    marginalized_backprojection,
    nominal_momentum_baseline,
)
from muonscope.reconstructors.map_em import MAPEMReconstructor            # noqa: E402

# geometry straight from the phantom builder, in mm
INCLUSIONS = [
    ("air_void", -125.0, -125.0, 20.0),
    ("water",     125.0, -125.0, 25.0),
    ("iron",     -125.0,  125.0, 15.0),
    ("lead",      125.0,  125.0, 12.5),
]
BOUNDS = (-250.0, 250.0)


def build_masks(grid):
    xs = np.linspace(BOUNDS[0], BOUNDS[1], grid)
    xg, yg, _ = np.meshgrid(xs, xs, xs, indexing="ij")
    masks = {}
    for name, cx, cy, r in INCLUSIONS:
        masks[name] = np.sqrt((xg - cx) ** 2 + (yg - cy) ** 2) <= r
    any_incl = np.zeros_like(masks["lead"], dtype=bool)
    for m in masks.values():
        any_incl |= m
    masks["concrete_background"] = ~any_incl
    return masks


def cnr(volume, target_mask, background_mask):
    """The formula both report documents quote."""
    t = volume[target_mask]
    b = volume[background_mask]
    if t.size == 0 or b.size == 0:
        return None
    mu_t, mu_b = float(np.mean(t)), float(np.mean(b))
    sd_t, sd_b = float(np.std(t)), float(np.std(b))
    denom = float(np.sqrt(sd_t ** 2 + sd_b ** 2))
    return abs(mu_t - mu_b) / denom if denom > 1e-12 else None


def edge_width_mm(volume, truth, grid, axis_centre_mm, other_centre_mm, half_width_mm=40.0):
    """10-90% rise distance of the reconstruction across a truth edge, in mm.

    Returns None when the reconstruction has no usable dynamic range across the
    edge, which is the honest answer when there are too few events.
    """
    vox = (BOUNDS[1] - BOUNDS[0]) / grid
    j = int(round((other_centre_mm - BOUNDS[0]) / vox))
    k = grid // 2
    lo = int(round((axis_centre_mm - half_width_mm - BOUNDS[0]) / vox))
    hi = int(round((axis_centre_mm + half_width_mm - BOUNDS[0]) / vox))
    lo, hi = max(0, lo), min(grid, hi)
    if hi - lo < 4:
        return None
    prof = np.asarray(volume[lo:hi, j, k], dtype=float)
    tprof = np.asarray(truth[lo:hi, j, k], dtype=float)
    if prof.max() - prof.min() < 1e-12 or tprof.max() - tprof.min() < 1e-12:
        return None
    p = (prof - prof.min()) / (prof.max() - prof.min())
    t = (tprof - tprof.min()) / (tprof.max() - tprof.min())
    edge_idx = int(np.argmax(np.abs(np.diff(t))))
    lo_v, hi_v = 0.1, 0.9

    def crossing(level, direction):
        rng = range(edge_idx, len(p)) if direction > 0 else range(edge_idx, -1, -1)
        for i in rng:
            if (p[i] >= level) if direction > 0 else (p[i] <= level):
                return i
        return None

    if t[edge_idx + 1] > t[edge_idx]:
        a, b = crossing(lo_v, -1), crossing(hi_v, 1)
    else:
        a, b = crossing(hi_v, -1), crossing(lo_v, 1)
    if a is None or b is None:
        return None
    return abs(b - a) * vox


def region_table(volume, truth, masks):
    """Per-region truth vs reconstruction, plus CNR against the background."""
    bg = masks["concrete_background"]
    out = {}
    for name, _cx, _cy, _r in INCLUSIONS:
        m = masks[name]
        out[name] = {
            "truth_lambda_mean": float(np.mean(truth[m])),
            "reconstructed_mean": float(np.mean(volume[m])),
            "ratio": float(np.mean(volume[m]) / np.mean(truth[m]))
            if np.mean(truth[m]) > 1e-12 else None,
            "cnr": cnr(volume, m, bg),
        }
    out["concrete_background"] = {
        "truth_lambda_mean": float(np.mean(truth[bg])),
        "reconstructed_mean": float(np.mean(volume[bg])),
        "ratio": None,
        "cnr": None,
    }
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--events", type=int, default=5000)
    ap.add_argument("--grid", type=int, default=64)
    ap.add_argument("--iterations", type=int, default=20)
    ap.add_argument("--with-map-em", action="store_true",
                    help="also run MAP-EM (needs a Siddon matrix; slow)")
    ap.add_argument("--seed", type=int, default=12345, help="fixes the RNG so runs are comparable")
    ap.add_argument("--beta", type=float, default=0.0, help="Huber prior weight for MAP-EM")
    ap.add_argument("--ideal-tracking", action="store_true",
                    help="use the ray tracer's exact angles instead of the detector "
                         "model; useful to separate tracking error from reconstruction error")
    ap.add_argument("--sigma-pos-mm", type=float, default=0.75,
                    help="per-layer barycentre position resolution")
    ap.add_argument("--out", type=str,
                    default=os.path.join("output_reconstruction", "accuracy_report.json"))
    args = ap.parse_args()

    grid = args.grid
    scene = build_iso_astm_calibration_phantom((grid, grid, grid))
    truth = scene.get_scattering_density()
    masks = build_masks(grid)
    bg = masks["concrete_background"]

    geometry = load_detector_geometry()
    z_top, z_bot = geometry.entry_exit_z_mm()
    tracer = MonteCarloRayTracer(scene, top_panel_z_mm=z_top, bottom_panel_z_mm=z_bot)

    energies, directions, charges = sample_cosmic_muons(
        args.events, rng=np.random.default_rng(args.seed))
    t0 = time.time()
    events = tracer.trace_batch(args.events, energies, directions, charges,
                                rng=np.random.default_rng(args.seed + 1))
    trace_s = time.time() - t0

    # --- detector-level measurement -----------------------------------------
    response = DetectorResponse(sigma_pos_mm=args.sigma_pos_mm)
    hodoscope = HodoscopeSimulator(geometry, response,
                                   rng=np.random.default_rng(args.seed + 2))
    t1 = time.time()
    measured = hodoscope.measure_batch(events)
    detect_s = time.time() - t1

    by_id = {e.event_id: e for e in events}
    lever = 0.5 * (geometry.top.lever_arm_mm("X") + geometry.bottom.lever_arm_mm("X"))
    sigma_theta = response.scattering_sigma_rad(lever)
    offset = response.theta_sq_offset_rad2(lever)

    if args.ideal_tracking:
        # Exact angles, still no momentum oracle: this isolates reconstruction
        # error from tracking error.
        p_in_list = np.array([e.p_in for e in events])
        p_out_list = np.array([e.p_out for e in events])
        angles_sq = np.array([e.scattering_angle_rad ** 2 for e in events], dtype=np.float64)
        sigma_used = 0.0
        offset_used = 0.0
    else:
        p_in_list = np.array([m.position_in_mm for m in measured])
        p_out_list = np.array([m.position_out_mm for m in measured])
        angles_sq = np.array([m.theta_measured_sq for m in measured], dtype=np.float64)
        sigma_used = float(np.mean([m.sigma_theta_rad for m in measured])) if measured else 0.0
        offset_used = float(np.mean([m.theta_sq_offset_rad2 for m in measured])) if measured else 0.0

    n_rays = len(p_in_list)
    bounds3 = (BOUNDS[0], BOUNDS[0], BOUNDS[0])
    bounds3max = (BOUNDS[1], BOUNDS[1], BOUNDS[1])

    # --- PoCA baseline -------------------------------------------------------
    poca = PoCAReconstructor(grid_size=(grid, grid, grid),
                             bounds_min_mm=bounds3, bounds_max_mm=bounds3max)
    t2 = time.time()
    if args.ideal_tracking:
        accepted = sum(1 for ev in events if poca.process_event(
            ev.p_in, ev.d_in, ev.p_out, ev.d_out))
    else:
        accepted = sum(1 for ev in events if poca.process_event(
            ev.p_in, ev.d_in, ev.p_out, ev.d_out,
            momentum_mev=3000.0))
    poca_vol = poca.get_reconstructed_density()
    poca_hits = np.asarray(poca.hit_count_grid, dtype=np.int64)
    poca_s = time.time() - t2

    # --- system matrix -------------------------------------------------------
    t3 = time.time()
    W = build_sparse_system_matrix(p_in_list, p_out_list, (grid, grid, grid),
                                   bounds3, bounds3max)
    siddon_s = time.time() - t3

    prior = gaisser_momentum_prior()
    mean_k = float(np.sum(prior.weights * prior.highland_factor()))
    nominal_p_mev = float(np.average(prior.momentum_mev, weights=prior.weights))

    t4 = time.time()
    norm_vol, norm_info = marginalized_backprojection(
        W, angles_sq, max(sigma_used, 1e-9), prior=prior, offset_rad2=offset_used)
    norm_vol = norm_vol.reshape(grid, grid, grid)
    norm_s = time.time() - t4

    nominal_vol = nominal_momentum_baseline(
        W, angles_sq, nominal_p_mev, offset_rad2=offset_used).reshape(grid, grid, grid)

    # --- oracle variant: NOT achievable, an upper bound only -----------------
    # Computed in both tracking modes so the documented 3.41 can be decomposed
    # into "detector smearing" and "momentum marginalisation" rather than just
    # being asserted to differ.
    oracle_vol = None
    if args.ideal_tracking:
        event_p_true = np.array([e.energy_initial_gev * 1000.0 for e in events],
                                dtype=np.float64)
    else:
        event_p_true = np.array(
            [by_id[m.event_id].energy_initial_gev * 1000.0 for m in measured],
            dtype=np.float64)
    if event_p_true.size:
        y_oracle = angles_sq / momentum_factor(event_p_true)
        oracle_vol, _ = marginalized_backprojection(
            W, y_oracle * mean_k, max(sigma_used, 1e-9), prior=prior,
            offset_rad2=offset_used)
        oracle_vol = oracle_vol.reshape(grid, grid, grid)

    # --- MAP-EM on the marginalised data -------------------------------------
    mapem_vol = None
    mapem_s = 0.0
    if args.with_map_em:
        t5 = time.time()
        mapem = MAPEMReconstructor(grid_size=(grid, grid, grid), beta=args.beta,
                                   max_iterations=args.iterations, use_torch=False)
        y_rays = angles_sq / mean_k
        mapem_vol = mapem.reconstruct(W, y_rays, initial_density=norm_vol)
        mapem_s = time.time() - t5

    # --- report --------------------------------------------------------------
    report = {
        "harness": "accuracy_harness",
        "seed": args.seed,
        "grid": grid,
        "ideal_tracking": bool(args.ideal_tracking),
        "events_requested": args.events,
        "events_traced": len(events),
        "events_measured": len(measured) if not args.ideal_tracking else len(events),
        "events_accepted_by_poca": accepted,
        "acceptance_ratio": (len(events) / args.events) if args.events else None,
        "detector_acceptance": (len(measured) / len(events)) if events and not args.ideal_tracking else None,
        "total_voxels": grid ** 3,
        "voxels_with_any_hit": int(np.count_nonzero(poca_hits)),
        "fraction_voxels_with_hit": float(np.count_nonzero(poca_hits)) / (grid ** 3),
        "total_hits": int(poca_hits.sum()),
        "trace_seconds": round(trace_s, 2),
        "detector_seconds": round(detect_s, 2),
        "poca_seconds": round(poca_s, 2),
        "siddon_seconds": round(siddon_s, 2),
        "solve_seconds": round(norm_s, 2),
        "truth_units": "cm^-1 (1 / radiation length)",
        "poca_output_units": "cm^-1 (Highland inversion of mean theta^2)",
        "tracking": {
            "lever_arm_mm": lever,
            "sigma_pos_mm": args.sigma_pos_mm,
            "sigma_theta_mrad": sigma_theta * 1e3,
            "theta_sq_offset_rad2": offset,
            "offset_applied_in_reconstruction_rad2": offset_used,
        },
        "momentum": {
            "treatment": "prior-marginalised (no per-event truth)",
            "prior_nodes": prior.size,
            "prior_p_min_mev": float(prior.momentum_mev.min()),
            "prior_p_max_mev": float(prior.momentum_mev.max()),
            "mean_highland_factor": mean_k,
            "nominal_momentum_mev": nominal_p_mev,
        },
        "methods": {},
        "claimed_in_documents": {
            "acceptance_ratio_range": "0.238-0.288",
            "lead_cnr_gap_analysis": 27.60,
            "lead_cnr_validation_report": 5.92,
            "iron_cnr_validation_report": 4.88,
            "reconstructed_lead_lambda_validation_report": 0.412,
            "note": "these are contradicted by docs/ACCURACY_MEASURED.md; reported here only "
                    "so the gap stays visible",
        },
    }

    method_volumes = [
        ("poca", "PoCA baseline", poca_vol),
        ("nominal", "one nominal momentum", nominal_vol),
        ("marginalised", "prior-marginalised", norm_vol),
    ]
    if oracle_vol is not None:
        method_volumes.append(("oracle", "ORACLE truth momentum (not achievable)", oracle_vol))
    if mapem_vol is not None:
        method_volumes.append(("map_em", "MAP-EM on marginalised data", mapem_vol))

    for key, label, vol in method_volumes:
        report["methods"][key] = {
            "label": label,
            "regions": region_table(vol, truth, masks),
            "rmse_vs_truth": float(np.sqrt(np.mean((vol - truth) ** 2))),
            "lead_over_background": (
                float(np.mean(vol[masks["lead"]]) / np.mean(vol[bg]))
                if np.mean(vol[bg]) > 0 else None),
        }

    report["edge_width_mm"] = {
        "lead_vs_concrete_nominal": edge_width_mm(nominal_vol, truth, grid, 125.0, 125.0),
        "lead_vs_concrete_marginalised": edge_width_mm(norm_vol, truth, grid, 125.0, 125.0),
        "iron_vs_concrete_marginalised": edge_width_mm(norm_vol, truth, grid, -125.0, 125.0),
        "claimed_in_validation_report": 6.2,
    }
    report["marginalised_info"] = norm_info.as_dict()

    os.makedirs(os.path.dirname(args.out) or ".", exist_ok=True)
    with open(args.out, "w", encoding="utf-8") as fh:
        json.dump(report, fh, indent=2)

    _print_report(report, args, n_rays)
    print(f"  written: {args.out}")


def _print_report(report, args, n_rays):
    print("=" * 84)
    print("  MUONSCOPE ACCURACY HARNESS - measured, not asserted")
    print("=" * 84)
    tr = report["tracking"]
    print(f"  events traced      {report['events_traced']}"
          f"   measured by detector {report['events_measured']}"
          f"   acceptance {100.0*(report['acceptance_ratio'] or 0):.1f}%")
    print(f"  tracking lever arm {tr['lever_arm_mm']:.1f} mm"
          f"   sigma_theta {tr['sigma_theta_mrad']:.2f} mrad"
          f"   theta^2 floor {tr['theta_sq_offset_rad2']:.3e} rad^2")
    print(f"  ideal tracking     {report['ideal_tracking']}"
          f"   (True means the detector model was bypassed)")
    mp = report["momentum"]
    print(f"  momentum           {mp['treatment']}")
    print(f"                     prior {mp['prior_nodes']} nodes, "
          f"{mp['prior_p_min_mev']:.0f}-{mp['prior_p_max_mev']:.0f} MeV/c, "
          f"<K> = {mp['mean_highland_factor']:.4e}")
    print(f"  grid {report['grid']}^3   rays {n_rays}   "
          f"siddon {report['siddon_seconds']}s   solve {report['solve_seconds']}s")
    print("-" * 84)
    print(f"  {'method':<40}{'lead L':>11}{'bg L':>11}{'ratio':>8}{'CNR':>8}{'RMSE':>9}")
    order = ["poca", "nominal", "marginalised", "map_em", "oracle"]
    for key in order:
        entry = report["methods"].get(key)
        if entry is None:
            continue
        lead = entry["regions"]["lead"]["reconstructed_mean"]
        back = entry["regions"]["concrete_background"]["reconstructed_mean"]
        lead_cnr = entry["regions"]["lead"]["cnr"]
        ratio = entry["lead_over_background"]
        print(f"  {entry['label']:<40}{lead:>11.4f}{back:>11.4f}"
              f"{(f'{ratio:.2f}' if ratio else 'n/a'):>8}"
              f"{(f'{lead_cnr:.2f}' if lead_cnr is not None else 'n/a'):>8}"
              f"{entry['rmse_vs_truth']:>9.3f}")
    tr_lead = float(np.mean([report["methods"]["marginalised"]["regions"]["lead"]["truth_lambda_mean"]]))
    tr_bg = float(np.mean([report["methods"]["marginalised"]["regions"]["concrete_background"]["truth_lambda_mean"]]))
    print(f"  {'TRUTH':<40}{tr_lead:>11.4f}{tr_bg:>11.4f}"
          f"{(tr_lead/tr_bg if tr_bg else 0):>8.2f}{'':>8}{'':>9}")
    print("-" * 84)
    if "oracle" in report["methods"]:
        gap_norm = report["methods"]["marginalised"]["rmse_vs_truth"]
        gap_oracle = report["methods"]["oracle"]["rmse_vs_truth"]
        print(f"  the oracle row is an upper bound only: it is given each muon's true")
        print(f"  momentum, which the instrument cannot measure. Marginalised RMSE "
              f"{gap_norm:.3f} vs oracle {gap_oracle:.3f}.")
    print(f"  per-inclusion CNR (marginalised):")
    for name, _a, _b, _c in INCLUSIONS:
        entry = report["methods"]["marginalised"]["regions"][name]
        c = entry["cnr"]
        print(f"    {name:<14} lambda {entry['reconstructed_mean']:>8.4f}"
              f"   truth {entry['truth_lambda_mean']:>8.4f}"
              f"   CNR {'n/a' if c is None else f'{c:.2f}'}")
    print("=" * 84)


if __name__ == "__main__":
    main()
