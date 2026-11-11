"""
Base classes for differentially private models.

Defines the abstract interface that all DP models must implement:
train with a given privacy budget, generate synthetic data,
and report privacy spend.
"""

from __future__ import annotations

import abc
from dataclasses import dataclass

import numpy as np
from torch import nn

from dpsynth.privacy.accountant import PrivacyAccountant, PrivacyBudget


@dataclass
class TrainingConfig:
    """Configuration for DP model training.

    Attributes:
        epsilon: Total privacy budget ε to spend.
        delta: Privacy parameter δ (typically 1e-5 or 1e-6).
        epochs: Number of training epochs.
        batch_size: Batch size for training.
        learning_rate: Optimizer learning rate.
        noise_multiplier: σ parameter (if None, computed from ε/δ budget).
        max_grad_norm: Clipping threshold C.
        device: Device to train on ('cpu' or 'cuda').
        verbose: Whether to log training progress.
    """

    epsilon: float = 10.0
    delta: float = 1e-5
    epochs: int = 100
    batch_size: int = 64
    learning_rate: float = 1e-4
    noise_multiplier: float | None = None
    max_grad_norm: float = 1.0
    device: str = "cpu"
    verbose: bool = True

    def __post_init__(self) -> None:
        if self.epsilon <= 0:
            raise ValueError(f"epsilon must be positive, got {self.epsilon}")
        if self.delta <= 0 or self.delta >= 1:
            raise ValueError(f"delta must be in (0, 1), got {self.delta}")
        if self.epochs < 1:
            raise ValueError(f"epochs must be >= 1, got {self.epochs}")
        if self.batch_size < 1:
            raise ValueError(f"batch_size must be >= 1, got {self.batch_size}")


class DPModule(nn.Module):
    """Base class for DP-aware PyTorch modules.

    Extends nn.Module with privacy-related hooks.
    """

    def __init__(self) -> None:
        super().__init__()
        self._noise_multiplier: float = 0.0

    @property
    def noise_multiplier(self) -> float:
        """The noise multiplier used during training."""
        return self._noise_multiplier

    def set_noise_multiplier(self, sigma: float) -> None:
        """Set the noise multiplier for DP training.

        Args:
            sigma: Noise multiplier σ.
        """
        self._noise_multiplier = sigma


class DPModel(abc.ABC, nn.Module):
    """Abstract base for all differentially private synthesizer models.

    Provides a standard interface for:
    - Training with differential privacy guarantees
    - Generating synthetic data
    - Querying privacy budget consumption

    Usage:
        >>> class MyDPModel(DPModel):
        ...     def _build_model(self): ...
        ...     def generate(self, n_samples): ...
        >>> model = MyDPModel()
        >>> model.fit(real_data, epsilon=1.0, delta=1e-5)
        >>> synthetic = model.generate(1000)
        >>> budget = model.get_privacy_spent()
    """

    def __init__(
        self,
        config: TrainingConfig | None = None,
    ):
        """Initialize the DP model.

        Args:
            config: Training configuration. If None, defaults are used.
        """
        super().__init__()
        self.config = config or TrainingConfig()
        self.accountant = PrivacyAccountant()
        self._trained = False

    @abc.abstractmethod
    def _build_model(self) -> None:
        """Build the underlying neural network architecture.

        Subclasses must implement this to define their specific
        model architecture.
        """
        ...

    @abc.abstractmethod
    def generate(
        self,
        n_samples: int,
        condition: np.ndarray | None = None,
    ) -> np.ndarray:
        """Generate synthetic data samples.

        Args:
            n_samples: Number of synthetic samples to generate.
            condition: Optional conditioning vector for conditional generation.

        Returns:
            Array of shape (n_samples, n_features) with synthetic data.
        """
        ...

    def fit(
        self,
        data: np.ndarray,
        epsilon: float | None = None,
        delta: float | None = None,
    ) -> PrivacyBudget:
        """Train the model with differential privacy.

        Args:
            data: Training data of shape (n_samples, n_features).
            epsilon: Privacy budget ε. If None, uses config.epsilon.
            delta: Privacy parameter δ. If None, uses config.delta.

        Returns:
            PrivacyBudget with the total privacy cost of training.
        """
        epsilon = epsilon if epsilon is not None else self.config.epsilon
        delta = delta if delta is not None else self.config.delta

        self.train()
        self._run_training(data, epsilon, delta)
        self._trained = True
        self.eval()

        return self.get_privacy_spent()

    @abc.abstractmethod
    def _run_training(
        self,
        data: np.ndarray,
        epsilon: float,
        delta: float,
    ) -> None:
        """Internal training loop with DP-SGD.

        Subclasses implement this with their specific training procedure.

        Args:
            data: Training data.
            epsilon: Target privacy budget.
            delta: Target δ parameter.
        """
        ...

    def get_privacy_spent(self, delta: float | None = None) -> PrivacyBudget:
        """Get the total privacy budget consumed so far.

        Args:
            delta: δ parameter to compute ε for (uses config.delta if None).

        Returns:
            PrivacyBudget with (ε, δ) consumed.
        """
        delta = delta if delta is not None else self.config.delta
        return self.accountant.get_privacy_spent(delta)

    def reset_privacy_budget(self) -> None:
        """Reset the privacy accountant (e.g., for multi-stage training)."""
        self.accountant.reset()

    @property
    def is_trained(self) -> bool:
        """Whether the model has been trained at least once."""
        return self._trained
