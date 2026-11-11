# Differential Privacy Synthesizer (DPSynth) — Comprehensive Architecture & Planning Documentation

> **Project Directory:** `Differential-Privacy-Synthesizer/`  
> **Primary Language / Toolchain:** `Python`  
> **Verification Command:** `pytest dpsynth/tests -q`  
> **Reference Sources:** `KELLERSYSTEMS Spine.md` · `PaySlip.canvas` · `Life Work.canvas`

---

## 1. Executive Spine & System Overview

**Differential Privacy Synthesizer (DPSynth)** — A high-assurance synthetic data generator that creates statistically representative tabular datasets with provable, mathematically sound (epsilon, delta)-differential privacy guarantees.

### Current Engineering Baseline
A Python mathematical synthetic data engine (`dpsynth/`) with 12 test suites implementing Rényi Differential Privacy (RDP) accounting (`accountant.py`), per-sample gradient clipping (`clipping.py`), calibrated Gaussian/Laplace mechanisms (`noise.py`), Exponential mechanism (`exponential.py`), DP-SGD optimizer (`optimizer.py`), DP-CTGAN (`ctgan_dp.py`), and privacy/utility evaluation (`privacy.py`, `utility.py`).

---

## 2. Canonical Specifications & Mathematical Invariants (`PaySlip.canvas` / `Spine`)

The following 12 architectural pillars and formal invariants define the target engineering state of the system:

- **Epsilon-Privacy Budget ($\epsilon$):** Ein mathematisches Maß, das genau definiert, wie viel Information über eine einzelne Person maximal preisgegeben werden darf. Je kleiner $\epsilon$, desto stärker ist der Schutz der Privatsphäre.
    
- **Gradient Clipping:** Während des KI-Trainings werden die Einflüsse einzelner Datensätze (Gradienten) begrenzt, um zu verhindern, dass die KI sich zu stark an extremen Einzelwerten orientiert und diese "auswendig lernt".
    
- **Noise Injection (Laplace/Gauss):** Gezielte mathematische Störung der Daten während des Lernprozesses. Das Rauschen sorgt dafür, dass die KI zwar die Struktur der Menge versteht, aber keine Rückschlüsse auf das Individuum ziehen kann.
    
- **Synthetic Data Generation:** Die Erstellung eines komplett neuen Datensatzes aus dem "Gedächtnis" der KI. Dieser enthält keine Kopien der Originaldaten, sondern statistisch identische, aber künstliche Profile.
    
- **Utility vs. Privacy Trade-off:** Feinjustierung des Algorithmus, um die optimale Balance zwischen der Genauigkeit der Datenanalyse (Utility) und der Unumkehrbarkeit der Anonymisierung (Privacy) zu finden.
    
- **DP-GAN Architecture:** Einsatz von Generative Adversarial Networks, bei denen der "Discriminator" unter Differential Privacy Bedingungen trainiert wird, um realistische, aber sichere synthetische Daten zu erzwingen.
    
- **Membership Inference Protection:** Mathematische Garantie, dass ein Angreifer selbst mit Zugriff auf den synthetischen Datensatz nicht feststellen kann, ob eine bestimmte reale Person im Original-Datensatz enthalten war.
    
- **Feature Correlation Preservation:** Sicherstellung, dass komplexe Zusammenhänge (z. B. "Personen mit Eigenschaft A haben oft auch Eigenschaft B") im synthetischen Datensatz präzise erhalten bleiben.
    
- **Exponential Mechanism:** Ein Verfahren zur Auswahl der besten synthetischen Repräsentation, das sicherstellt, dass die Wahl selbst keine privaten Informationen über die Eingabedaten verrät.
    
- **Post-Processing Invariance:** Einmal erzeugte synthetische Daten bleiben unter Differential Privacy geschützt, egal wie tiefgreifend sie im Nachhinein analysiert oder mit anderen Datensätzen kombiniert werden.

---

## 4. Codebase Architecture & Module Inventory

Total tracked source and configuration files in `Differential-Privacy-Synthesizer/`: **33**

| File Path | Lines | Architectural Role |
| :--- | :---: | :--- |
| `dpsynth/pyproject.toml` | 120 | Build / Package Manifest |
| `dpsynth/setup.py` | 9 | Core Implementation Module |
| `dpsynth/dpsynth/__init__.py` | 9 | Core Implementation Module |
| `dpsynth/dpsynth/__main__.py` | 30 | Core Implementation Module |
| `dpsynth/dpsynth/evaluation/__init__.py` | 13 | Core Implementation Module |
| `dpsynth/dpsynth/evaluation/postprocess.py` | 284 | Core Implementation Module |
| `dpsynth/dpsynth/evaluation/privacy.py` | 214 | Core Implementation Module |
| `dpsynth/dpsynth/evaluation/utility.py` | 324 | Core Implementation Module |
| `dpsynth/dpsynth/models/__init__.py` | 13 | Core Implementation Module |
| `dpsynth/dpsynth/models/base.py` | 202 | Core Implementation Module |
| `dpsynth/dpsynth/models/ctgan_dp.py` | 602 | Core Implementation Module |
| `dpsynth/dpsynth/models/layers.py` | 252 | Core Implementation Module |
| `dpsynth/dpsynth/privacy/__init__.py` | 22 | Core Implementation Module |
| `dpsynth/dpsynth/privacy/accountant.py` | 241 | Core Implementation Module |
| `dpsynth/dpsynth/privacy/clipping.py` | 356 | Core Implementation Module |
| `dpsynth/dpsynth/privacy/exponential.py` | 267 | Core Implementation Module |
| `dpsynth/dpsynth/privacy/noise.py` | 248 | Core Implementation Module |
| `dpsynth/dpsynth/privacy/optimizer.py` | 230 | Core Implementation Module |
| `dpsynth/dpsynth/utils/__init__.py` | 8 | Core Implementation Module |
| `dpsynth/dpsynth/utils/data.py` | 339 | Core Implementation Module |
| `dpsynth/examples/quickstart.py` | 84 | Core Implementation Module |
| `dpsynth/tests/__init__.py` | 1 | Verification & Test Suite |
| `dpsynth/tests/test_accountant.py` | 193 | Verification & Test Suite |
| `dpsynth/tests/test_clipping.py` | 192 | Verification & Test Suite |
| `dpsynth/tests/test_data.py` | 180 | Verification & Test Suite |
| `dpsynth/tests/test_e2e.py` | 131 | Verification & Test Suite |
| `dpsynth/tests/test_evaluation.py` | 180 | Verification & Test Suite |
| `dpsynth/tests/test_exponential.py` | 205 | Verification & Test Suite |
| `dpsynth/tests/test_fuzz_accountant.py` | 168 | Verification & Test Suite |
| `dpsynth/tests/test_models.py` | 371 | Verification & Test Suite |
| `dpsynth/tests/test_noise.py` | 174 | Verification & Test Suite |
| `dpsynth/tests/test_optimizer.py` | 128 | Verification & Test Suite |
| `dpsynth/tests/test_postprocess.py` | 151 | Verification & Test Suite |

---

## 5. Gap Analysis & Production-Readiness Audit

- **Strict Privacy Budget Exhaustion Gate (`dpsynth/privacy/accountant.py`):** Must enforce a hard `PrivacyBudgetExceededError` lock inside `DPTrainingOptimizer.step()` and `DPCTGAN.fit()` the instant accumulated $\varepsilon(\delta)$ reaches the configured `max_epsilon` budget.
- **Cryptographic CSPRNG Noise Generation (`dpsynth/privacy/noise.py`):** Floating-point Box-Muller / standard NumPy PRNG draws must be guarded against seed reuse and floating-point LSB reconstruction attacks (Mironov snapping mechanism or `secrets.SystemRandom` seeded generator).
- **Membership Inference Attack (MIA) Auditing (`dpsynth/evaluation/privacy.py`):** Needs an automated shadow-model / distance-to-closest-record (DCR) empirical privacy audit gate that blocks dataset release if synthetic rows memorize training singletons.

---

## 7. Actionable Development & Verification Queue

Each task below is synchronized with `DEV/ROADMAP.md` and `DEV/TASKS.json` for automated or manual execution:

| Task ID | Title | Target Files | Verification Command |
| :---: | :--- | :--- | :--- |
| **TASK-01** | Enforce hard privacy budget exhaustion gate in RDP accountant and DP-CTGAN loop | `dpsynth/dpsynth/privacy/accountant.py`, `dpsynth/dpsynth/models/ctgan_dp.py` | `pytest dpsynth/tests/test_accountant.py dpsynth/tests/test_models.py -q` |
| **TASK-02** | Implement Mironov floating-point safe snapping mechanism in noise.py | `dpsynth/dpsynth/privacy/noise.py`, `dpsynth/tests/test_noise.py` | `pytest dpsynth/tests/test_noise.py -q` |
| **TASK-03** | Add singleton memorization and Distance-to-Closest-Record (DCR) release gate | `dpsynth/dpsynth/evaluation/privacy.py`, `dpsynth/tests/test_evaluation.py` | `pytest dpsynth/tests/test_evaluation.py -q` |
