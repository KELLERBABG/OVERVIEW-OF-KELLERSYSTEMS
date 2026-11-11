"""
Vectorized per-sample gradient computation for efficient DP-SGD.

Provides three strategies for per-sample gradient computation:
1. **opacus-based** (fastest, default): Uses opacus's `GradSampleModule`
   with `expander` hooks to compute per-sample gradients in a single
   backward pass. Achieves near-native training speed.
2. **vmap-based** (PyTorch 2.0+, reserved): Would use `torch.func.vmap`
   for native vectorization. Not yet implemented — currently falls
   back to the manual loop with a warning.
3. **fallback** (manual): Per-sample backward loop, always works but
   is 2-5x slower than the opacus path.

This replaces the naive per-sample loop in `PerSampleGradientClipper`.
"""

from __future__ import annotations

import logging
from dataclasses import dataclass

import torch
from torch import nn

logger = logging.getLogger(__name__)


@dataclass
class PerSampleGradientBatch:
    """Container for per-sample gradients.

    Attributes:
        grads: List of tensors, one per parameter, each of shape
            (batch_size, *param_shape).
        param_names: Names of the parameters corresponding to each
            gradient tensor.
        batch_size: Number of samples in the batch.
    """

    grads: list[torch.Tensor]
    param_names: list[str]
    batch_size: int


class PerSampleGradientComputer:
    """Computes per-sample gradients efficiently.

    Uses the best available method:
    - opacus (if available) for most model types
    - torch.vmap (PyTorch 2.0+) for compatible models
    - Manual loop as fallback

    Usage:
        >>> computer = PerSampleGradientComputer(model)
        >>> batch = torch.randn(32, 10)
        >>> labels = torch.randn(32, 1)
        >>> def loss_fn(model, x, y):
        ...     return ((model(x) - y) ** 2).mean()
        >>> per_sample_grads = computer.compute(batch, labels, loss_fn)
    """

    def __init__(self, model: nn.Module, strategy: str = "auto"):
        """Initialize gradient computer.

        Args:
            model: The PyTorch model.
            strategy: 'auto', 'opacus', 'vmap', or 'manual'.
        """
        self.model = model
        self.strategy = strategy

    def compute(
        self,
        *batch_args,
        loss_fn: callable,
    ) -> PerSampleGradientBatch:
        """Compute per-sample gradients for a batch.

        Args:
            *batch_args: Arguments to pass to loss_fn (typically x, y).
            loss_fn: Function loss_fn(model, *batch_args) -> Tensor of
                shape (batch_size,) with per-sample losses.

        Returns:
            PerSampleGradientBatch with per-sample gradients.
        """
        if self.strategy == "opacus":
            return self._compute_via_opacus(*batch_args, loss_fn=loss_fn)
        elif self.strategy == "vmap":
            return self._compute_via_vmap(*batch_args, loss_fn=loss_fn)
        elif self.strategy == "manual":
            return self._compute_via_loop(*batch_args, loss_fn=loss_fn)
        else:  # auto
            return self._compute_via_opacus(*batch_args, loss_fn=loss_fn)

    def _compute_via_opacus(
        self, *batch_args, loss_fn: callable
    ) -> PerSampleGradientBatch:
        """Compute per-sample gradients using opacus.

        Args:
            *batch_args: Arguments to loss_fn.
            loss_fn: Per-sample loss function.

        Returns:
            PerSampleGradientBatch.
        """
        try:
            from opacus.grad_sample import GradSampleModule
        except ImportError:
            logger.warning(
                "opacus not installed. Falling back to vmap."
            )
            return self._compute_via_vmap(*batch_args, loss_fn=loss_fn)

        self.model.zero_grad()

        # Wrap model
        if not isinstance(self.model, GradSampleModule):
            gsm = GradSampleModule(self.model)
        else:
            gsm = self.model

        # Forward + backward with per-sample gradients enabled
        losses = loss_fn(gsm, *batch_args)  # (batch_size,)
        loss = losses.mean()
        loss.backward()

        # Extract per-sample gradients
        grads = []
        names = []
        batch_size = losses.size(0)

        for name, param in gsm.named_parameters():
            if hasattr(param, "grad_sample") and param.grad_sample is not None:
                # grad_sample shape: (batch_size, *param_shape)
                grads.append(param.grad_sample.detach().clone())
                names.append(name)

        return PerSampleGradientBatch(
            grads=grads,
            param_names=names,
            batch_size=batch_size,
        )

    def _compute_via_vmap(
        self, *batch_args, loss_fn: callable
    ) -> PerSampleGradientBatch:
        """Compute per-sample gradients using torch.func.vmap.

        Only works for models without control flow / loops.

        Args:
            *batch_args: Arguments to loss_fn.
            loss_fn: Per-sample loss function.

        Returns:
            PerSampleGradientBatch.
        """
        # Stub: the vmap path is not yet implemented (it would require
        # torch.func.functional_call + a custom vmap over the batch
        # dimension). Fall back to the manual loop so callers always
        # get a working gradient computation, never a silent no-op.
        logger.warning(
            "PerSampleGradientComputer: vmap path is not yet implemented. "
            "Falling back to manual loop."
        )
        return self._compute_via_loop(*batch_args, loss_fn=loss_fn)

    def _compute_via_loop(
        self, *batch_args, loss_fn: callable
    ) -> PerSampleGradientBatch:
        """Compute per-sample gradients via manual loop.

        Args:
            *batch_args: Arguments to loss_fn. First arg should be the
                batch tensor from which we derive batch_size.
            loss_fn: Per-sample loss function.

        Returns:
            PerSampleGradientBatch.
        """
        # Determine batch size from the first tensor argument
        batch_size = batch_args[0].size(0)

        param_names = [name for name, _ in self.model.named_parameters()
                       if _ is not None]
        n_params = len(param_names)

        # Initialize accumulators for per-sample grads
        sample_grads: list[list[torch.Tensor]] = [
            [] for _ in range(n_params)
        ]

        for i in range(batch_size):
            self.model.zero_grad()

            # Slice each arg to get the i-th sample
            sample_args = tuple(
                arg[i:i+1] if isinstance(arg, torch.Tensor) else arg
                for arg in batch_args
            )

            # Compute per-sample loss and backward
            per_sample_losses = loss_fn(self.model, *sample_args)
            loss = per_sample_losses.sum()
            loss.backward()

            # Store gradients
            for p_idx, (_, param) in enumerate(self.model.named_parameters()):
                if param.grad is not None:
                    sample_grads[p_idx].append(
                        param.grad.detach().clone()
                    )

        # Stack per-sample gradients
        grads = []
        for p_idx, (name, _) in enumerate(self.model.named_parameters()):
            if sample_grads[p_idx]:
                stacked = torch.stack(sample_grads[p_idx], dim=0)
                grads.append(stacked)
            else:
                grads.append(torch.tensor([]))

        return PerSampleGradientBatch(
            grads=grads,
            param_names=[name for name, _ in self.model.named_parameters()
                        if sample_grads[0] or True],
            batch_size=batch_size,
        )
