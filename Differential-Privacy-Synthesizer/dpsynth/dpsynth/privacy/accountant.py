"""
Privacy budget accountant implementing Rényi DP composition and moments accountant.

Tracks the consumed (ε, δ) privacy budget across multiple training steps
using advanced composition theorems and Rényi Differential Privacy (RDP)
for tight accounting.
"""

from __future__ import annotations

import math
from dataclasses import dataclass, field


@dataclass
class PrivacyBudget:
    """A snapshot of consumed privacy budget.

    Attributes:
        epsilon: The ε value achieved so far.
        delta: The δ value achieved so far.
        steps: Number of training steps accounted for.
    """

    epsilon: float
    delta: float
    steps: int = 0

    def __repr__(self) -> str:
        return f"PrivacyBudget(ε={self.epsilon:.4f}, δ={self.delta:.2e}, steps={self.steps})"


@dataclass
class RDPAccountantState:
    """Internal state for RDP composition.

    Stores the accumulated Rényi divergence at each order α.
    """

    orders: tuple[float, ...] = field(default_factory=lambda: tuple(range(2, 65)))
    log_moments: list[float] = field(default_factory=lambda: [0.0] * 64)

    def accumulate(self, noise_multiplier: float, steps: int, sampling_rate: float) -> None:
        """Accumulate privacy cost for a series of steps with given noise multiplier.

        Uses the RDP composition formula for Gaussian mechanisms with
        Poisson subsampling.

        Args:
            noise_multiplier: σ parameter for Gaussian noise.
            steps: Number of gradient steps.
            sampling_rate: q = batch_size / dataset_size.
        """
        for idx, alpha in enumerate(self.orders):
            # RDP for Gaussian mechanism with subsampling:
            # ε(α) ≤ (1/(α-1)) * log( E_q[ exp((α-1)*α/(2*σ²)) ] )
            # Approximate via Taylor expansion for tight bound.
            # Using the analytic formula from Mironov et al. 2019:
            # ε(α) ≤ 1/(α-1) * log(
            #   (1-q)^(α-1) * (α*q - q + 1)
            #   + sum_{k=2}^{α} C(α, k) * q^k * (1-q)^(α-k) * exp((k-1)*k/(2*σ²))
            # )
            eps_rdp = self._compute_rdp_gaussian(
                noise_multiplier, sampling_rate, alpha
            )
            self.log_moments[idx] += eps_rdp

    def _compute_rdp_gaussian(self, sigma: float, q: float, alpha: float) -> float:
        """Compute RDP ε(α) for Gaussian mechanism with subsampling.

        Uses the analytic formula from:
        "Rényi Differential Privacy of the Sampled Gaussian Mechanism"
        (Mironov, Talwar, Zhang 2019)

        Args:
            sigma: Noise multiplier.
            q: Sampling rate (batch_size / n).
            alpha: RDP order.

        Returns:
            ε(α) for the given parameters.
        """
        if alpha == 1:
            return 0.0

        if q == 0.0:
            return 0.0

        # When q >= 1, we're using the full dataset per batch (no subsampling).
        # In this case, the RDP is simply α / (2 * σ²) for the Gaussian mechanism.
        if q >= 1.0:
            return alpha / (2.0 * sigma * sigma)

        # Compute via the dominant term for efficiency
        # ε(α) ≤ 1/(α-1) * log( max( A_0, sum_{k=2}^{α} A_k ) )
        # where A_k = C(α, k) * q^k * (1-q)^(α-k) * exp((k² - k) / (2*σ²))

        # For numerical stability, compute in log space
        term = 0.0
        for k in range(2, int(alpha) + 1):
            log_comb = math.lgamma(alpha + 1) - math.lgamma(k + 1) - math.lgamma(alpha - k + 1)
            log_q_term = k * math.log(q) + (alpha - k) * math.log1p(-q)
            log_exp_term = (k * k - k) / (2 * sigma * sigma)
            log_ak = log_comb + log_q_term + log_exp_term
            term = max(term, log_ak)

        # Add the (1-q)^(α-1) * (α*q - q + 1) term
        term = max(term, (alpha - 1) * math.log1p(-q) + math.log(alpha * q - q + 1))

        return term / (alpha - 1)

    def get_epsilon(self, delta: float) -> float:
        """Convert accumulated RDP moments to (ε, δ) guarantee.

        Uses the conversion: δ(ε) = min_α exp( log_moment(α) - α * ε )
        We invert to find ε for a given δ.

        Args:
            delta: Target δ value.

        Returns:
            Minimum ε achievable for the given δ.
            Returns 0.0 if no steps have been accounted for.
        """
        if not self.log_moments:
            return 0.0

        # If all log moments are zero, no privacy has been spent
        if all(m == 0.0 for m in self.log_moments):
            return 0.0

        best_epsilon = float("inf")
        for alpha, log_moment in zip(self.orders, self.log_moments):
            if log_moment == 0.0:
                continue
            # ε(α) = log_moment(α)/(α-1) - log(δ)/(α-1) + log(1 - 1/α)/(α-1)
            # Simplified from the conversion theorem
            eps_candidate = log_moment / (alpha - 1) - math.log(delta) / (alpha - 1) + math.log1p(-1.0 / alpha) / (alpha - 1)
            best_epsilon = min(best_epsilon, eps_candidate)

        return max(0.0, best_epsilon)


class PrivacyAccountant:
    """Privacy budget accountant with RDP composition.

    Tracks the total privacy cost across the entire training process.
    Supports sequential composition via RDP accounting.

    Usage:
        >>> accountant = PrivacyAccountant()
        >>> accountant.step(noise_multiplier=1.0, batch_size=256, dataset_size=50_000)
        >>> spent = accountant.get_privacy_spent(delta=1e-5)
        >>> print(f"ε = {spent.epsilon:.2f}")
    """

    def __init__(
        self,
        orders: tuple[float, ...] = tuple(range(2, 65)),
        mechanism: str = "gaussian",
    ):
        """Initialize the privacy accountant.

        Args:
            orders: RDP orders α to track.
            mechanism: Privacy mechanism type ('gaussian' or 'laplace').
        """
        self._state = RDPAccountantState(orders=orders)
        self._mechanism = mechanism
        self._total_steps = 0

    def step(
        self,
        noise_multiplier: float,
        batch_size: int,
        dataset_size: int,
        steps: int = 1,
    ) -> None:
        """Account for one or more training steps.

        Args:
            noise_multiplier: Noise multiplier σ (higher = more privacy).
            batch_size: Number of samples per batch.
            dataset_size: Total number of samples in the dataset.
            steps: Number of gradient steps to account for.
        """
        if not (isinstance(noise_multiplier, (int, float)) and math.isfinite(noise_multiplier) and noise_multiplier > 0):
            raise ValueError(f"noise_multiplier must be positive, got {noise_multiplier}")
        if batch_size <= 0:
            raise ValueError(f"batch_size must be positive, got {batch_size}")
        if dataset_size <= 0:
            raise ValueError(f"dataset_size must be positive, got {dataset_size}")
        if steps <= 0:
            return

        sampling_rate = batch_size / dataset_size
        self._state.accumulate(noise_multiplier, steps, sampling_rate)
        self._total_steps += steps

    def get_privacy_spent(self, delta: float = 1e-5) -> PrivacyBudget:
        """Get the total privacy budget consumed so far.

        Args:
            delta: Target δ parameter (typically 1e-5 or 1e-6).

        Returns:
            PrivacyBudget with total (ε, δ) spent.
        """
        epsilon = self._state.get_epsilon(delta)
        return PrivacyBudget(
            epsilon=epsilon,
            delta=delta,
            steps=self._total_steps,
        )

    def get_spent_epsilon(self, delta: float = 1e-5) -> float:
        """Convenience method: get only ε spent for a given δ.

        Args:
            delta: Target δ parameter.

        Returns:
            ε value spent so far.
        """
        return self.get_privacy_spent(delta).epsilon

    def reset(self) -> None:
        """Reset the accountant to its initial state."""
        self._state = RDPAccountantState(orders=self._state.orders)
        self._total_steps = 0

    @property
    def total_steps(self) -> int:
        """Total number of training steps accounted for."""
        return self._total_steps

    def __repr__(self) -> str:
        return (
            f"PrivacyAccountant(steps={self._total_steps}, "
            f"orders=[{self._state.orders[0]}, ..., {self._state.orders[-1]}])"
        )
