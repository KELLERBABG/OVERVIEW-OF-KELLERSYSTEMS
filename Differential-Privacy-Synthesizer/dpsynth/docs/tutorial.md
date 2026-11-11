# Tutorial

## 1. Quick Start: Your First Synthetic Dataset

```python
import numpy as np
from dpsynth.models.ctgan_dp import CTGAN_DP, CTGANConfig
from dpsynth.evaluation.utility import UtilityEvaluator
from dpsynth.evaluation.privacy import MembershipInferenceAttack

# ── Generate sample data ──────────────────────────────────────
rng = np.random.RandomState(42)
n = 2000
age = rng.randint(18, 70, n)
income = 30000 + age * 500 + rng.randn(n) * 10000
education = rng.choice(["HS", "BS", "MS", "PhD"], n, p=[0.3, 0.4, 0.2, 0.1])
category = rng.choice(["A", "B", "C"], n, p=[0.5, 0.3, 0.2])

real_data = np.column_stack([age, income, education, category])
real_data = real_data.astype(object)  # mixed types

# ── Configure and train ───────────────────────────────────────
config = CTGANConfig(
    epsilon=10.0,       # Privacy budget (lower = more privacy)
    delta=1e-5,         # Privacy failure probability
    epochs=20,
    batch_size=200,
    latent_dim=64,
    max_grad_norm=1.0,
    device="cpu",
    verbose=True,
)

model = CTGAN_DP(config)
budget = model.fit(real_data)
print(f"Training complete. ε spent: {budget.epsilon:.2f}")

# ── Generate synthetic data ───────────────────────────────────
synthetic = model.generate(5000)
print(f"Generated {len(synthetic)} samples, shape: {synthetic.shape}")

# ── Evaluate utility ──────────────────────────────────────────
evaluator = UtilityEvaluator()
report = evaluator.evaluate(real_data, synthetic)
print(f"Overall utility: {report.overall_score:.2%}")
print(f"Column similarity: {report.column_wise_similarity}")
print(f"Correlation distance: {report.correlation_distance:.4f}")

# ── Audit privacy ─────────────────────────────────────────────
mia = MembershipInferenceAttack(n_shadow=10)
privacy_report = mia.evaluate(real_data, synthetic)
print(f"Membership inference AUC: {privacy_report.attack_auc:.3f}")
if privacy_report.is_protected:
    print("✓ Data is protected against membership inference")
else:
    print("⚠ Data may have privacy leakage — consider lower ε")
```

## 2. Understanding the Privacy Budget

The central concept in differential privacy is the privacy budget **ε** (epsilon).

### What ε values mean

| ε | Privacy Level | Typical Use Case |
|---|--------------|------------------|
| 0.01 – 0.1 | Very strong | Medical records, financial data |
| 0.1 – 1.0 | Strong | Census data, user analytics |
| 1.0 – 10.0 | Moderate | Product recommendations, A/B tests |
| > 10.0 | Weak | Research data with minimal sensitivity |

### How to choose ε

```python
from dpsynth.privacy import PrivacyAccountant

# Simulate privacy cost before training
accountant = PrivacyAccountant()
for epoch in range(50):
    accountant.step(
        noise_multiplier=0.8,
        batch_size=128,
        dataset_size=10_000,
        steps=78,  # steps per epoch = n / batch_size
    )

budget = accountant.get_privacy_spent(delta=1e-5)
print(f"50 epochs at σ=0.8 will consume ε = {budget.epsilon:.2f}")
```

Run this simulation with different values of `noise_multiplier`, `batch_size`, and `epochs` to find the right balance before committing to a full training run.

## 3. Effect of DP on Data Quality

Higher privacy (lower ε) means more noise, which reduces data quality.

```python
from dpsynth.models.ctgan_dp import CTGAN_DP, CTGANConfig
from dpsynth.evaluation.utility import UtilityEvaluator

rng = np.random.RandomState(42)
data = rng.randn(1000, 5)

for epsilon in [0.1, 1.0, 10.0, 100.0]:
    config = CTGANConfig(
        epsilon=epsilon,
        delta=1e-5,
        epochs=10,
        batch_size=100,
        latent_dim=32,
        device="cpu",
        verbose=False,
    )
    model = CTGAN_DP(config)
    model.fit(data)

    synthetic = model.generate(2000)
    report = UtilityEvaluator().evaluate(data, synthetic)

    print(f"ε = {epsilon:6.1f} → utility = {report.overall_score:.2%}")
```

Expected output pattern:
```
ε =    0.1 → utility = 12.3%   (strong privacy, poor quality)
ε =    1.0 → utility = 45.7%   (good privacy, moderate quality)
ε =   10.0 → utility = 78.2%   (moderate privacy, good quality)
ε =  100.0 → utility = 91.5%   (weak privacy, near-perfect quality)
```

## 4. Verifying Post-Processing Invariance

Once data is generated with DP, any transformation preserves the guarantee.

```python
from dpsynth.evaluation import PostProcessingVerifier

verifier = PostProcessingVerifier(epsilon_threshold=0.5)
report = verifier.verify(real_data, synthetic)

print(f"Invariant: {report.is_invariant}")
print(f"ε drift:   {report.epsilon_drift:.4f}")
print(f"Violations: {report.violations_found}/{report.transformations_tested}")
```

## 5. Using the Exponential Mechanism for Model Selection

```python
from dpsynth.privacy import ExponentialMechanism

mech = ExponentialMechanism(epsilon=1.0, sensitivity=1.0)

# You have 5 candidate synthetic datasets, want to pick the best
# without leaking privacy about which one is best
candidates = [...]  # list of synthetic datasets
scores = [utility_function(real_data, c) for c in candidates]

result = mech.select(
    real_data, candidates, utility_scores=scores
)
print(f"Selected candidate {result.selected_index}")
print(f"Selection probability: {result.selection_probability:.1%}")
```

## 6. Benchmark Datasets

```python
from dpsynth.utils.data import load_benchmark_dataset, DataPreprocessor

# Load Adult Census dataset
dataset = load_benchmark_dataset("adult")
if dataset is not None:
    print(f"Dataset: {dataset.name}")
    print(f"Samples: {dataset.n_samples}, Features: {dataset.n_features}")

    prep = DataPreprocessor(
        categorical_columns=dataset.categorical_columns,
        continuous_columns=dataset.continuous_columns,
    )
    X = prep.fit_transform(dataset.data)
    y = X[:, dataset.target_column]
```

Supported datasets: `"adult"`, `"credit"`, `"covertype"`.

## 7. Custom DP Model

```python
from dpsynth.models import DPModel, TrainingConfig
from dpsynth.privacy import GradientClipper, NoiseInjection, PrivacyAccountant

class MyCustomDPModel(DPModel):
    def _build_model(self):
        # Build your architecture here
        pass

    def _run_training(self, data, epsilon, delta):
        for epoch in range(self.config.epochs):
            for batch in self._batches(data):
                loss = self._compute_loss(batch)
                loss.backward()

                # DP: clip + noise
                clipper = GradientClipper(self.config.max_grad_norm)
                clipper.clip_parameters(self)

                noise = NoiseInjection(
                    mechanism="gaussian",
                    epsilon=epsilon,
                    delta=delta,
                    sensitivity=self.config.max_grad_norm,
                )
                noise.apply_gradients(
                    self._optimizer.param_groups,
                    clip_norm=self.config.max_grad_norm,
                )

                self._optimizer.step()
                self._optimizer.zero_grad()

                self.accountant.step(
                    noise_multiplier=1.0,
                    batch_size=batch.shape[0],
                    dataset_size=data.shape[0],
                )

    def generate(self, n_samples, condition=None):
        return self._generator(n_samples)
```

## 8. Performance Optimization: Per-Sample Gradients

The naive per-sample gradient computation loops over each sample individually, which is 2-5x slower than standard training. Use the `PerSampleGradientComputer` for near-native speed:

```python
from dpsynth.privacy import PerSampleGradientComputer

# Auto strategy: uses opacus if installed, falls back gracefully
computer = PerSampleGradientComputer(model, strategy="auto")

# Define a per-sample loss function
def loss_fn(model, x):
    return model(x).squeeze()  # shape: (batch_size,)

# Compute all per-sample gradients in one vectorized pass
result = computer.compute(batch, loss_fn=loss_fn)

# result.grads[p] has shape (batch_size, *param_shape)
# Clip each sample's gradient individually
for grad in result.grads:
    per_sample_norms = grad.norm(dim=tuple(range(1, grad.ndim)))
    scale = C / (per_sample_norms + 1e-12)
    clip_factor = scale.clamp(max=1.0)
    # ... apply clipping to original gradients
```

For maximum speed, ensure `opacus` is installed:
```bash
pip install opacus
```

The three strategies at a glance:

| Strategy | Speed | Dependency | Reliability |
|----------|-------|------------|-------------|
| `"opacus"` | ⚡ 1x (native) | opacus 1.4+ | Production-ready |
| `"vmap"` | ⚡ 1-2x | PyTorch 2.0+ | Experimental |
| `"manual"` | 🐢 2-5x slower | None | Always works |

## 9. Adaptive Noise Scheduling for Low ε

When ε < 1.0, the discriminator gradients become extremely noisy, which can cause the generator to collapse to uniform noise. Starting with full noise from epoch 1 means the generator never learns meaningful structure.

The CTGAN-DP trainer now includes **adaptive noise scheduling**: when ε < 1.0, noise ramps linearly from σ_start (0.5) to σ_target over the first half of training epochs. This gives the generator time to learn coarse data structure before strong DP noise is applied.

```python
config = CTGANConfig(
    epsilon=0.5,          # Low epsilon → adaptive scheduling activates automatically
    noise_multiplier=2.0,  # Optional: override automatic σ estimation
    epochs=100,            # First 50 epochs use ramping noise
)
```

The noise multiplier is also **automatically estimated** from (ε, δ, epochs, batch_size, n) when not explicitly set:

```python
# σ is estimated from the privacy budget
config = CTGANConfig(epsilon=0.5, epochs=100, batch_size=200)
model = CTGAN_DP(config)
model.fit(data)  # σ computed internally, ramped over first half of training
```

## 10. Budgeting for Hyperparameter Tuning

Hyperparameter selection leaks privacy if done naively. Use the Exponential Mechanism to budget ε for tuning separately from ε for training:

```python
from dpsynth.privacy import ExponentialMechanism
from dpsynth.models.ctgan_dp import CTGAN_DP, CTGANConfig

# Reserve ε=0.1 for tuning, ε=1.0 for final training
tuning_epsilon = 0.1
training_epsilon = 1.0

# Candidates: different hyperparameter configurations
hp_values = [
    {"learning_rate": 1e-4, "latent_dim": 64},
    {"learning_rate": 5e-4, "latent_dim": 128},
    {"learning_rate": 1e-3, "latent_dim": 256},
]

def candidate_generator(hp):
    config = CTGANConfig(epsilon=training_epsilon, epochs=5, **hp, verbose=False)
    model = CTGAN_DP(config)
    model.fit(data)
    return model.generate(1000)

def evaluation_metric(real, synthetic):
    return -np.mean((real - synthetic) ** 2)

# DP-safe selection
mech = ExponentialMechanism(epsilon=tuning_epsilon, sensitivity=1.0)
best_hp, result = mech.select_hyperparameter(
    data, candidate_generator, hp_values, evaluation_metric
)
print(f"Selected: {best_hp} (probability: {result.selection_probability:.1%})")

# Final training with full budget — total ε = tuning_epsilon + training_epsilon
print(f"Total privacy cost: ε = {tuning_epsilon + training_epsilon}")
```

## 11. Common Pitfalls

### Pitfall 1: Forgetting to clip before noise
```python
# WRONG — noise without clipping leaks sensitivity
param.grad += torch.randn_like(param.grad) * sigma

# RIGHT — clip first, then noise
scale = C / param.grad.norm()
param.grad *= min(scale, 1.0)
param.grad += torch.randn_like(param.grad) * sigma * C
```

### Pitfall 2: Using the privacy budget twice
```python
# WRONG — calling fit() again starts fresh
model.fit(data, epsilon=1.0)
model.fit(data, epsilon=1.0)  # Actually spent ε=2.0 total!

# RIGHT — track cumulative spend
budget = model.get_privacy_spent()
print(f"Total ε spent so far: {budget.epsilon}")
```

### Pitfall 3: Not accounting for the generator in GAN
In CTGAN-DP, only the discriminator (which sees real data) is privatized.
The generator never accesses real data directly, only through the discriminator's
gradients. This is correct — the generator itself needs no DP protection.

### Pitfall 4: Applying post-processing that re-introduces privacy risk
```python
# This is safe — DP is post-processing invariant
synthetic = model.generate(1000)
averaged = synthetic.mean(axis=0)  # ✓ Still DP

# But this uses private data again
combined = np.vstack([synthetic, real_data])  # ✗ Real data leaks