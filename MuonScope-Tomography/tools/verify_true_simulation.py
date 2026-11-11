"""Smoke test for the new detector-level, momentum-marginalised path.

Run from anywhere:
    py tools/verify_true_simulation.py
"""
import os
import sys
import time

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, ".."))
sys.path.insert(0, os.path.join(ROOT, "src", "inversion"))

from muonscope.physics.gaisser import gaisser_flux, sample_cosmic_muons          # noqa: E402
from muonscope.physics.momentum_prior import gaisser_momentum_prior              # noqa: E402
from muonscope.simulation.geometry import load_detector_geometry                 # noqa: E402
from muonscope.simulation.detector import DetectorResponse, HodoscopeSimulator   # noqa: E402
from muonscope.simulation.ray_tracer import MonteCarloRayTracer                  # noqa: E402
from muonscope.simulation.synthetic_scene import create_heavy_inclusion_phantom  # noqa: E402


def main():
    print("=" * 78)
    print("  TRUE-SIMULATION SMOKE TEST")
    print("=" * 78)

    geo = load_detector_geometry()
    print("\n[1] geometry loaded from config/detector_geometry.yaml")
    for k, v in geo.summary().items():
        print(f"      {k:<32} {v}")

    resp = DetectorResponse()
    lever = 0.5 * (geo.top.lever_arm_mm("X") + geo.bottom.lever_arm_mm("X"))
    print("\n[2] resolution implied by that geometry (not quoted, computed)")
    print(f"      lever arm used            {lever:.1f} mm")
    print(f"      predicted sigma_theta     {resp.scattering_sigma_rad(lever)*1e3:.2f} mrad")
    print(f"      theta^2 offset floor      {resp.theta_sq_offset_rad2(lever):.4e} rad^2")
    print(f"      (docs claim 2.1 mrad; the 600 mm term would give "
          f"{resp.scattering_sigma_rad(600.0)*1e3:.2f} mrad)")

    prior = gaisser_momentum_prior()
    print("\n[3] momentum prior (physics input, identical for every event)")
    print(f"      nodes                     {prior.size}")
    print(f"      p range                   {prior.momentum_mev.min():.0f}-"
          f"{prior.momentum_mev.max():.0f} MeV/c")
    k = prior.highland_factor()
    print(f"      <K>                       {float(np.sum(prior.weights*k)):.4e}")
    print(f"      Var[K]                    "
          f"{float(np.sum(prior.weights*k*k) - np.sum(prior.weights*k)**2):.4e}")

    print("\n[4] sample spectrum actually uses gaisser_flux (rejection sampled)")
    e, d, q = sample_cosmic_muons(200000, rng=np.random.default_rng(7))
    print(f"      mean E                    {np.mean(e)*1000:.0f} MeV")
    print(f"      charge ratio mu+/mu-      {np.mean(q > 0)/(1-np.mean(q > 0)):.3f}  (expect 1.25)")
    # compare sampled histogram against the analytic density
    edges = np.array([0.5, 1.0, 2.0, 5.0, 20.0])
    hist, _ = np.histogram(e, bins=edges)
    ana = np.array([
        np.trapezoid(gaisser_flux(np.linspace(a, b, 40), np.zeros(40)), np.linspace(a, b, 40))
        for a, b in zip(edges[:-1], edges[1:])
    ])
    frac_s = hist / hist.sum()
    frac_a = ana / ana.sum()
    print(f"      E-bin fraction  sampled   {np.round(frac_s, 3)}")
    print(f"      E-bin fraction  gaisser   {np.round(frac_a, 3)}")

    print("\n[5] detector-level measurement")
    scene = create_heavy_inclusion_phantom(grid_size=(16, 16, 16))
    z_top, z_bot = geo.entry_exit_z_mm()
    tracer = MonteCarloRayTracer(scene, top_panel_z_mm=z_top, bottom_panel_z_mm=z_bot)
    energies, directions, charges = sample_cosmic_muons(
        4000, rng=np.random.default_rng(11))
    t0 = time.time()
    truth = tracer.trace_batch(4000, energies, directions, charges,
                               rng=np.random.default_rng(12))
    print(f"      traced (accepted by tracer) {len(truth)}/{4000}  ({time.time()-t0:.1f}s)")

    sim = HodoscopeSimulator(geo, DetectorResponse(), rng=np.random.default_rng(13))
    measured = sim.measure_batch(truth)
    print(f"      detected by 8-layer hodoscope {len(measured)}"
          f"  ({len(measured)/max(len(truth),1)*100:.1f}% of traced)")

    t_meas = np.array([m.theta_measured_rad for m in measured])
    by_id = {e.event_id: e for e in truth}
    t_true = np.array([by_id[m.event_id].scattering_angle_rad for m in measured])
    print(f"      measured theta RMS        {np.sqrt(np.mean(t_meas**2))*1e3:.2f} mrad")
    print(f"      truth    theta RMS        {np.sqrt(np.mean(t_true**2))*1e3:.2f} mrad")
    print(f"      noise-only floor          {np.sqrt(8)*0.75/lever*1e3:.2f} mrad")
    # Compare like with like: only events whose true scattering is well below the
    # floor isolate the detector term, since E[theta_m^2 - theta_t^2] = 8 s^2 / L^2
    # holds exactly and is independent of theta_t.
    resid = t_meas**2 - t_true**2
    print(f"      <theta_meas^2 - theta_true^2> {np.mean(resid):.4e} rad^2"
          f"  (predicted floor {resp.theta_sq_offset_rad2(lever):.4e})")
    print(f"      ratio measured/predicted  {np.mean(resid)/resp.theta_sq_offset_rad2(lever):.3f}"
          "  (expect ~1.00)")
    frac_below = np.mean(np.array([m.theta_sq_excess for m in measured]) < 0)
    print(f"      events below the floor    {frac_below*100:.1f}%")

    print("\n[6] no-truth-leak check on the observable record")
    print(f"      MeasuredTrack fields      {sorted(measured[0].truth_free_summary().keys())}")
    leaked = [f for f in ("energy", "momentum", "p_mev", "energy_initial")
              if any(f in k for k in measured[0].truth_free_summary())]
    print(f"      momentum-bearing fields   {leaked or 'none'}")

    print("\n" + "=" * 78)
    print("  SMOKE TEST COMPLETE")
    print("=" * 78)


if __name__ == "__main__":
    main()
