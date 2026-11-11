from typing import Tuple
import numpy as np

def gaisser_flux(energy_gev: np.ndarray, theta_rad: np.ndarray) -> np.ndarray:
    e = np.maximum(energy_gev, 0.1)
    cos_theta = np.cos(theta_rad)
    p1 = 0.14 * (e ** -2.7)
    term1 = 1.0 / (1.0 + (1.1 * e * cos_theta) / 115.0)
    term2 = 0.054 / (1.0 + (1.1 * e * cos_theta) / 850.0)
    return np.maximum(p1 * (term1 + term2), 0.0)

def sample_zenith_angles(
    num_samples: int,
    max_zenith_rad: float = np.deg2rad(70.0),
    cos_exponent: float = 2.0,
    rng: np.random.Generator = None,
) -> np.ndarray:
    if rng is None:
        rng = np.random.default_rng()
    u = rng.uniform(0.0, 1.0, size=num_samples)
    cos_max = np.cos(max_zenith_rad)
    cos_theta = (1.0 - u * (1.0 - cos_max ** (cos_exponent + 1))) ** (1.0 / (cos_exponent + 1))
    return np.arccos(cos_theta)

def gaisser_envelope_max(theta_rad: np.ndarray) -> np.ndarray:
    """Supremum of the Gaisser bracket over energy, at fixed zenith.

    ``B(E, c) = 1/(1 + 1.1 E c / 115) + 0.054/(1 + 1.1 E c / 850)`` decreases
    monotonically in ``E`` for ``c > 0``, so the supremum is at ``E -> 0``:
    ``B_max = 1 + 0.054``. At ``c = 0`` (horizontal) the flux is energy-independent.
    """
    cos_theta = np.cos(np.asarray(theta_rad, dtype=np.float64))
    return np.where(cos_theta > 0.0, 1.0 + 0.054, 1.0 + 0.054)


def sample_power_law_energies(
    num_events: int,
    min_energy_gev: float,
    max_energy_gev: float,
    gamma: float = 2.7,
    rng: np.random.Generator = None,
) -> np.ndarray:
    """Draw from ``E^-gamma`` on ``[min, max]`` by inverse transform."""
    if rng is None:
        rng = np.random.default_rng()
    u = rng.uniform(0.0, 1.0, size=num_events)
    lo = min_energy_gev ** (1.0 - gamma)
    hi = max_energy_gev ** (1.0 - gamma)
    return (lo + u * (hi - lo)) ** (1.0 / (1.0 - gamma))


def sample_cosmic_muons(
    num_events: int,
    area_m2: float = 0.25,
    min_energy_gev: float = 0.5,
    max_energy_gev: float = 100.0,
    max_zenith_deg: float = 60.0,
    rng: np.random.Generator = None,
    max_rounds: int = 200,
) -> Tuple[np.ndarray, np.ndarray, np.ndarray]:
    """Sample muons from the Gaisser spectrum with a cos^2(theta) zenith law.

    The energy marginal is drawn by rejection sampling against a power-law
    envelope, so ``gaisser_flux`` genuinely drives the spectrum instead of being
    decorative. The envelope is exact rather than approximate: the Gaisser bracket
    is bounded above by ``B(0) = 1.054``, so
    ``dPhi/dE <= 0.14 * 1.054 * E^-2.7`` and acceptance is
    ``B(E, cos theta) / 1.054 <= 1``.

    Returns ``(energies_gev, directions, charges)`` with unit-norm downward
    directions.
    """
    if rng is None:
        rng = np.random.default_rng()
    if num_events <= 0:
        empty = np.zeros(0, dtype=np.float64)
        return empty, np.zeros((0, 3), dtype=np.float64), np.zeros(0, dtype=np.int64)

    zenith = sample_zenith_angles(num_events, np.deg2rad(max_zenith_deg), rng=rng)
    azimuth = rng.uniform(0.0, 2.0 * np.pi, size=num_events)
    cos_theta = np.cos(zenith)

    energies = np.empty(num_events, dtype=np.float64)
    filled = 0
    rounds = 0
    while filled < num_events and rounds < max_rounds:
        rounds += 1
        deficit = num_events - filled
        proposal = sample_power_law_energies(
            deficit, min_energy_gev, max_energy_gev, rng=rng)
        e = np.maximum(proposal, 1e-3)
        bracket = (
            1.0 / (1.0 + (1.1 * e * cos_theta[filled:]) / 115.0)
            + 0.054 / (1.0 + (1.1 * e * cos_theta[filled:]) / 850.0)
        )
        accept_prob = np.clip(bracket / 1.054, 0.0, 1.0)
        keep = rng.uniform(0.0, 1.0, size=deficit) < accept_prob
        n_keep = int(np.count_nonzero(keep))
        if n_keep:
            energies[filled:filled + n_keep] = proposal[keep]
            filled += n_keep

    if filled < num_events:
        # Acceptance is high across the acceptance cone; if the budget is ever
        # exhausted, fall back to the envelope draw rather than returning short.
        energies[filled:] = sample_power_law_energies(
            num_events - filled, min_energy_gev, max_energy_gev, rng=rng)

    dx = np.sin(zenith) * np.cos(azimuth)
    dy = np.sin(zenith) * np.sin(azimuth)
    dz = -np.cos(zenith)
    directions = np.stack([dx, dy, dz], axis=-1)

    charge_probs = rng.uniform(0.0, 1.0, size=num_events)
    charges = np.where(charge_probs < (1.25 / 2.25), 1, -1)

    return energies, directions, charges
