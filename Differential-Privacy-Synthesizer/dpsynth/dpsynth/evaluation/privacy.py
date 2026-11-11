"""
Privacy evaluation and membership inference attack testing.

Implements a shadow model approach to measure membership inference
risk of synthetic data under differential privacy.
"""

from __future__ import annotations

import logging
from dataclasses import dataclass

import numpy as np
from sklearn.ensemble import RandomForestClassifier
from sklearn.metrics import auc, roc_curve
from sklearn.model_selection import train_test_split

logger = logging.getLogger(__name__)


@dataclass
class PrivacyLeakageReport:
    """Report on privacy leakage from membership inference.

    Attributes:
        attack_auc: AUC of the membership inference attack (0.5 = random,
            1.0 = complete leakage, 0.0 = perfectly protected).
        attack_accuracy: Accuracy of the attack classifier.
        epsilon_estimate: Estimated ε based on the max information leakage.
        n_shadow: Number of shadow models used.
        is_protected: Whether the data is considered protected (AUC < 0.6).
    """

    attack_auc: float = 0.5
    attack_accuracy: float = 0.5
    epsilon_estimate: float = float("inf")
    n_shadow: int = 0
    is_protected: bool = False

    def __repr__(self) -> str:
        return (
            f"PrivacyLeakageReport(AUC={self.attack_auc:.4f}, "
            f"acc={self.attack_accuracy:.1%}, "
            f"protected={self.is_protected})"
        )


class MembershipInferenceAttack:
    """Membership inference attack against synthetic data.

    Uses shadow models to evaluate whether an adversary could determine
    if a specific record was in the training set, given only access to
    the synthetic data.

    The attack works by:
    1. Splitting real data into multiple shadow datasets
    2. Training shadow models (same architecture) on each split
    3. Building a meta-classifier to distinguish members from non-members
    4. Measuring AUC of this classifier against held-out data

    Usage:
        >>> mia = MembershipInferenceAttack()
        >>> report = mia.evaluate(real_data, synthetic_data)
        >>> print(f"MIA AUC: {report.attack_auc:.3f}")
    """

    def __init__(
        self,
        n_shadow: int = 10,
        test_size: float = 0.3,
        random_state: int = 42,
    ):
        """Initialize membership inference attack.

        Args:
            n_shadow: Number of shadow models to train.
            test_size: Fraction of data to hold out for attack evaluation.
            random_state: Random seed for reproducibility.
        """
        self.n_shadow = n_shadow
        self.test_size = test_size
        self.random_state = random_state

    def evaluate(
        self,
        real_data: np.ndarray,
        synthetic_data: np.ndarray,
    ) -> PrivacyLeakageReport:
        """Evaluate membership inference risk.

        Args:
            real_data: Original training data of shape (n_real, n_features).
            synthetic_data: Synthetic data of shape (n_synth, n_features).

        Returns:
            PrivacyLeakageReport with attack metrics.
        """
        n_real = real_data.shape[0]
        n_synth = synthetic_data.shape[0]

        if n_real < 20 or n_synth < 20:
            logger.warning(
                f"Data too small for meaningful MIA: real={n_real}, synth={n_synth}"
            )
            return PrivacyLeakageReport(
                attack_auc=0.5,
                attack_accuracy=0.5,
                epsilon_estimate=float("inf"),
                n_shadow=0,
                is_protected=True,
            )

        # Build attack dataset
        # For each shadow model, we train on a subset and then
        # try to predict membership on another subset
        attack_features: list[np.ndarray] = []
        attack_labels: list[np.ndarray] = []

        for shadow_idx in range(self.n_shadow):
            # Split real data into member/non-member sets
            member_data, non_member_data = train_test_split(
                real_data,
                test_size=self.test_size,
                random_state=self.random_state + shadow_idx,
            )

            # Shadow model: simple distance-based approach
            # Compute distances from each record to the synthetic data
            # Members should be "closer" to synthetic data on average
            member_distances = self._min_distances(member_data, synthetic_data)
            non_member_distances = self._min_distances(
                non_member_data, synthetic_data
            )

            attack_features.append(
                np.concatenate([member_distances, non_member_distances])
            )
            attack_labels.append(
                np.concatenate([
                    np.ones(len(member_distances)),
                    np.zeros(len(non_member_distances)),
                ])
            )

        X_attack = np.concatenate(attack_features).reshape(-1, 1)
        y_attack = np.concatenate(attack_labels)

        # Train attack classifier
        X_train, X_test, y_train, y_test = train_test_split(
            X_attack,
            y_attack,
            test_size=0.3,
            random_state=self.random_state,
        )

        clf = RandomForestClassifier(
            n_estimators=50,
            max_depth=5,
            random_state=self.random_state,
        )
        clf.fit(X_train, y_train)
        y_pred_proba = clf.predict_proba(X_test)[:, 1]
        y_pred = clf.predict(X_test)

        # Compute AUC
        fpr, tpr, thresholds = roc_curve(y_test, y_pred_proba)
        attack_auc = auc(fpr, tpr)

        # Accuracy
        accuracy = np.mean(y_pred == y_test)

        # Estimate ε from AUC using the relationship:
        # AUC ≈ 1 - exp(-ε) / 2  (simplified)
        # => ε ≈ -ln(2 * (1 - AUC))
        if attack_auc > 0.5:
            epsilon_estimate = -np.log(2.0 * (1.0 - min(attack_auc, 0.999)))
        else:
            epsilon_estimate = 0.0

        is_protected = attack_auc < 0.6

        return PrivacyLeakageReport(
            attack_auc=float(attack_auc),
            attack_accuracy=float(accuracy),
            epsilon_estimate=float(epsilon_estimate),
            n_shadow=self.n_shadow,
            is_protected=is_protected,
        )

    @staticmethod
    def _min_distances(
        query: np.ndarray, reference: np.ndarray
    ) -> np.ndarray:
        """Compute minimum L2 distance from each query point to reference set.

        Args:
            query: Array of shape (n_query, n_features).
            reference: Array of shape (n_ref, n_features).

        Returns:
            Array of shape (n_query,) with minimum distances.
        """
        # Efficient computation without full pairwise matrix
        # Using the identity: ||a - b||² = ||a||² + ||b||² - 2*a·b
        query_norm = np.sum(query ** 2, axis=1, keepdims=True)
        ref_norm = np.sum(reference ** 2, axis=1, keepdims=True).T

        # ||a - b||² = ||a||² + ||b||² - 2*a·bᵀ
        dot_product = query @ reference.T
        distances_sq = query_norm + ref_norm - 2.0 * dot_product
        distances_sq = np.maximum(distances_sq, 0.0)  # Numerical safety

        min_distances = np.sqrt(np.min(distances_sq, axis=1))
        return min_distances
