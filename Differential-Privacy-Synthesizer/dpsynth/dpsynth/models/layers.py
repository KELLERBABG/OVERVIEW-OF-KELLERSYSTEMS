"""
DP-aware neural network layers for differentially private training.

Provides layer implementations that integrate with the privacy
accounting and noise injection systems.
"""

from __future__ import annotations

from collections.abc import Sequence

import torch
from torch import nn

from dpsynth.privacy.noise import GaussianNoiseLayer


class DPDense(nn.Module):
    """Differentially private dense (linear) layer.

    A fully-connected layer that can inject calibrated Gaussian noise
    into its activations during DP training. Integrates with the
    noise injection and clipping framework.

    Usage:
        >>> layer = DPDense(64, 128, activation='relu')
        >>> x = torch.randn(32, 64)
        >>> out = layer(x)  # shape: (32, 128)
        >>> layer.set_noise_multiplier(0.5)  # DP mode
    """

    def __init__(
        self,
        in_features: int,
        out_features: int,
        activation: str | None = None,
        dropout: float = 0.0,
        use_noise: bool = True,
        bias: bool = True,
    ):
        """Initialize DP dense layer.

        Args:
            in_features: Number of input features.
            out_features: Number of output features.
            activation: Activation function ('relu', 'tanh', 'leaky_relu',
                'sigmoid', or None).
            dropout: Dropout rate (0.0 = no dropout).
            use_noise: Whether to add Gaussian noise after the linear
                transformation (for DP training).
            bias: Whether to include a bias term.
        """
        super().__init__()
        self.linear = nn.Linear(in_features, out_features, bias=bias)
        self.dropout_layer = nn.Dropout(dropout) if dropout > 0 else None
        self.noise_layer = GaussianNoiseLayer(sigma=0.0) if use_noise else None
        self.activation_name = activation

        self._noise_multiplier = 0.0

        if activation == "relu":
            self.activation_fn = nn.ReLU()
        elif activation == "tanh":
            self.activation_fn = nn.Tanh()
        elif activation == "leaky_relu":
            self.activation_fn = nn.LeakyReLU(0.2)
        elif activation == "sigmoid":
            self.activation_fn = nn.Sigmoid()
        else:
            self.activation_fn = nn.Identity()

    def set_noise_multiplier(self, sigma: float) -> None:
        """Set the noise multiplier for Gaussian noise injection.

        Args:
            sigma: Noise multiplier σ.
        """
        self._noise_multiplier = sigma
        if self.noise_layer is not None:
            self.noise_layer.sigma = sigma

    @property
    def noise_multiplier(self) -> float:
        return self._noise_multiplier

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        """Forward pass with optional DP noise.

        Args:
            x: Input tensor of shape (batch_size, in_features).

        Returns:
            Output tensor of shape (batch_size, out_features).
        """
        out = self.linear(x)

        if self.noise_layer is not None and self._noise_multiplier > 0:
            out = self.noise_layer(out)

        out = self.activation_fn(out)

        if self.dropout_layer is not None:
            out = self.dropout_layer(out)

        return out

    def extra_repr(self) -> str:
        return (
            f"in_features={self.linear.in_features}, "
            f"out_features={self.linear.out_features}, "
            f"activation={self.activation_name}, "
            f"noise_sigma={self._noise_multiplier:.2e}"
        )


class DPSequential(nn.Sequential):
    """A sequential container that can set noise multipliers on all DP layers.

    Usage:
        >>> model = DPSequential(
        ...     DPDense(10, 64, activation='relu'),
        ...     DPDense(64, 32, activation='relu'),
        ...     DPDense(32, 5),
        ... )
        >>> model.set_noise_multiplier(0.5)
    """

    def set_noise_multiplier(self, sigma: float) -> None:
        """Set noise multiplier on all submodules that support it.

        Args:
            sigma: Noise multiplier σ.
        """
        for module in self:
            if isinstance(module, (DPDense, GaussianNoiseLayer)):
                module.set_noise_multiplier(sigma)
            elif hasattr(module, "set_noise_multiplier"):
                module.set_noise_multiplier(sigma)  # type: ignore


class Generator(nn.Module):
    """Standard generator network for DP-GAN.

    Maps latent noise vectors to synthetic data samples.
    """

    def __init__(
        self,
        latent_dim: int,
        output_dim: int,
        hidden_dims: Sequence[int] = (256, 256),
        activation: str = "relu",
        dropout: float = 0.0,
    ):
        """Initialize generator.

        Args:
            latent_dim: Dimension of the latent noise space.
            output_dim: Dimension of the output (data) space.
            hidden_dims: Dimensions of hidden layers.
            activation: Activation function.
            dropout: Dropout rate.
        """
        super().__init__()
        self.latent_dim = latent_dim

        layers: list[nn.Module] = []
        prev_dim = latent_dim
        for hidden_dim in hidden_dims:
            layers.append(
                DPDense(
                    prev_dim,
                    hidden_dim,
                    activation=activation,
                    dropout=dropout,
                    use_noise=False,
                )
            )
            prev_dim = hidden_dim

        # Output layer (no activation, no noise)
        layers.append(nn.Linear(prev_dim, output_dim))

        self.net = nn.Sequential(*layers)

    def forward(self, z: torch.Tensor) -> torch.Tensor:
        """Generate synthetic data from latent vectors.

        Args:
            z: Latent vectors of shape (batch_size, latent_dim).

        Returns:
            Generated data of shape (batch_size, output_dim).
        """
        return self.net(z)


class Discriminator(nn.Module):
    """Standard discriminator network for DP-GAN.

    The discriminator is where differential privacy is enforced
    via gradient clipping and noise injection during training.
    """

    def __init__(
        self,
        input_dim: int,
        hidden_dims: Sequence[int] = (256, 256),
        activation: str = "leaky_relu",
        dropout: float = 0.2,
    ):
        """Initialize discriminator.

        Args:
            input_dim: Dimension of the input data.
            hidden_dims: Dimensions of hidden layers.
            activation: Activation function (typically leaky_relu for GANs).
            dropout: Dropout rate.
        """
        super().__init__()

        use_noise = False

        layers: list[nn.Module] = []
        prev_dim = input_dim
        for hidden_dim in hidden_dims:
            layers.append(
                DPDense(
                    prev_dim,
                    hidden_dim,
                    activation=activation,
                    dropout=dropout,
                    use_noise=use_noise,
                )
            )
            prev_dim = hidden_dim

        # Output layer (single logit)
        layers.append(nn.Linear(prev_dim, 1))

        self.net = nn.Sequential(*layers)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        """Discriminate real vs. generated data.

        Args:
            x: Input data of shape (batch_size, input_dim).

        Returns:
            Logits of shape (batch_size, 1).
        """
        return self.net(x)
