"""Momentum prior for a two-panel hodoscope.

Why this module exists
----------------------
A four-plane scintillator hodoscope measures *directions*. It does not measure the
muon's momentum. The Highland relation

    theta0 = (13.6 MeV / (beta p)) * sqrt(x / X0)

couples the observed scattering angle to ``1/(beta p)``, so a reconstruction that
wants scattering density ``lambda = 1/X0`` from an observed ``theta^2`` needs a
momentum. Feeding it the simulator's own ``energy_initial_gev`` is an oracle: it is
a quantity the detector cannot produce, and it makes the reconstructed contrast a
property of the simulator rather than of the instrument.

The physically honest substitute is the *known cosmic-ray momentum spectrum*: an
a-priori physical input, the same for every event, carrying no per-event
information. This module supplies it as quadrature nodes and weights so a
reconstructor can marginalise over momentum instead of being told it.

The distribution is the Gaisser sea-level flux integrated over the detector's
zenith acceptance and weighted by ``cos(theta)`` (the projected area of a
horizontal plane). Nothing here is fitted to the data being reconstructed.
"""
from dataclasses import dataclass
from typing import Optional

import numpy as np

from .gaisser import gaisser_flux

MUON_MASS_GEV = 0.1056583755


@dataclass(frozen=True)
class MomentumPrior:
    """Discrete momentum prior as quadrature nodes with normalised weights.

    Attributes
    ----------
    momentum_mev : (S,) float64
        Quadrature nodes in MeV/c, ascending.
    weights : (S,) float64
        Normalised quadrature weights, ``weights.sum() == 1``.
    """

    momentum_mev: np.ndarray
    weights: np.ndarray

    def __post_init__(self) -> None:
        if self.momentum_mev.shape != self.weights.shape:
            raise ValueError("momentum_mev and weights must have the same shape")
        if self.momentum_mev.ndim != 1:
            raise ValueError("MomentumPrior is one-dimensional")
        if np.any(self.momentum_mev <= 0.0):
            raise ValueError("momentum nodes must be strictly positive")
        total = float(self.weights.sum())
        if not np.isfinite(total) or total <= 0.0:
            raise ValueError("prior weights must have a positive finite sum")

    @property
    def size(self) -> int:
        return int(self.momentum_mev.size)

    def normalised(self) -> "MomentumPrior":
        return MomentumPrior(self.momentum_mev, self.weights / self.weights.sum())

    def energy_gev(self) -> np.ndarray:
        """Total energy of each node, GeV."""
        p = self.momentum_mev / 1000.0
        return np.sqrt(p * p + MUON_MASS_GEV * MUON_MASS_GEV)

    def beta(self) -> np.ndarray:
        """Speed factor ``beta = p / E`` at each node."""
        p_gev = self.momentum_mev / 1000.0
        return p_gev / self.energy_gev()

    def highland_factor(self) -> np.ndarray:
        """``K_s = (13.6 / (beta_s p_s))^2`` with ``p_s`` in MeV.

        This is the factor that turns scattering density into observed ``theta^2``:
        ``E[theta^2] = K_s * sum_v W[i,v] * lambda_v`` for a muon in class ``s``.
        """
        return (13.6 / (self.beta() * self.momentum_mev)) ** 2

    def mean_factor(self) -> float:
        """Prior-averaged Highland factor, ``sum_s w_s K_s``."""
        return float(np.sum(self.weights * self.highland_factor()))


def gaisser_energy_density(
    energy_gev: np.ndarray,
    max_zenith_deg: float = 60.0,
    num_zenith: int = 64,
) -> np.ndarray:
    """Detected muon energy density, up to a normalisation.

    Integrates the Gaisser differential flux over the zenith acceptance, weighted
    by ``cos(theta) sin(theta)``, which is the flux through a horizontal surface.
    """
    e = np.asarray(energy_gev, dtype=np.float64)
    theta_max = np.deg2rad(max_zenith_deg)
    theta = np.linspace(0.0, theta_max, num_zenith)
    weight = np.cos(theta) * np.sin(theta)
    # gaisser_flux broadcasts over the trailing energy axis
    flux = gaisser_flux(e[:, np.newaxis], theta[np.newaxis, :])
    density = np.trapezoid(flux * weight[np.newaxis, :], theta, axis=1)
    return np.maximum(density, 0.0)


def gaisser_momentum_prior(
    num_nodes: int = 32,
    min_energy_gev: float = 0.5,
    max_energy_gev: float = 100.0,
    max_zenith_deg: float = 60.0,
    num_zenith: int = 64,
) -> MomentumPrior:
    """Build a momentum prior from the Gaisser spectrum.

    The energy axis is discretised on a log grid so that the steep ``E^-2.7``
    fall-off is resolved at low energy without wasting nodes at high energy. Each
    bin becomes one quadrature node, placed at the bin's density-weighted mean
    energy, with weight equal to its integrated density.
    """
    if num_nodes < 2:
        raise ValueError("need at least two momentum nodes")
    if min_energy_gev <= 0.0 or max_energy_gev <= min_energy_gev:
        raise ValueError("invalid energy range")

    # A fine grid for integration, then aggregate onto num_nodes bins of equal
    # density so every node carries comparable statistical weight.
    fine_edges = np.geomspace(min_energy_gev, max_energy_gev, 512)
    fine_centres = np.sqrt(fine_edges[:-1] * fine_edges[1:])
    density = gaisser_energy_density(fine_centres, max_zenith_deg, num_zenith)
    bin_width = np.diff(fine_edges)
    bin_mass = density * bin_width

    cumulative = np.cumsum(bin_mass)
    total = cumulative[-1]
    if not np.isfinite(total) or total <= 0.0:
        raise ValueError("Gaisser density integrated to zero over the given range")

    # Split the fine grid into num_nodes groups of equal integrated density.
    targets = total * np.arange(1, num_nodes) / num_nodes
    group_index = np.searchsorted(cumulative, targets)

    nodes = np.empty(num_nodes, dtype=np.float64)
    weights = np.empty(num_nodes, dtype=np.float64)
    start = 0
    for k in range(num_nodes):
        stop = int(group_index[k]) + 1 if k < num_nodes - 1 else bin_mass.size
        stop = max(stop, start + 1)
        mass = bin_mass[start:stop]
        mass_sum = float(mass.sum())
        if mass_sum <= 0.0:
            nodes[k] = float(fine_centres[min(start, fine_centres.size - 1)])
            weights[k] = 0.0
        else:
            nodes[k] = float(np.sum(mass * fine_centres[start:stop]) / mass_sum)
            weights[k] = mass_sum
        start = stop

    # Convert total energy (GeV) to momentum (MeV/c).
    momentum_mev = 1000.0 * np.sqrt(np.maximum(nodes**2 - MUON_MASS_GEV**2, 1e-12))
    prior = MomentumPrior(momentum_mev=momentum_mev, weights=weights)
    return prior.normalised()


_DEFAULT_PRIOR: Optional[MomentumPrior] = None


def default_momentum_prior() -> MomentumPrior:
    """Cached prior over the spectrum the pipeline actually samples."""
    global _DEFAULT_PRIOR
    if _DEFAULT_PRIOR is None:
        _DEFAULT_PRIOR = gaisser_momentum_prior()
    return _DEFAULT_PRIOR
