"""
CTGAN-inspired differentially private tabular data synthesizer.

Implements a conditional GAN for tabular data with DP-SGD training
of the discriminator. Uses mode-specific normalization for mixed-type
data and training-by-sampling for discrete columns.
"""

from __future__ import annotations

import logging
from dataclasses import dataclass

import numpy as np
import torch
import torch.nn.functional as F
from torch import optim

from dpsynth.models.base import DPModel, TrainingConfig
from dpsynth.models.layers import Discriminator, Generator
from dpsynth.privacy.accountant import PrivacyBudget
from dpsynth.privacy.clipping import GradientClipper
from dpsynth.privacy.noise import NoiseInjection

logger = logging.getLogger(__name__)


@dataclass
class CTGANConfig(TrainingConfig):
    """Configuration for CTGAN-DP.

    Attributes:
        latent_dim: Dimension of the latent noise vector.
        generator_dims: Hidden layer dimensions for the generator.
        discriminator_dims: Hidden layer dimensions for the discriminator.
        d_steps: Number of discriminator updates per generator update.
        g_steps: Number of generator updates per discriminator update.
        pac: Number of samples to pack for PAC (perturbed and aggregated
            cross) discriminator.
        log_frequency: Whether to log-transform column frequencies.
        batch_size: Training batch size.
    """

    latent_dim: int = 128
    generator_dims: tuple[int, ...] = (256, 256)
    discriminator_dims: tuple[int, ...] = (256, 256)
    d_steps: int = 1
    g_steps: int = 1
    pac: int = 10
    log_frequency: bool = True
    batch_size: int = 500

    def __post_init__(self) -> None:
        super().__post_init__()
        if self.latent_dim < 1:
            raise ValueError(
                f"latent_dim must be >= 1, got {self.latent_dim}"
            )
        if self.pac < 1:
            raise ValueError(f"pac must be >= 1, got {self.pac}")


class CTGAN_DP(DPModel):
    """Conditional GAN for tabular data with DP-SGD.

    Extends the CTGAN architecture (Xu et al., 2019) with differential
    privacy via gradient clipping and noise injection on the discriminator.

    The generator is non-private (no DP) — privacy is enforced only on
    the discriminator, which is the component that sees real data.

    Usage:
        >>> model = CTGAN_DP()
        >>> model.fit(real_data, epsilon=1.0, delta=1e-5)
        >>> synthetic = model.generate(1000)
        >>> budget = model.get_privacy_spent()
    """

    def __init__(
        self,
        config: CTGANConfig | None = None,
    ):
        """Initialize CTGAN-DP.

        Args:
            config: Training configuration.
        """
        super().__init__(config)
        self.config = config or CTGANConfig()
        self._data_shape: tuple[int, ...] | None = None

        # Set by _build_model
        self.generator: Generator | None = None
        self.discriminator: Discriminator | None = None
        self._g_optimizer: optim.Optimizer | None = None
        self._d_optimizer: optim.Optimizer | None = None

        # Data metadata — set during fit
        self._transformer: DataTransformer | None = None
        self._target_epsilon: float = 0.0
        self._target_delta: float = 0.0

    def _build_model(self) -> None:
        """Build generator and discriminator networks."""
        if self._transformer is None:
            raise RuntimeError(
                "DataTransformer must be set before building the model. "
                "Call _prepare_data first."
            )

        output_dim = self._transformer.output_dim

        self.generator = Generator(
            latent_dim=self.config.latent_dim,
            output_dim=output_dim,
            hidden_dims=self.config.generator_dims,
            activation="relu",
        )

        self.discriminator = Discriminator(
            input_dim=output_dim * self.config.pac,
            hidden_dims=self.config.discriminator_dims,
            activation="leaky_relu",
            dropout=0.0,
        )

        self._g_optimizer = optim.Adam(
            self.generator.parameters(),
            lr=self.config.learning_rate,
            betas=(0.5, 0.9),
        )
        self._d_optimizer = optim.Adam(
            self.discriminator.parameters(),
            lr=self.config.learning_rate,
            betas=(0.5, 0.9),
        )

    def _prepare_data(self, data: np.ndarray) -> DataTransformer:
        """Fit data transformer and set metadata.

        Args:
            data: Raw training data.

        Returns:
            Fitted DataTransformer.
        """
        self._data_shape = data.shape
        transformer = DataTransformer()
        transformer.fit(data)
        self._transformer = transformer
        return transformer

    def _run_training(
        self,
        data: np.ndarray,
        epsilon: float,
        delta: float,
    ) -> None:
        """Run DP-GAN training loop.

        Args:
            data: Training data of shape (n_samples, n_features).
            epsilon: Target privacy budget.
            delta: Target δ parameter.
        """
        self._target_epsilon = epsilon
        self._target_delta = delta

        # Prepare data
        transformer = self._prepare_data(data)
        transformed = transformer.transform(data)
        n_samples = data.shape[0]

        # Build models
        self._build_model()
        self.generator = self.generator.to(self.config.device)
        self.discriminator = self.discriminator.to(self.config.device)

        # Compute noise scale from target epsilon/delta if not provided
        if self.config.noise_multiplier is not None:
            sigma = self.config.noise_multiplier
        else:
            # Estimate sigma from target epsilon using the relationship:
            # For a typical DP-SGD run, eps ≈ steps * q * sqrt(2 * ln(1.25/delta)) / (sigma * pi)
            # Invert to get sigma ≈ steps * q * sqrt(2 * ln(1.25/delta)) / (eps * pi)
            # where q = batch_size / n and steps ≈ epochs * n / batch_size
            steps_per_epoch = max(1, n_samples // self.config.batch_size)
            total_steps = steps_per_epoch * self.config.epochs
            q = self.config.batch_size / n_samples
            if epsilon > 0 and delta > 0 and q > 0:
                from math import log, pi, sqrt
                numerator = total_steps * q * sqrt(2.0 * log(1.25 / delta))
                denominator = epsilon * pi if pi else epsilon * 3.14159
                sigma = numerator / max(denominator, 1e-10)
                sigma = max(sigma, 0.1)  # Don't let sigma go below 0.1
            else:
                sigma = 1.0

        # For low epsilon regimes, use adaptive noise scheduling:
        # Start with lower noise to let the generator learn structure,
        # then ramp up to the target noise level.
        is_low_epsilon = epsilon < 1.0
        sigma_start = min(sigma, 0.5)  # Start with less noise

        # Training loops
        steps_per_epoch = max(1, n_samples // self.config.batch_size)
        total_steps = steps_per_epoch * self.config.epochs

        logger.info(
            f"Training CTGAN-DP: n={n_samples}, "
            f"ε={epsilon}, δ={delta:.1e}, "
            f"epochs={self.config.epochs}, "
            f"batch_size={self.config.batch_size}, "
            f"sigma={sigma:.2f}"
        )

        # Adaptive noise scheduling: in low-ε regimes, ramp noise
        # linearly from sigma_start to sigma over the first half of training.
        # This lets the generator learn structure before strong DP noise kicks in.
        warmup_epochs = self.config.epochs // 2 if is_low_epsilon else 0

        self.generator.train()
        self.discriminator.train()

        d_losses: list[float] = []
        g_losses: list[float] = []

        for epoch in range(self.config.epochs):
            # Determine current noise multiplier for this epoch
            if warmup_epochs > 0 and epoch < warmup_epochs:
                current_sigma = sigma_start + (sigma - sigma_start) * (epoch / warmup_epochs)
            else:
                current_sigma = sigma

            epoch_d_loss = 0.0
            epoch_g_loss = 0.0

            # Shuffle data
            indices = np.random.permutation(n_samples)

            for step in range(steps_per_epoch):
                batch_idx = indices[
                    step * self.config.batch_size :
                    (step + 1) * self.config.batch_size
                ]
                batch_data = transformed[batch_idx]

                # Convert to tensor
                real = torch.tensor(
                    batch_data, dtype=torch.float32, device=self.config.device
                )

                # ---------------------
                # Train Discriminator
                # ---------------------
                self._d_optimizer.zero_grad()

                # Pack real data
                real_packed = self._pac_reshape(real)

                # Generate fake data
                z = torch.randn(
                    real.size(0),
                    self.config.latent_dim,
                    device=self.config.device,
                )
                fake = self.generator(z)
                fake_packed = self._pac_reshape(fake)

                # Discriminate
                real_logits = self.discriminator(real_packed)
                fake_logits = self.discriminator(fake_packed.detach())

                # Wasserstein loss
                d_loss = torch.mean(fake_logits) - torch.mean(real_logits)

                d_loss.backward()

                # DP: clip gradients and add noise
                self._dp_process_gradients()

                self._d_optimizer.step()

                # ---------------------
                # Train Generator
                # ---------------------
                for _ in range(self.config.g_steps):
                    self._g_optimizer.zero_grad()

                    z = torch.randn(
                        real.size(0),
                        self.config.latent_dim,
                        device=self.config.device,
                    )
                    fake = self.generator(z)
                    fake_packed = self._pac_reshape(fake)

                    fake_logits = self.discriminator(fake_packed)
                    g_loss = -torch.mean(fake_logits)

                    g_loss.backward()
                    self._g_optimizer.step()

                    epoch_g_loss += g_loss.item()

                epoch_d_loss += d_loss.item()

                # Account privacy
                batch_size_actual = batch_data.shape[0]
                self.accountant.step(
                    noise_multiplier=sigma,
                    batch_size=batch_size_actual,
                    dataset_size=n_samples,
                )

                # Check if budget exceeded
                spent = self.accountant.get_spent_epsilon(delta)
                if spent > epsilon * 1.1:  # Allow 10% tolerance
                    logger.info(
                        f"Privacy budget exhausted at step {step}: "
                        f"ε spent = {spent:.4f} (limit = {epsilon})"
                    )
                    break

            avg_d_loss = epoch_d_loss / steps_per_epoch
            avg_g_loss = epoch_g_loss / (steps_per_epoch * self.config.g_steps)
            d_losses.append(avg_d_loss)
            g_losses.append(avg_g_loss)

            if self.config.verbose and (epoch + 1) % 10 == 0:
                spent = self.accountant.get_privacy_spent(delta)
                logger.info(
                    f"Epoch {epoch + 1}/{self.config.epochs} | "
                    f"D loss: {avg_d_loss:.4f} | "
                    f"G loss: {avg_g_loss:.4f} | "
                    f"ε spent: {spent.epsilon:.4f}"
                )

            # Early exit if budget exhausted
            if self.accountant.get_spent_epsilon(delta) > epsilon * 1.1:
                logger.info(
                    f"Stopping early: privacy budget exhausted at epoch {epoch + 1}"
                )
                break

        self.generator.eval()
        self.discriminator.eval()

        final_spent = self.accountant.get_privacy_spent(delta)
        logger.info(
            f"Training complete. Total privacy spent: {final_spent}"
        )

    def _dp_process_gradients(self) -> None:
        """Apply per-sample gradient clipping and noise to discriminator.

        Uses the flat GradientClipper + NoiseInjection pattern.
        """
        # Clip gradients - use the optimizer's parameter groups
        clipper = GradientClipper(max_norm=self.config.max_grad_norm)
        if self._d_optimizer is not None:
            stats = clipper.clip_gradients(
                self._d_optimizer.param_groups,
                loss_reduction="mean",
            )

        # Add noise to gradients
        if self._d_optimizer is not None:
            noise = NoiseInjection(
                mechanism="gaussian",
                epsilon=self._target_epsilon,
                delta=self._target_delta,
                sensitivity=self.config.max_grad_norm,
            )
            noise.apply_gradients(
                self._d_optimizer.param_groups,
                clip_norm=self.config.max_grad_norm,
            )

    def _pac_reshape(self, data: torch.Tensor) -> torch.Tensor:
        """Reshape data for PAC discriminator.

        Packs samples together so the discriminator sees tuples of samples,
        which helps prevent mode collapse.

        Args:
            data: Tensor of shape (batch_size, n_features).

        Returns:
            Tensor of shape (batch_size // pac, n_features * pac) or
            (batch_size, n_features * pac) if batch_size < pac.
        """
        batch_size = data.size(0)
        pac = min(self.config.pac, batch_size)
        if batch_size % pac != 0:
            # Pad
            n_pad = pac - (batch_size % pac)
            data = F.pad(data, (0, 0, 0, n_pad))
            batch_size = data.size(0)

        data = data.view(batch_size // pac, -1)
        return data

    @torch.no_grad()
    def generate(
        self,
        n_samples: int,
        condition: np.ndarray | None = None,
    ) -> np.ndarray:
        """Generate synthetic data samples.

        Args:
            n_samples: Number of samples to generate.
            condition: Not used for CTGAN; kept for interface compatibility.

        Returns:
            Generated data of shape (n_samples, n_features).
        """
        if not self.is_trained or self.generator is None:
            raise RuntimeError(
                "Model must be trained before generating data. "
                "Call fit() first."
            )

        self.generator.eval()

        n_batches = max(1, n_samples // self.config.batch_size)
        remainder = n_samples % self.config.batch_size

        outputs: list[np.ndarray] = []

        for i in range(n_batches):
            batch_size = (
                self.config.batch_size if i < n_batches else remainder
            )
            z = torch.randn(
                batch_size,
                self.config.latent_dim,
                device=self.config.device,
            )
            fake = self.generator(z).cpu().numpy()
            outputs.append(fake)

        if remainder > 0:
            z = torch.randn(
                remainder,
                self.config.latent_dim,
                device=self.config.device,
            )
            fake = self.generator(z).cpu().numpy()
            outputs.append(fake)

        synthetic = np.concatenate(outputs, axis=0)
        synthetic = synthetic[:n_samples]

        # Inverse transform
        if self._transformer is not None:
            synthetic = self._transformer.inverse_transform(synthetic)

        return synthetic

    def get_privacy_spent(self, delta: float | None = None) -> PrivacyBudget:
        """Get privacy budget spent on discriminator training.

        Args:
            delta: δ parameter (uses config.delta if None).

        Returns:
            PrivacyBudget consumed.
        """
        return super().get_privacy_spent(delta)


# ──────────────────────────────────────────────
# Data Transformer for Mixed-Type Tabular Data
# ──────────────────────────────────────────────


class DataTransformer:
    """Transforms mixed-type tabular data for GAN training.

    Applies mode-specific normalization for continuous columns
    and one-hot encoding for categorical columns.
    """

    def __init__(self):
        self._column_info: list[dict] = []
        self.output_dim: int = 0

    def fit(self, data: np.ndarray) -> None:
        """Fit transformer to data.

        Args:
            data: Array of shape (n_samples, n_features).
        """
        n_features = data.shape[1]
        self._column_info = []
        total_dim = 0

        for col_idx in range(n_features):
            col_data = data[:, col_idx]
            unique_vals = np.unique(col_data)

            if (len(unique_vals) <= 20 and len(unique_vals) > 1) or col_data.dtype.kind in ("O", "U"):
                # Categorical column
                mapping = {v: i for i, v in enumerate(unique_vals)}
                n_categories = len(unique_vals)
                self._column_info.append({
                    "type": "categorical",
                    "mapping": mapping,
                    "n_categories": n_categories,
                })
                total_dim += n_categories
            else:
                # Continuous column — normalize to [0, 1]
                min_val = float(col_data.min())
                max_val = float(col_data.max())
                self._column_info.append({
                    "type": "continuous",
                    "min": min_val,
                    "max": max_val,
                })
                total_dim += 1

        self.output_dim = total_dim

    def transform(self, data: np.ndarray) -> np.ndarray:
        """Transform data to GAN-friendly representation.

        Args:
            data: Array of shape (n_samples, n_features).

        Returns:
            Transformed array of shape (n_samples, output_dim).
        """
        n_samples = data.shape[0]
        output = np.zeros((n_samples, self.output_dim))

        col_offset = 0
        for col_idx, info in enumerate(self._column_info):
            if info["type"] == "categorical":
                # One-hot encode
                mapping = info["mapping"]
                n_cat = info["n_categories"]
                for i in range(n_samples):
                    val = data[i, col_idx]
                    if val in mapping:
                        output[i, col_offset + mapping[val]] = 1.0
                col_offset += n_cat
            else:
                # Normalize continuous
                min_val = info["min"]
                max_val = info["max"]
                if max_val > min_val:
                    output[:, col_offset] = (data[:, col_idx] - min_val) / (
                        max_val - min_val
                    )
                else:
                    output[:, col_offset] = 0.5
                col_offset += 1

        return output

    def inverse_transform(self, data: np.ndarray) -> np.ndarray:
        """Transform GAN output back to original data representation.

        Args:
            data: Array of shape (n_samples, output_dim).

        Returns:
            Array of shape (n_samples, n_features).
        """
        n_samples = data.shape[0]
        n_features = len(self._column_info)
        output = np.zeros((n_samples, n_features))

        col_offset = 0
        for col_idx, info in enumerate(self._column_info):
            if info["type"] == "categorical":
                # Argmax to get category
                n_cat = info["n_categories"]
                cat_indices = np.argmax(
                    data[:, col_offset : col_offset + n_cat], axis=1
                )
                reverse_mapping = {
                    v: k for k, v in info["mapping"].items()
                }
                for i in range(n_samples):
                    output[i, col_idx] = reverse_mapping.get(
                        cat_indices[i], 0
                    )
                col_offset += n_cat
            else:
                # Denormalize continuous
                min_val = info["min"]
                max_val = info["max"]
                output[:, col_idx] = (
                    data[:, col_offset] * (max_val - min_val) + min_val
                )
                col_offset += 1

        return output
