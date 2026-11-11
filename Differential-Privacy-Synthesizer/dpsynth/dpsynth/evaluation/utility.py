"""
Utility evaluation for synthetic data quality.

Provides metrics to compare synthetic data against real data:
column-wise distribution similarity, pairwise correlation preservation,
and downstream ML utility (train-on-synthetic, test-on-real).
"""

from __future__ import annotations

from dataclasses import dataclass, field

import numpy as np
from scipy import stats
from sklearn.ensemble import RandomForestClassifier, RandomForestRegressor
from sklearn.metrics import accuracy_score, mean_squared_error, r2_score
from sklearn.model_selection import train_test_split


@dataclass
class UtilityReport:
    """Comprehensive utility evaluation report.

    Attributes:
        column_wise_similarity: Dict mapping column index or name to
            similarity score (1.0 = identical).
        correlation_distance: Frobenius norm of correlation matrix difference.
        ml_utility: Dict with downstream ML performance metrics.
        overall_score: Weighted combination of metrics (0-1).
    """

    column_wise_similarity: dict[str, float] = field(default_factory=dict)
    correlation_distance: float = 0.0
    ml_utility: dict[str, float] = field(default_factory=dict)
    overall_score: float = 0.0
    n_features: int = 0

    def __repr__(self) -> str:
        return (
            f"UtilityReport(overall={self.overall_score:.4f}, "
            f"corr_dist={self.correlation_distance:.4f}, "
            f"n_features={self.n_features}, "
            f"ml_utility={self.ml_utility})"
        )


class UtilityEvaluator:
    """Evaluates the statistical utility of synthetic data.

    Compares real and synthetic datasets across multiple dimensions:
    - Per-feature distribution similarity (KS test / Wasserstein distance)
    - Pairwise correlation preservation
    - Downstream machine learning utility

    Usage:
        >>> evaluator = UtilityEvaluator()
        >>> report = evaluator.evaluate(real_data, synthetic_data)
        >>> print(f"Overall utility: {report.overall_score:.2%}")
    """

    def __init__(
        self,
        categorical_threshold: int = 20,
        random_state: int = 42,
    ):
        """Initialize evaluator.

        Args:
            categorical_threshold: Max unique values for a column to be
                considered categorical.
            random_state: Random seed for reproducibility.
        """
        self.categorical_threshold = categorical_threshold
        self.random_state = random_state

    def evaluate(
        self,
        real: np.ndarray,
        synthetic: np.ndarray,
        column_names: list[str] | None = None,
        target_column: int | None = None,
    ) -> UtilityReport:
        """Evaluate synthetic data utility vs. real data.

        Args:
            real: Real data array of shape (n_real, n_features).
            synthetic: Synthetic data array of shape (n_synth, n_features).
            column_names: Optional list of column names for reporting.
            target_column: Optional column index to use as ML target
                for downstream evaluation.

        Returns:
            UtilityReport with all evaluation metrics.
        """
        if real.shape[1] != synthetic.shape[1]:
            raise ValueError(
                f"Feature dimension mismatch: real={real.shape[1]}, "
                f"synthetic={synthetic.shape[1]}"
            )

        n_features = real.shape[1]
        if column_names is None:
            column_names = [f"col_{i}" for i in range(n_features)]

        # 1. Per-column distribution similarity
        col_similarity: dict[str, float] = {}
        for col_idx in range(n_features):
            col_name = column_names[col_idx]
            real_col = real[:, col_idx]
            synth_col = synthetic[:, col_idx]
            is_cat = self._is_categorical(real_col)

            if is_cat:
                sim = self._categorical_similarity(real_col, synth_col)
            else:
                sim = self._continuous_similarity(real_col, synth_col)

            col_similarity[col_name] = sim

        # 2. Correlation matrix distance
        corr_dist = self._correlation_distance(real, synthetic)

        # 3. Downstream ML utility
        ml_utility: dict[str, float] = {}
        if target_column is not None:
            ml_utility = self._ml_utility(
                real, synthetic, target_column
            )

        # 4. Overall score
        col_sim_values = list(col_similarity.values())
        avg_col_sim = float(np.mean(col_sim_values))
        corr_score = max(0.0, 1.0 - corr_dist / 2.0)  # Normalize corr dist

        if ml_utility.get("accuracy") is not None:
            ml_score = ml_utility.get("accuracy", 0.0)
        elif ml_utility.get("r2") is not None:
            ml_score = ml_utility.get("r2", 0.0)
        else:
            ml_score = 0.5  # Neutral if no target

        overall = 0.4 * avg_col_sim + 0.3 * corr_score + 0.3 * ml_score

        return UtilityReport(
            column_wise_similarity=col_similarity,
            correlation_distance=corr_dist,
            ml_utility=ml_utility,
            overall_score=overall,
            n_features=n_features,
        )

    def _is_categorical(self, column: np.ndarray) -> bool:
        """Determine if a column is categorical.

        Args:
            column: Data column.

        Returns:
            True if the column appears categorical.
        """
        unique_count = len(np.unique(column[~np.isnan(column)]))
        return unique_count <= self.categorical_threshold

    def _categorical_similarity(
        self, real: np.ndarray, synthetic: np.ndarray
    ) -> float:
        """Compute similarity between categorical distributions.

        Uses the total variation distance: 1 - 0.5 * sum(|p_real - p_synth|).

        Args:
            real: Real data column.
            synthetic: Synthetic data column.

        Returns:
            Similarity score in [0, 1] where 1 = identical.
        """
        all_categories = np.unique(np.concatenate([real, synthetic]))
        real_counts = np.array([
            np.sum(real == c) for c in all_categories
        ], dtype=float)
        synth_counts = np.array([
            np.sum(synthetic == c) for c in all_categories
        ], dtype=float)

        real_probs = real_counts / max(real_counts.sum(), 1)
        synth_probs = synth_counts / max(synth_counts.sum(), 1)

        total_variation = 0.5 * np.sum(np.abs(real_probs - synth_probs))
        return 1.0 - total_variation

    def _continuous_similarity(
        self, real: np.ndarray, synthetic: np.ndarray
    ) -> float:
        """Compute similarity between continuous distributions.

        Uses Wasserstein distance converted to a similarity score.

        Args:
            real: Real data column.
            synthetic: Synthetic data column.

        Returns:
            Similarity score in [0, 1] where 1 = identical.
        """
        # Remove NaNs
        real = real[~np.isnan(real)]
        synthetic = synthetic[~np.isnan(synthetic)]

        if len(real) == 0 or len(synthetic) == 0:
            return 0.0

        # Wasserstein distance (also known as Earth Mover's Distance)
        wasserstein = stats.wasserstein_distance(real, synthetic)

        # Normalize by data range
        data_range = max(real.max() - real.min(), 1e-10)
        normalized_dist = wasserstein / data_range

        # Convert to similarity: exp(-distance) gives 1.0 at perfect match
        similarity = float(np.exp(-normalized_dist))
        return similarity

    def _correlation_distance(
        self, real: np.ndarray, synthetic: np.ndarray
    ) -> float:
        """Compute Frobenius norm of the difference between correlation matrices.

        Args:
            real: Real data.
            synthetic: Synthetic data.

        Returns:
            Frobenius norm of (corr_real - corr_synthetic).
        """
        real_corr = np.corrcoef(real.T)
        synth_corr = np.corrcoef(synthetic.T)

        # Handle NaN in correlation (e.g., constant columns)
        real_corr = np.nan_to_num(real_corr, nan=0.0)
        synth_corr = np.nan_to_num(synth_corr, nan=0.0)

        diff = real_corr - synth_corr
        distance = float(np.linalg.norm(diff, ord="fro"))
        return distance

    def _ml_utility(
        self,
        real: np.ndarray,
        synthetic: np.ndarray,
        target_column: int,
    ) -> dict[str, float]:
        """Evaluate downstream ML utility.

        Trains a model on synthetic data, evaluates on real data.

        Args:
            real: Real data.
            synthetic: Synthetic data.
            target_column: Column index of the target variable.

        Returns:
            Dict with ML performance metrics.
        """
        X_real = np.delete(real, target_column, axis=1)
        y_real = real[:, target_column]
        X_synth = np.delete(synthetic, target_column, axis=1)
        y_synth = synthetic[:, target_column]

        # Split real data for evaluation
        X_train_real, X_test_real, y_train_real, y_test_real = train_test_split(
            X_real, y_real, test_size=0.3, random_state=self.random_state
        )

        # Determine if classification or regression
        unique_targets = len(np.unique(y_real))
        is_classification = unique_targets <= self.categorical_threshold

        if is_classification:
            # Train on synthetic, evaluate on real
            clf = RandomForestClassifier(
                n_estimators=100, random_state=self.random_state
            )
            clf.fit(X_synth, y_synth)
            y_pred = clf.predict(X_test_real)
            acc = accuracy_score(y_test_real, y_pred)

            # Train on real (for reference)
            clf_real = RandomForestClassifier(
                n_estimators=100, random_state=self.random_state
            )
            clf_real.fit(X_train_real, y_train_real)
            y_pred_real = clf_real.predict(X_test_real)
            ref_acc = accuracy_score(y_test_real, y_pred_real)

            return {
                "accuracy": float(acc),
                "reference_accuracy": float(ref_acc),
                "task": "classification",
            }
        else:
            # Regression
            reg = RandomForestRegressor(
                n_estimators=100, random_state=self.random_state
            )
            reg.fit(X_synth, y_synth)
            y_pred = reg.predict(X_test_real)
            mse = mean_squared_error(y_test_real, y_pred)
            r2 = r2_score(y_test_real, y_pred)

            # Reference model trained on real
            reg_real = RandomForestRegressor(
                n_estimators=100, random_state=self.random_state
            )
            reg_real.fit(X_train_real, y_train_real)
            y_pred_real = reg_real.predict(X_test_real)
            ref_r2 = r2_score(y_test_real, y_pred_real)

            return {
                "mse": float(mse),
                "r2": float(r2),
                "reference_r2": float(ref_r2),
                "task": "regression",
            }
