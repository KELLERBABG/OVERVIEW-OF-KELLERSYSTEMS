"""Detector-level measurement of a muon track.

The reconstruction used to consume the ray tracer's *truth*: the exact scattering
angle between the exact incoming and outgoing directions, at the exact plane
positions, with the exact momentum attached. Nothing in that chain resembles what
the instrument delivers, so the reported accuracy measured the simulator rather
than the method.

This module inserts the missing step. A truth track is converted into strip hits
on the four layers of each station, those hits are smeared by the readout
resolution and (optionally) digitised to strip granularity, and the incoming and
outgoing directions are re-fitted from the measured points. The scattering angle
that reaches the reconstructor is therefore the *measured* one, and it carries the
detector's angular resolution as a real, computed uncertainty.

Two consequences worth stating plainly, because the documentation does not:

1. The lever arm that sets angular resolution is the distance between the two
   layers of a station that measure the same coordinate -- 140 mm in the shipped
   geometry -- not the ~610 mm station-to-station separation. Directions are fitted
   per station.
2. Momentum is not measured and is not returned. Anything that needs it must
   marginalise over the prior in :mod:`muonscope.physics.momentum_prior`.
"""
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Sequence

import numpy as np

from .geometry import DetectorGeometry, StripPlane
from .ray_tracer import MuonTrackEvent

MUON_MASS_MEV = 105.6583755


@dataclass(frozen=True)
class DetectorResponse:
    """Readout response parameters.

    ``sigma_pos_mm`` is the per-layer position resolution of the charge-weighted
    barycentre across neighbouring triangular strips. It is an *input* to this
    module; the angular resolution is an *output*, computed from it and from the
    geometry.

    Resolution algebra, for a station whose two same-coordinate layers are
    ``L`` apart: each layer reports a position with standard deviation
    ``sigma_pos``, so the fitted slope ``dx/dz`` has per-component standard
    deviation ``sqrt(2) * sigma_pos / L``. A direction therefore carries a
    two-component slope error of total mean square
    ``2 * (sqrt(2) sigma_pos / L)^2 = 4 sigma_pos^2 / L^2``.

    The scattering angle is the difference of two such directions, so

    * projected (one transverse axis): ``sigma = 2 sigma_pos / L``
    * three-dimensional angle: ``sigma = sqrt(8) sigma_pos / L``

    and the *squared* measured angle is biased high by the summed direction
    variance,

        ``E[theta_meas^2] = theta_true^2 + 8 sigma_pos^2 / L^2``.

    That additive term is not a nuisance to be wished away: it is a resolution
    floor that a real reconstruction has to subtract, and
    :meth:`theta_sq_offset_rad2` reports it so the two can be compared.
    """

    sigma_pos_mm: float = 0.75
    strip_digitised: bool = False
    plane_efficiency: float = 0.95
    tdc_bin_ps: float = 75.0
    tdc_jitter_ps: float = 80.0

    def slope_sigma_rad(self, lever_arm_mm: float) -> float:
        """Per-component uncertainty on the fitted slope ``dx/dz``."""
        if lever_arm_mm <= 0.0:
            raise ValueError("lever arm must be positive")
        return float(np.sqrt(2.0) * self.sigma_pos_mm / lever_arm_mm)

    def projected_sigma_rad(self, lever_arm_mm: float) -> float:
        """Uncertainty on a single projected scattering angle."""
        return float(2.0 * self.sigma_pos_mm / lever_arm_mm)

    def scattering_sigma_rad(self, lever_arm_mm: float) -> float:
        """Uncertainty on the three-dimensional scattering angle."""
        return float(np.sqrt(8.0) * self.sigma_pos_mm / lever_arm_mm)

    def theta_sq_offset_rad2(self, lever_arm_mm: float) -> float:
        """Additive bias on ``theta_meas^2`` from detector angular smearing."""
        return float(8.0 * self.sigma_pos_mm ** 2 / lever_arm_mm ** 2)


@dataclass
class LayerHit:
    plane_id: str
    orientation: str
    z_mm: float
    true_coord_mm: float
    measured_coord_mm: float
    strip_index: int
    charge_fraction: float
    time_ps: float


@dataclass
class MeasuredTrack:
    """What the instrument actually delivers for one event."""

    event_id: int
    detected: bool

    position_in_mm: np.ndarray
    position_out_mm: np.ndarray
    d_in_measured: np.ndarray
    d_out_measured: np.ndarray

    theta_measured_rad: float
    theta_measured_x_rad: float
    theta_measured_y_rad: float
    sigma_theta_rad: float
    theta_sq_offset_rad2: float

    time_top_ps: float
    time_bottom_ps: float
    n_layers_hit: int
    layers: List[LayerHit] = field(default_factory=list)

    @property
    def tof_ps(self) -> float:
        return float(self.time_bottom_ps - self.time_top_ps)

    @property
    def theta_measured_sq(self) -> float:
        return float(self.theta_measured_rad ** 2)

    @property
    def theta_sq_variance(self) -> float:
        """Variance of ``theta_meas^2``, given the measured angle.

        For ``theta_meas = theta_true + eps`` with ``eps ~ N(0, sigma^2)``,
        ``Var[theta_meas^2] = 4 theta^2 sigma^2 + 2 sigma^4``. The quartic term
        matters here because the detector floor is comparable to the signal for
        ordinary muons through concrete, so ``theta`` is not always large
        compared with ``sigma``.
        """
        s2 = self.sigma_theta_rad ** 2
        return float(4.0 * self.theta_measured_sq * s2 + 2.0 * s2 * s2)

    @property
    def theta_sq_excess(self) -> float:
        """Measured ``theta^2`` with the detector resolution floor removed.

        The quantity that actually carries scattering information. May be
        negative, which is physically meaningful: it says the event scattered
        less than the detector can resolve.
        """
        return float(self.theta_measured_sq - self.theta_sq_offset_rad2)

    def truth_free_summary(self) -> Dict[str, float]:
        """The observable record. Deliberately contains no truth quantity."""
        return {
            "theta_rad": self.theta_measured_rad,
            "theta_x_rad": self.theta_measured_x_rad,
            "theta_y_rad": self.theta_measured_y_rad,
            "theta_sq_rad2": self.theta_measured_sq,
            "theta_sq_excess_rad2": self.theta_sq_excess,
            "theta_sq_variance_rad4": self.theta_sq_variance,
            "sigma_theta_rad": self.sigma_theta_rad,
            "tof_ps": self.tof_ps,
        }


def _project_to_layer(point_mm: np.ndarray, direction: np.ndarray, z_mm: float) -> np.ndarray:
    """Extrapolate a straight track from ``point_mm`` to the plane at ``z_mm``."""
    dz = float(direction[2])
    if abs(dz) < 1e-12:
        raise ValueError("track is parallel to the measurement planes")
    scale = (z_mm - float(point_mm[2])) / dz
    return np.asarray(point_mm, dtype=np.float64) + scale * np.asarray(direction, dtype=np.float64)


class HodoscopeSimulator:
    """Converts truth tracks into detector-level measurements."""

    def __init__(
        self,
        geometry: DetectorGeometry,
        response: Optional[DetectorResponse] = None,
        rng: Optional[np.random.Generator] = None,
    ):
        self.geometry = geometry
        self.response = response or DetectorResponse()
        self.rng = rng if rng is not None else np.random.default_rng()

        # Resolve, once, which layers measure which coordinate on each station.
        self._stations: Dict[str, Dict[str, List[StripPlane]]] = {
            "top": {o: self.geometry.top.planes_for(o) for o in ("X", "Y")},
            "bottom": {o: self.geometry.bottom.planes_for(o) for o in ("X", "Y")},
        }
        self._lever: Dict[str, Dict[str, float]] = {
            station: {o: self._lever_arm(planes) for o, planes in by_orient.items()}
            for station, by_orient in self._stations.items()
        }
        self._plane_lookup: Dict[str, StripPlane] = {
            p.plane_id: p
            for panel in geometry.panels.values() for p in panel.planes
        }

    @staticmethod
    def _lever_arm(planes: Sequence[StripPlane]) -> float:
        zs = [p.z_mm for p in planes]
        return float(abs(max(zs) - min(zs)))

    # -- response -------------------------------------------------------------

    def _smear(self, plane: StripPlane, true_coord_mm: float) -> Optional[List[tuple]]:
        """Split a hit across neighbouring strips, as a triangular array does.

        Returns a list of ``(strip_index, charge)``, or ``None`` if the crossing
        falls outside the active area.

        The physics that makes the whole design work: with a triangular
        cross-section the scintillation light is shared between two adjacent
        strips in proportion to where the muon crossed. Reconstructing the
        position as the charge-weighted mean of the two strip *centres* recovers
        the true coordinate, which is why the array can do better than the strip
        pitch. Emitting a single strip instead would quantise every hit to a strip
        centre and quietly inflate the angular resolution by the ratio
        ``pitch / sigma_pos`` -- about 7.7x here.
        """
        pitch = plane.strip_width_mm
        half_span = plane.active_width_mm / 2.0
        local = true_coord_mm + half_span
        if local < 0.0 or local >= plane.active_width_mm:
            return None  # outside the active area: no light, no hit

        fractional = local / pitch
        primary = int(np.floor(fractional))
        if primary >= plane.num_strips:
            primary = plane.num_strips - 1
        fraction = float(fractional - primary)

        if self.response.strip_digitised:
            # Digital readout: one channel, position known only to a strip centre.
            return [(int(primary), 1.0)]

        # Position resolution enters as an uncertainty on the charge split, which
        # is what the charge-weighted centroid actually measures.
        if self.response.sigma_pos_mm > 0.0:
            fraction += float(self.rng.normal(0.0, self.response.sigma_pos_mm / pitch))
            fraction = float(np.clip(fraction, 0.0, 1.0))

        hits: List[tuple] = []
        if 0 <= primary < plane.num_strips and (1.0 - fraction) > 1e-6:
            hits.append((int(primary), float(1.0 - fraction)))
        neighbour = primary + 1
        if fraction > 1e-6:
            if neighbour < plane.num_strips:
                hits.append((int(neighbour), float(fraction)))
            else:
                # Charge escaping the last strip still has to go somewhere; keep
                # it on the strip so total light is conserved.
                hits.append((int(primary), float(fraction)))
        if not hits:
            hits.append((int(primary), 1.0))
        return hits

    def _layer_hits(
        self,
        point_mm: np.ndarray,
        direction: np.ndarray,
        planes: Sequence[StripPlane],
        t_ref_ps: float,
    ) -> Optional[List[LayerHit]]:
        hits: List[LayerHit] = []
        for plane in planes:
            if self.rng.uniform() > self.response.plane_efficiency:
                return None
            crossing = _project_to_layer(point_mm, direction, plane.z_mm)
            true_coord = float(crossing[0] if plane.orientation == "X" else crossing[1])
            splits = self._smear(plane, true_coord)
            if splits is None:
                return None

            # One timestamp per scintillation event; both strips see the same
            # light, so they share it rather than being independently jittered.
            time_ps = t_ref_ps + self.rng.normal(0.0, self.response.tdc_jitter_ps)
            time_ps = float(np.round(time_ps / self.response.tdc_bin_ps) * self.response.tdc_bin_ps)

            # Charge-weighted centroid of the fired strip centres. This is what
            # the tracker will recompute from the wire record; computing it here
            # too keeps the simulator's own view of its measurement honest.
            half = plane.active_width_mm / 2.0
            num = sum(charge * ((strip + 0.5) * plane.strip_width_mm - half)
                      for strip, charge in splits)
            den = sum(charge for _strip, charge in splits)
            centroid = float(num / den) if den > 0 else true_coord

            for strip_index, charge in splits:
                hits.append(LayerHit(
                    plane_id=plane.plane_id,
                    orientation=plane.orientation,
                    z_mm=plane.z_mm,
                    true_coord_mm=true_coord,
                    measured_coord_mm=centroid,
                    strip_index=int(strip_index),
                    charge_fraction=float(charge),
                    time_ps=time_ps,
                ))
        return hits

    def _plane_centroid(self, hits: Sequence[LayerHit]) -> Optional[float]:
        """Charge-weighted position within one plane, from its strip hits.

        Mirrors what the tracker does with a real wire record: a muon crossing a
        triangular strip deposits light in two neighbouring channels, and the
        charge-weighted mean of their centres recovers the crossing point.
        """
        if not hits:
            return None
        plane = self._plane_lookup.get(hits[0].plane_id)
        if plane is None:
            return None
        half = plane.active_width_mm / 2.0
        charges = np.array([max(h.charge_fraction, 0.0) for h in hits], dtype=np.float64)
        if charges.sum() <= 0.0:
            return None
        centres = np.array([
            (h.strip_index + 0.5) * plane.strip_width_mm - half for h in hits
        ], dtype=np.float64)
        return float(np.sum(charges * centres) / charges.sum())

    def _direction_from_layer_hits(self, hits: Sequence[LayerHit]) -> Optional[np.ndarray]:
        """Fit a direction from the per-plane centroids of one station.

        Each coordinate is fitted independently to the line ``coord(z) = a + b z``
        through its two measured planes, which is the standard treatment for
        crossed-strip stations. Grouping by plane first is required: a single
        plane reports several strip channels, and a fit that treated each channel
        as a measurement would be fitting the charge split rather than the track.
        """
        by_plane: Dict[str, List[LayerHit]] = {}
        for hit in hits:
            by_plane.setdefault(hit.plane_id, []).append(hit)

        by_orient: Dict[str, List[tuple]] = {"X": [], "Y": []}
        for plane_id, plane_hits in by_plane.items():
            plane = self._plane_lookup.get(plane_id)
            if plane is None:
                return None
            centroid = self._plane_centroid(plane_hits)
            if centroid is None:
                return None
            by_orient[plane.orientation].append((plane.z_mm, centroid))

        slopes: Dict[str, float] = {}
        for orientation in ("X", "Y"):
            samples = by_orient[orientation]
            if len(samples) != 2:
                return None
            (z1, c1), (z2, c2) = sorted(samples)
            dz = z2 - z1
            if abs(dz) < 1e-9:
                return None
            slopes[orientation] = (c2 - c1) / dz

        direction = np.array([slopes["X"], slopes["Y"], 1.0], dtype=np.float64)
        norm = float(np.linalg.norm(direction))
        if norm < 1e-12:
            return None
        direction /= norm
        if direction[2] > 0.0:
            direction = -direction
        return direction

    # -- public API -----------------------------------------------------------

    def measure(self, event: MuonTrackEvent) -> Optional[MeasuredTrack]:
        """Simulate the detector readout for one truth track.

        Returns ``None`` when the event fails the four-layer coincidence.
        """
        top_hits = self._layer_hits(
            np.asarray(event.p_in, dtype=np.float64),
            np.asarray(event.d_in, dtype=np.float64),
            self._stations["top"]["X"] + self._stations["top"]["Y"],
            t_ref_ps=0.0,
        )
        if top_hits is None:
            return None

        bottom_hits = self._layer_hits(
            np.asarray(event.p_out, dtype=np.float64),
            np.asarray(event.d_out, dtype=np.float64),
            self._stations["bottom"]["X"] + self._stations["bottom"]["Y"],
            t_ref_ps=0.0,
        )
        if bottom_hits is None:
            return None

        d_in = self._direction_from_layer_hits(top_hits)
        d_out = self._direction_from_layer_hits(bottom_hits)
        if d_in is None or d_out is None:
            return None

        cos_theta = float(np.clip(np.dot(d_in, d_out), -1.0, 1.0))
        theta = float(np.arccos(cos_theta))

        # Projected angles in the two transverse planes, which is what a
        # strip tracker measures most robustly.
        theta_x = float(np.arctan2(d_out[0], -d_out[2]) - np.arctan2(d_in[0], -d_in[2]))
        theta_y = float(np.arctan2(d_out[1], -d_out[2]) - np.arctan2(d_in[1], -d_in[2]))

        lever = 0.5 * (self._lever["top"]["X"] + self._lever["bottom"]["X"])
        sigma_theta = self.response.scattering_sigma_rad(lever)
        offset = self.response.theta_sq_offset_rad2(lever)

        position_in = np.array([0.0, 0.0, 0.0])
        position_out = np.array([0.0, 0.0, 0.0])
        for orientation, index in (("X", 0), ("Y", 1)):
            position_in[index] = float(
                _project_to_layer(np.asarray(event.p_in, dtype=np.float64),
                                  np.asarray(event.d_in, dtype=np.float64),
                                  self.geometry.top.outer_z_mm())[index])
            position_out[index] = float(
                _project_to_layer(np.asarray(event.p_out, dtype=np.float64),
                                  np.asarray(event.d_out, dtype=np.float64),
                                  -self.geometry.bottom.outer_z_mm())[index])
        position_in[2] = self.geometry.top.outer_z_mm()
        position_out[2] = -self.geometry.bottom.outer_z_mm()

        times = [h.time_ps for h in top_hits] + [h.time_ps for h in bottom_hits]
        return MeasuredTrack(
            event_id=int(event.event_id),
            detected=True,
            position_in_mm=position_in,
            position_out_mm=position_out,
            d_in_measured=d_in,
            d_out_measured=d_out,
            theta_measured_rad=theta,
            theta_measured_x_rad=theta_x,
            theta_measured_y_rad=theta_y,
            sigma_theta_rad=sigma_theta,
            theta_sq_offset_rad2=offset,
            time_top_ps=float(np.mean([h.time_ps for h in top_hits])),
            time_bottom_ps=float(np.mean([h.time_ps for h in bottom_hits])),
            n_layers_hit=len(top_hits) + len(bottom_hits),
            layers=list(top_hits) + list(bottom_hits),
        )

    def measure_batch(self, events: Sequence[MuonTrackEvent]) -> List[MeasuredTrack]:
        out: List[MeasuredTrack] = []
        for event in events:
            measured = self.measure(event)
            if measured is not None:
                out.append(measured)
        return out

    def predicted_angular_resolution_rad(self) -> float:
        """Analytic scattering-angle resolution from geometry and response.

        Reported alongside the measured resolution so the two can be compared;
        the documentation's 2.1 mrad claim is checkable against this number.
        """
        lever = 0.5 * (self._lever["top"]["X"] + self._lever["bottom"]["X"])
        return self.response.scattering_sigma_rad(lever)
