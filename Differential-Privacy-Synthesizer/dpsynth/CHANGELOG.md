# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-07-30

### Added

#### Privacy Module
- `PrivacyAccountant`: RDP-based privacy budget tracking with 64-order composition
- `GradientClipper`: Fixed-threshold DP-SGD gradient clipping
- `AdaptiveGradientClipper`: Quantile-based adaptive clipping threshold
- `PerSampleGradientClipper`: Micro-batch per-sample gradient clipping
- `GaussianNoiseLayer` / `LaplaceNoiseLayer`: Calibrated noise PyTorch modules
- `NoiseInjection`: Configurable noise with automatic scale computation from (ε, δ, Δf)
- `ExponentialMechanism`: DP selection via exponential weighting with hyperparameter tuning
- `PerSampleGradientComputer`: Vectorized per-sample gradients (opacus/vmap/manual)

#### Models Module
- `DPModel` / `DPModule`: Abstract base classes for DP model interface
- `DPDense`: DP-aware dense layer with configurable noise injection
- `Generator` / `Discriminator`: Standard GAN architecture components
- `CTGAN_DP`: Full tabular DP-GAN with DP-SGD on discriminator, Wasserstein loss, PAC packing
- `DataTransformer`: Mixed-type column handling (categorical one-hot + continuous min-max)

#### Evaluation Module
- `UtilityEvaluator`: Column-wise distribution similarity, correlation matrix distance, downstream ML utility
- `MembershipInferenceAttack`: Shadow model-based MIA with AUC reporting
- `PostProcessingVerifier`: Tests 10+ transformations to verify DP guarantee persistence

#### Utils Module
- `DataPreprocessor`: Missing value imputation, categorical encoding, feature scaling
- `Dataset` dataclass for benchmark data
- `load_benchmark_dataset()`: Adult Census, Credit Card Default, Forest Covertype loaders

#### Infrastructure
- `LICENSE`: Proprietary (custom KELLERBABG proprietary license)
- `.gitignore`: Python standard ignores
- `setup.py`: pip compatibility fallback
- `.github/workflows/ci.yml`: GitHub Actions CI (Python 3.11-3.13)
- `CHANGELOG.md`: Version history
- `Makefile`: Convenience commands
- `examples/`: Runnable scripts
- `CONTRIBUTING.md`: Developer setup and PR guidelines
- `docs/api.md`: Full API reference
- `docs/tutorial.md`: 11-section hands-on tutorial
- `WHITEPAPER.md`: Academic-style technical paper

### Performance Optimizations
- Vectorized per-sample gradients via opacus hooks (2-5x speedup)
- Adaptive noise scheduling for low-ε regimes (prevents generator collapse)
- Auto-σ estimation from (ε, δ, epochs, batch_size, n)

### Testing
- 160 tests across 9 test files
- 96% code coverage
- Smoke tests for CTGAN-DP training loop
- Property-based checks for RDP composition
- NaN/imputation edge cases in data preprocessing

## [0.2.0] - 2026-08-16

### Changed
- **License**: Reaffirmed and formalized Proprietary licensing across `LICENSE`, `pyproject.toml`, `README.md`, `WHITEPAPER.md`, and `DEVELOPMENT_PLAN.md`. The 0.1.0 CHANGELOG entry previously mislabeled the LICENSE as "Apache 2.0"; the LICENSE file itself has been proprietary from day one. **PEP 639**: `license` is now the SPDX expression `LicenseRef-KELLERBABG-Proprietary`; the legacy `License :: Other Proprietary` classifier is intentionally dropped because `setuptools>=77` forbids mixing SPDX license expressions with the old classifier form. PyPI surfaces dpsynth under its `License` chip automatically from the SPDX string; if you were filtering PyPI by `License :: Other Proprietary`, change your tooling to `License :: LicenseRef-...` or query the SPDX expression directly.
- CI hardening: pip caching, ruff as a real gate (not warning-only), coverage-under-95 gate on Python 3.12, dedicated fuzz job.
- `pyproject.toml`: bump `version` to `0.2.0`; pin `setuptools>=77` for `license-files` (PEP 639); drop ambiguous ruff rule families (`PL`, `RUF`, `SIM`, `C4`) from the lint gate for v0.2.0 with a follow-up backlog to re-enable; register `e2e` and `fuzz` pytest markers.

### Added
- `tests/test_e2e.py` — two end-to-end tests:
  - Subprocess smoke test of the `python -m dpsynth` CLI entry point.
  - Full `CTGAN_DP.fit(real).generate(N)` roundtrip on a tiny synthetic dataset, asserting shape, finiteness, and that privacy was actually spent (`budget.epsilon > 0`).
- `tests/test_fuzz_accountant.py` — Hypothesis-driven property tests on `PrivacyAccountant`:
  - Monotonicity invariant: doubling the noise multiplier never increases spent ε.
  - Input validation: negative or zero σ / `batch_size` / `dataset_size` raises `ValueError`.
  - Composition invariant: two `step` calls accumulate to ≤ 2× a single doubled step (loose sanity bound).
- `.github/workflows/release.yml` — tag-triggered release pipeline:
  - Builds sdist + wheel via `python -m build` on `v*.*.*` tags.
  - Publishes sdist + wheel to **TestPyPI** on `-rc.*` / `-beta.*` / `-alpha.*` tags via trusted publishing (OIDC).
  - Publishes to **PyPI** on stable `vX.Y.Z` tags via trusted publishing (OIDC).
  - Creates a GitHub Release with auto-generated notes and uploaded artifacts.
- `RELEASE.md` — operator-facing release procedure (cutting a release, credential setup, trusted-publishing configuration, rollback).

### Security
- Trusted publishing (PyPI OIDC) eliminates long-lived API tokens from CI.