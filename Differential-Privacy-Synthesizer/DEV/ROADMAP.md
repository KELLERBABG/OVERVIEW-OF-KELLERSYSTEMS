# Differential-Privacy-Synthesizer — Development Roadmap & Action Tasks

> This file is the authoritative internal development roadmap and task sequencing plan.
> **Language:** `Python`
> **Summary:** A Python mathematical synthetic data engine (`dpsynth/`) with 12 test suites implementing Rényi Differential Privacy (RDP) accounting (`accountant.py`), per-sample gradient clipping (`clipping.py`), calibrated Gaussian/Laplace mechanisms (`noise.py`), Exponential mechanism (`exponential.py`), DP-SGD optimizer (`optimizer.py`), DP-CTGAN (`ctgan_dp.py`), and privacy/utility evaluation (`privacy.py`, `utility.py`).

## Task Queue

- [ ] **TASK-01: Enforce hard privacy budget exhaustion gate in RDP accountant and DP-CTGAN loop**
  - **Target Files:** `dpsynth/dpsynth/privacy/accountant.py`, `dpsynth/dpsynth/models/ctgan_dp.py`
  - **Context Files:** `dpsynth/dpsynth/privacy/optimizer.py`, `dpsynth/tests/test_accountant.py`
  - **Verify:** `pytest dpsynth/tests/test_accountant.py dpsynth/tests/test_models.py -q`
  - **Spec:** Harden `accountant.py` and `ctgan_dp.py` so that before every gradient step or marginal query, the prospective Rényi divergence $\varepsilon_{\text{next}}(\delta)$ is pre-computed.

1. If $\varepsilon_{\text{next}}(\delta) > \varepsilon_{\max}$, raise `PrivacyBudgetExhaustedError` and halt training immediately without applying the unbudgeted update.
2. Emit a signed/hashed `PrivacySpendCertificate` summarizing total steps, noise multiplier $\sigma$, clipping norm $C$, $\delta$, and final $\varepsilon$.
3. Add unit tests in `dpsynth/tests/test_accountant.py` verifying that training halts at the exact step where $\varepsilon_{\max}$ is reached.

- [ ] **TASK-02: Implement Mironov floating-point safe snapping mechanism in noise.py**
  - **Target Files:** `dpsynth/dpsynth/privacy/noise.py`, `dpsynth/tests/test_noise.py`
  - **Context Files:** `dpsynth/dpsynth/privacy/clipping.py`
  - **Verify:** `pytest dpsynth/tests/test_noise.py -q`
  - **Spec:** Enhance `dpsynth/dpsynth/privacy/noise.py` with floating-point rounding protection per Mironov (2012) so IEEE-754 mantissa gaps cannot leak raw sensitivity values.

1. Round noisy outputs to the nearest power-of-two grid step $\eta$ proportional to scale $\lambda$, and draw entropy from OS CSPRNG (`np.random.Generator(np.random.PCG64DXSM(secrets.randbits(128)))`) when no explicit test seed is passed.
2. Add statistical and bit-level unit tests in `dpsynth/tests/test_noise.py`.

- [ ] **TASK-03: Add singleton memorization and Distance-to-Closest-Record (DCR) release gate**
  - **Target Files:** `dpsynth/dpsynth/evaluation/privacy.py`, `dpsynth/tests/test_evaluation.py`
  - **Context Files:** `dpsynth/dpsynth/evaluation/utility.py`, `dpsynth/dpsynth/evaluation/postprocess.py`
  - **Verify:** `pytest dpsynth/tests/test_evaluation.py -q`
  - **Spec:** Extend `dpsynth/dpsynth/evaluation/privacy.py` with a `ReleaseGateAuditor` that compares Distance-to-Closest-Record (DCR) between `(synthetic, train)` and `(synthetic, holdout)` splits.

1. Fail the release gate if the 5th-percentile DCR to training records is statistically significantly smaller than to holdout records (indicating overfitting/memorization) or if any exact singleton row collision occurs.
2. Add unit tests in `dpsynth/tests/test_evaluation.py` verifying that leaked training rows trigger a gate rejection.
