# PacePulse — Development Roadmap & Action Tasks

> This file is the authoritative internal development roadmap and task sequencing plan.
> **Language:** `Python`
> **Summary:** A Python + Z3 formally verified cardiac pacemaker real-time OS research toolchain (`src/herzschrittmacher/pacing.py`, `invariants.py`, `constraints.py`, `verification.py`, `execution_gate.py`, `pulse_analysis.py`, `taint.py`, `isolation.py`, `modes.py`) with 9 test suites and 101 passing Z3 tests.

## Task Queue

- [ ] **TASK-01: Add Z3 formal proof for R-on-T vulnerable period exclusion and runaway ceiling**
  - **Target Files:** `src/herzschrittmacher/invariants.py`, `tests/test_invariants.py`
  - **Context Files:** `src/herzschrittmacher/execution_gate.py`, `src/herzschrittmacher/verification.py`
  - **Verify:** `pytest tests/test_invariants.py -q`
  - **Spec:** Extend `src/herzschrittmacher/invariants.py` and `tests/test_invariants.py` with an explicit `R_on_T_VulnerableWindowInvariant`.

1. Using Z3 constraints, prove that no ventricular pace pulse can ever be emitted inside the vulnerable repolarization window $[t_{\text{intrinsic\_R}} + V_{\text{vuln\_start}}, t_{\text{intrinsic\_R}} + V_{\text{vuln\_end}}]$ (typically $150\text{--}300\text{ ms}$ post-QRS), preventing pacing-induced ventricular fibrillation.
2. Add unit tests in `tests/test_invariants.py` verifying that `ExecutionGate` blocks any synthetic pulse command falling inside the vulnerable T-wave window.

- [ ] **TASK-02: Prove glitch-free ERI battery depletion fallback from DDDR to VVI mode**
  - **Target Files:** `src/herzschrittmacher/modes.py`, `tests/test_modes.py`
  - **Context Files:** `src/herzschrittmacher/pacing.py`
  - **Verify:** `pytest tests/test_modes.py -q`
  - **Spec:** Enhance `src/herzschrittmacher/modes.py` to model Elective Replacement Indicator (ERI) battery voltage step-down from dual-chamber `DDDR` to energy-conserving `VVI` mode.

1. Prove that across the mode-switch cycle boundary, the inter-beat interval satisfies $\text{URI} \le \Delta t_{\text{beat}} \le \text{LRI}$ (no missed beat / asystole gap during state transition).
2. Add unit tests in `tests/test_modes.py`.
