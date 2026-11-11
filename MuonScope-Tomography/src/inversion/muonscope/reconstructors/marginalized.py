"""Reconstruction with the muon momentum marginalised, not supplied.

The rest of the package used to be handed each event's true momentum by the
simulator. That is an oracle: a two-station hodoscope measures directions only, so
the momentum enters the reconstruction as an *unknown* with a known prior (the
cosmic-ray spectrum), and the honest treatment is to marginalise over it.

Forward model
-------------
For a muon in momentum class ``s`` traversing the voxel grid,

    E[theta^2 | s] = K_s * (W lambda) + b ,   K_s = (13.6 / (beta_s p_s))^2

with ``W`` the Siddon path-length matrix in cm, ``lambda`` the scattering density
in cm^-1, and ``b`` the additive resolution floor of the detector (see
:class:`muonscope.simulation.detector.DetectorResponse`). Averaging over the prior
``w_s`` gives a model with both a mean and a variance:

    E[y]      = <K> (W lambda) + b ,        <K>   = sum_s w_s K_s
    Var[y|lam] = Var[K] (W lambda)^2 + v_det

The variance term is not decoration. Across a 0.5-100 GeV spectrum ``K`` varies by
orders of magnitude, so an event's ``theta^2`` is far more uncertain than the
detector alone implies, and a reconstruction that ignores it over-weights the
high-momentum tail.

Both estimators below use only observables -- measured angles, their per-event
variances, the geometry, and the prior. Nothing per-event is taken from truth.
"""
from dataclasses import dataclass
from typing import Dict, Optional, Tuple

import numpy as np
import scipy.sparse as sp

from ..physics.momentum_prior import MomentumPrior, default_momentum_prior


@dataclass
class MarginalizedInfo:
    """Diagnostics the caller should surface rather than swallow."""

    rays: int
    voxels: int
    matrix_nonzeros: int
    mean_k: float
    var_k: float
    offset_applied: float
    fraction_negative_excess: float
    voxels_with_path: int
    iterations: int
    negative_numerator_floor_hits: int

    def as_dict(self) -> Dict[str, float]:
        return {
            "rays": self.rays,
            "voxels": self.voxels,
            "matrix_nonzeros": self.matrix_nonzeros,
            "mean_highland_factor": self.mean_k,
            "var_highland_factor": self.var_k,
            "theta_sq_offset_applied": self.offset_applied,
            "fraction_events_below_offset": self.fraction_negative_excess,
            "voxels_with_path": self.voxels_with_path,
            "iterations": self.iterations,
            "negative_numerator_floor_hits": self.negative_numerator_floor_hits,
        }


def prior_moments(prior: MomentumPrior) -> Tuple[float, float]:
    """Return ``(<K>, Var[K])`` for the prior."""
    k = prior.highland_factor()
    w = prior.weights
    mean_k = float(np.sum(w * k))
    var_k = float(np.sum(w * k * k) - mean_k ** 2)
    return mean_k, max(var_k, 0.0)


# The largest scattering density any material has: uranium, 1/X0 = 1/0.32 cm.
# Nothing physical exceeds this, so it is a hard bound rather than a tuning knob.
MAX_PHYSICAL_LAMBDA_CM = 1.0 / 0.32


def _detector_variance(theta_sq: np.ndarray, sigma_theta_rad: float) -> np.ndarray:
    """Per-event variance of the measured ``theta^2`` from angular smearing."""
    s2 = float(sigma_theta_rad) ** 2
    return 4.0 * np.asarray(theta_sq, dtype=np.float64) * s2 + 2.0 * s2 * s2


def marginalized_backprojection(
    system_matrix_csr: sp.csr_matrix,
    theta_sq_rad2: np.ndarray,
    sigma_theta_rad: float,
    prior: Optional[MomentumPrior] = None,
    offset_rad2: float = 0.0,
    min_path_sum: float = 1e-9,
    max_lambda_cm: float = MAX_PHYSICAL_LAMBDA_CM,
) -> Tuple[np.ndarray, MarginalizedInfo]:
    """One-shot prior-marginalised estimate of ``lambda``, in cm^-1.

    This is the exact least-squares gradient step from zero for the
    prior-averaged model, i.e. the same fixed point the iterative solver moves
    toward, so it makes a fair and cheap baseline.

    The result is clamped at ``max_lambda_cm``. The estimator divides by the
    squared chord length through each voxel, so a voxel grazed by a single short
    chord can come out arbitrarily large -- at low statistics that dominated
    everything downstream. The clamp is the physical maximum (uranium), not a
    tuned regulariser, so it removes impossible values without suppressing real
    contrast. Pass ``max_lambda_cm=inf`` to disable it.
    """
    prior = prior or default_momentum_prior()
    mean_k, var_k = prior_moments(prior)

    W = system_matrix_csr.tocsr()
    y = np.asarray(theta_sq_rad2, dtype=np.float64)
    excess = y - float(offset_rad2)

    numerator = np.asarray(W.T.dot(excess)).ravel()
    denominator = np.asarray(W.multiply(W).sum(axis=0)).ravel() * mean_k

    usable = denominator > min_path_sum
    lam = np.zeros_like(numerator, dtype=np.float64)
    np.divide(numerator, denominator, out=lam, where=usable)
    negative = int(np.count_nonzero(lam < 0.0))
    lam = np.clip(lam, 0.0, max_lambda_cm)

    info = MarginalizedInfo(
        rays=int(W.shape[0]),
        voxels=int(W.shape[1]),
        matrix_nonzeros=int(W.nnz),
        mean_k=mean_k,
        var_k=var_k,
        offset_applied=float(offset_rad2),
        fraction_negative_excess=float(np.mean(excess < 0.0)) if excess.size else 0.0,
        voxels_with_path=int(np.count_nonzero(usable)),
        iterations=0,
        negative_numerator_floor_hits=negative,
    )
    return lam, info


class MarginalizedMLEM:
    """Iteratively reweighted non-negative solver with momentum marginalised.

    Each iteration recomputes the per-event variance from the current estimate
    (the unknown-momentum term grows with the predicted signal) and takes a
    multiplicative step. The multiplicative form keeps ``lambda >= 0`` without a
    projection step, and a ratio is floored at a small epsilon so that a voxel
    whose rays all scattered *less* than the detector floor is driven to zero
    rather than negative.
    """

    def __init__(
        self,
        prior: Optional[MomentumPrior] = None,
        iterations: int = 30,
        beta: float = 0.0,
        huber_delta: float = 0.01,
        ratio_floor: float = 1e-12,
        max_lambda_cm: float = MAX_PHYSICAL_LAMBDA_CM,
    ):
        self.prior = prior or default_momentum_prior()
        self.iterations = int(iterations)
        self.beta = float(beta)
        self.huber_delta = float(huber_delta)
        self.ratio_floor = float(ratio_floor)
        self.max_lambda_cm = float(max_lambda_cm)

    def reconstruct(
        self,
        system_matrix_csr: sp.csr_matrix,
        theta_sq_rad2: np.ndarray,
        sigma_theta_rad: float,
        offset_rad2: float = 0.0,
        initial_density: Optional[np.ndarray] = None,
        grid_size: Optional[Tuple[int, int, int]] = None,
    ) -> Tuple[np.ndarray, MarginalizedInfo]:
        prior = self.prior
        mean_k, var_k = prior_moments(prior)

        W = system_matrix_csr.tocsr()
        n_rays, n_voxels = W.shape
        y = np.asarray(theta_sq_rad2, dtype=np.float64)
        excess = y - float(offset_rad2)
        det_var = _detector_variance(y, sigma_theta_rad)

        if initial_density is None:
            lam = np.full(n_voxels, 0.0865, dtype=np.float64)
        else:
            lam = np.clip(np.asarray(initial_density, dtype=np.float64).ravel(),
                          1e-12, self.max_lambda_cm)

        path_sum = np.asarray(
            W.T.dot(np.ones(n_rays, dtype=np.float64))).ravel()
        reachable = path_sum > 1e-9

        floor_hits = 0
        for _ in range(self.iterations):
            # wlam is (W lambda), the path-length-weighted density, dimensionless.
            # mu = <K> * wlam is the predicted signal. The normal equation for the
            # model E[y - b] = <K> (W lambda) is
            #     <K>   * W^T U (y - b)  =  <K>^2 * W^T U (W lambda)
            # so the denominator carries <K>^2 -- written against wlam, not
            # against mu. Using mu there puts a third factor of <K> in the
            # denominator and inflates every update by 1/<K>, which at
            # <K> ~ 3.6e-4 drives the whole volume to its ceiling in one step.
            wlam = np.asarray(W.dot(lam)).ravel()
            mu = mean_k * wlam
            variance = np.maximum(det_var + var_k * mu * mu, 1e-30)
            inv_var = 1.0 / variance

            numerator = mean_k * np.asarray(W.T.dot(inv_var * excess)).ravel()
            denominator = mean_k * mean_k * np.asarray(
                W.T.dot(inv_var * wlam)).ravel()

            if self.beta > 0.0 and grid_size is not None:
                prior_grad = self._huber_gradient(
                    lam.reshape(grid_size), self.huber_delta).ravel()
                denominator = denominator - self.beta * prior_grad

            floor_hits += int(np.count_nonzero((numerator <= 0.0) & reachable))
            ratio = np.full(n_voxels, self.ratio_floor)
            np.divide(numerator, np.maximum(denominator, 1e-30), out=ratio,
                      where=reachable & (denominator > 1e-30))
            ratio = np.maximum(ratio, self.ratio_floor)
            lam = np.clip(lam * ratio, 0.0, self.max_lambda_cm)

        info = MarginalizedInfo(
            rays=int(n_rays),
            voxels=int(n_voxels),
            matrix_nonzeros=int(W.nnz),
            mean_k=mean_k,
            var_k=var_k,
            offset_applied=float(offset_rad2),
            fraction_negative_excess=float(np.mean(excess < 0.0)) if excess.size else 0.0,
            voxels_with_path=int(np.count_nonzero(reachable)),
            iterations=self.iterations,
            negative_numerator_floor_hits=int(floor_hits),
        )
        if grid_size is not None:
            lam = lam.reshape(grid_size)
        return lam, info

    @staticmethod
    def _huber_gradient(image: np.ndarray, delta: float) -> np.ndarray:
        """Gradient of the Huber total-variation prior (periodic boundaries)."""
        grad = np.zeros_like(image)
        for dim in (0, 1, 2):
            fwd = np.roll(image, shift=-1, axis=dim) - image
            bwd = image - np.roll(image, shift=1, axis=dim)
            grad += (np.where(np.abs(fwd) <= delta, fwd, np.sign(fwd))
                     - np.where(np.abs(bwd) <= delta, bwd, np.sign(bwd)))
        return grad


def nominal_momentum_baseline(
    system_matrix_csr: sp.csr_matrix,
    theta_sq_rad2: np.ndarray,
    nominal_momentum_mev: float,
    beta: float = 1.0,
    offset_rad2: float = 0.0,
) -> np.ndarray:
    """Reference solve using one nominal momentum for every event.

    Kept because it is the honest comparison for the marginalised estimator: it
    is what the architecture gives you if you refuse to model the spectrum, and
    it costs a ``1/p^2`` error of up to ~10^4 across a 0.5-100 GeV beam.
    """
    from .normalized import momentum_factor

    W = system_matrix_csr.tocsr()
    y = np.asarray(theta_sq_rad2, dtype=np.float64) - float(offset_rad2)
    k = float(momentum_factor(np.array([nominal_momentum_mev]), beta=beta)[0])

    numerator = np.asarray(W.T.dot(y / k)).ravel()
    denominator = np.asarray(W.multiply(W).sum(axis=0)).ravel()
    lam = np.zeros_like(numerator)
    usable = denominator > 1e-9
    np.divide(numerator, denominator, out=lam, where=usable)
    return np.maximum(lam, 0.0)
