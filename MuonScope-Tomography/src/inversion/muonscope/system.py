"""The MuonScope acquisition, tracking and reconstruction system.

This is the instrument's software. Its job is to turn bytes into a density map,
and it is written so that the bytes may come from a serial port, a recorded
capture, or the instrument simulator, with no branch anywhere in the middle.

The layering is deliberate and enforced by the types:

    EventSource  --bytes-->  PacketReader  --EventRecord-->  TrackFitter
                 --Track-->  reconstructors  --volume-->     export

Nothing below :class:`PacketReader` knows the source. Nothing below
:class:`TrackFitter` knows the detector. The reconstructors receive measured
angles and their variances, and never a truth quantity.

Validation is kept strictly outside that path. :func:`compare_to_truth` takes a
completed :class:`SystemRun` plus a separately supplied truth table, so a physics
check can never feed the reconstruction it is checking.
"""
from __future__ import annotations

import json
import os
import time
from dataclasses import dataclass, field
from typing import Dict, Iterable, List, Optional, Sequence, Tuple

import numpy as np
import scipy.sparse as sp

from .acquisition import CaptureWriter, EventSource, PacketReader
from .physics.momentum_prior import MomentumPrior, default_momentum_prior
from .reconstructors.marginalized import (
    MarginalizedInfo,
    MarginalizedMLEM,
    marginalized_backprojection,
    nominal_momentum_baseline,
    prior_moments,
)
from .reconstructors.siddon import build_sparse_system_matrix
from .simulation.geometry import DetectorGeometry, load_detector_geometry
from .tracking import TdcCalibration, Track, TrackFitter, TrackingStats
from .wire import FLAG_CALIBRATION, EventRecord


@dataclass
class SystemConfig:
    """Everything the system needs that is not in the geometry file."""

    grid_size: int = 32
    bounds_min_mm: Tuple[float, float, float] = (-250.0, -250.0, -200.0)
    bounds_max_mm: Tuple[float, float, float] = (250.0, 250.0, 200.0)
    calibration_fraction: float = 0.25
    max_chi2_ndf: float = 2.5
    min_planes: int = 8
    sigma_pos_mm: float = 0.75
    iterations: int = 30
    beta: float = 0.0
    huber_delta: float = 0.01
    capture_path: Optional[str] = None
    max_records: Optional[int] = None


@dataclass
class SystemRun:
    """Result of one acquisition and reconstruction run."""

    geometry_summary: Dict[str, object]
    reader_stats: Dict[str, object]
    tracking_stats: Dict[str, int]
    calibration_summary: Dict[str, object]
    n_tracks: int
    volumes: Dict[str, np.ndarray] = field(repr=False, default_factory=dict)
    recon_info: Dict[str, object] = field(default_factory=dict)
    metrics: Dict[str, object] = field(default_factory=dict)
    timings_s: Dict[str, float] = field(default_factory=dict)
    tracks: List[Track] = field(repr=False, default_factory=list)

    def summary(self) -> Dict[str, object]:
        return {
            "geometry": self.geometry_summary,
            "reader": self.reader_stats,
            "tracking": self.tracking_stats,
            "calibration": self.calibration_summary,
            "n_tracks": self.n_tracks,
            "metrics": self.metrics,
            "timings_s": self.timings_s,
            "recon_info": self.recon_info,
        }


class MuonScopeSystem:
    """Acquire, calibrate, track, reconstruct."""

    def __init__(
        self,
        config: Optional[SystemConfig] = None,
        geometry: Optional[DetectorGeometry] = None,
        prior: Optional[MomentumPrior] = None,
    ):
        self.config = config or SystemConfig()
        self.geometry = geometry or load_detector_geometry()
        self.prior = prior or default_momentum_prior()
        self.reader = PacketReader()
        self.tracks: List[Track] = []
        self.calibration: Optional[TdcCalibration] = None
        self.fitter: Optional[TrackFitter] = None

    # -- stage 1: acquire -----------------------------------------------------

    def acquire(self, source: EventSource) -> List[EventRecord]:
        """Read, frame and decode a source into records, optionally recording."""
        records: List[EventRecord] = []
        writer: Optional[CaptureWriter] = None
        if self.config.capture_path:
            writer = CaptureWriter(self.config.capture_path)
            writer.open()
        try:
            for record in self.reader.records_from_source(source):
                records.append(record)
                if writer is not None:
                    writer.write(record)
                if (self.config.max_records is not None
                        and len(records) >= self.config.max_records):
                    break
        finally:
            if writer is not None:
                writer.close()
        return records

    # -- stage 2: calibrate ---------------------------------------------------

    def calibrate(self, records: Sequence[EventRecord]) -> TdcCalibration:
        """Derive the timing calibration from the run's calibration records.

        A dedicated pulser/ring-oscillator run is what makes the bin width
        measurable, so those records are used preferentially. Falling back to
        physics records is allowed but flagged, because their clustered arrival
        times bias a code-density estimate.
        """
        cal_records = [r for r in records if r.flags & FLAG_CALIBRATION]
        if cal_records:
            used = cal_records
            source = "calibration_run"
        else:
            n_cal = max(1, int(len(records) * self.config.calibration_fraction))
            used = list(records[:n_cal])
            source = "physics_fallback"

        self.calibration = TdcCalibration.from_records(
            used,
            channel_map=TrackFitter(self.geometry).channel_map,
            clock_period_ns=5.0,
            nominal_bin_ps=75.0,
        )
        self._calibration_source = source
        self._calibration_records = len(used)
        return self.calibration

    # -- stage 3: track -------------------------------------------------------

    def build_tracks(self, records: Sequence[EventRecord]) -> List[Track]:
        if self.calibration is None:
            self.calibrate(records)
        self.fitter = TrackFitter(
            self.geometry,
            calibration=self.calibration,
            max_chi2_ndf=self.config.max_chi2_ndf,
            min_planes=self.config.min_planes,
        )
        # Calibration records are not physics: they carry a pulser pattern, not a
        # muon, so they must never reach the tracker.
        physics = [r for r in records if not (r.flags & FLAG_CALIBRATION)]
        self._physics_records = len(physics)
        self.tracks = self.fitter.fit_all(physics)
        return self.tracks

    # -- stage 4: reconstruct -------------------------------------------------

    def reconstruct(self, tracks: Sequence[Track]) -> Dict[str, np.ndarray]:
        cfg = self.config
        grid = (cfg.grid_size,) * 3
        if not tracks:
            empty = np.zeros(grid, dtype=np.float64)
            return {"empty": empty}

        p_in = np.array([t.position_in_mm for t in tracks], dtype=np.float64)
        p_out = np.array([t.position_out_mm for t in tracks], dtype=np.float64)
        theta_sq = np.array([t.theta_sq for t in tracks], dtype=np.float64)
        sigma_theta = float(np.mean([t.sigma_theta_rad for t in tracks]))
        offset = float(np.mean([t.theta_sq_offset_rad2 for t in tracks]))

        W = build_sparse_system_matrix(
            p_in, p_out, grid, cfg.bounds_min_mm, cfg.bounds_max_mm)

        bp, bp_info = marginalized_backprojection(
            W, theta_sq, sigma_theta, prior=self.prior, offset_rad2=offset)
        bp = bp.reshape(grid)

        solver = MarginalizedMLEM(
            prior=self.prior, iterations=cfg.iterations,
            beta=cfg.beta, huber_delta=cfg.huber_delta)
        mlem, mlem_info = solver.reconstruct(
            W, theta_sq, sigma_theta, offset_rad2=offset,
            initial_density=bp, grid_size=grid)

        nominal_p = float(np.average(
            self.prior.momentum_mev, weights=self.prior.weights))
        nominal = nominal_momentum_baseline(
            W, theta_sq, nominal_p, offset_rad2=offset).reshape(grid)

        self._last_recon_info = {
            "backprojection": bp_info.as_dict(),
            "mlem": mlem_info.as_dict(),
            "nominal_momentum_mev": nominal_p,
            "matrix_nonzeros": int(W.nnz),
        }
        return {
            "backprojection": bp,
            "marginalized_mlem": mlem,
            "nominal_momentum": nominal,
        }

    # -- orchestration --------------------------------------------------------

    def run(self, source: EventSource) -> SystemRun:
        timings: Dict[str, float] = {}

        t0 = time.time()
        records = self.acquire(source)
        timings["acquire_s"] = time.time() - t0

        t0 = time.time()
        calibration = self.calibrate(records)
        timings["calibrate_s"] = time.time() - t0

        t0 = time.time()
        tracks = self.build_tracks(records)
        timings["track_s"] = time.time() - t0

        t0 = time.time()
        volumes = self.reconstruct(tracks)
        timings["reconstruct_s"] = time.time() - t0

        theta = np.array([t.theta_rad for t in tracks]) if tracks else np.zeros(0)
        tof = np.array([t.tof_ns for t in tracks]) if tracks else np.zeros(0)
        chi2 = np.array([t.chi2_ndf for t in tracks]) if tracks else np.zeros(0)

        metrics: Dict[str, object] = {
            "n_tracks": len(tracks),
            "record_to_track_efficiency": (
                len(tracks) / len(records) if records else None),
            "theta_rms_mrad": float(np.sqrt(np.mean(theta ** 2)) * 1e3) if theta.size else None,
            "theta_median_mrad": float(np.median(theta) * 1e3) if theta.size else None,
            "sigma_theta_from_geometry_mrad": (
                float(np.mean([t.sigma_theta_rad for t in tracks]) * 1e3) if tracks else None),
            "tof_median_ns": float(np.median(tof)) if tof.size else None,
            "chi2_ndf_median": float(np.median(chi2)) if chi2.size else None,
            "mean_highland_factor": prior_moments(self.prior)[0],
            "theta_sq_offset_rad2": (
                float(np.mean([t.theta_sq_offset_rad2 for t in tracks])) if tracks else None),
            "fraction_theta_sq_below_offset": (
                float(np.mean([t.theta_sq_excess < 0.0 for t in tracks])) if tracks else None),
        }

        return SystemRun(
            geometry_summary=self.geometry.summary(),
            reader_stats=dict(self.reader.stats),
            tracking_stats=self.fitter.stats.as_dict() if self.fitter else {},
            calibration_summary={
                "fine_bin_ps": calibration.fine_bin_ps,
                "clock_period_ns": calibration.clock_period_ns,
                "channels_calibrated": calibration.channels_calibrated,
                "residual_ps": calibration.per_channel_residual_ps,
                "source": getattr(self, "_calibration_source", "unknown"),
                "records_used": getattr(self, "_calibration_records", 0),
            },
            n_tracks=len(tracks),
            volumes=volumes,
            recon_info=getattr(self, "_last_recon_info", {}),
            metrics=metrics,
            timings_s=timings,
            tracks=tracks,
        )

    def run_from_source(self, source: EventSource) -> SystemRun:
        with source:
            return self.run(source)


# ---------------------------------------------------------------------------
# validation (kept outside the processing path)
# ---------------------------------------------------------------------------

def compare_to_truth(
    run: SystemRun,
    truth_by_sequence: Dict[int, np.ndarray],
) -> Dict[str, object]:
    """Compare fitted tracks against a truth table, for simulation only.

    ``truth_by_sequence`` maps an event sequence number to
    ``(theta_true_rad, energy_gev)``. Nothing in this function is reachable from
    :meth:`MuonScopeSystem.run`, which is what allows the reconstruction to be
    checked without being contaminated.
    """
    matched = [(t, truth_by_sequence[t.sequence]) for t in run.tracks
               if t.sequence in truth_by_sequence]
    if not matched:
        return {"matched": 0}

    theta_meas = np.array([t.theta_rad for t, _ in matched])
    theta_true = np.array([tv[0] for _, tv in matched])
    energy = np.array([tv[1] for _, tv in matched])

    # Resolution on the squared angle is the quantity the reconstruction uses.
    excess = theta_meas ** 2 - theta_true ** 2
    offset = float(np.mean([t.theta_sq_offset_rad2 for t, _ in matched]))

    # Split by true scattering strength: below the floor the measured angle is
    # pure detector noise, which is the honest way to report a resolution.
    weak = theta_true < np.sqrt(offset) if offset > 0 else np.zeros_like(theta_true, bool)

    return {
        "matched": len(matched),
        "theta_true_rms_mrad": float(np.sqrt(np.mean(theta_true ** 2)) * 1e3),
        "theta_meas_rms_mrad": float(np.sqrt(np.mean(theta_meas ** 2)) * 1e3),
        "theta_sq_offset_predicted_rad2": offset,
        "mean_theta_sq_excess_rad2": float(np.mean(excess)),
        "resolution_ratio": float(np.mean(excess) / offset) if offset > 0 else None,
        "n_weak_scatter": int(np.count_nonzero(weak)),
        "energy_median_gev": float(np.median(energy)),
        "theta_meas_over_truth_median": float(np.median(theta_meas / np.maximum(theta_true, 1e-9))),
    }
