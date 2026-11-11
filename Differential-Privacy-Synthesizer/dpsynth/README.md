# Differentially Private Synthesizer (dpsynth)

[![Python 3.11+](https://img.shields.io/badge/python-3.11%20%7C%203.12%20%7C%203.13-blue)](pyproject.toml)
[![Coverage](https://img.shields.io/badge/coverage-96%25-brightgreen)](tests/)
[![Version](https://img.shields.io/badge/version-0.2.0-blue)](CHANGELOG.md)

An AI/ML system that generates synthetic datasets with formal differential privacy guarantees, using DP-GAN architectures, gradient clipping, noise injection, and membership inference protection.

## Features

- **Privacy Accounting**: Rényi Differential Privacy composition with moments accountant (α = 2..64)
- **Gradient Clipping**: Fixed, adaptive quantile-based, and per-sample strategies
- **Noise Injection**: Calibrated Laplace (ε-DP) and Gaussian ((ε,δ)-DP) noise
- **DP-GAN Architecture**: CTGAN-inspired tabular data generator with differentially private discriminator
- **Utility Evaluation**: Column-wise distribution similarity, correlation preservation, downstream ML utility
- **Privacy Auditing**: Distance-based membership inference attack (shadow model approach)
- **Mixed-Type Data**: Automatic handling of categorical + continuous + ordinal columns
- **Benchmark Datasets**: Built-in loaders for Adult Census, Credit Card, and Covertype
- **Exponential Mechanism**: Differentially private selection and hyperparameter tuning
- **Post-Processing Verification**: Tests 10+ transformations for DP invariance
- **Vectorized Per-Sample Gradients**: 2-5x speedup via opacus hooks

## What's new in 0.2.0

- License phrasing formalized across `LICENSE`, `pyproject.toml`, `WHITEPAPER.md`, and `CONTRIBUTING.md`. The 0.1.0 release notes mistakenly called the license "Apache 2.0"; the file itself has been proprietary from day one — this release just makes the documents agree.
- CI hardened: pip caching, ruff as a real gate (not a warning), coverage-under-95 on Python 3.12, dedicated Hypothesis fuzz job.
- New `tests/test_fuzz_accountant.py` — property-based invariants on the RDP accountant (noise monotonicity, composition, input validation).
- New `tests/test_e2e.py` — subprocess `python -m dpsynth` smoke + accountant pipeline roundtrip + full `CTGAN_DP.fit/generate` roundtrip (torch-gated).
- Full release pipeline (`.github/workflows/release.yml`) with **PyPI trusted publishing (OIDC)** — no API tokens in CI. See [`RELEASE.md`](RELEASE.md) for the operator guide.

## Installation

```bash
pip install -e .
```

For development:
```bash
pip install -e ".[dev]"
```

## Quick Start

```python
from dpsynth.models.ctgan_dp import CTGAN_DP, CTGANConfig
from dpsynth.evaluation import UtilityEvaluator, MembershipInferenceAttack
import numpy as np

# Generate sample data
real_data = np.random.randn(1000, 5)

# Train with differential privacy
model = CTGAN_DP(CTGANConfig(epsilon=10.0, epochs=20))
budget = model.fit(real_data)
print(f"Privacy spent: ε = {budget.epsilon:.2f}")

# Generate synthetic data
synthetic = model.generate(2000)

# Evaluate utility
report = UtilityEvaluator().evaluate(real_data, synthetic)
print(f"Utility score: {report.overall_score:.2%}")

# Audit privacy
mia = MembershipInferenceAttack()
print(f"MIA AUC: {mia.evaluate(real_data, synthetic).attack_auc:.3f}")
```

## Project Structure

```
dpsynth/
├── dpsynth/
│   ├── privacy/           # Core DP mechanisms
│   │   ├── accountant.py  # RDP privacy accounting
│   │   ├── clipping.py    # Gradient clipping (fixed, adaptive, per-sample)
│   │   ├── noise.py       # Gaussian & Laplace noise layers
│   │   ├── exponential.py # DP selection & hyperparameter tuning
│   │   └── optimizer.py   # Vectorized per-sample gradients
│   ├── models/
│   │   ├── base.py        # Abstract DP model interface
│   │   ├── layers.py      # DP-aware layers (DPDense, Generator, Discriminator)
│   │   └── ctgan_dp.py    # CTGAN-DP: tabular synthesizer with DP-SGD
│   ├── evaluation/
│   │   ├── utility.py     # Utility metrics (distribution, correlation, ML)
│   │   ├── privacy.py     # Membership inference attack
│   │   └── postprocess.py # Post-processing invariance verification
│   └── utils/
│       └── data.py        # Dataset loaders & preprocessing
├── tests/                 # 160 tests (pytest)
├── docs/                  # API reference & tutorial
├── examples/              # Runnable scripts
├── WHITEPAPER.md          # Academic-style technical paper
├── CONTRIBUTING.md        # Developer guidelines
├── CHANGELOG.md           # Version history
└── Makefile               # Convenience commands
```

## Running Tests

```bash
make test        # Full test suite with coverage
make test-fast   # Tests without coverage (faster)
```

## Documentation

- **[API Reference](docs/api.md)**: Complete API documentation with examples
- **[Tutorial](docs/tutorial.md)**: 11-section hands-on guide
- **[Whitepaper](WHITEPAPER.md)**: Technical paper with architecture, privacy analysis, and evaluation

## Key Concepts

- **ε (Epsilon)**: Privacy budget — lower values mean stronger privacy
- **δ (Delta)**: Probability of privacy failure (typically 1e-5 or 1e-6)
- **Noise Multiplier (σ)**: Controls noise added during training
- **Gradient Clipping (C)**: Bounds the influence of any single data point
- **RDP Composition**: Tight bounds on cumulative privacy cost

## License

Proprietary. See [LICENSE](LICENSE) for details.
