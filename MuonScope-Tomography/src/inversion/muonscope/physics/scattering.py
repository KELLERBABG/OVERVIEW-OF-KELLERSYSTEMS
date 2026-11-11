from dataclasses import dataclass
from typing import Tuple
import numpy as np

MUON_MASS_MEV = 105.6583755

@dataclass(frozen=True)
class Material:
    name: str
    effective_z: float
    effective_a: float
    density_g_cm3: float
    radiation_length_cm: float
    dedx_mev_per_cm: float
    capture_lifetime_ns: float = 2197.0

def highland_rms_angle(
    momentum_mev: float,
    path_length_cm: float,
    radiation_length_cm: float,
    charge: int = 1,
) -> float:
    if path_length_cm <= 1e-9 or radiation_length_cm <= 1e-9:
        return 0.0

    energy_mev = np.sqrt(momentum_mev**2 + MUON_MASS_MEV**2)
    beta = momentum_mev / energy_mev
    x_over_x0 = max(path_length_cm / radiation_length_cm, 1e-12)
    log_term = max(1.0 + 0.038 * np.log(x_over_x0), 0.0)

    return float((13.6 / (beta * momentum_mev)) * abs(charge) * np.sqrt(x_over_x0) * log_term)

def sample_moliere_scattering(
    momentum_mev: float,
    path_length_cm: float,
    radiation_length_cm: float,
    rng: np.random.Generator = None,
) -> Tuple[float, float, float, float]:
    if rng is None:
        rng = np.random.default_rng()

    theta_0 = highland_rms_angle(momentum_mev, path_length_cm, radiation_length_cm)
    if theta_0 <= 1e-12:
        return 0.0, 0.0, 0.0, 0.0

    x = path_length_cm
    var_theta = theta_0 ** 2
    cov_matrix = np.array([
        [var_theta, (x / 2.0) * var_theta],
        [(x / 2.0) * var_theta, (x ** 2 / 3.0) * var_theta],
    ])

    tx, dx = rng.multivariate_normal([0.0, 0.0], cov_matrix)
    ty, dy = rng.multivariate_normal([0.0, 0.0], cov_matrix)

    return float(tx), float(ty), float(dx), float(dy)
