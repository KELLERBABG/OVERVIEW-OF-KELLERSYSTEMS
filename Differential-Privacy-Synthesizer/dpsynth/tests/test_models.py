"""Tests for model base classes and layers."""

import numpy as np
import pytest
import torch

from dpsynth.models.base import DPModel, DPModule, TrainingConfig
from dpsynth.models.layers import Discriminator, DPDense, DPSequential, Generator


class TestTrainingConfig:
    """Test TrainingConfig dataclass."""

    def test_default_values(self):
        """Test default configuration."""
        config = TrainingConfig()
        assert config.epsilon == 10.0
        assert config.delta == 1e-5
        assert config.epochs == 100
        assert config.batch_size == 64

    def test_invalid_epsilon_raises(self):
        """Test that non-positive epsilon raises ValueError."""
        with pytest.raises(ValueError):
            TrainingConfig(epsilon=0)

    def test_invalid_delta_raises(self):
        """Test that invalid delta raises ValueError."""
        with pytest.raises(ValueError):
            TrainingConfig(delta=0)
        with pytest.raises(ValueError):
            TrainingConfig(delta=1)

    def test_invalid_epochs_raises(self):
        """Test that invalid epochs raises ValueError."""
        with pytest.raises(ValueError):
            TrainingConfig(epochs=0)


class TestDPModule:
    """Test DPModule base class."""

    def test_init(self):
        """Test initialization."""
        module = DPModule()
        assert module.noise_multiplier == 0.0

    def test_set_noise_multiplier(self):
        """Test setting noise multiplier."""
        module = DPModule()
        module.set_noise_multiplier(0.5)
        assert module.noise_multiplier == 0.5


class TestDPDense:
    """Test DPDense layer."""

    def test_init(self):
        """Test initialization."""
        layer = DPDense(10, 20, activation="relu")
        assert layer.linear.in_features == 10
        assert layer.linear.out_features == 20
        assert layer.noise_multiplier == 0.0

    def test_forward_shape(self):
        """Test forward pass preserves shape."""
        layer = DPDense(10, 20, activation="relu")
        x = torch.randn(32, 10)
        out = layer(x)
        assert out.shape == (32, 20)

    def test_forward_with_noise(self):
        """Test forward pass with noise (should still work)."""
        layer = DPDense(10, 20, activation="relu", use_noise=True)
        layer.set_noise_multiplier(0.5)
        x = torch.randn(32, 10)
        out = layer(x)
        assert out.shape == (32, 20)

    def test_different_activations(self):
        """Test different activation functions."""
        for activation in ["relu", "tanh", "leaky_relu", "sigmoid", None]:
            layer = DPDense(10, 10, activation=activation)
            x = torch.randn(4, 10)
            out = layer(x)
            assert out.shape == (4, 10)

    def test_forward_with_dropout(self):
        """Test forward pass with dropout."""
        layer = DPDense(10, 20, activation="relu", dropout=0.5)
        layer.train()
        x = torch.randn(32, 10)
        out = layer(x)
        assert out.shape == (32, 20)

    def test_no_bias(self):
        """Test layer without bias."""
        layer = DPDense(10, 5, bias=False)
        assert layer.linear.bias is None


class TestDPSequential:
    """Test DPSequential container."""

    def test_set_noise_multiplier(self):
        """Test that set_noise_multiplier propagates."""
        model = DPSequential(
            DPDense(10, 20, activation="relu", use_noise=True),
            DPDense(20, 5, activation="relu", use_noise=True),
        )
        model.set_noise_multiplier(0.5)
        for module in model:
            assert module.noise_multiplier == 0.5

    def test_forward(self):
        """Test forward pass."""
        model = DPSequential(
            DPDense(10, 20, activation="relu"),
            DPDense(20, 5),
        )
        x = torch.randn(4, 10)
        out = model(x)
        assert out.shape == (4, 5)


class TestGenerator:
    """Test Generator network."""

    def test_init(self):
        """Test initialization."""
        gen = Generator(latent_dim=128, output_dim=10)
        assert gen.latent_dim == 128

    def test_forward(self):
        """Test generation."""
        gen = Generator(latent_dim=128, output_dim=10)
        z = torch.randn(4, 128)
        out = gen(z)
        assert out.shape == (4, 10)

    def test_forward_with_hidden(self):
        """Test generation with custom hidden dims."""
        gen = Generator(latent_dim=64, output_dim=5, hidden_dims=(128, 64))
        z = torch.randn(2, 64)
        out = gen(z)
        assert out.shape == (2, 5)


class TestDiscriminator:
    """Test Discriminator network."""

    def test_init(self):
        """Test initialization."""
        disc = Discriminator(input_dim=10)
        assert disc is not None

    def test_forward(self):
        """Test discrimination (returns single logit)."""
        disc = Discriminator(input_dim=10)
        x = torch.randn(4, 10)
        out = disc(x)
        assert out.shape == (4, 1)

    def test_forward_with_hidden(self):
        """Test discrimination with custom hidden dims."""
        disc = Discriminator(input_dim=20, hidden_dims=(64, 32))
        x = torch.randn(8, 20)
        out = disc(x)
        assert out.shape == (8, 1)


class TestDataTransformer:
    """Test the DataTransformer used by CTGAN-DP."""

    @pytest.fixture
    def transformer(self):
        from dpsynth.models.ctgan_dp import DataTransformer
        return DataTransformer()

    def test_fit_continuous(self, transformer):
        """Test fitting on all-continuous data."""
        data = np.random.randn(100, 3).astype(np.float32)
        transformer.fit(data)
        assert transformer.output_dim == 3

    def test_fit_categorical(self, transformer):
        """Test fitting on data with categorical columns."""
        data = np.zeros((100, 2))
        data[:, 0] = np.random.randint(0, 5, 100)  # 5 categories
        data[:, 1] = np.random.randn(100)  # continuous
        transformer.fit(data)
        # 5 one-hot + 1 continuous = 6
        assert transformer.output_dim == 6

    def test_transform_continuous(self, transformer):
        """Test transform on continuous data."""
        data = np.random.randn(50, 2).astype(np.float32)
        transformer.fit(data)
        transformed = transformer.transform(data)
        assert transformed.shape == (50, transformer.output_dim)
        # Values should be in [0, 1]
        assert transformed.min() >= -0.1
        assert transformed.max() <= 1.1

    def test_transform_and_inverse(self, transformer):
        """Test round-trip transform + inverse for continuous data."""
        data = np.random.randn(20, 3).astype(np.float32)
        transformer.fit(data)
        transformed = transformer.transform(data)
        reconstructed = transformer.inverse_transform(transformed)
        assert reconstructed.shape == data.shape

    def test_transform_and_inverse_categorical(self, transformer):
        """Test round-trip for categorical data."""
        data = np.zeros((20, 2))
        data[:, 0] = np.random.randint(0, 3, 20)  # 3 categories
        data[:, 1] = np.random.randn(20)  # continuous
        transformer.fit(data)
        transformed = transformer.transform(data)
        reconstructed = transformer.inverse_transform(transformed)
        assert reconstructed.shape == data.shape
        # Categories should be preserved
        assert np.array_equal(reconstructed[:, 0], data[:, 0])

    def test_fit_constant_column(self, transformer):
        """Test fitting with a constant column."""
        data = np.ones((20, 1))
        transformer.fit(data)
        assert transformer.output_dim == 1
        transformed = transformer.transform(data)
        assert np.allclose(transformed, 0.5)


class TestDPModelAbstract:
    """Test DPModel abstract base class error paths."""

    def test_cannot_instantiate_directly(self):
        """Test that DPModel cannot be instantiated directly."""
        with pytest.raises(TypeError):
            DPModel()  # Can't instantiate abstract class

    def test_generate_not_implemented(self):
        """Test that generate raises NotImplementedError on direct subclass without impl."""

        class IncompleteModel(DPModel):
            def _build_model(self):
                pass

            def _run_training(self, data, epsilon, delta):
                pass

        with pytest.raises(TypeError):
            IncompleteModel()  # generate is still abstract

    def test_fit_without_training(self):
        """Test error when calling fit without required setup."""

        class MinimalModel(DPModel):
            def _build_model(self):
                pass

            def _run_training(self, data, epsilon, delta):
                pass

            def generate(self, n_samples, condition=None):
                return np.zeros((n_samples, 2))

        model = MinimalModel(TrainingConfig(epochs=1, batch_size=10))
        # Should complete without error with proper config
        result = model.fit(np.random.randn(20, 2), epsilon=100.0, delta=1e-5)
        assert result.epsilon >= 0


class TestCTGANSmoke:
    """Fast smoke tests for CTGAN-DP training (1-2 epochs)."""

    @pytest.fixture
    def small_data(self):
        rng = np.random.RandomState(42)
        return rng.randn(50, 3)

    def test_ctgan_instantiation(self):
        """Test that CTGAN_DP can be instantiated."""
        from dpsynth.models.ctgan_dp import CTGAN_DP, CTGANConfig

        config = CTGANConfig(
            epochs=1,
            batch_size=50,
            latent_dim=8,
            device="cpu",
            verbose=False,
        )
        model = CTGAN_DP(config)
        assert model is not None
        assert model.config.epochs == 1

    def test_ctgan_fit_smoke(self, small_data):
        """Test CTGAN-DP fit runs end-to-end for 1 epoch (smoke test)."""
        from dpsynth.models.ctgan_dp import CTGAN_DP, CTGANConfig

        config = CTGANConfig(
            epsilon=100.0,
            delta=1e-5,
            epochs=1,
            batch_size=50,
            latent_dim=8,
            generator_dims=(16, 16),
            discriminator_dims=(16, 16),
            max_grad_norm=1.0,
            device="cpu",
            verbose=False,
        )
        model = CTGAN_DP(config)
        budget = model.fit(small_data)
        assert budget.epsilon >= 0
        assert budget.steps > 0
        assert model.is_trained

    def test_ctgan_generate_after_fit(self, small_data):
        """Test that generate produces correct shape after fit."""
        from dpsynth.models.ctgan_dp import CTGAN_DP, CTGANConfig

        config = CTGANConfig(
            epsilon=100.0,
            delta=1e-5,
            epochs=1,
            batch_size=50,
            latent_dim=8,
            generator_dims=(16, 16),
            discriminator_dims=(16, 16),
            device="cpu",
            verbose=False,
        )
        model = CTGAN_DP(config)
        model.fit(small_data)

        synthetic = model.generate(20)
        assert synthetic.shape == (20, 3)
        assert not np.any(np.isnan(synthetic))

    def test_ctgan_generate_without_fit_raises(self, small_data):
        """Test that generate before fit raises RuntimeError."""
        from dpsynth.models.ctgan_dp import CTGAN_DP, CTGANConfig

        config = CTGANConfig(
            epochs=1, batch_size=50, latent_dim=8, verbose=False
        )
        model = CTGAN_DP(config)
        with pytest.raises(RuntimeError):
            model.generate(10)

    def test_privacy_accounting_respected(self, small_data):
        """Test that privacy budget is tracked during training."""
        from dpsynth.models.ctgan_dp import CTGAN_DP, CTGANConfig

        config = CTGANConfig(
            epsilon=100.0,
            delta=1e-5,
            epochs=2,
            batch_size=25,
            latent_dim=8,
            generator_dims=(16, 16),
            discriminator_dims=(16, 16),
            device="cpu",
            verbose=False,
        )
        model = CTGAN_DP(config)
        budget = model.fit(small_data)
        spent = model.get_privacy_spent()
        assert spent.epsilon > 0
        assert spent.steps > 0
