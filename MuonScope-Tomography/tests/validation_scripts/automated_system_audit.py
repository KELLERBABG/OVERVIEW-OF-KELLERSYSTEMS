"""MuonScope system audit: a real gate on the real processing chain.

This replaces a 162-line HTTP client that printed ``[PASS]`` unconditionally --
including ``[PASS] Lead Target CNR: {value} (ASTM Standard Target >= 4.5)``
without ever comparing the value to 4.5, and a closing
``100% OPERATIONAL`` line. The project's own ``docs/ACCURACY_MEASURED.md``
already recorded that its "PASS strings are hardcoded".

What this does instead
----------------------
Runs the instrument software end to end on simulator bytes and checks numbers it
computes itself. Every check is evaluated before it is printed, and the process
exits non-zero if any check fails.

    py tests/validation_scripts/automated_system_audit.py

Exit codes: 0 all checks passed, 1 a check failed, 2 the audit could not run.
"""
import os
import sys
import time

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "src", "inversion"))

from muonscope.acquisition import PacketReader, SimulatedSource, SimulatorConfig  # noqa: E402
from muonscope.simulation.detector import DetectorResponse, HodoscopeSimulator    # noqa: E402
from muonscope.simulation.geometry import load_detector_geometry                  # noqa: E402
from muonscope.simulation.ray_tracer import MonteCarloRayTracer                   # noqa: E402
from muonscope.simulation.synthetic_scene import create_heavy_inclusion_phantom   # noqa: E402
from muonscope.system import MuonScopeSystem, SystemConfig, compare_to_truth      # noqa: E402
from muonscope.tracking import ChannelMap, TdcCalibration, Track                  # noqa: E402
from muonscope.wire import EventRecord                                            # noqa: E402

GRID = 12
EVENTS = 6000
CAL_PACKETS = 400
SEED = 20261007
TDC_TRUE_BIN_PS = 75.0
TDC_JITTER_PS = 80.0
PANEL_SEPARATION_MM = 600.0
SPEED_OF_LIGHT_MM_PER_NS = 299.792458


class Audit:
    """Collects gating checks and non-gating findings.

    A check gates the exit code: it asserts something about the *software*.
    A finding records a real property of the *design* that the software is
    correctly reporting -- for example that a documented quality cut cannot be
    computed with the shipped plane count. Findings do not fail the audit, but
    they are printed, because an audit that hides them is how the previous
    version ended up claiming 100% operational.
    """

    def __init__(self):
        self.checks = []
        self.findings = []

    def check(self, label, measured, ok, note=""):
        self.checks.append((label, str(measured), bool(ok)))
        return ok

    def finding(self, label, measured, note=""):
        self.findings.append((label, str(measured), note))

    def report(self):
        print()
        print(f"  {'check':<52}{'measured':>26}  result")
        print("  " + "-" * 92)
        failed = 0
        for label, measured, ok in self.checks:
            print(f"  {label:<52}{measured:>26}  {'PASS' if ok else 'FAIL'}")
            if not ok:
                failed += 1
        if self.findings:
            print()
            print("  findings -- real design limits the software detects (not gated):")
            for label, measured, note in self.findings:
                print(f"    - {label}: {measured}")
                if note:
                    print(f"      {note}")
        return failed


def main():
    t_start = time.time()
    print("=" * 96)
    print("  MUONSCOPE SYSTEM AUDIT -- every number below is computed here")
    print("=" * 96)

    audit = Audit()

    # ---------------------------------------------------------------- geometry
    geometry = load_detector_geometry()
    cmap = ChannelMap(geometry)
    audit.check("geometry: channels mapped", cmap.num_channels(),
                cmap.num_channels() == geometry.total_channels())
    audit.check("geometry: station lever arm (mm)", f"{geometry.top.lever_arm_mm('X'):.1f}",
                geometry.top.lever_arm_mm("X") > 0)
    audit.check("geometry: no channel aliasing between banks",
                f"{cmap.locate(0).panel_key} / {cmap.locate(256).panel_key}",
                cmap.locate(0).panel_key != cmap.locate(256).panel_key)

    # ------------------------------------------------------- wire + acquisition
    scene = create_heavy_inclusion_phantom(grid_size=(GRID,) * 3)
    sim_cfg = SimulatorConfig(num_events=EVENTS, seed=SEED,
                              calibration_packets=CAL_PACKETS)
    source = SimulatedSource(scene, geometry, config=sim_cfg)
    reader = PacketReader()
    records = list(reader.records_from_source(source))

    cal_records = [r for r in records if r.flags & 0x02]
    phys_records = [r for r in records if not (r.flags & 0x02)]
    audit.check("wire: packets decoded", len(records), len(records) > 0)
    audit.check("wire: packets rejected by CRC/framing",
                reader.stats["packets_bad"], reader.stats["packets_bad"] == 0)
    audit.check("wire: calibration run present", len(cal_records), len(cal_records) > 0)
    audit.check("wire: physics records present", len(phys_records), len(phys_records) > 0)

    # ------------------------------------------------------------- calibration
    calibration = TdcCalibration.from_records(
        records, cmap, clock_period_ns=5.0, nominal_bin_ps=75.0)
    bin_err = abs(calibration.fine_bin_ps - TDC_TRUE_BIN_PS) / TDC_TRUE_BIN_PS
    audit.check("calibration: TDC bin width (ps, true 75)",
                f"{calibration.fine_bin_ps:.2f}", bin_err < 0.05)
    audit.check("calibration: channels calibrated",
                calibration.channels_calibrated,
                calibration.channels_calibrated == geometry.total_channels())
    audit.check("calibration: deskew residual (ps, jitter 80)",
                f"{calibration.per_channel_residual_ps:.1f}",
                calibration.per_channel_residual_ps < 3.0 * TDC_JITTER_PS)

    # ---------------------------------------------------------------- tracking
    config = SystemConfig(
        grid_size=GRID,
        bounds_min_mm=tuple(float(v) for v in scene.bounds_min_mm),
        bounds_max_mm=tuple(float(v) for v in scene.bounds_max_mm),
        iterations=10,
    )
    system = MuonScopeSystem(config=config, geometry=geometry)
    run = system.run_from_source(source)

    eff = run.metrics["record_to_track_efficiency"]
    audit.check("tracking: physics records fitted to tracks", run.n_tracks, run.n_tracks > 0)
    audit.check("tracking: record -> track efficiency",
                f"{eff:.3f}" if eff else "n/a", (eff or 0) > 0.5)

    tof_expected = PANEL_SEPARATION_MM / SPEED_OF_LIGHT_MM_PER_NS
    tof = run.metrics["tof_median_ns"]
    audit.check("tracking: median ToF (ns, expect ~2.0)", f"{tof:.3f}",
                tof is not None and abs(tof - tof_expected) < 0.5)

    # The documentation gates on a chi-squared cut (max_chi2_ndf = 2.5), but each
    # station fits a straight line through exactly two layers per coordinate, so
    # the fit has no degrees of freedom. The software reports that honestly
    # instead of inventing a number. What is gated here is only that it does not
    # fabricate a chi-squared; the absence of the cut itself is a finding.
    no_redundancy = run.tracking_stats.get("chi2_not_available_no_redundancy", 0)
    consistent = all(
        (t.chi2_ndf == 0.0) == (t.chi2_ndf_dof == 0) for t in run.tracks)
    audit.check("tracking: chi2 reported consistently, not fabricated",
                "consistent" if consistent else "inconsistent", consistent)
    if no_redundancy:
        audit.finding(
            "chi-squared track cut is not computable",
            f"{no_redundancy} tracks with ndf=0",
            "each station fits a line through 2 layers per coordinate, so ndf=0; "
            "the documented chi2/ndf <= 2.5 gate cannot be implemented until a "
            "third layer per coordinate is added")

    # --------------------------------------------------- detector resolution
    # Coarse check only. The angle-space excess E[theta_meas^2 - theta_true^2]
    # carries a large cross term, 2 E[delta_s_true . delta], whose sampling error
    # at these event counts is comparable to the signal -- measured ratios range
    # roughly 0.7 to 1.2 run to run around a true value of 1.0. The precise check
    # uses the slope residual, whose variance is well conditioned, and lives in
    # src/inversion/tests/test_system.py.
    truth = compare_to_truth(run, source.truth_by_sequence)
    ratio = truth.get("resolution_ratio")
    audit.check("resolution: model within a factor of two (coarse)",
                f"{ratio:.3f}" if ratio else "n/a",
                ratio is not None and 0.5 < ratio < 2.0)

    # ----------------------------------------------------------------- recon
    volumes = run.volumes
    audit.check("reconstruction: volumes produced", len(volumes), len(volumes) >= 3)
    finite = all(np.all(np.isfinite(v)) and np.all(v >= 0.0) for v in volumes.values())
    audit.check("reconstruction: finite and non-negative", "yes" if finite else "no", finite)

    truth_volume = scene.get_scattering_density()
    lead = truth_volume > 1.0
    lead_truth = float(truth_volume[lead].mean())
    err_marginal = abs(float(volumes["backprojection"][lead].mean()) - lead_truth)
    err_nominal = abs(float(volumes["nominal_momentum"][lead].mean()) - lead_truth)
    audit.check("reconstruction: prior beats single nominal momentum",
                f"{err_marginal:.3f} < {err_nominal:.3f}",
                err_marginal <= err_nominal)

    # ------------------------------------------------------- truth containment
    leaked = [f for f in Track.__dataclass_fields__
              if "energy" in f or "momentum" in f or f.endswith("_true")]
    audit.check("containment: Track exposes no truth quantity",
                leaked or "none", not leaked)
    record_leak = [f for f in EventRecord.__dataclass_fields__
                   if "energy" in f or "momentum" in f]
    audit.check("containment: wire record carries no truth quantity",
                record_leak or "none", not record_leak)

    # ------------------------------------------------------------------ report
    failed = audit.report()
    elapsed = time.time() - t_start
    print("=" * 96)
    if failed:
        print(f"  AUDIT FAILED: {failed} check(s) did not pass.  ({elapsed:.1f}s)")
        print("=" * 96)
        return 1
    print(f"  AUDIT PASSED: {len(audit.checks)} checks, all evaluated.  ({elapsed:.1f}s)")
    print("=" * 96)
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as exc:  # pragma: no cover - reported, not swallowed
        print(f"  audit could not run: {type(exc).__name__}: {exc}")
        sys.exit(2)
