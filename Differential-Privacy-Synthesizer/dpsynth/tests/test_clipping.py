"""Tests for the gradient clipping modules."""

import pytest
import torch
from torch import nn

from dpsynth.privacy.clipping import (
    AdaptiveGradientClipper,
    ClippingStats,
    GradientClipper,
    PerSampleGradientClipper,
)


class SimpleModel(nn.Module):
    """A simple model for testing."""

    def __init__(self):
        super().__init__()
        self.fc1 = nn.Linear(10, 20)
        self.fc2 = nn.Linear(20, 1)

    def forward(self, x):
        x = torch.relu(self.fc1(x))
        return self.fc2(x)


class TestGradientClipper:
    """Test the flat-threshold GradientClipper."""

    def test_init_default(self):
        """Test default initialization."""
        clipper = GradientClipper()
        assert clipper.max_norm == 1.0

    def test_init_custom_norm(self):
        """Test custom clipping threshold."""
        clipper = GradientClipper(max_norm=2.5)
        assert clipper.max_norm == 2.5

    def test_invalid_norm_raises(self):
        """Test that non-positive norm raises ValueError."""
        with pytest.raises(ValueError):
            GradientClipper(max_norm=0)
        with pytest.raises(ValueError):
            GradientClipper(max_norm=-1.0)

    def test_set_norm(self):
        """Test updating the clipping threshold."""
        clipper = GradientClipper(max_norm=1.0)
        clipper.set_norm(5.0)
        assert clipper.max_norm == 5.0

    def test_set_norm_raises(self):
        """Test that setting invalid norm raises ValueError."""
        clipper = GradientClipper()
        with pytest.raises(ValueError):
            clipper.set_norm(0)
        with pytest.raises(ValueError):
            clipper.set_norm(-1.0)

    def test_clip_parameters_no_grad(self):
        """Test clipping when no gradients exist."""
        model = SimpleModel()
        clipper = GradientClipper(max_norm=1.0)
        # No backward pass done, so gradients are None
        stats = clipper.clip_parameters(model)
        assert stats.clip_norm == 1.0
        assert stats.max_grad_norm == 0.0
        assert stats.mean_grad_norm == 0.0

    def test_clip_parameters_reduces_norm(self):
        """Test that clipping actually reduces gradient norms."""
        model = SimpleModel()
        x = torch.randn(4, 10)
        y = model(x).sum()
        y.backward()

        # Get max norm before clipping
        norms_before = []
        for param in model.parameters():
            if param.grad is not None:
                norms_before.append(param.grad.data.norm(2).item())
        max_before = max(norms_before)

        # Clip with aggressive threshold
        clipper = GradientClipper(max_norm=0.01)
        stats = clipper.clip_parameters(model)

        # Check that norms are now bounded
        for param in model.parameters():
            if param.grad is not None:
                assert param.grad.data.norm(2) <= 0.01 + 1e-6, (
                    f"Gradient norm {param.grad.data.norm(2):.6f} exceeds clip norm"
                )

        assert stats.clip_norm == 0.01
        assert stats.fraction_clipped > 0, "Expected some gradients to be clipped"

    def test_clip_gradients_optimizer_style(self):
        """Test clipping through optimizer-style parameter groups."""
        model = SimpleModel()
        x = torch.randn(4, 10)
        y = model(x).sum()
        y.backward()

        param_groups = [{"params": list(model.parameters())}]

        clipper = GradientClipper(max_norm=0.5)
        stats = clipper.clip_gradients(param_groups)

        for group in param_groups:
            for param in group["params"]:
                if param.grad is not None:
                    assert param.grad.data.norm(2) <= 0.5 + 1e-6

        assert stats.clip_norm == 0.5

    def test_clipping_stats_repr(self):
        """Test that ClippingStats has a useful repr."""
        stats = ClippingStats(
            clip_norm=1.0,
            fraction_clipped=0.5,
            max_grad_norm=2.0,
            mean_grad_norm=1.0,
        )
        rep = repr(stats)
        assert "C=" in rep
        assert "clipped=" in rep


class TestAdaptiveGradientClipper:
    """Test the adaptive gradient clipper."""

    def test_init(self):
        """Test initialization."""
        clipper = AdaptiveGradientClipper(initial_norm=2.0)
        assert clipper.max_norm == 2.0

    def test_adapt_threshold_changes_norm(self):
        """Test that threshold adapts after clipping."""
        clipper = AdaptiveGradientClipper(
            initial_norm=1.0,
            adaptation_rate=0.1,
            target_quantile=0.5,
        )
        initial_norm = clipper.max_norm

        model = SimpleModel()
        x = torch.randn(4, 10)
        y = model(x).sum()
        y.backward()

        stats = clipper.clip_parameters(model)
        # The norm should have changed after adaptation
        # (may increase or decrease depending on clipping fraction)
        assert clipper.max_norm != initial_norm


class TestPerSampleGradientClipper:
    """Test the per-sample gradient clipper."""

    def test_init(self):
        """Test initialization."""
        clipper = PerSampleGradientClipper(max_norm=1.0, micro_batch_size=2)
        assert clipper.max_norm == 1.0

    def test_invalid_norm_raises(self):
        """Test that non-positive norm raises ValueError."""
        with pytest.raises(ValueError):
            PerSampleGradientClipper(max_norm=0)

    def test_invalid_micro_batch_raises(self):
        """Test that invalid micro_batch_size raises ValueError."""
        with pytest.raises(ValueError):
            PerSampleGradientClipper(max_norm=1.0, micro_batch_size=0)

    def test_clip_per_sample(self):
        """Test per-sample clipping."""
        model = SimpleModel()
        x = torch.randn(1, 10)  # Single sample at a time

        clipper = PerSampleGradientClipper(max_norm=1.0, micro_batch_size=1)

        # Run the clipper with a single loss value
        output = model(x)
        loss = output.squeeze()

        stats = clipper.clip_per_sample(model, loss.unsqueeze(0))

        assert stats.clip_norm == 1.0
        assert stats.max_grad_norm >= 0
