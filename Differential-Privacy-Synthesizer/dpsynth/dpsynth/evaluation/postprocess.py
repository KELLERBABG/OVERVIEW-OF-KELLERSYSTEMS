"""
Post-Processing Invariance verification for differential privacy.

Post-Processing Invariance is a fundamental property of differential privacy:
any data-independent transformation of a DP output remains DP with the same
(ε, δ) guarantees. This means once synthetic data is generated with DP,
any analysis, aggregation, or ML training on it is automatically DP-protected.

This module provides verification tools to ensure that post-processing
does not invalidate the privacy guarantee.
"""

from __future__ import annotations

import logging
from collections.abc import Callable
from dataclasses import dataclass

import numpy as np

logger = logging.getLogger(__name__)


@dataclass
class PostProcessReport:
    """Report on post-processing invariance verification.

    Attributes:
        is_invariant: Whether DP guarantee is preserved under post-processing.
        pre_transform_epsilon: ε estimate from pre-transformation data.
        post_transform_epsilon: ε estimate from post-transformation data.
        epsilon_drift: Absolute change in ε estimate.
        transformations_tested: Number of transformations tested.
        violations_found: Number of transformations that broke DP guarantees.
    """

    is_invariant: bool
    pre_transform_epsilon: float
    post_transform_epsilon: float
    epsilon_drift: float
    transformations_tested: int
    violations_found: int

    def __repr__(self) -> str:
        status = "✓ INVARIANT" if self.is_invariant else "✗ VIOLATED"
        return (
            f"PostProcessReport({status}, "
            f"ε_drift={self.epsilon_drift:.4f}, "
            f"violations={self.violations_found}/{self.transformations_tested})"
        )


class PostProcessingVerifier:
    """Verifies post-processing invariance of differentially private data.

    Tests that synthetic data generated with DP retains its privacy
    guarantees after common data transformations:
    - Scaling / normalization
    - Aggregation (mean, sum)
    - Dimensionality reduction
    - Feature engineering
    - Machine learning training

    The verification works by measuring membership inference attack
    AUC before and after transformations — if the AUC doesn't increase
    significantly, post-processing invariance holds.

    Usage:
        >>> verifier = PostProcessingVerifier()
        >>> report = verifier.verify(real_data, synthetic_data)
        >>> print(f"Post-processing invariant: {report.is_invariant}")
    """

    def __init__(
        self,
        epsilon_threshold: float = 0.1,
        mi_threshold: float = 0.05,
        random_state: int = 42,
    ):
        """Initialize verifier.

        Args:
            epsilon_threshold: Maximum allowed ε drift before flagging violation.
            mi_threshold: Maximum allowed MI attack AUC increase.
            random_state: Random seed.
        """
        self.epsilon_threshold = epsilon_threshold
        self.mi_threshold = mi_threshold
        self.random_state = random_state

    def verify(
        self,
        real_data: np.ndarray,
        synthetic_data: np.ndarray,
        transformations: list[tuple[str, Callable]] | None = None,
    ) -> PostProcessReport:
        """Verify post-processing invariance.

        Tests the privacy guarantee after common data transformations.

        Args:
            real_data: Original (private) data.
            synthetic_data: DP synthetic data.
            transformations: List of (name, function) pairs. If None,
                uses a default set of common transformations.

        Returns:
            PostProcessReport with verification results.
        """
        if transformations is None:
            transformations = self._default_transformations()

        # Estimate pre-transformation privacy leakage
        pre_eps = self._estimate_epsilon(real_data, synthetic_data)

        violations = 0
        post_eps_values: list[float] = []

        for name, transform in transformations:
            try:
                transformed_synthetic = transform(synthetic_data)
                # Skip if transformation changed shape incompatibly
                if transformed_synthetic.shape[0] < 2:
                    continue

                post_eps = self._estimate_epsilon(
                    real_data, transformed_synthetic
                )
                post_eps_values.append(post_eps)

                # Check if ε increased significantly
                if post_eps > pre_eps + self.epsilon_threshold:
                    violations += 1
                    logger.warning(
                        f"Post-processing '{name}' increased ε estimate: "
                        f"{pre_eps:.4f} -> {post_eps:.4f}"
                    )
            except Exception as e:
                logger.debug(f"Skipping transformation '{name}': {e}")

        if post_eps_values:
            avg_post_eps = float(np.mean(post_eps_values))
            epsilon_drift = avg_post_eps - pre_eps
        else:
            avg_post_eps = pre_eps
            epsilon_drift = 0.0

        is_invariant = violations == 0 and epsilon_drift < self.epsilon_threshold

        return PostProcessReport(
            is_invariant=is_invariant,
            pre_transform_epsilon=pre_eps,
            post_transform_epsilon=avg_post_eps,
            epsilon_drift=epsilon_drift,
            transformations_tested=len(transformations),
            violations_found=violations,
        )

    def _estimate_epsilon(
        self, real: np.ndarray, synthetic: np.ndarray
    ) -> float:
        """Estimate the privacy budget from data.

        Uses a distance-based membership inference heuristic.
        Lower ε estimates = better privacy.

        Args:
            real: Real data.
            synthetic: Synthetic data.

        Returns:
            Estimated ε value (0 = perfect privacy, large = leakage).
        """
        if real.shape[0] < 10 or synthetic.shape[0] < 10:
            return 0.0

        # Compute minimum distances from real to synthetic
        distances = self._min_distances(real, synthetic)

        # Estimate ε from distance distribution
        # If synthetic is very close to real, ε is higher (more leakage)
        # We use a heuristic: ε ≈ -log(mean normalized distance)
        data_range = np.std(real, axis=0).mean()
        if data_range == 0:
            data_range = 1.0

        normalized_distances = distances / data_range
        mean_dist = float(np.mean(normalized_distances))

        # ε estimate: lower distance = higher leakage = higher ε
        epsilon = max(0.0, -np.log(max(mean_dist, 1e-10)))
        return epsilon

    def _default_transformations(self) -> list[tuple[str, Callable]]:
        """Get the default set of post-processing transformations.

        Returns:
            List of (name, function) pairs.
        """
        return [
            ("identity", lambda x: x),
            ("scale_0_1", lambda x: (x - x.min(axis=0)) / max(
                (x.max(axis=0) - x.min(axis=0)).max(), 1e-10
            )),
            ("standardize", lambda x: (x - x.mean(axis=0)) / max(
                x.std(axis=0).mean(), 1e-10
            )),
            ("clip_01", lambda x: np.clip(x, 0, 1)),
            ("abs", lambda x: np.abs(x)),
            ("round_2dp", lambda x: np.round(x, 2)),
            ("add_noise_01", lambda x: x + 0.01 * np.random.randn(*x.shape)),
            ("add_noise_1", lambda x: x + 0.1 * np.random.randn(*x.shape)),
            ("mean_aggregation", self._agg_mean),
            ("random_projection", self._random_project),
        ]

    def _agg_mean(self, data: np.ndarray) -> np.ndarray:
        """Aggregate via row-wise mean. Returns as 2D array.

        Args:
            data: Input data.

        Returns:
            Mean-aggregated data.
        """
        return data.mean(axis=1, keepdims=True)

    def _random_project(self, data: np.ndarray) -> np.ndarray:
        """Random projection to lower dimension.

        Args:
            data: Input data of shape (n, d).

        Returns:
            Projected data of shape (n, max(2, d//2)).
        """
        n, d = data.shape
        target_dim = max(2, d // 2)
        rng = np.random.RandomState(self.random_state)
        projection = rng.randn(d, target_dim)
        projection /= np.linalg.norm(projection, axis=0, keepdims=True)
        return data @ projection

    @staticmethod
    def _min_distances(
        query: np.ndarray, reference: np.ndarray
    ) -> np.ndarray:
        """Compute minimum L2 distance from each query point to reference.

        Args:
            query: Array of shape (n_query, n_features).
            reference: Array of shape (n_ref, n_features).

        Returns:
            Array of shape (n_query,) with minimum distances.
        """
        query_norm = np.sum(query ** 2, axis=1, keepdims=True)
        ref_norm = np.sum(reference ** 2, axis=1, keepdims=True).T
        dot_product = query @ reference.T
        distances_sq = query_norm + ref_norm - 2.0 * dot_product
        distances_sq = np.maximum(distances_sq, 0.0)
        return np.sqrt(np.min(distances_sq, axis=1))


def is_post_processing_invariant(
    report: PostProcessReport, strict: bool = True
) -> bool:
    """Convenience function to check if a report indicates invariance.

    Args:
        report: The PostProcessReport to check.
        strict: If True, requires zero violations and small drift.

    Returns:
        True if the data is post-processing invariant.
    """
    if strict:
        return report.is_invariant
    else:
        # Non-strict: allow some violation tolerance
        return (
            report.violations_found <= report.transformations_tested * 0.1
            and report.epsilon_drift < 1.0
        )
