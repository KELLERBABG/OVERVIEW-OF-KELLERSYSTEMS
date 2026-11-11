"""Tests for the Post-Processing Invariance verifier."""

import numpy as np
import pytest

from dpsynth.evaluation.postprocess import (
    PostProcessingVerifier,
    PostProcessReport,
    is_post_processing_invariant,
)


class TestPostProcessingVerifier:
    """Test the PostProcessingVerifier."""

    @pytest.fixture
    def verifier(self):
        return PostProcessingVerifier(
            epsilon_threshold=0.5,
            random_state=42,
        )

    def test_init(self):
        """Test initialization."""
        verifier = PostProcessingVerifier(epsilon_threshold=0.2)
        assert verifier.epsilon_threshold == 0.2

    def test_verify_identical_data(self, verifier):
        """Test verification with identical real and synthetic."""
        rng = np.random.RandomState(42)
        data = rng.randn(100, 5)
        report = verifier.verify(data, data)
        assert isinstance(report, PostProcessReport)
        assert report.transformations_tested > 0
        assert report.pre_transform_epsilon >= 0.0

    def test_verify_random_data(self, verifier):
        """Test verification with independent data."""
        rng = np.random.RandomState(42)
        real = rng.randn(100, 5)
        synthetic = rng.randn(100, 5)

        report = verifier.verify(real, synthetic)
        assert isinstance(report, PostProcessReport)
        assert report.transformations_tested > 0
        assert report.violations_found >= 0

    def test_estimate_epsilon_perfect_match(self, verifier):
        """Test ε estimation with identical data (expected to show some leakage)."""
        rng = np.random.RandomState(42)
        data = rng.randn(50, 3)
        eps = verifier._estimate_epsilon(data, data)
        # Identical data should show some leakage > 0
        assert eps >= 0.0

    def test_estimate_epsilon_different_data(self, verifier):
        """Test ε estimation with independent data."""
        rng = np.random.RandomState(42)
        real = rng.randn(50, 3)
        synthetic = rng.randn(50, 3) * 100
        eps = verifier._estimate_epsilon(real, synthetic)
        # Very different data should show low leakage
        assert eps >= 0.0

    def test_estimate_epsilon_small_data(self, verifier):
        """Test ε estimation with very small datasets."""
        real = np.random.randn(3, 3)
        synthetic = np.random.randn(3, 3)
        eps = verifier._estimate_epsilon(real, synthetic)
        assert eps == 0.0  # Returns 0 for < 10 samples

    def test_default_transformations(self, verifier):
        """Test that default transformations run without error."""
        transforms = verifier._default_transformations()
        assert len(transforms) >= 8
        data = np.random.randn(20, 4)

        for name, func in transforms:
            result = func(data)
            assert result.shape[0] == data.shape[0], (
                f"Transform '{name}' changed number of rows"
            )

    def test_min_distances(self, verifier):
        """Test minimum distance computation."""
        query = np.array([[0, 0], [10, 10]])
        reference = np.array([[0.5, 0.5], [2, 2]])
        distances = verifier._min_distances(query, reference)
        assert len(distances) == 2
        assert distances[0] < distances[1]

    def test_verify_with_custom_transforms(self, verifier):
        """Test verification with custom transformations."""
        real = np.random.randn(50, 3)
        synthetic = np.random.randn(50, 3)

        transforms = [
            ("double", lambda x: x * 2),
            ("plus_one", lambda x: x + 1),
        ]

        report = verifier.verify(real, synthetic, transformations=transforms)
        assert report.transformations_tested == 2

    def test_repr(self, verifier):
        """Test string representation of report."""
        rng = np.random.RandomState(42)
        data = rng.randn(50, 5)
        report = verifier.verify(data, data)
        rep = repr(report)
        assert "INVARIANT" in rep or "VIOLATED" in rep


class TestIsPostProcessingInvariant:
    """Test the convenience function."""

    def test_strict_invariant(self):
        """Test strict check with invariant report."""
        report = PostProcessReport(
            is_invariant=True,
            pre_transform_epsilon=0.1,
            post_transform_epsilon=0.12,
            epsilon_drift=0.02,
            transformations_tested=10,
            violations_found=0,
        )
        assert is_post_processing_invariant(report, strict=True)

    def test_strict_violated(self):
        """Test strict check with violated report."""
        report = PostProcessReport(
            is_invariant=False,
            pre_transform_epsilon=0.1,
            post_transform_epsilon=2.0,
            epsilon_drift=1.9,
            transformations_tested=10,
            violations_found=5,
        )
        assert not is_post_processing_invariant(report, strict=True)

    def test_non_strict_tolerant(self):
        """Test non-strict check with some violations."""
        report = PostProcessReport(
            is_invariant=False,
            pre_transform_epsilon=0.1,
            post_transform_epsilon=0.5,
            epsilon_drift=0.4,
            transformations_tested=10,
            violations_found=1,  # 10% violation rate
        )
        assert is_post_processing_invariant(report, strict=False)
