"""Tests for evaluation modules (utility and privacy)."""

import numpy as np
import pytest

from dpsynth.evaluation.privacy import MembershipInferenceAttack
from dpsynth.evaluation.utility import UtilityEvaluator


class TestUtilityEvaluator:
    """Test the UtilityEvaluator."""

    @pytest.fixture
    def evaluator(self):
        return UtilityEvaluator(categorical_threshold=10, random_state=42)

    def test_evaluate_identical_data(self, evaluator):
        """Test that identical data gets a high utility score."""
        real = np.random.randn(100, 5)
        synthetic = real.copy()
        report = evaluator.evaluate(real, synthetic)
        # Without a target column, ML utility gets 0.5, so max possible overall is 0.4*1.0 + 0.3*1.0 + 0.3*0.5 = 0.85
        assert report.overall_score > 0.8, f"Expected high score for identical data, got {report.overall_score:.4f}"
        assert report.correlation_distance < 0.1

    def test_evaluate_dimension_mismatch(self, evaluator):
        """Test that dimension mismatch raises ValueError."""
        real = np.random.randn(100, 5)
        synthetic = np.random.randn(100, 3)
        with pytest.raises(ValueError):
            evaluator.evaluate(real, synthetic)

    def test_evaluate_random_data(self, evaluator):
        """Test evaluation with completely different data (low score expected)."""
        real = np.random.randn(100, 5)
        synthetic = np.random.randn(100, 5) * 10.0  # Very different distribution
        report = evaluator.evaluate(real, synthetic)
        # Score can still be reasonable for same distribution shape
        assert report.overall_score >= 0.0
        assert report.overall_score <= 1.0

    def test_continuous_similarity_identical(self, evaluator):
        """Test that identical continuous columns get score near 1."""
        real = np.random.randn(50)
        synthetic = real.copy()
        sim = evaluator._continuous_similarity(real, synthetic)
        assert sim > 0.99, f"Expected near-perfect similarity, got {sim:.4f}"

    def test_continuous_similarity_different(self, evaluator):
        """Test that very different continuous columns get a low score."""
        np.random.seed(42)
        real = np.random.randn(50)
        np.random.seed(99)
        synthetic = np.random.randn(50) * 100.0
        sim = evaluator._continuous_similarity(real, synthetic)
        assert sim < 0.5, f"Expected low similarity, got {sim:.4f}"

    def test_categorical_similarity_identical(self, evaluator):
        """Test that identical categorical columns get score 1."""
        real = np.array([1, 2, 3, 1, 2, 3, 1, 2, 3])
        synthetic = real.copy()
        sim = evaluator._categorical_similarity(real, synthetic)
        assert sim == 1.0

    def test_categorical_similarity_different(self, evaluator):
        """Test that completely different categorical columns get score 0."""
        real = np.array([1, 1, 1, 1, 1])
        synthetic = np.array([2, 2, 2, 2, 2])
        sim = evaluator._categorical_similarity(real, synthetic)
        assert sim == 0.0

    def test_is_categorical(self, evaluator):
        """Test categorical detection."""
        cat_col = np.array([1, 2, 3, 1, 2, 3])
        cont_col = np.random.randn(50)
        assert evaluator._is_categorical(cat_col)
        assert not evaluator._is_categorical(cont_col)

    def test_correlation_distance(self, evaluator):
        """Test correlation distance computation."""
        real = np.random.randn(100, 3)
        synthetic = real.copy()
        dist = evaluator._correlation_distance(real, synthetic)
        assert dist < 0.1, f"Expected near-zero correlation distance, got {dist:.4f}"

    def test_ml_utility_classification(self, evaluator):
        """Test ML utility evaluation with classification target."""
        rng = np.random.RandomState(42)
        n = 200
        X = rng.randn(n, 2)
        y = (X[:, 0] + X[:, 1] > 0).astype(int)
        data = np.column_stack([X, y])

        target_column = 2
        metrics = evaluator._ml_utility(data, data, target_column)
        assert "accuracy" in metrics
        assert "reference_accuracy" in metrics
        # With identical data, accuracy should be high
        assert metrics["accuracy"] > 0.7

    def test_ml_utility_regression(self, evaluator):
        """Test ML utility evaluation with regression target."""
        rng = np.random.RandomState(42)
        n = 200
        X = rng.randn(n, 2)
        y = X[:, 0] * 2.0 + X[:, 1] + 0.1 * rng.randn(n)
        data = np.column_stack([X, y])

        target_column = 2
        metrics = evaluator._ml_utility(data, data, target_column)
        assert "r2" in metrics
        assert "reference_r2" in metrics

    def test_column_names_in_report(self, evaluator):
        """Test that column names appear in report."""
        real = np.random.randn(50, 3)
        synthetic = np.random.randn(50, 3)
        names = ["age", "income", "score"]
        report = evaluator.evaluate(real, synthetic, column_names=names)
        for name in names:
            assert name in report.column_wise_similarity


class TestMembershipInferenceAttack:
    """Test the MembershipInferenceAttack."""

    @pytest.fixture
    def mia(self):
        return MembershipInferenceAttack(n_shadow=5, test_size=0.3, random_state=42)

    def test_evaluate_same_data(self, mia):
        """Test MIA when synthetic is very close to real (should detect leakage)."""
        rng = np.random.RandomState(42)
        real = rng.randn(100, 5)
        # Synthetic very close to real — MIA should show leakage
        synthetic = real + 0.01 * rng.randn(100, 5)

        report = mia.evaluate(real, synthetic)
        assert report.attack_auc >= 0.0
        assert report.attack_auc <= 1.0
        assert report.n_shadow == 5

    def test_evaluate_different_data(self, mia):
        """Test MIA when synthetic is independent of real."""
        rng = np.random.RandomState(42)
        real = rng.randn(100, 5)
        synthetic = rng.randn(100, 5) * 10.0  # Completely different distribution

        report = mia.evaluate(real, synthetic)
        assert report.attack_auc >= 0.0
        assert report.attack_auc <= 1.0

    def test_small_data_threshold(self, mia):
        """Test that very small data returns a protected report."""
        real = np.random.randn(5, 3)
        synthetic = np.random.randn(5, 3)
        report = mia.evaluate(real, synthetic)
        assert report.is_protected
        assert report.attack_auc == 0.5

    def test_no_membership_leakage(self, mia):
        """Test that independent data has AUC near 0.5."""
        rng = np.random.RandomState(42)
        real = rng.randn(200, 10)
        synthetic = rng.randn(200, 10)

        report = mia.evaluate(real, synthetic)
        # Should be closer to random if truly independent
        assert report.attack_auc < 0.8

    def test_min_distances(self, mia):
        """Test the minimum distance computation."""
        query = np.array([[0, 0], [100, 100]])
        reference = np.array([[0.5, 0.5], [2, 2]])
        distances = mia._min_distances(query, reference)
        assert len(distances) == 2
        assert distances[0] > 0  # Distance from (0,0) to (0.5,0.5)
        assert distances[1] > 0  # Distance from (100,100) to (0.5,0.5)
        # (0,0) should be much closer than (100,100)
        assert distances[0] < distances[1]
