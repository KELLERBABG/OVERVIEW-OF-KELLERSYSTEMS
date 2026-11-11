# API Reference

## Package Structure

```
dpsynth/
├── privacy/           # Core DP mechanisms
│   ├── accountant.py  # Privacy budget tracking
│   ├── clipping.py    # Gradient clipping
│   ├── noise.py       # Noise injection
│   └── exponential.py # DP selection
├── models/            # DP model architectures
│   ├── base.py        # Abstract interfaces
│   ├── layers.py      # DP-aware layers
│   └── ctgan_dp.py    # Tabular DP-GAN
├── evaluation/        # Quality & privacy evaluation
│   ├── utility.py     # Utility metrics
│   ├── privacy.py     # Membership inference
│   └── postprocess.py # Post-processing invariance
└── utils/             # Data handling
    └── data.py        # Dataset loaders & preprocessing
```

---

## dpsynth.privacy

### PrivacyAccountant

Tracks the accumulated (ε, δ) privacy budget across training steps using Rényi Differential Privacy (RDP) composition.

```python
from dpsynth.privacy import PrivacyAccountant

accountant = PrivacyAccountant()
accountant.step(noise_multiplier=1.0, batch_size=64, dataset_size=50_000, steps=100)
budget = accountant.get_privacy_spent(delta=1e-5)
print(f"ε = {budget.epsilon:.2f}, steps = {budget.steps}")
```

#### `PrivacyAccountant(orders: tuple[float, ...] = range(2, 65), mechanism: str = "gaussian")`
- `orders`: RDP orders α to track (default 2..64)
- `mechanism`: 'gaussian' or 'laplace'

#### `step(noise_multiplier, batch_size, dataset_size, steps=1)`
Account for one or more training steps.
- `noise_multiplier`: σ parameter (higher = more privacy)
- `batch_size`: Samples per batch
- `dataset_size`: Total dataset size
- `steps`: Number of gradient steps

#### `get_privacy_spent(delta=1e-5) -> PrivacyBudget`
Returns `PrivacyBudget(epsilon, delta, steps)`.

#### `get_spent_epsilon(delta=1e-5) -> float`
Convenience: returns just ε.

#### `reset()`
Reset all accumulated state.

### PrivacyBudget

Dataclass with `epsilon`, `delta`, `steps` fields.

---

### GradientClipper

Flat-threshold gradient clipper for DP-SGD training.

```python
from dpsynth.privacy import GradientClipper

clipper = GradientClipper(max_norm=1.0)
stats = clipper.clip_parameters(model)  # clips model.grad in-place
```

#### `GradientClipper(max_norm=1.0)`
- `max_norm`: Clipping threshold C. Gradients with ||g||₂ > C are scaled down.

#### `clip_parameters(model) -> ClippingStats`
Clip all parameter gradients in a PyTorch model.

#### `clip_gradients(parameter_groups, loss_reduction="mean") -> ClippingStats`
Clip gradients from optimizer-style parameter groups.

#### `set_norm(norm)`
Update clipping threshold.

### AdaptiveGradientClipper

Extends `GradientClipper` with automatic threshold adjustment based on clipping fraction.

```python
clipper = AdaptiveGradientClipper(
    initial_norm=1.0,
    target_quantile=0.5,
    adaptation_rate=0.01,
    min_norm=0.01,
    max_norm_bound=100.0,
)
```

### PerSampleGradientClipper

Per-sample clipper for micro-batch DP training. Computes backward for each sample individually.

```python
clipper = PerSampleGradientClipper(max_norm=1.0, micro_batch_size=1)
stats = clipper.clip_per_sample(model, loss_per_sample)
```

### ClippingStats

Dataclass with `clip_norm`, `fraction_clipped`, `max_grad_norm`, `mean_grad_norm`.

---

### NoiseInjection

Calibrated noise specification. Computes the correct noise scale for (ε, δ)-DP or ε-DP.

```python
from dpsynth.privacy import NoiseInjection

noise = NoiseInjection(mechanism="gaussian", epsilon=1.0, delta=1e-5, sensitivity=1.0)
noisy_tensor = noise.apply(tensor)
noise.apply_gradients(optimizer.param_groups, clip_norm=1.0)
```

### GaussianNoiseLayer

PyTorch module that adds N(0, σ²) noise to its input.

```python
layer = GaussianNoiseLayer(sigma=1.0)
noisy = layer(x)
```

### LaplaceNoiseLayer

PyTorch module that adds Lap(0, b) noise to its input.

```python
layer = LaplaceNoiseLayer(scale=1.0)
noisy = layer(x)
```

---

### ExponentialMechanism

The Exponential Mechanism (McSherry & Talwar 2007) for differentially private selection.

```python
from dpsynth.privacy import ExponentialMechanism

mech = ExponentialMechanism(epsilon=1.0, sensitivity=1.0)
result = mech.select(
    real_data, candidates,
    utility_function=lambda r, c: -np.mean((r - c) ** 2)
)
print(f"Selected candidate {result.selected_index}")
```

#### `ExponentialMechanism(epsilon=1.0, sensitivity=1.0)`

#### `select(real_data, candidates, utility_function=None, utility_scores=None) -> ExponentialMechanismResult`
Select a candidate with probability ∝ exp(ε · u(data, r) / (2 · Δu)).

#### `select_hyperparameter(real_data, candidate_generator, hyperparameter_values, evaluation_metric) -> (best_hp, result)`
DP hyperparameter selection.

### ExponentialMechanismResult

Dataclass with `selected_index`, `selected_score`, `epsilon`, `sensitivity`, `selection_probability`.

### Utility Functions

- `utility_negative_mse(real, synthetic) -> float`: Negative mean squared error.
- `utility_column_correlation(real, synthetic) -> float`: Correlation matrix similarity [0, 1].

---

### PerSampleGradientComputer (New in v0.1.0)

Vectorized per-sample gradient computation for efficient DP-SGD. Replaces the naive loop approach with opacus-backed or vmap-backed strategies for 2-5x speedup.

```python
from dpsynth.privacy import PerSampleGradientComputer

computer = PerSampleGradientComputer(model, strategy="auto")
# strategy: "auto" → opacus if available, "vmap", "manual"

def loss_fn(model, x):
    return model(x).squeeze()  # per-sample loss

result = computer.compute(batch, loss_fn=loss_fn)
# result.grads: list of tensors with shape (batch_size, *param_shape)
# result.param_names: names of each gradient tensor
# result.batch_size: batch dimension
```

**Strategies:**
- `"opacus"` (fastest): Uses `opacus.GradSampleModule` with expander hooks for native vectorized per-sample gradients
- `"vmap"` (PyTorch 2.0+): Uses `torch.func.vmap` — experimental, falls through to manual
- `"manual"` (fallback): Per-sample backward loop — always works but 2-5x slower
- `"auto"` (default): Tries opacus first, falls back gracefully

### PerSampleGradientBatch

Dataclass with `grads`, `param_names`, `batch_size` fields.

---

## dpsynth.models

### DPModel (abstract base)

```python
from dpsynth.models import DPModel

class MyModel(DPModel):
    def _build_model(self): ...
    def _run_training(self, data, epsilon, delta): ...
    def generate(self, n_samples, condition=None): ...
```

#### `fit(data, epsilon=None, delta=None) -> PrivacyBudget`
Train with differential privacy. Returns privacy spent.

#### `generate(n_samples, condition=None) -> np.ndarray`
Generate synthetic data.

#### `get_privacy_spent(delta=None) -> PrivacyBudget`
Query consumed privacy budget.

#### `reset_privacy_budget()`
Reset accountant for multi-stage training.

### TrainingConfig

Dataclass: `epsilon`, `delta`, `epochs`, `batch_size`, `learning_rate`, `noise_multiplier`, `max_grad_norm`, `device`, `verbose`.

### DPDense

Differentially private dense layer with configurable noise injection.

```python
layer = DPDense(64, 128, activation="relu", dropout=0.1, use_noise=True)
layer.set_noise_multiplier(0.5)
out = layer(x)
```

### DPSequential

Sequential container that propagates noise multipliers.

### Generator

Standard GAN generator network. Maps latent vectors → synthetic data.

```python
gen = Generator(latent_dim=128, output_dim=10, hidden_dims=(256, 256))
z = torch.randn(32, 128)
synthetic = gen(z)
```

### Discriminator

Standard GAN discriminator network. Returns logits.

```python
disc = Discriminator(input_dim=10, hidden_dims=(256, 256))
logits = disc(x)  # shape: (batch_size, 1)
```

### CTGAN_DP

CTGAN-inspired tabular data synthesizer with DP-SGD on the discriminator.

```python
from dpsynth.models.ctgan_dp import CTGAN_DP, CTGANConfig

config = CTGANConfig(
    epsilon=10.0, delta=1e-5,
    epochs=50, batch_size=100,
    latent_dim=128, device="cpu",
)
model = CTGAN_DP(config)
model.fit(real_data)
synthetic = model.generate(2000)
```

#### Training Details
- **Privacy only on discriminator**: The generator never sees real data directly.
- **Wasserstein loss** with PAC (Perturbed and Aggregated Cross) discriminator.
- **Mixed-type data**: Automatically handles categorical (one-hot) and continuous (min-max) columns.
- **Early stopping**: Stops training when privacy budget is exhausted.

### DataTransformer

Transforms mixed-type data for GAN training.

```python
from dpsynth.models.ctgan_dp import DataTransformer

t = DataTransformer()
t.fit(data)                     # detects column types
transformed = t.transform(data)  # → one-hot + normalized
reconstructed = t.inverse_transform(transformed)
```

---

## dpsynth.evaluation

### UtilityEvaluator

Comprehensive synthetic data quality evaluation.

```python
from dpsynth.evaluation import UtilityEvaluator

evaluator = UtilityEvaluator()
report = evaluator.evaluate(real, synthetic, column_names=["age", "income"])
print(f"Overall utility: {report.overall_score:.2%}")
```

#### Metrics
1. **Column-wise distribution similarity**: Wasserstein distance (continuous), total variation (categorical)
2. **Correlation preservation**: Frobenius norm of correlation matrix difference
3. **Downstream ML utility**: Train-on-synthetic, test-on-real using Random Forest

### UtilityReport

Fields: `column_wise_similarity`, `correlation_distance`, `ml_utility`, `overall_score`, `n_features`.

---

### MembershipInferenceAttack

Shadow model-based membership inference attack.

```python
from dpsynth.evaluation import MembershipInferenceAttack

mia = MembershipInferenceAttack(n_shadow=10)
report = mia.evaluate(real_data, synthetic_data)
print(f"Attack AUC: {report.attack_auc:.3f}")
```

- **Attack**: Measures whether an adversary can determine if a record was in training.
- **Defense verification**: AUC < 0.6 → protected, AUC > 0.8 → leakage detected.
- **Method**: Shadow models using distance-to-synthetic-distribution as attack feature.

### PrivacyLeakageReport

Fields: `attack_auc`, `attack_accuracy`, `epsilon_estimate`, `n_shadow`, `is_protected`.

---

### PostProcessingVerifier

Verifies that DP guarantees hold after data transformations.

```python
from dpsynth.evaluation import PostProcessingVerifier

verifier = PostProcessingVerifier()
report = verifier.verify(real_data, synthetic_data)
print(f"Post-processing invariant: {report.is_invariant}")
```

Tests 10+ transformations: identity, scaling, standardization, clipping, rounding, noise addition, aggregation, random projection.

### PostProcessReport

Fields: `is_invariant`, `pre_transform_epsilon`, `post_transform_epsilon`, `epsilon_drift`, `transformations_tested`, `violations_found`.

---

## dpsynth.utils

### DataPreprocessor

Preprocesses mixed-type data for DP model training.

```python
from dpsynth.utils import DataPreprocessor

prep = DataPreprocessor()
transformed = prep.fit_transform(data)  # auto-detect column types
```

Handles: missing value imputation, categorical encoding, feature scaling.

### Dataset

Dataclass for benchmark datasets: `data`, `target_column`, `column_names`, `categorical_columns`, `continuous_columns`, `name`.

### load_benchmark_dataset(name: str) -> Dataset | None

Supported: `"adult"`, `"credit"`, `"covertype"`.

```python
dataset = load_benchmark_dataset("adult")
if dataset is not None:
    preprocessor = DataPreprocessor(
        categorical_columns=dataset.categorical_columns,
        continuous_columns=dataset.continuous_columns,
    )
    X = preprocessor.fit_transform(dataset.data)