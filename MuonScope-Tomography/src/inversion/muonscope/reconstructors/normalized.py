"""Path-normalised scattering-density reconstruction.

PoCA attributes a muon's entire-path scattering to a single voxel, so its per-voxel
value tracks how much material the muon crossed rather than what sits in the voxel.
That is why its CNR is near zero and why no scaling constant can repair it.

The forward model is

    theta_i^2 = K_i * sum_v W[i,v] * lambda_v ,   K_i = (13.6 / (beta p_i))^2

with W the Siddon path-length matrix in cm and lambda in cm^-1. The least-squares
normal equation gives a closed-form estimate:

    lambda_v = sum_i W[i,v] * y_i / sum_i W[i,v]^2 ,   y_i = theta_i^2 / K_i

which is one exact gradient step from zero and the same fixed point MAP-EM iterates
toward. It therefore makes a fair baseline to measure MAP-EM against.
"""
from typing import Optional, Tuple

import numpy as np
import scipy.sparse as sp


def momentum_factor(momentum_mev, beta: float = 1.0) -> np.ndarray:
    """K_i = (13.6 / (beta p))^2, the Highland prefactor, p in MeV."""
    p = np.maximum(np.asarray(momentum_mev, dtype=np.float64), 1e-6)
    return (13.6 / (beta * p)) ** 2


def normalized_backprojection(
    system_matrix_csr: sp.csr_matrix,
    theta2_rad2: np.ndarray,
    momentum_mev,
    grid_size: Optional[Tuple[int, int, int]] = None,
    min_path_sum: float = 1e-9,
):
    """One-shot path-normalised estimate of lambda, in cm^-1.

    Returns (lambda, info). Voxels no ray passes through are left at zero rather
    than divided by an epsilon, and the count is reported so an empty volume is
    never mistaken for a good one.
    """
    W = system_matrix_csr.tocsr()
    y = np.asarray(theta2_rad2, dtype=np.float64) / momentum_factor(momentum_mev)

    numerator = np.asarray(W.T.dot(y)).ravel()
    denominator = np.asarray(W.multiply(W).sum(axis=0)).ravel()

    usable = denominator > min_path_sum
    lam = np.zeros_like(numerator, dtype=np.float64)
    np.divide(numerator, denominator, out=lam, where=usable)

    info = {
        "rays": int(W.shape[0]),
        "voxels": int(W.shape[1]),
        "matrix_nonzeros": int(W.nnz),
        "voxels_with_path": int(np.count_nonzero(usable)),
        "fraction_voxels_with_path": float(np.count_nonzero(usable)) / float(W.shape[1]),
        "median_path_squared_sum": float(np.median(denominator[usable])) if np.any(usable) else 0.0,
        "momentum_mev_used": float(np.mean(np.asarray(momentum_mev, dtype=np.float64))),
    }
    if grid_size is not None:
        lam = lam.reshape(grid_size)
    return lam, info
