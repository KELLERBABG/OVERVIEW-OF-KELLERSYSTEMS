"""
Exponential Mechanism for differentially private selection.

Implements the Exponential Mechanism (McSherry & Talwar, 2007) for
choosing the best synthetic representation without leaking private
information about the original data.

The Exponential Mechanism selects an output from a set of candidates
with probability proportional to exp(ε * u(data, r) / (2 * Δu))
where u is a utility function and Δu is its sensitivity.
"""

from __future__ import annotations

import logging
from collections.abc import Callable
from dataclasses import dataclass

import numpy as np

logger = logging.getLogger(__name__)


@dataclass
class ExponentialMechanismResult:
    """Result from the Exponential Mechanism.

    Attributes:
        selected_index: Index of the selected candidate.
        selected_score: Utility score of the selected candidate.
        epsilon: Privacy budget consumed.
        sensitivity: Sensitivity of the utility function.
        selection_probability: Probability with which the selection was made.
    """

    selected_index: int
    selected_score: float
    epsilon: float
    sensitivity: float
    selection_probability: float = 0.0

    def __repr__(self) -> str:
        return (
            f"ExponentialMechanismResult(selected={self.selected_index}, "
            f"score={self.selected_score:.4f}, "
            f"ε={self.epsilon}, "
            f"P={self.selection_probability:.2%})"
        )


class ExponentialMechanism:
    """The Exponential Mechanism for differentially private selection.

    Given a set of candidates and a utility function u(data, candidate),
    selects a candidate with probability ~ exp(ε * u(data, r) / (2 * Δu)).

    This provides ε-differential privacy for the selection itself,
    independent of any downstream use of the selected candidate.

    Usage:
        >>> mechanism = ExponentialMechanism(epsilon=1.0)
        >>> candidates = [np.array([...]), np.array([...])]
        >>> def utility(real_data, candidate):
        ...     return -np.mean((real_data - candidate) ** 2)
        >>> result = mechanism.select(real_data, candidates, utility)
        >>> print(f"Selected candidate {result.selected_index}")
    """

    def __init__(
        self,
        epsilon: float = 1.0,
        sensitivity: float = 1.0,
    ):
        """Initialize Exponential Mechanism.

        Args:
            epsilon: Privacy budget ε for the selection.
            sensitivity: Global sensitivity Δu of the utility function.
                Must be positive.
        """
        if epsilon <= 0:
            raise ValueError(f"epsilon must be positive, got {epsilon}")
        if sensitivity <= 0:
            raise ValueError(
                f"sensitivity must be positive, got {sensitivity}"
            )

        self.epsilon = epsilon
        self.sensitivity = sensitivity

    def select(
        self,
        real_data: np.ndarray,
        candidates: list[np.ndarray],
        utility_function: Callable[[np.ndarray, np.ndarray], float] | None = None,
        utility_scores: np.ndarray | None = None,
    ) -> ExponentialMechanismResult:
        """Select a candidate using the Exponential Mechanism.

        Provide either a utility_function or pre-computed utility_scores.

        Args:
            real_data: The real (private) data.
            candidates: List of candidate synthetic datasets/representations.
            utility_function: Function u(real_data, candidate) -> score.
                Higher scores mean better utility.
            utility_scores: Pre-computed utility scores for each candidate.
                Array of shape (n_candidates,).

        Returns:
            ExponentialMechanismResult with the selected index, score, and metadata.

        Raises:
            ValueError: If neither utility_function nor utility_scores provided,
                or if inputs are empty.
        """
        n_candidates = len(candidates)
        if n_candidates == 0:
            raise ValueError("candidates list is empty")

        if utility_scores is not None and utility_function is not None:
            raise ValueError(
                "Provide either utility_function or utility_scores, not both"
            )
        if utility_scores is None and utility_function is None:
            raise ValueError(
                "Provide either utility_function or utility_scores"
            )

        # Compute scores
        if utility_function is not None:
            scores = np.array([
                utility_function(real_data, candidate)
                for candidate in candidates
            ], dtype=float)
        else:
            scores = np.asarray(utility_scores, dtype=float)

        # Apply Exponential Mechanism
        # Pr[select r] ∝ exp(ε * u(data, r) / (2 * Δu))
        exponent = self.epsilon * scores / (2.0 * self.sensitivity)

        # Numerical stability: subtract max
        exponent_shifted = exponent - np.max(exponent)
        weights = np.exp(exponent_shifted)

        # Handle numerical issues (all weights near zero)
        if weights.sum() <= 0 or np.isinf(weights).any():
            # Fall back to uniform selection with warning
            logger.warning(
                "Exponential weights underflow/overflow. Falling back to uniform."
            )
            weights = np.ones_like(weights)

        probabilities = weights / weights.sum()

        # Sample from the distribution
        selected_idx = np.random.choice(
            n_candidates, p=probabilities
        )

        return ExponentialMechanismResult(
            selected_index=int(selected_idx),
            selected_score=float(scores[selected_idx]),
            epsilon=self.epsilon,
            sensitivity=self.sensitivity,
            selection_probability=float(probabilities[selected_idx]),
        )

    def select_hyperparameter(
        self,
        real_data: np.ndarray,
        candidate_generator: Callable[[object], np.ndarray],
        hyperparameter_values: list[object],
        evaluation_metric: Callable[[np.ndarray, np.ndarray], float],
    ) -> tuple[object, ExponentialMechanismResult]:
        """Select the best hyperparameter value using the Exponential Mechanism.

        Useful for tuning privacy/utility trade-off parameters like
        noise multiplier, clipping threshold, or model architecture.

        Args:
            real_data: The real (private) data.
            candidate_generator: Function that takes a hyperparameter value
                and returns a synthetic dataset.
            hyperparameter_values: List of hyperparameter values to evaluate.
            evaluation_metric: Function that scores a synthetic dataset
                against real data (higher = better).

        Returns:
            Tuple of (best_hyperparameter, ExponentialMechanismResult).
        """
        candidates: list[np.ndarray] = []
        for value in hyperparameter_values:
            synthetic = candidate_generator(value)
            candidates.append(synthetic)

        scores = np.array([
            evaluation_metric(real_data, candidate)
            for candidate in candidates
        ])

        result = self.select(
            real_data=real_data,
            candidates=candidates,
            utility_scores=scores,
        )

        best_hp = hyperparameter_values[result.selected_index]
        return best_hp, result

    def __repr__(self) -> str:
        return (
            f"ExponentialMechanism(ε={self.epsilon}, "
            f"Δu={self.sensitivity})"
        )


def utility_negative_mse(
    real_data: np.ndarray, synthetic: np.ndarray
) -> float:
    """Utility function: negative mean squared error.

    Higher scores are better (less error). Sensitivity depends on
    the data range.

    Args:
        real_data: Real data.
        synthetic: Synthetic data candidate.

    Returns:
        Negative MSE score.
    """
    mse = np.mean((real_data - synthetic) ** 2)
    return -float(mse)


def utility_column_correlation(
    real_data: np.ndarray, synthetic: np.ndarray
) -> float:
    """Utility function: correlation preservation.

    Measures how well pairwise correlations are preserved.
    Score of 1.0 = perfect preservation.

    Args:
        real_data: Real data.
        synthetic: Synthetic data candidate.

    Returns:
        Similarity of correlation matrices (1.0 = identical).
    """
    real_corr = np.corrcoef(real_data.T)
    synth_corr = np.corrcoef(synthetic.T)

    # Handle NaN
    real_corr = np.nan_to_num(real_corr, nan=0.0)
    synth_corr = np.nan_to_num(synth_corr, nan=0.0)

    # Frobenius norm difference, converted to similarity
    diff = np.linalg.norm(real_corr - synth_corr, ord="fro")
    max_diff = np.linalg.norm(real_corr, ord="fro") + np.linalg.norm(
        synth_corr, ord="fro"
    )

    similarity = 1.0 - (diff / max(max_diff, 1e-10))
    return float(max(0.0, similarity))
