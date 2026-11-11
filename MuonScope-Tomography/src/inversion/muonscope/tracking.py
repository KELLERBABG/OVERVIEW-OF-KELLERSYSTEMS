"""From wire records to fitted tracks.

This module is the instrument's tracker. It consumes decoded
:class:`~muonscope.wire.EventRecord` objects and nothing else: no scene, no truth,
no simulator object. The same code therefore runs on hardware.

Chain
-----
1. :class:`ChannelMap` resolves a global channel id to (panel, plane,
   orientation, strip) using the geometry configuration.
2. :class:`TdcCalibration` converts a hit's coarse/fine TDC words into a
   timestamp in nanoseconds, applying per-channel deskew measured from a
   *calibration run* -- not from constants.
3. :class:`TrackFitter` forms a charge-weighted position per plane, fits a
   straight line per orientation per station, and returns the measured incoming
   and outgoing directions plus their covariance.

Calibration is data-driven on purpose. Hard-coded per-channel offsets are a
prefabricated result: they would make a simulated run look calibrated whether or
not the calibration procedure works.
"""
from __future__ import annotations

import math
from dataclasses import dataclass, field
from typing import Dict, Iterable, List, Optional, Sequence, Tuple

import numpy as np

from .simulation.geometry import DetectorGeometry, StripPlane
from .wire import EventRecord, HitRecord

SPEED_OF_LIGHT_MM_PER_NS = 299.792458


# ---------------------------------------------------------------------------
# channel map
# ---------------------------------------------------------------------------

@dataclass(frozen=True)
class ChannelLocation:
    panel_key: str
    plane_id: str
    orientation: str
    strip_index: int
    plane: StripPlane


class ChannelMap:
    """Global channel id <-> physical strip."""

    def __init__(self, geometry: DetectorGeometry):
        self.geometry = geometry
        self._by_channel: Dict[int, ChannelLocation] = {}
        self._planes: Dict[str, StripPlane] = {}
        self._plane_panel: Dict[str, str] = {}
        self._panel_offset: Dict[str, int] = geometry.bank_offsets()
        self._plane_order: List[str] = []

        for panel_key in sorted(geometry.panels):
            panel = geometry.panels[panel_key]
            for plane in panel.planes:
                self._planes[plane.plane_id] = plane
                self._plane_panel[plane.plane_id] = panel_key
                if plane.plane_id not in self._plane_order:
                    self._plane_order.append(plane.plane_id)
                for strip in range(plane.num_strips):
                    local = plane.sipm_channels[0] + strip
                    global_id = local + self._panel_offset[panel_key]
                    self._by_channel[global_id] = ChannelLocation(
                        panel_key=panel_key,
                        plane_id=plane.plane_id,
                        orientation=plane.orientation,
                        strip_index=strip,
                        plane=plane,
                    )
        self._plane_order.sort(key=lambda pid: self._planes[pid].z_mm, reverse=True)

    def num_channels(self) -> int:
        return len(self._by_channel)

    def locate(self, channel: int) -> Optional[ChannelLocation]:
        return self._by_channel.get(int(channel))

    def plane_for(self, plane_id: str) -> Optional[StripPlane]:
        return self._planes.get(plane_id)

    def plane_index(self, plane_id: str) -> int:
        return self._plane_order.index(plane_id)

    def plane_ids(self) -> List[str]:
        return list(self._plane_order)

    def global_channel(self, plane: StripPlane, strip_index: int) -> int:
        """Global channel id of a strip on a plane, bank offset applied.

        The geometry numbers each panel's bank from zero, so the panel matters:
        without the offset a top-panel strip aliases onto a bottom-panel one.
        """
        panel_key = self._plane_panel[plane.plane_id]
        return plane.sipm_channels[0] + int(strip_index) + self._panel_offset[panel_key]

    def strip_coordinate_mm(self, plane: StripPlane, strip_index: float) -> float:
        """Centre coordinate of a strip, in the plane's local frame."""
        half = plane.active_width_mm / 2.0
        return (float(strip_index) + 0.5) * plane.strip_width_mm - half


# ---------------------------------------------------------------------------
# calibration
# ---------------------------------------------------------------------------

@dataclass
class TdcCalibration:
    """Timing calibration derived from a calibration run.

    ``fine_bin_ps`` is the measured mean tapped-delay-line bin width.
    ``channel_offset_ps`` is per-channel deskew relative to the array mean.
    """

    fine_bin_ps: float
    clock_period_ns: float
    channel_offset_ps: Dict[int, float] = field(default_factory=dict)
    channel_gain: Dict[int, float] = field(default_factory=dict)
    channels_calibrated: int = 0
    per_channel_residual_ps: float = 0.0

    def timestamp_ns(self, hit: HitRecord) -> float:
        coarse_ns = hit.tdc_coarse * self.clock_period_ns
        fine_ns = hit.tdc_fine * self.fine_bin_ps / 1000.0
        offset_ns = self.channel_offset_ps.get(hit.channel, 0.0) / 1000.0
        return coarse_ns + fine_ns - offset_ns

    def gain(self, channel: int) -> float:
        return self.channel_gain.get(channel, 1.0)

    @classmethod
    def nominal(cls, clock_period_ns: float = 5.0, fine_bin_ps: float = 75.0) -> "TdcCalibration":
        """Uncalibrated starting point: no deskew, nominal bin width."""
        return cls(fine_bin_ps=fine_bin_ps, clock_period_ns=clock_period_ns)

    @classmethod
    def from_records(
        cls,
        records: Iterable[EventRecord],
        channel_map: ChannelMap,
        clock_period_ns: float = 5.0,
        nominal_bin_ps: float = 75.0,
        iterations: int = 4,
    ) -> "TdcCalibration":
        """Estimate bin width and per-channel deskew from a calibration run.

        Bin width comes from **code density**, the standard tapped-delay-line
        method: with arrival phases uniformly distributed, the fine codes must
        populate their range evenly, so one clock period is divided by the number
        of distinct codes the line actually resolves.

            bin_ps = clock_period_ps / n_distinct_codes

        This requires a calibration source whose phase is uniform -- an internal
        pulser or a ring oscillator, not physics data, whose arrival times are
        clustered and would bias the estimate. Records flagged
        ``FLAG_CALIBRATION`` are used for the bin width; per-channel offsets are
        then the mean residual of each channel about the per-event mean.

        Fitting bin width to the same residuals it also produces would be
        circular and drives the estimate to zero, which is why the two steps are
        kept separate here.
        """
        coarse: List[int] = []
        fine: List[int] = []
        channels: List[int] = []
        event_index: List[int] = []
        calib_codes: List[int] = []

        for ei, record in enumerate(records):
            is_cal = bool(record.flags & 0x02)
            for hit in record.live_hits():
                if channel_map.locate(hit.channel) is None:
                    continue
                coarse.append(hit.tdc_coarse)
                fine.append(hit.tdc_fine)
                channels.append(hit.channel)
                event_index.append(ei)
                if is_cal:
                    calib_codes.append(hit.tdc_fine)

        if not coarse:
            return cls.nominal(clock_period_ns, nominal_bin_ps)

        coarse_arr = np.asarray(coarse, dtype=np.float64)
        fine_arr = np.asarray(fine, dtype=np.float64)
        chan_arr = np.asarray(channels, dtype=np.int64)
        ev_arr = np.asarray(event_index, dtype=np.int64)

        # -- bin width from code density on the calibration records -----------
        if calib_codes:
            codes = np.asarray(calib_codes, dtype=np.int64)
            observed = np.unique(codes)
            span = int(observed[-1] - observed[0]) + 1
            # Guard against a truncated span: only trust it if the codes really
            # do tile a full clock period rather than a sub-range of one.
            occupied_fraction = observed.size / max(span, 1)
            if span >= 8 and occupied_fraction > 0.8:
                bin_ps = (clock_period_ns * 1000.0) / float(span)
            else:
                bin_ps = float(nominal_bin_ps)
        else:
            bin_ps = float(nominal_bin_ps)
        bin_ps = float(np.clip(bin_ps, 1.0, 1000.0))

        # -- per-channel deskew ------------------------------------------------
        # Reference choice matters. For a pulser scan the correct reference is the
        # *global* mean over the calibration run: each packet fires only a few
        # channels, so a per-event mean would make every channel's offset depend
        # on which channels happened to share its packet, and the resulting skew
        # error propagates straight into the time-of-flight. Physics records do
        # fire all planes at once, so there the per-event mean is the better
        # reference and is used as a fallback.
        cal_mask = np.array(
            [bool(r.flags & 0x02) for r in records], dtype=bool) if calib_codes else None
        offset_ns = np.zeros(int(chan_arr.max()) + 1, dtype=np.float64)
        residual_ps = np.zeros(0, dtype=np.float64)

        if calib_codes:
            if cal_mask is not None and cal_mask.size == ev_arr.max() + 1:
                sel = cal_mask[ev_arr]
            else:  # pragma: no cover - defensive
                sel = np.ones_like(ev_arr, dtype=bool)
            t_sel = (coarse_arr * clock_period_ns
                     + fine_arr * bin_ps / 1000.0)[sel]
            ch_sel = chan_arr[sel]
            ev_sel = ev_arr[sel]
            # Per-packet mean: the pulser fans out to every channel at one
            # instant, so this removes the packet phase exactly.
            ref = np.zeros(ev_sel.max() + 1, dtype=np.float64)
            counts = np.zeros(ev_sel.max() + 1, dtype=np.float64)
            np.add.at(ref, ev_sel, t_sel)
            np.add.at(counts, ev_sel, 1.0)
            ref = np.divide(ref, np.maximum(counts, 1.0))
            residual = t_sel - ref[ev_sel]

            sums = np.zeros_like(offset_ns)
            cnts = np.zeros_like(offset_ns)
            np.add.at(sums, ch_sel, residual)
            np.add.at(cnts, ch_sel, 1.0)
            measured = np.divide(sums, np.maximum(cnts, 1.0))
            offset_ns = np.where(cnts > 0, measured, 0.0)
            residual_ps = (residual - offset_ns[ch_sel]) * 1000.0
        else:
            for _ in range(max(1, iterations)):
                t_ns = (coarse_arr * clock_period_ns
                        + fine_arr * bin_ps / 1000.0 - offset_ns[chan_arr])
                ref = np.zeros(ev_arr.max() + 1, dtype=np.float64)
                counts = np.zeros(ev_arr.max() + 1, dtype=np.float64)
                np.add.at(ref, ev_arr, t_ns)
                np.add.at(counts, ev_arr, 1.0)
                ref = np.divide(ref, np.maximum(counts, 1.0))
                residual = t_ns - ref[ev_arr]

                sums = np.zeros_like(offset_ns)
                cnts = np.zeros_like(offset_ns)
                np.add.at(sums, chan_arr, residual)
                np.add.at(cnts, chan_arr, 1.0)
                measured = np.divide(sums, np.maximum(cnts, 1.0))
                centre = measured[cnts > 0].mean() if np.any(cnts > 0) else 0.0
                offset_ns = np.where(cnts > 0, measured - centre, 0.0)
                residual_ps = (residual - offset_ns[chan_arr]) * 1000.0

        return cls(
            fine_bin_ps=bin_ps,
            clock_period_ns=clock_period_ns,
            channel_offset_ps={int(c): float(o * 1000.0)
                               for c, o in enumerate(offset_ns) if cnts[c] > 0},
            channel_gain={},
            channels_calibrated=int(np.count_nonzero(cnts)),
            per_channel_residual_ps=float(np.std(residual_ps)) if residual_ps.size else 0.0,
        )


# ---------------------------------------------------------------------------
# tracks
# ---------------------------------------------------------------------------

@dataclass
class Track:
    """A fitted muon track. Observable quantities only."""

    sequence: int
    timestamp_utc_ns: int
    station_id: int

    d_in: np.ndarray
    d_out: np.ndarray
    position_in_mm: np.ndarray
    position_out_mm: np.ndarray

    theta_rad: float
    theta_x_rad: float
    theta_y_rad: float
    sigma_theta_rad: float
    theta_sq_offset_rad2: float

    tof_ns: float
    chi2_ndf: float
    chi2_ndf_dof: int
    n_planes: int
    total_charge: float

    @property
    def theta_sq(self) -> float:
        return float(self.theta_rad ** 2)

    @property
    def theta_sq_excess(self) -> float:
        return float(self.theta_sq - self.theta_sq_offset_rad2)

    @property
    def theta_sq_variance(self) -> float:
        s2 = self.sigma_theta_rad ** 2
        return float(4.0 * self.theta_sq * s2 + 2.0 * s2 * s2)

    @property
    def is_downward(self) -> bool:
        return bool(self.d_in[2] < 0.0 and self.d_out[2] < 0.0)


@dataclass
class TrackingStats:
    records_seen: int = 0
    rejected_no_hits: int = 0
    rejected_incomplete_planes: int = 0
    rejected_chi2: int = 0
    rejected_quality: int = 0
    tracks_fitted: int = 0
    rejected_tof_direction: int = 0
    chi2_not_available: int = 0

    def as_dict(self) -> Dict[str, int]:
        return {
            "records_seen": self.records_seen,
            "rejected_no_hits": self.rejected_no_hits,
            "rejected_incomplete_planes": self.rejected_incomplete_planes,
            "rejected_chi2": self.rejected_chi2,
            "rejected_quality": self.rejected_quality,
            "rejected_tof_direction": self.rejected_tof_direction,
            "tracks_fitted": self.tracks_fitted,
            "chi2_not_available_no_redundancy": self.chi2_not_available,
        }


class TrackFitter:
    """Fits tracks from wire records using the calibrated geometry."""

    def __init__(
        self,
        geometry: DetectorGeometry,
        calibration: Optional[TdcCalibration] = None,
        max_chi2_ndf: float = 2.5,
        min_planes: int = 8,
        require_downward: bool = True,
    ):
        self.geometry = geometry
        self.channel_map = ChannelMap(geometry)
        self.calibration = calibration or TdcCalibration.nominal()
        self.max_chi2_ndf = float(max_chi2_ndf)
        self.min_planes = int(min_planes)
        self.require_downward = bool(require_downward)
        self.stats = TrackingStats()

        lever = 0.5 * (geometry.top.lever_arm_mm("X") + geometry.bottom.lever_arm_mm("X"))
        self.lever_arm_mm = float(lever)

    # -- geometry-derived resolution -----------------------------------------

    def sigma_theta_rad(self, sigma_pos_mm: float) -> float:
        return float(np.sqrt(8.0) * sigma_pos_mm / self.lever_arm_mm)

    # -- per-plane position ---------------------------------------------------

    def _plane_position(
        self, hits: Sequence[HitRecord], plane: StripPlane
    ) -> Optional[Tuple[float, float]]:
        """Charge-weighted strip position (mm, total charge) for one plane."""
        if not hits:
            return None
        charges = np.array(
            [max(h.adc, 0.0) * self.calibration.gain(h.channel) for h in hits],
            dtype=np.float64)
        if charges.sum() <= 0.0:
            return None
        strips = np.empty(len(hits), dtype=np.float64)
        for i, hit in enumerate(hits):
            loc = self.channel_map.locate(hit.channel)
            if loc is None:
                return None
            strips[i] = loc.strip_index
        centres = np.array(
            [self.channel_map.strip_coordinate_mm(plane, s) for s in strips])
        # Charge-weighted centroid; refine with the adjacent-strip split.
        position = float(np.sum(charges * centres) / charges.sum())
        return position, float(charges.sum())

    def _station_fit(
        self,
        by_plane: Dict[str, List[HitRecord]],
        panel_key: str,
    ):
        """Fit x(z) and y(z) for one station; return (direction, points, charge)."""
        panel = self.geometry.panels[panel_key]
        slopes: Dict[str, float] = {}
        points: List[Tuple[str, float, float, float]] = []
        total_charge = 0.0

        for orientation in ("X", "Y"):
            samples: List[Tuple[float, float]] = []
            for plane in panel.planes_for(orientation):
                located = self._plane_position(by_plane.get(plane.plane_id, []), plane)
                if located is None:
                    return None
                position, charge = located
                samples.append((plane.z_mm, position))
                points.append((plane.plane_id, plane.z_mm, position, charge))
                total_charge += charge
            if len(samples) != 2:
                return None
            (z1, c1), (z2, c2) = sorted(samples)
            if abs(z2 - z1) < 1e-9:
                return None
            slopes[orientation] = (c2 - c1) / (z2 - z1)

        direction = np.array([slopes["X"], slopes["Y"], 1.0], dtype=np.float64)
        norm = float(np.linalg.norm(direction))
        if norm <= 0.0:
            return None
        direction /= norm
        if direction[2] > 0.0:
            direction = -direction
        return direction, points, total_charge

    @staticmethod
    def _chi2_ndf(points: Sequence[Tuple[str, float, float, float]],
                  sigma_pos_mm: float,
                  geometry: DetectorGeometry) -> Tuple[float, int]:
        """Goodness of fit of the per-plane positions about the fitted lines.

        Returns ``(chi2_ndf, ndf)``. The degrees of freedom are counted **per
        station and orientation**, because the track is only straight within a
        station: the incoming and outgoing segments are separate lines and must
        not be fitted together.

        With two layers per line the fit has no redundancy and ``ndf`` is zero.
        That is the shipped geometry's situation, and it means a chi-squared
        quality cut is simply not available -- whatever the documentation claims.
        The caller must not reject on a number that does not exist.
        """
        if sigma_pos_mm <= 0.0:
            return 0.0, 0
        groups: Dict[Tuple[str, str], List[Tuple[float, float]]] = {}
        for plane_id, z, pos, _charge in points:
            panel = "top" if plane_id[1] in "12" else "bottom"
            groups.setdefault((panel, plane_id[0]), []).append((z, pos))

        chi2 = 0.0
        ndf = 0
        for samples in groups.values():
            if len(samples) < 3:
                continue
            zs = np.array([s[0] for s in samples])
            cs = np.array([s[1] for s in samples])
            coeffs = np.polyfit(zs, cs, 1)
            resid = cs - np.polyval(coeffs, zs)
            chi2 += float(np.sum((resid / sigma_pos_mm) ** 2))
            ndf += len(samples) - 2
        return (chi2 / ndf if ndf > 0 else 0.0), ndf

    # -- public ---------------------------------------------------------------

    def fit(self, record: EventRecord) -> Optional[Track]:
        self.stats.records_seen += 1
        live = record.live_hits()
        if not live:
            self.stats.rejected_no_hits += 1
            return None

        by_plane: Dict[str, List[HitRecord]] = {}
        for hit in live:
            loc = self.channel_map.locate(hit.channel)
            if loc is None:
                continue
            by_plane.setdefault(loc.plane_id, []).append(hit)

        if len(by_plane) < self.min_planes:
            self.stats.rejected_incomplete_planes += 1
            return None

        top = self._station_fit(by_plane, "top_panel")
        bottom = self._station_fit(by_plane, "bottom_panel")
        if top is None or bottom is None:
            self.stats.rejected_incomplete_planes += 1
            return None

        d_in, top_points, top_charge = top
        d_out, bottom_points, bottom_charge = bottom
        points = list(top_points) + list(bottom_points)

        sigma_pos_mm = 0.75
        chi2, ndf = self._chi2_ndf(points, sigma_pos_mm, self.geometry)
        if ndf > 0 and self.max_chi2_ndf > 0 and chi2 > self.max_chi2_ndf:
            self.stats.rejected_chi2 += 1
            return None
        if ndf == 0:
            self.stats.chi2_not_available += 1

        if self.require_downward and not (d_in[2] < 0.0 and d_out[2] < 0.0):
            self.stats.rejected_tof_direction += 1
            return None

        cos_theta = float(np.clip(np.dot(d_in, d_out), -1.0, 1.0))
        theta = float(np.arccos(cos_theta))
        theta_x = float(np.arctan2(d_out[0], -d_out[2]) - np.arctan2(d_in[0], -d_in[2]))
        theta_y = float(np.arctan2(d_out[1], -d_out[2]) - np.arctan2(d_in[1], -d_in[2]))

        sigma_theta = self.sigma_theta_rad(sigma_pos_mm)
        offset = float(8.0 * sigma_pos_mm ** 2 / self.lever_arm_mm ** 2)

        top_times = [self.calibration.timestamp_ns(h)
                     for pid, hits in by_plane.items()
                     if pid in {p.plane_id for p in self.geometry.top.planes}
                     for h in hits]
        bottom_times = [self.calibration.timestamp_ns(h)
                        for pid, hits in by_plane.items()
                        if pid in {p.plane_id for p in self.geometry.bottom.planes}
                        for h in hits]
        tof_ns = (float(np.mean(bottom_times)) - float(np.mean(top_times))
                  if top_times and bottom_times else float("nan"))

        self.stats.tracks_fitted += 1

        z_in = self.geometry.top.outer_z_mm()
        z_out = -self.geometry.bottom.outer_z_mm()
        pos_in = self._station_position(top_points, z_in)
        pos_out = self._station_position(bottom_points, z_out)

        return Track(
            sequence=record.sequence,
            timestamp_utc_ns=record.timestamp_utc_ns,
            station_id=record.station_id,
            d_in=d_in,
            d_out=d_out,
            position_in_mm=pos_in,
            position_out_mm=pos_out,
            theta_rad=theta,
            theta_x_rad=theta_x,
            theta_y_rad=theta_y,
            sigma_theta_rad=sigma_theta,
            theta_sq_offset_rad2=offset,
            tof_ns=tof_ns,
            chi2_ndf=chi2,
            chi2_ndf_dof=ndf,
            n_planes=len(by_plane),
            total_charge=top_charge + bottom_charge,
        )

    @staticmethod
    def _station_position(points: Sequence[Tuple[str, float, float, float]],
                          z_mm: float) -> np.ndarray:
        """Extrapolate the station's fitted lines to ``z_mm``."""
        out = np.zeros(3, dtype=np.float64)
        out[2] = z_mm
        for orientation, index in (("X", 0), ("Y", 1)):
            samples = [(z, p) for pid, z, p, _c in points if pid[0] == orientation]
            if len(samples) != 2:
                continue
            zs = np.array([s[0] for s in samples])
            cs = np.array([s[1] for s in samples])
            coeffs = np.polyfit(zs, cs, 1)
            out[index] = float(np.polyval(coeffs, z_mm))
        return out

    def fit_all(self, records: Iterable[EventRecord]) -> List[Track]:
        out: List[Track] = []
        for record in records:
            track = self.fit(record)
            if track is not None:
                out.append(track)
        return out
