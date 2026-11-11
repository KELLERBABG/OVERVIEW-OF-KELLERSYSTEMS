"""Accuracy gate: fail the build when the reconstruction is not in absolute units.

Why this exists
---------------
CNR divides the mean difference between two regions by their combined standard
deviation. Multiply an entire reconstruction by any constant and CNR is unchanged.
The pipeline once fed MAP-EM raw theta^2 where the forward model needs theta^2 / K,
with K = (13.6 / (beta p))^2 ~ 1.4e-4 at 1.2 GeV. Every reconstructed value was
therefore about 8000x too large, and the project's audit - gating only on CNR -
reported PASS. Nothing in the repository compared a reconstructed value against a
known phantom in absolute cm^-1 until the harness did.

This gate does exactly that, and nothing else. It is deliberately a separate file:
the audit script is a 162-line HTTP client whose structure is easy to misread, and
three attempts to patch it produced a false success, an unmatched pattern, and a
syntax error respectively.

The gate reads the harness report, so it must be updated whenever the harness
schema changes. It now reads the post-marginalisation schema: momentum is no longer
supplied per event, and the oracle is reported separately as an unachievable bound.

Run it:
    py tests/validation_scripts/accuracy_gate.py

Exit codes: 0 all hard checks passed, 1 a check failed, 2 the harness could not run.
"""
import json
import os
import subprocess
import sys

HERE = os.path.abspath(os.path.dirname(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
HARNESS = os.path.join(ROOT, "benchmarks", "accuracy_harness.py")
REPORT = os.path.join(ROOT, "output_reconstruction", "accuracy_gate.json")

EVENTS = 20000
GRID = 16
SEED = 12345
BETA = 0.3

# Bounds on "is the reconstruction in absolute units at all". This is a units
# gate, not an accuracy claim: it is set to catch the order-of-magnitude class of
# error the gate was written for. The measured ratio is printed either way.
MIN_SCALE = 0.01
MAX_SCALE = 100.0


def run_harness():
    print(f"  running accuracy harness: {EVENTS} muons, grid {GRID}^3, seed {SEED}, "
          f"beta {BETA}, detector-level tracking")
    res = subprocess.run(
        [sys.executable, HARNESS, "--events", str(EVENTS), "--grid", str(GRID),
         "--seed", str(SEED), "--beta", str(BETA), "--with-map-em",
         "--out", REPORT],
        cwd=ROOT, capture_output=True, text=True,
    )
    if res.returncode != 0 or not os.path.exists(REPORT):
        tail = (res.stderr or res.stdout or "").strip().splitlines()[-5:]
        print("  harness did not produce a report:")
        for line in tail:
            print("      " + line)
        return None
    with open(REPORT, encoding="utf-8") as fh:
        return json.load(fh)


def main():
    print("=" * 74)
    print("  MUONSCOPE ACCURACY GATE")
    print("=" * 74)

    rep = run_harness()
    if rep is None:
        return 2

    methods = rep.get("methods", {})
    map_em = methods.get("map_em")
    poca = methods.get("poca")
    marg = methods.get("marginalised")
    if map_em is None or poca is None or marg is None:
        print("  report is missing poca / marginalised / map_em entries")
        return 2

    lead = map_em["regions"]["lead"]
    truth_lead = lead["truth_lambda_mean"]
    recon_lead = lead["reconstructed_mean"]
    scale = recon_lead / truth_lead if truth_lead else float("inf")

    marg_lead = marg["regions"]["lead"]
    marg_scale = marg_lead["reconstructed_mean"] / truth_lead if truth_lead else float("inf")

    coverage = rep["fraction_voxels_with_hit"]
    treatment = rep.get("momentum", {}).get("treatment", "")

    checks = [
        ("MAP-EM lead lambda in absolute cm^-1 (0.01x-100x of truth)",
         f"{recon_lead:.4f} vs {truth_lead:.4f} = {scale:.3f}x",
         MIN_SCALE <= scale <= MAX_SCALE),
        ("marginalised one-shot also in absolute units",
         f"{marg_lead['reconstructed_mean']:.4f} vs {truth_lead:.4f} = {marg_scale:.3f}x",
         MIN_SCALE <= marg_scale <= MAX_SCALE),
        ("MAP-EM beats the PoCA baseline on RMSE",
         f"{map_em['rmse_vs_truth']:.4f} < {poca['rmse_vs_truth']:.4f}",
         map_em["rmse_vs_truth"] < poca["rmse_vs_truth"]),
        ("voxel coverage above 30% at this grid",
         f"{100.0 * coverage:.2f}%",
         coverage > 0.30),
        ("truth field is scattering density, not counts",
         f"lead truth {truth_lead:.4f} cm^-1",
         truth_lead > 1.0),
        ("no per-event momentum oracle in the default path",
         treatment or "unknown",
         "oracle" not in treatment.lower() and "marginalis" in treatment.lower()),
    ]

    print()
    print(f"  {'check':<58}{'measured':>30}  result")
    print("  " + "-" * 96)
    failed = 0
    for label, measured, ok in checks:
        print(f"  {label:<58}{measured:>30}  {'PASS' if ok else 'FAIL'}")
        if not ok:
            failed += 1

    print()
    print("  informational - not gating:")
    print(f"    tracking lever arm      {rep['tracking']['lever_arm_mm']:.1f} mm")
    print(f"    theta resolution        {rep['tracking']['sigma_theta_mrad']:.2f} mrad")
    print(f"    detector acceptance     {100.0 * (rep['detector_acceptance'] or 0):.1f}%")
    for key in ("poca", "nominal", "marginalised", "map_em", "oracle"):
        entry = methods.get(key)
        if entry is None:
            continue
        cnr = entry["regions"]["lead"]["cnr"]
        print(f"    lead CNR {key:<14} {'n/a' if cnr is None else f'{cnr:.2f}'}"
              f"   RMSE {entry['rmse_vs_truth']:.3f}")
    if "oracle" in methods:
        print("    the oracle row is an upper bound: it is handed each muon's true")
        print("    momentum, which the instrument cannot measure.")
    print("=" * 74)

    if failed:
        print(f"  FAILED: {failed} check(s). The reconstruction is not in absolute units"
              " or has regressed.")
        return 1
    print("  PASSED: reconstruction is in absolute cm^-1 and improves on the baseline.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
