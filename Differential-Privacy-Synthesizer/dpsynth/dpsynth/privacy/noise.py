"""
Noise injection modules for differential privacy.

Provides calibrated Laplace and Gaussian noise layers that inject
the correct amount of noise based on sensitivity (or clipping bound)
and the target privacy parameters (ε, δ).
"""

from __future__ import annotations

import logging
import math
from dataclasses import dataclass
from typing import Literal

import torch
from torch import nn

logger = logging.getLogger(__name__)


def gaussian_noise(tensor: torch.Tensor, sigma: float) -> torch.Tensor:
    """Add Gaussian noise with scale sigma to a tensor.

    Args:
        tensor: Input tensor.
        sigma: Noise scale (standard deviation).

    Returns:
        Tensor with noise added.
    """
    noise = torch.normal(
        mean=0.0,
        std=sigma,
        size=tensor.shape,
        device=tensor.device,
        dtype=tensor.dtype,
    )
    return tensor + noise


def laplace_noise(tensor: torch.Tensor, scale: float) -> torch.Tensor:
    """Add Laplace noise with scale b to a tensor.

    Args:
        tensor: Input tensor.
        scale: Noise scale b (Laplace has mean 0, variance 2*b²).

    Returns:
        Tensor with noise added.
    """
    noise = torch.distributions.Laplace(
        loc=0.0,
        scale=scale,
    ).sample(tensor.shape).to(device=tensor.device, dtype=tensor.dtype)
    return tensor + noise


@dataclass
class NoiseInjection:
    """Calibrated noise injection specification.

    Computes the correct noise scale for a given privacy budget,
    sensitivity, and mechanism.

    Attributes:
        mechanism: 'gaussian' or 'laplace'.
        epsilon: Privacy budget ε.
        delta: Privacy parameter δ (used only for Gaussian).
        sensitivity: L2 sensitivity of the function (for Gaussian)
            or L1 sensitivity (for Laplace).
        scale: Computed noise scale.
    """

    mechanism: Literal["gaussian", "laplace"]
    epsilon: float
    delta: float = 1e-5
    sensitivity: float = 1.0
    scale: float = 0.0

    def __post_init__(self) -> None:
        """Compute the noise scale from the given parameters."""
        if self.epsilon <= 0:
            raise ValueError(f"epsilon must be positive, got {self.epsilon}")
        if self.delta <= 0 or self.delta >= 1:
            raise ValueError(f"delta must be in (0, 1), got {self.delta}")
        if self.sensitivity <= 0:
            raise ValueError(
                f"sensitivity must be positive, got {self.sensitivity}"
            )

        if self.mechanism == "gaussian":
            # For Gaussian: σ = sqrt(2 * ln(1.25/δ)) * Δf / ε
            # This gives (ε, δ)-DP
            self.scale = (
                self.sensitivity
                * math.sqrt(2.0 * math.log(1.25 / self.delta))
                / self.epsilon
            )
        elif self.mechanism == "laplace":
            # For Laplace: b = Δf / ε
            self.scale = self.sensitivity / self.epsilon
        else:
            raise ValueError(
                f"Unknown mechanism: {self.mechanism}. Use 'gaussian' or 'laplace'."
            )

    def apply(self, tensor: torch.Tensor) -> torch.Tensor:
        """Inject calibrated noise into a tensor.

        Args:
            tensor: The tensor to add noise to.

        Returns:
            Noisy tensor.
        """
        if self.mechanism == "gaussian":
            return gaussian_noise(tensor, self.scale)
        else:
            return laplace_noise(tensor, self.scale)

    def apply_gradients(
        self, parameter_groups: list[dict], clip_norm: float
    ) -> float:
        """Add calibrated noise to model gradients.

        Assumes gradients have already been clipped to clip_norm.
        Noise is scaled to clip_norm * sigma where sigma is computed
        from the privacy parameters.

        Args:
            parameter_groups: List of optimizer parameter groups.
            clip_norm: The clipping threshold C used.

        Returns:
            The noise scale (sigma * clip_norm) that was applied.
        """
        effective_scale = self.scale * clip_norm

        for group in parameter_groups:
            for param in group["params"]:
                if param.grad is None:
                    continue
                noise = torch.normal(
                    mean=0.0,
                    std=effective_scale,
                    size=param.grad.shape,
                    device=param.grad.device,
                    dtype=param.grad.dtype,
                )
                param.grad.data.add_(noise)

        return effective_scale

    def __repr__(self) -> str:
        return (
            f"NoiseInjection(mechanism={self.mechanism}, "
            f"ε={self.epsilon}, δ={self.delta:.1e}, "
            f"sensitivity={self.sensitivity}, scale={self.scale:.4f})"
        )


class GaussianNoiseLayer(nn.Module):
    """PyTorch module that adds Gaussian noise to its input.

    Useful as a layer in DP neural networks, placed after activations
    or before output layers to provide differential privacy.

    The noise scale σ is computed from the target (ε, δ) budget and
    the L2 sensitivity of the preceding computation.

    Usage:
        >>> noise_layer = GaussianNoiseLayer(sigma=1.0)
        >>> x = torch.randn(32, 64)
        >>> noisy_x = noise_layer(x)  # x + N(0, 1.0)
    """

    def __init__(
        self,
        sigma: float = 1.0,
        noise_multiplier: float | None = None,
    ):
        """Initialize Gaussian noise layer.

        Args:
            sigma: Noise scale (standard deviation). Can be 0.0 for
                no-op mode (no noise added).
            noise_multiplier: Alternative: specify noise multiplier
                for DP-SGD style accounting.
        """
        super().__init__()
        if noise_multiplier is not None:
            sigma = noise_multiplier
        if sigma < 0:
            raise ValueError(f"sigma must be non-negative, got {sigma}")
        self.sigma = sigma

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        """Add Gaussian noise to input.

        Args:
            x: Input tensor.

        Returns:
            x + noise where noise ~ N(0, sigma²)
        """
        return gaussian_noise(x, self.sigma)

    def extra_repr(self) -> str:
        return f"sigma={self.sigma}"


class LaplaceNoiseLayer(nn.Module):
    """PyTorch module that adds Laplace noise to its input.

    Provides ε-DP (pure differential privacy) when used with
    appropriate sensitivity calibration.

    Usage:
        >>> noise_layer = LaplaceNoiseLayer(scale=0.5)
        >>> x = torch.randn(32, 64)
        >>> noisy_x = noise_layer(x)  # x + Lap(0, 0.5)
    """

    def __init__(self, scale: float = 1.0):
        """Initialize Laplace noise layer.

        Args:
            scale: Noise scale b (Laplace variance = 2*b²).
        """
        super().__init__()
        if scale <= 0:
            raise ValueError(f"scale must be positive, got {scale}")
        self.scale = scale

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        """Add Laplace noise to input.

        Args:
            x: Input tensor.

        Returns:
            x + noise where noise ~ Lap(0, scale)
        """
        return laplace_noise(x, self.scale)

    def extra_repr(self) -> str:
        return f"scale={self.scale}"
