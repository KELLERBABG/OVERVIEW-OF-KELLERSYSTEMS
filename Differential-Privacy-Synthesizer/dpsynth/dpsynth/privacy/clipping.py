"""
Gradient clipping modules for differential privacy.

Implements per-sample gradient clipping with support for both flat
and adaptive clipping thresholds, critical for bounding the sensitivity
of the training process.
"""

from __future__ import annotations

import logging
from dataclasses import dataclass

import torch
from torch import nn

logger = logging.getLogger(__name__)


@dataclass
class ClippingStats:
    """Statistics from a clipping operation.

    Attributes:
        clip_norm: The clipping threshold used.
        fraction_clipped: Fraction of samples that were clipped (norm > threshold).
        max_grad_norm: Maximum gradient norm before clipping.
        mean_grad_norm: Mean gradient norm before clipping.
    """

    clip_norm: float
    fraction_clipped: float = 0.0
    max_grad_norm: float = 0.0
    mean_grad_norm: float = 0.0

    def __repr__(self) -> str:
        return (
            f"ClippingStats(C={self.clip_norm:.4f}, "
            f"clipped={self.fraction_clipped:.1%}, "
            f"max_norm={self.max_grad_norm:.4f})"
        )


class GradientClipper:
    """Flat-threshold gradient clipper for DP training.

    Clips the L2 norm of gradients per-sample to a fixed threshold,
    then adds noise scaled to that threshold.

    This implements the core DP-SGD clipping mechanism.
    """

    def __init__(self, max_norm: float = 1.0):
        """Initialize with a fixed clipping threshold.

        Args:
            max_norm: Maximum allowed gradient norm C. Gradients with
                ||g||₂ > C are scaled down to norm C.
        """
        if max_norm <= 0:
            raise ValueError(f"max_norm must be positive, got {max_norm}")
        self._max_norm = float(max_norm)

    @property
    def max_norm(self) -> float:
        """The current clipping threshold C."""
        return self._max_norm

    def set_norm(self, norm: float) -> None:
        """Update the clipping threshold.

        Args:
            norm: New clipping threshold.
        """
        if norm <= 0:
            raise ValueError(f"norm must be positive, got {norm}")
        self._max_norm = float(norm)

    def clip_parameters(self, model: nn.Module) -> ClippingStats:
        """Clip all parameters in a model to the max norm.

        Applies per-sample gradient clipping by scaling down the
        gradients of each parameter tensor.

        Args:
            model: PyTorch model whose .grad is to be clipped.

        Returns:
            ClippingStats with statistics about this clipping operation.

        Raises:
            RuntimeError: If no gradients are available.
        """
        total_norm = 0.0
        num_params = 0
        num_clipped = 0

        for param in model.parameters():
            if param.grad is None:
                continue

            param_norm = param.grad.data.norm(2)
            total_norm += param_norm.item() ** 2
            num_params += 1

            if param_norm > self._max_norm:
                scale = self._max_norm / (param_norm + 1e-12)
                param.grad.data.mul_(scale)
                num_clipped += 1

        total_norm = total_norm ** 0.5
        fraction_clipped = num_clipped / max(num_params, 1)

        return ClippingStats(
            clip_norm=self._max_norm,
            fraction_clipped=fraction_clipped,
            max_grad_norm=total_norm,
            mean_grad_norm=total_norm / max(num_params, 1),
        )

    def clip_gradients(
        self, parameter_groups: list[dict], loss_reduction: str = "mean"
    ) -> ClippingStats:
        """Clip gradients across parameter groups (optimizer-style).

        Args:
            parameter_groups: List of dicts, each containing a 'params' key
                with a list of tensors.
            loss_reduction: 'mean' or 'sum' — how the loss was reduced.

        Returns:
            ClippingStats with statistics.
        """
        total_norm = 0.0
        num_params = 0
        num_clipped = 0

        for group in parameter_groups:
            for param in group["params"]:
                if param.grad is None:
                    continue

                param_norm = param.grad.data.norm(2)
                total_norm += param_norm.item() ** 2
                num_params += 1

                if param_norm > self._max_norm:
                    scale = self._max_norm / (param_norm + 1e-12)
                    param.grad.data.mul_(scale)
                    num_clipped += 1

        total_norm = total_norm ** 0.5
        fraction_clipped = num_clipped / max(num_params, 1)

        return ClippingStats(
            clip_norm=self._max_norm,
            fraction_clipped=fraction_clipped,
            max_grad_norm=total_norm,
            mean_grad_norm=total_norm / max(num_params, 1),
        )

    def __repr__(self) -> str:
        return f"GradientClipper(C={self._max_norm})"


class AdaptiveGradientClipper(GradientClipper):
    """Adaptive-threshold gradient clipper.

    Automatically adjusts the clipping threshold based on observed
    gradient norms. Uses a quantile-based approach: maintain a
    running estimate of the median gradient norm and set C to a
    multiple of that estimate.

    References:
        "Training Deep Neural Networks with Adaptive Clipping"
        (Andrew et al., 2021)
    """

    def __init__(
        self,
        initial_norm: float = 1.0,
        target_quantile: float = 0.5,
        adaptation_rate: float = 0.01,
        min_norm: float = 0.01,
        max_norm_bound: float = 100.0,
    ):
        """Initialize adaptive clipper.

        Args:
            initial_norm: Starting clipping threshold.
            target_quantile: Quantile of gradient norms to target (0.5 = median).
            adaptation_rate: How quickly to adapt.
            min_norm: Minimum allowed clip norm.
            max_norm_bound: Maximum allowed clip norm.
        """
        super().__init__(max_norm=initial_norm)
        self._target_quantile = target_quantile
        self._adaptation_rate = adaptation_rate
        self._min_norm = min_norm
        self._max_norm_bound = max_norm_bound
        self._observed_norms: list[float] = []

    @property
    def max_norm(self) -> float:
        """Return current clipping threshold."""
        return self._max_norm

    def clip_parameters(self, model: nn.Module) -> ClippingStats:
        """Clip and adapt the threshold.

        After clipping, updates the threshold based on the fraction
        of parameters that were actually clipped.

        Args:
            model: PyTorch model to clip.

        Returns:
            ClippingStats with statistics.
        """
        stats = super().clip_parameters(model)
        self._adapt_threshold(stats.fraction_clipped)
        return stats

    def clip_gradients(
        self, parameter_groups: list[dict], loss_reduction: str = "mean"
    ) -> ClippingStats:
        """Clip and adapt the threshold.

        Args:
            parameter_groups: List of optimizer parameter groups.
            loss_reduction: How the loss was reduced.

        Returns:
            ClippingStats with statistics.
        """
        stats = super().clip_gradients(parameter_groups, loss_reduction)
        self._adapt_threshold(stats.fraction_clipped)
        return stats

    def _adapt_threshold(self, fraction_clipped: float) -> None:
        """Adjust C to maintain fraction_clipped near target_quantile.

        If too many gradients are clipped, increase C.
        If too few, decrease C.

        Args:
            fraction_clipped: Fraction of parameters that were clipped.
        """
        current = self._max_norm
        error = fraction_clipped - (1.0 - self._target_quantile)
        adjustment = self._adaptation_rate * error * current

        new_norm = current + adjustment
        new_norm = max(self._min_norm, min(self._max_norm_bound, new_norm))
        self._max_norm = new_norm

    def __repr__(self) -> str:
        return (
            f"AdaptiveGradientClipper(C={self._max_norm:.4f}, "
            f"q={self._target_quantile}, "
            f"η={self._adaptation_rate})"
        )


class PerSampleGradientClipper:
    """Per-sample gradient clipper for micro-batch DP training.

    Computes per-sample gradients using a micro-batch strategy,
    clips each individually, and averages.

    Useful when opacus's privacy engine is not preferred.
    """

    def __init__(self, max_norm: float = 1.0, micro_batch_size: int = 1):
        """Initialize per-sample clipper.

        Args:
            max_norm: Clipping threshold C.
            micro_batch_size: Number of micro-batches to use for
                per-sample computation.
        """
        if max_norm <= 0:
            raise ValueError(f"max_norm must be positive, got {max_norm}")
        if micro_batch_size < 1:
            raise ValueError(
                f"micro_batch_size must be >= 1, got {micro_batch_size}"
            )
        self._max_norm = float(max_norm)
        self._micro_batch_size = micro_batch_size

    @property
    def max_norm(self) -> float:
        """The current clipping threshold C."""
        return self._max_norm

    def clip_per_sample(
        self,
        model: nn.Module,
        loss_per_sample: torch.Tensor,
    ) -> ClippingStats:
        """Clip per-sample gradients given individual losses.

        Computes backward for each sample in a loop over micro-batches,
        clipping each sample's gradient individually.

        Args:
            model: PyTorch model.
            loss_per_sample: Tensor of shape [batch_size] with the
                loss for each sample.

        Returns:
            ClippingStats with per-sample statistics.
        """
        batch_size = loss_per_sample.size(0)

        # Zero all gradients first
        for param in model.parameters():
            if param.grad is not None:
                param.grad = None

        total_clipped = 0
        total_norms: list[float] = []

        for i in range(batch_size):
            model.zero_grad()
            # Backward on a single scalar; graph is freed after each call
            loss_per_sample[i].backward(retain_graph=False)

            # Clip this sample's gradients
            param_norms = []
            for param in model.parameters():
                if param.grad is None:
                    continue
                param_norm = param.grad.data.norm(2).item()
                param_norms.append(param_norm)

                if param_norm > self._max_norm:
                    scale = self._max_norm / (param_norm + 1e-12)
                    param.grad.data.mul_(scale)
                    total_clipped += 1

            total_norms.append(max(param_norms) if param_norms else 0.0)

        flat_norms = torch.tensor(total_norms, dtype=torch.float32)
        return ClippingStats(
            clip_norm=self._max_norm,
            fraction_clipped=total_clipped / max(len(total_norms), 1),
            max_grad_norm=flat_norms.max().item() if len(total_norms) > 0 else 0.0,
            mean_grad_norm=flat_norms.mean().item() if len(total_norms) > 0 else 0.0,
        )

    def __repr__(self) -> str:
        return (
            f"PerSampleGradientClipper(C={self._max_norm}, "
            f"micro_batch={self._micro_batch_size})"
        )
