"""Tests for the PrivacyAccountant and RDP accounting."""

import pytest

from dpsynth.privacy.accountant import PrivacyAccountant, RDPAccountantState


class TestPrivacyAccountant:
    """Test the PrivacyAccountant class."""

    def test_init(self):
        """Test initialization."""
        accountant = PrivacyAccountant()
        assert accountant.total_steps == 0
        budget = accountant.get_privacy_spent(delta=1e-5)
        assert budget.epsilon == 0.0
        assert budget.delta == 1e-5
        assert budget.steps == 0

    def test_step_records_correctly(self):
        """Test that step() increments the step counter."""
        accountant = PrivacyAccountant()
        accountant.step(noise_multiplier=1.0, batch_size=64, dataset_size=1000, steps=10)
        assert accountant.total_steps == 10

    def test_multiple_steps(self):
        """Test that multiple step calls accumulate."""
        accountant = PrivacyAccountant()
        accountant.step(noise_multiplier=1.0, batch_size=64, dataset_size=1000, steps=5)
        accountant.step(noise_multiplier=1.0, batch_size=64, dataset_size=1000, steps=3)
        assert accountant.total_steps == 8

    def test_epsilon_increases_with_steps(self):
        """Test that epsilon increases as we take more steps."""
        accountant = PrivacyAccountant()
        eps1 = accountant.get_spent_epsilon(delta=1e-5)
        accountant.step(noise_multiplier=0.5, batch_size=64, dataset_size=1000, steps=100)
        eps2 = accountant.get_spent_epsilon(delta=1e-5)
        assert eps2 >= eps1, "Epsilon should increase with more steps"

    def test_smaller_noise_gives_less_privacy(self):
        """Test that smaller noise (smaller σ) yields larger ε."""
        acct1 = PrivacyAccountant()
        acct2 = PrivacyAccountant()

        acct1.step(noise_multiplier=0.5, batch_size=64, dataset_size=1000, steps=100)
        acct2.step(noise_multiplier=5.0, batch_size=64, dataset_size=1000, steps=100)

        eps1 = acct1.get_spent_epsilon(delta=1e-5)
        eps2 = acct2.get_spent_epsilon(delta=1e-5)

        assert eps1 > eps2, "Smaller noise should result in larger ε"

    def test_reset_clears_state(self):
        """Test that reset() clears accumulated privacy."""
        accountant = PrivacyAccountant()
        accountant.step(noise_multiplier=1.0, batch_size=64, dataset_size=1000, steps=50)
        assert accountant.total_steps > 0
        assert accountant.get_spent_epsilon() > 0

        accountant.reset()
        assert accountant.total_steps == 0
        assert accountant.get_spent_epsilon() == 0.0

    def test_invalid_noise_multiplier_raises(self):
        """Test that negative noise multiplier raises ValueError."""
        accountant = PrivacyAccountant()
        with pytest.raises(ValueError):
            accountant.step(noise_multiplier=-1.0, batch_size=64, dataset_size=1000)

    def test_zero_batch_size_raises(self):
        """Test that zero batch size raises ValueError."""
        accountant = PrivacyAccountant()
        with pytest.raises(ValueError):
            accountant.step(noise_multiplier=1.0, batch_size=0, dataset_size=1000)

    def test_zero_dataset_size_raises(self):
        """Test that zero dataset size raises ValueError."""
        accountant = PrivacyAccountant()
        with pytest.raises(ValueError):
            accountant.step(noise_multiplier=1.0, batch_size=64, dataset_size=0)

    def test_zero_steps_is_noop(self):
        """Test that step with steps=0 doesn't change anything."""
        accountant = PrivacyAccountant()
        eps_before = accountant.get_spent_epsilon()
        accountant.step(noise_multiplier=1.0, batch_size=64, dataset_size=1000, steps=0)
        eps_after = accountant.get_spent_epsilon()
        assert eps_before == eps_after
        assert accountant.total_steps == 0

    def test_epsilon_positive_for_tight_delta(self):
        """Test that epsilon is a positive number for reasonable parameters."""
        accountant = PrivacyAccountant()
        accountant.step(noise_multiplier=1.0, batch_size=256, dataset_size=50_000, steps=1000)
        eps = accountant.get_spent_epsilon(delta=1e-5)
        assert eps > 0.0, f"Expected positive epsilon, got {eps}"
        assert eps < 100.0, f"Epsilon seems unreasonably large: {eps}"

    def test_epsilon_larger_with_smaller_delta(self):
        """Test that epsilon increases as we require smaller delta.

        A smaller δ means a stronger privacy guarantee for a given ε,
        so to achieve a smaller δ with the same noise, ε must be larger.
        """
        accountant = PrivacyAccountant()
        accountant.step(noise_multiplier=1.0, batch_size=256, dataset_size=50_000, steps=100)

        eps1 = accountant.get_spent_epsilon(delta=1e-3)
        eps2 = accountant.get_spent_epsilon(delta=1e-7)

        # Smaller δ should give larger ε (stricter guarantee)
        assert eps2 >= eps1, (
            f"Epsilon should be larger for smaller delta, "
            f"got eps(δ=1e-3)={eps1:.4f}, eps(δ=1e-7)={eps2:.4f}"
        )


class TestRDPAccountantState:
    """Test internal RDP accounting state."""

    def test_accumulate_empty(self):
        """Test that accumulate doesn't crash with reasonable params."""
        state = RDPAccountantState()
        state.accumulate(noise_multiplier=1.0, steps=10, sampling_rate=0.01)
        # Should have updated log_moments
        assert any(m > 0 for m in state.log_moments), "RDP should produce positive moments"

    def test_get_epsilon_zero_when_no_steps(self):
        """Test that get_epsilon returns 0 when no steps taken."""
        state = RDPAccountantState()
        eps = state.get_epsilon(delta=1e-5)
        assert eps == 0.0

    def test_rdp_gaussian_analytic(self):
        """Test RDP computation produces expected results."""
        state = RDPAccountantState()

        # Test with very small noise -> should produce large RDP
        eps_small_noise = state._compute_rdp_gaussian(sigma=0.1, q=0.01, alpha=2.0)
        assert eps_small_noise > 0, "RDP should be positive"

        # Test with very large noise -> should produce small RDP (near 0)
        eps_large_noise = state._compute_rdp_gaussian(sigma=100.0, q=0.01, alpha=2.0)
        assert eps_large_noise >= 0, "RDP should be non-negative"
        assert eps_large_noise < eps_small_noise, (
            "Larger noise should give smaller RDP"
        )

    def test_q_zero_returns_zero_rdp(self):
        """Test that sampling rate = 0 gives RDP = 0."""
        state = RDPAccountantState()
        eps = state._compute_rdp_gaussian(sigma=1.0, q=0.0, alpha=2.0)
        assert eps == 0.0

    def test_alpha_one_returns_zero_rdp(self):
        """Test that alpha = 1 gives RDP = 0."""
        state = RDPAccountantState()
        eps = state._compute_rdp_gaussian(sigma=1.0, q=0.5, alpha=1.0)
        assert eps == 0.0

    def test_log_moments_propagate(self):
        """Test that accumulate adds to log moments correctly."""
        state = RDPAccountantState()
        initial_moments = list(state.log_moments)

        state.accumulate(noise_multiplier=1.0, steps=5, sampling_rate=0.01)

        for i, (initial, current) in enumerate(zip(initial_moments, state.log_moments)):
            if current != initial:
                # At least some orders should have changed
                break
        else:
            pytest.fail("No log moments changed after accumulation")


class TestPrivacyBudget:
    """Test the PrivacyBudget dataclass."""

    def test_repr(self):
        """Test string representation."""
        from dpsynth.privacy.accountant import PrivacyBudget
        budget = PrivacyBudget(epsilon=1.0, delta=1e-5, steps=100)
        rep = repr(budget)
        assert "ε=" in rep
        assert "δ=" in rep
        assert "steps=" in rep

    def test_default_steps_zero(self):
        """Test that default steps is 0."""
        from dpsynth.privacy.accountant import PrivacyBudget
        budget = PrivacyBudget(epsilon=0.0, delta=1e-5)
        assert budget.steps == 0
