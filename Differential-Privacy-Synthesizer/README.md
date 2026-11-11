<div align="center">

# &#128274; Differential Privacy Synthesizer

**High-Assurance Synthetic Data Generation with Strict (ε, δ)-Differential Privacy Guarantees**

<br>

[![Release](https://img.shields.io/badge/Release-v0.3.0-2563eb?style=flat-square)](https://github.com/KELLERBABG/Differential-Privacy-Synthesizer/releases/latest)
[![License](https://img.shields.io/badge/License-MIT-059669?style=flat-square)](LICENSE)
[![Language](https://img.shields.io/badge/Language-Python%20%7C%20Rust-orange?style=flat-square)](https://www.python.org/)
[![Privacy](https://img.shields.io/badge/Privacy-(%CE%B5%2C%20%CE%B4)--DP%20Guaranteed-emerald?style=flat-square)](DEVELOPMENT_PLAN.md)
[![Engine](https://img.shields.io/badge/Engine-Renyi%20DP%20%7C%20Gaussian-blueviolet?style=flat-square)](dpsynth/)

<br>

[**Development Plan**](DEVELOPMENT_PLAN.md) &bull; [**Interactive Portal**](https://dps.kellersystems.dev) &bull; [**Package Core**](dpsynth/)

</div>

---

## What is Differential Privacy Synthesizer?

**Differential Privacy Synthesizer (DPSynth)** is a mathematical synthesis framework designed to generate statistically faithful synthetic datasets with provable $(\varepsilon, \delta)$-differential privacy bounds.

DPSynth guarantees that the output dataset reveals mathematically bounded information about any individual record, completely preventing re-identification, reconstruction attacks, and membership inference attacks while preserving correlation structures, marginal distributions, and machine learning utility.

---

## Why Use It?

* **Formal (ε, δ)-Privacy Budgeting:** Precise privacy accounting utilizing Rényi Differential Privacy (RDP) and Zero-Concentrated Differential Privacy (zCDP), enabling tighter composition bounds across repetitive queries.
* **Mathematical Utility Preservation:** Employs copula-based correlation capture and private marginal estimation to replicate complex multivariate distributions without leakage.
* **Reconstruction Immunity:** Provides verifiable formal immunity against linkability, attribute disclosure, and state-level deanonymization attempts.
* **Drop-in ML Compatibility:** Export synthetic tables directly to Pandas, Polars, or Parquet for seamless training of deep learning and predictive models.

---

## Architecture Overview

```
DPSynth Framework
├── dpsynth/
│   ├── accounting/     # Rényi & Gaussian privacy budget accumulation engines
│   ├── mechanisms/     # Laplace, Gaussian, and Exponential bounded noise engines
│   ├── synthesizer/    # Copula & marginal generative synthesis pipelines
│   └── metrics/        # Statistical distance (Wasserstein, KS-test, Jensen-Shannon)
├── tests/              # Property-based testing via Hypothesis and verification suites
└── index.html          # Browser-based live synthesizer demonstration
```

---

## Quickstart

### Prerequisites
* Python 3.10+

### Installation & Execution

```bash
git clone https://github.com/KELLERBABG/Differential-Privacy-Synthesizer.git && cd Differential-Privacy-Synthesizer && pip install -e dpsynth
```

Run test suite:
```bash
pytest
```

---

<div align="center">
&copy; 2026 Keller Systems &bull; Built for Sovereign Machine Verification
</div>
