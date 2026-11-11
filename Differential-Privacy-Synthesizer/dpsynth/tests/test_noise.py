"""Tests for noise injection modules."""

import math

import pytest
import torch

from dpsynth.privacy.noise import (
    GaussianNoiseLayer,
    LaplaceNoiseLayer,
    NoiseInjection,
    gaussian_noise,
    laplace_noise,
)


class TestNoiseFunctions:
    """Test basic noise functions."""

    def test_gaussian_noise_shape(self):
        """Test that Gaussian noise preserves shape."""
        x = torch.randn(32, 64)
        noisy = gaussian_noise(x, sigma=0.5)
        assert noisy.shape == x.shape

    def test_gaussian_noise_different_values(self):
        """Test that noise actually changes values."""
        x = torch.randn(10, 10)
        noisy = gaussian_noise(x, sigma=1.0)
        assert not torch.allclose(x, noisy)

    def test_gaussian_noise_zero_sigma_identity(self):
        """Test that sigma=0 does nothing (returns x)."""
        x = torch.randn(10, 10)
        noisy = gaussian_noise(x, sigma=0.0)
        assert torch.allclose(x, noisy)

    def test_laplace_noise_shape(self):
        """Test that Laplace noise preserves shape."""
        x = torch.randn(32, 64)
        noisy = laplace_noise(x, scale=0.5)
        assert noisy.shape == x.shape

    def test_laplace_noise_different_values(self):
        """Test that Laplace noise actually changes values."""
        x = torch.randn(10, 10)
        noisy = laplace_noise(x, scale=1.0)
        assert not torch.allclose(x, noisy)


class TestNoiseInjection:
    """Test the NoiseInjection dataclass."""

    def test_gaussian_scale_computation(self):
        """Test that Gaussian scale is computed correctly."""
        noise = NoiseInjection(mechanism="gaussian", epsilon=1.0, delta=1e-5, sensitivity=1.0)
        expected_scale = math.sqrt(2.0 * math.log(1.25 / 1e-5)) / 1.0
        assert abs(noise.scale - expected_scale) < 1e-6

    def test_laplace_scale_computation(self):
        """Test that Laplace scale is computed correctly."""
        noise = NoiseInjection(mechanism="laplace", epsilon=0.5, sensitivity=2.0)
        expected_scale = 2.0 / 0.5  # sensitivity / epsilon
        assert abs(noise.scale - expected_scale) < 1e-6

    def test_gaussian_apply(self):
        """Test that apply() adds noise to tensor."""
        noise = NoiseInjection(mechanism="gaussian", epsilon=10.0, delta=1e-5, sensitivity=1.0)
        x = torch.ones(5, 5)
        noisy = noise.apply(x)
        assert noisy.shape == x.shape
        assert not torch.allclose(x, noisy)

    def test_laplace_apply(self):
        """Test that apply() adds Laplace noise to tensor."""
        noise = NoiseInjection(mechanism="laplace", epsilon=10.0, sensitivity=1.0)
        x = torch.ones(5, 5)
        noisy = noise.apply(x)
        assert noisy.shape == x.shape

    def test_invalid_mechanism_raises(self):
        """Test that unknown mechanism raises ValueError."""
        with pytest.raises(ValueError):
            NoiseInjection(mechanism="unknown", epsilon=1.0)

    def test_invalid_epsilon_raises(self):
        """Test that non-positive epsilon raises ValueError."""
        with pytest.raises(ValueError):
            NoiseInjection(mechanism="gaussian", epsilon=0, delta=1e-5)

    def test_invalid_delta_raises(self):
        """Test that invalid delta raises ValueError."""
        with pytest.raises(ValueError):
            NoiseInjection(mechanism="gaussian", epsilon=1.0, delta=0)
        with pytest.raises(ValueError):
            NoiseInjection(mechanism="gaussian", epsilon=1.0, delta=1.0)

    def test_invalid_sensitivity_raises(self):
        """Test that non-positive sensitivity raises ValueError."""
        with pytest.raises(ValueError):
            NoiseInjection(mechanism="gaussian", epsilon=1.0, sensitivity=0)

    def test_apply_gradients(self):
        """Test applying noise to gradients."""
        noise = NoiseInjection(mechanism="gaussian", epsilon=10.0, delta=1e-5, sensitivity=1.0)

        param = torch.nn.Parameter(torch.randn(10, 10))
        param.grad = torch.ones_like(param) * 0.5

        param_groups = [{"params": [param]}]
        scale = noise.apply_gradients(param_groups, clip_norm=1.0)

        assert scale > 0
        assert param.grad is not None
        # Gradients should have been changed from the original 0.5
        assert not torch.allclose(param.grad, torch.ones_like(param.grad) * 0.5)


class TestGaussianNoiseLayer:
    """Test GaussianNoiseLayer module."""

    def test_init_default(self):
        """Test default initialization."""
        layer = GaussianNoiseLayer(sigma=0.5)
        assert layer.sigma == 0.5

    def test_init_noise_multiplier(self):
        """Test initialization with noise_multiplier."""
        layer = GaussianNoiseLayer(noise_multiplier=1.5)
        assert layer.sigma == 1.5

    def test_invalid_sigma_raises(self):
        """Test that negative sigma raises ValueError."""
        with pytest.raises(ValueError):
            GaussianNoiseLayer(sigma=-1.0)

    def test_forward(self):
        """Test forward pass adds noise."""
        layer = GaussianNoiseLayer(sigma=1.0)
        x = torch.randn(32, 64)
        noisy = layer(x)
        assert noisy.shape == x.shape
        assert not torch.allclose(x, noisy)

    def test_forward_zero_sigma(self):
        """Test that sigma=0.0 passes through unchanged (but it's not allowed by constructor)."""
        # Through set later
        layer = GaussianNoiseLayer(sigma=1.0)
        layer.sigma = 0.0
        x = torch.randn(5, 5)
        noisy = layer(x)
        assert torch.allclose(x, noisy)


class TestLaplaceNoiseLayer:
    """Test LaplaceNoiseLayer module."""

    def test_init(self):
        """Test initialization."""
        layer = LaplaceNoiseLayer(scale=0.5)
        assert layer.scale == 0.5

    def test_invalid_scale_raises(self):
        """Test that non-positive scale raises ValueError."""
        with pytest.raises(ValueError):
            LaplaceNoiseLayer(scale=0)

    def test_forward(self):
        """Test forward pass adds noise."""
        layer = LaplaceNoiseLayer(scale=1.0)
        x = torch.randn(32, 64)
        noisy = layer(x)
        assert noisy.shape == x.shape
        assert not torch.allclose(x, noisy)
