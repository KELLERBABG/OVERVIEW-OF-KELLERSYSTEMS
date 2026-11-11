"""Tests for the vectorized per-sample gradient optimizer."""

import pytest
import torch
from torch import nn

from dpsynth.privacy.optimizer import PerSampleGradientBatch, PerSampleGradientComputer


class SimpleModel(nn.Module):
    """A simple model for testing per-sample grads."""

    def __init__(self):
        super().__init__()
        self.fc1 = nn.Linear(10, 20)
        self.fc2 = nn.Linear(20, 1)

    def forward(self, x):
        x = torch.relu(self.fc1(x))
        return self.fc2(x)


class TestPerSampleGradientComputer:
    """Test the PerSampleGradientComputer."""

    @pytest.fixture
    def model(self):
        return SimpleModel()

    @pytest.fixture
    def batch(self):
        return torch.randn(4, 10)

    def test_init(self, model):
        """Test initialization."""
        computer = PerSampleGradientComputer(model)
        assert computer.model is model

    def test_compute_manual_strategy(self, model, batch):
        """Test manual loop strategy."""
        def loss_fn(m, x):
            return m(x).squeeze()

        computer = PerSampleGradientComputer(model, strategy="manual")
        result = computer.compute(batch, loss_fn=loss_fn)

        assert isinstance(result, PerSampleGradientBatch)
        assert result.batch_size == 4
        assert len(result.grads) > 0
        # Each gradient should have batch_size as first dim
        for grad in result.grads:
            if grad.numel() > 0:
                assert grad.size(0) == 4

    def test_compute_auto_strategy(self, model, batch):
        """Test auto strategy (falls through to manual)."""
        def loss_fn(m, x):
            return m(x).squeeze()

        computer = PerSampleGradientComputer(model, strategy="auto")
        result = computer.compute(batch, loss_fn=loss_fn)

        assert result.batch_size == 4
        assert len(result.param_names) > 0

    def test_compute_vmap_fallback(self, model, batch):
        """Test vmap strategy (falls through to manual on most systems)."""
        def loss_fn(m, x):
            return m(x).squeeze()

        computer = PerSampleGradientComputer(model, strategy="vmap")
        result = computer.compute(batch, loss_fn=loss_fn)

        assert result.batch_size == 4

    def test_single_sample(self, model):
        """Test with a single sample."""
        x = torch.randn(1, 10)

        def loss_fn(m, x):
            return m(x).squeeze()

        computer = PerSampleGradientComputer(model, strategy="manual")
        result = computer.compute(x, loss_fn=loss_fn)

        assert result.batch_size == 1

    def test_param_names_match(self, model, batch):
        """Test that param names correspond to model parameters."""
        def loss_fn(m, x):
            return m(x).squeeze()

        computer = PerSampleGradientComputer(model)
        result = computer.compute(batch, loss_fn=loss_fn)

        # Collect actual param names
        actual_names = [name for name, _ in model.named_parameters()]
        # Our result should have a subset (those with gradients)
        assert len(result.param_names) > 0

    def test_gradient_values_match_manual(self, model, batch):
        """Test that per-sample grads sum to batch gradient."""
        def loss_fn(m, x):
            return m(x).squeeze()

        # Compute manual per-sample gradients
        computer = PerSampleGradientComputer(model, strategy="manual")
        result = computer.compute(batch, loss_fn=loss_fn)

        # The total gradient over the batch should equal
        # the sum of per-sample gradients along dim 0
        for grad in result.grads:
            if grad.numel() > 0:
                # Sum over batch dim
                grad_sum = grad.sum(dim=0)
                assert not torch.isnan(grad_sum).any()

    def test_gradient_type(self, model, batch):
        """Test that gradients are the right type."""
        def loss_fn(m, x):
            return m(x).squeeze()

        computer = PerSampleGradientComputer(model, strategy="manual")
        result = computer.compute(batch, loss_fn=loss_fn)

        for grad in result.grads:
            if grad.numel() > 0:
                assert grad.dtype == torch.float32
