"""
DEPRECATED -- superseded by :mod:`muonscope.simulation.detector`.

This module is a second, independently written detector model that contradicts
the one the pipeline actually uses:

* it smears the hit position directly (``true + N(0, sigma)``) instead of
  splitting charge between neighbouring strips, so its output is not what a
  charge-weighted barycentre reconstruction can invert;
* it timestamps each strip separately rather than sharing one timestamp per
  scintillation event;
* it has no notion of the geometry file, so its plane positions cannot agree with
  ``config/detector_geometry.yaml``.

It was imported by nothing except the package ``__init__``, and keeping two
detector models in one package is how a simulation and its reconstruction drift
apart. Use :class:`muonscope.simulation.detector.HodoscopeSimulator` and
:class:`muonscope.simulation.geometry.load_detector_geometry` instead.

The module is retained so that an existing import does not break, but it is no
longer re-exported from :mod:`muonscope.simulation` and it emits a
``DeprecationWarning``.
"""

import warnings

from dataclasses import dataclass
from typing import Tuple, List
import numpy as np

warnings.warn(
    "muonscope.simulation.detector_sim is deprecated; use "
    "muonscope.simulation.detector.HodoscopeSimulator with the geometry from "
    "config/detector_geometry.yaml",
    DeprecationWarning,
    stacklevel=2,
)


@dataclass
class SiPMHit:
    channel_id: int
    timestamp_ps: float
    adc_pulse_height: int
    is_dark_count: bool = False


class DetectorPlaneSim:
    """Simulates a 64-channel scintillator hodoscope layer with SiPM readouts."""

    def __init__(
        self,
        plane_id: str,
        orientation: str = "X",
        num_strips: int = 64,
        strip_width_mm: float = 20.0,
        z_pos_mm: float = 300.0,
        tdc_bin_ps: float = 75.0,
        tdc_jitter_ps: float = 80.0,
        sipm_pde: float = 0.45,
        dark_count_rate_khz: float = 80.0,
    ):
        self.plane_id = plane_id
        self.orientation = orientation
        self.num_strips = num_strips
        self.strip_width_mm = strip_width_mm
        self.z_pos_mm = z_pos_mm
        self.tdc_bin_ps = tdc_bin_ps
        self.tdc_jitter_ps = tdc_jitter_ps
        self.sipm_pde = sipm_pde
        self.dark_rate_khz = dark_count_rate_khz

    def simulate_intercept(
        self,
        hit_pos_mm: np.ndarray,
        transit_time_ps: float,
        deposited_energy_mev: float = 2.0,
        rng: np.random.Generator = None,
    ) -> List[SiPMHit]:
        """
        Simulates optical photon generation in triangular strips and SiPM readout.
        """
        if rng is None:
            rng = np.random.default_rng()

        coord = hit_pos_mm[0] if self.orientation == "X" else hit_pos_mm[1]
        half_total_width = (self.num_strips * self.strip_width_mm) / 2.0
        local_coord = coord + half_total_width

        if local_coord < 0 or local_coord >= (self.num_strips * self.strip_width_mm):
            return []

        # Find primary and secondary overlapping triangular strips
        strip_idx_f = local_coord / self.strip_width_mm
        primary_strip = int(np.floor(strip_idx_f))
        fraction = strip_idx_f - primary_strip

        # Scintillation light yield: ~10,000 photons/MeV
        total_photons = rng.poisson(deposited_energy_mev * 10000.0 * 0.05) # 5% fiber capture

        # Triangular charge sharing
        q1 = int(total_photons * (1.0 - fraction) * self.sipm_pde)
        q2 = int(total_photons * fraction * self.sipm_pde)

        hits = []
        if primary_strip < self.num_strips and q1 > 5:
            # TDC timestamp with jitter and bin quantization
            jittered_t = transit_time_ps + rng.normal(0.0, self.tdc_jitter_ps)
            tdc_code = int(np.round(jittered_t / self.tdc_bin_ps))
            hits.append(SiPMHit(channel_id=primary_strip, timestamp_ps=tdc_code * self.tdc_bin_ps, adc_pulse_height=q1))

        neighbor_strip = primary_strip + 1
        if neighbor_strip < self.num_strips and q2 > 5:
            jittered_t = transit_time_ps + rng.normal(0.0, self.tdc_jitter_ps)
            tdc_code = int(np.round(jittered_t / self.tdc_bin_ps))
            hits.append(SiPMHit(channel_id=neighbor_strip, timestamp_ps=tdc_code * self.tdc_bin_ps, adc_pulse_height=q2))

        return hits


def simulate_sipm_response(
    true_hit_pos_mm: np.ndarray,
    plane_z_mm: float = 300.0,
    rng: np.random.Generator = None,
) -> Tuple[float, float]:
    """Helper returning measured (x, y) coordinates with realistic SiPM noise and barycenter error."""
    if rng is None:
        rng = np.random.default_rng()
    
    # Triangular geometry yields sigma_pos <= 0.8 mm
    sigma_pos_mm = 0.75
    noisy_x = true_hit_pos_mm[0] + rng.normal(0.0, sigma_pos_mm)
    noisy_y = true_hit_pos_mm[1] + rng.normal(0.0, sigma_pos_mm)
    return float(noisy_x), float(noisy_y)
