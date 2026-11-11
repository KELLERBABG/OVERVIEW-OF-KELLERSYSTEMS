"""Tests for the Exponential Mechanism module."""

import numpy as np
import pytest

from dpsynth.privacy.exponential import (
    ExponentialMechanism,
    ExponentialMechanismResult,
    utility_column_correlation,
    utility_negative_mse,
)


class TestExponentialMechanism:
    """Test the ExponentialMechanism class."""

    def test_init_default(self):
        """Test default initialization."""
        mech = ExponentialMechanism()
        assert mech.epsilon == 1.0
        assert mech.sensitivity == 1.0

    def test_init_custom(self):
        """Test custom initialization."""
        mech = ExponentialMechanism(epsilon=0.5, sensitivity=2.0)
        assert mech.epsilon == 0.5
        assert mech.sensitivity == 2.0

    def test_invalid_epsilon_raises(self):
        """Test that non-positive epsilon raises ValueError."""
        with pytest.raises(ValueError):
            ExponentialMechanism(epsilon=0)
        with pytest.raises(ValueError):
            ExponentialMechanism(epsilon=-1.0)

    def test_invalid_sensitivity_raises(self):
        """Test that non-positive sensitivity raises ValueError."""
        with pytest.raises(ValueError):
            ExponentialMechanism(sensitivity=0)

    def test_select_with_function(self):
        """Test selection with utility function."""
        mech = ExponentialMechanism(epsilon=10.0, sensitivity=1.0)
        real = np.array([[0, 0], [1, 1]])
        candidates = [
            np.array([[0.5, 0.5], [1.5, 1.5]]),
            np.array([[100, 100], [101, 101]]),
        ]

        result = mech.select(real, candidates, utility_function=utility_negative_mse)
        assert 0 <= result.selected_index < len(candidates)
        assert result.epsilon == 10.0
        assert result.sensitivity == 1.0
        # First candidate should be much better (lower MSE)
        assert result.selected_index == 0

    def test_select_with_scores(self):
        """Test selection with pre-computed scores."""
        mech = ExponentialMechanism(epsilon=10.0, sensitivity=1.0)
        real = np.array([[0, 0]])
        candidates = [np.array([[0.1, 0.1]]), np.array([[100, 100]])]
        scores = np.array([-0.02, -20000.0])

        result = mech.select(real, candidates, utility_scores=scores)
        assert result.selected_index == 0
        assert result.selected_score == -0.02

    def test_select_empty_candidates_raises(self):
        """Test that empty candidates raises ValueError."""
        mech = ExponentialMechanism()
        with pytest.raises(ValueError):
            mech.select(np.array([[0]]), [])

    def test_select_no_function_or_scores_raises(self):
        """Test that providing neither raises ValueError."""
        mech = ExponentialMechanism()
        with pytest.raises(ValueError):
            mech.select(np.array([[0]]), [np.array([[0]])])

    def test_select_both_function_and_scores_raises(self):
        """Test that providing both raises ValueError."""
        mech = ExponentialMechanism()
        with pytest.raises(ValueError):
            mech.select(
                np.array([[0]]),
                [np.array([[0]])],
                utility_function=lambda x, y: 0.0,
                utility_scores=np.array([0.0]),
            )

    def test_selection_probability_higher_for_better(self):
        """Test that better candidates have higher selection probability."""
        mech = ExponentialMechanism(epsilon=100.0, sensitivity=1.0)
        real = np.array([[0, 0]])
        candidates = [
            np.array([[0.001, 0.001]]),  # Very close
            np.array([[100, 100]]),       # Very far
        ]

        # Run many times to check statistical tendency
        selections = []
        for _ in range(100):
            result = mech.select(
                real, candidates, utility_function=utility_negative_mse
            )
            selections.append(result.selected_index)

        # The first candidate should win significantly more often
        assert np.mean(selections) < 0.5, (
            "Expected better candidate to be selected more often"
        )

    def test_selection_with_tied_scores(self):
        """Test that tied scores lead to roughly equal probability."""
        mech = ExponentialMechanism(epsilon=0.01, sensitivity=1.0)
        real = np.array([[0, 0]])
        candidates = [np.array([[1, 1]]), np.array([[1, 1]])]
        scores = np.array([-2.0, -2.0])

        selections = []
        for _ in range(100):
            result = mech.select(
                real, candidates, utility_scores=scores
            )
            selections.append(result.selected_index)

        # With near-tied scores and tiny epsilon, selections should be ~50/50
        ratio = np.mean(selections)
        assert 0.2 < ratio < 0.8, (
            f"Expected roughly uniform selection, got {ratio:.2f}"
        )

    def test_select_hyperparameter(self):
        """Test hyperparameter selection."""
        mech = ExponentialMechanism(epsilon=5.0, sensitivity=1.0)

        def generator(hp):
            return np.array([[hp, hp]])

        def metric(real, synthetic):
            return utility_negative_mse(real, synthetic)

        real = np.array([[0.5, 0.5]])
        hp_values = [0.1, 0.5, 1.0, 10.0, 100.0]

        best_hp, result = mech.select_hyperparameter(
            real, generator, hp_values, metric
        )
        assert best_hp in hp_values
        assert isinstance(result, ExponentialMechanismResult)

    def test_repr(self):
        """Test string representation."""
        mech = ExponentialMechanism(epsilon=1.0, sensitivity=2.0)
        rep = repr(mech)
        assert "ε=" in rep or "epsilon=" in rep
        assert "Δu=" in rep or "sensitivity=" in rep


class TestUtilityFunctions:
    """Test built-in utility functions."""

    def test_negative_mse_identical(self):
        """Test that identical data gets score of 0 (no error)."""
        data = np.random.randn(10, 3)
        score = utility_negative_mse(data, data)
        assert score == 0.0

    def test_negative_mse_different(self):
        """Test that different data gets a negative score."""
        real = np.zeros((10, 3))
        synth = np.ones((10, 3)) * 100
        score = utility_negative_mse(real, synth)
        assert score < 0

    def test_column_correlation_identical(self):
        """Test that identical data gets score close to 1."""
        data = np.random.randn(50, 4)
        score = utility_column_correlation(data, data)
        assert score > 0.99

    def test_column_correlation_different(self):
        """Test that very different data gets a lower score."""
        rng = np.random.RandomState(42)
        real = rng.randn(50, 3)
        synth = rng.randn(50, 3) * 100
        score = utility_column_correlation(real, synth)
        assert 0.0 <= score <= 1.0


class TestExponentialMechanismResult:
    """Test the ExponentialMechanismResult dataclass."""

    def test_repr(self):
        """Test string representation."""
        result = ExponentialMechanismResult(
            selected_index=0,
            selected_score=-0.5,
            epsilon=1.0,
            sensitivity=1.0,
            selection_probability=0.75,
        )
        rep = repr(result)
        assert "selected=" in rep or "selected_index" in rep
        assert "score=" in rep
