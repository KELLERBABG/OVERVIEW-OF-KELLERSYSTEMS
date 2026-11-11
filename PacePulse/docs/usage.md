# Usage

## Installation

```bash
pip install -e .[dev]
```

Requires Python ≥ 3.10. The runtime dependency is `z3-solver>=4.12`;
developing requires `pytest` and `pytest-cov`.

## Quickstart

```python
from herzschrittmacher.modes import Chamber, PacingMode
from herzschrittmacher.pacing import PacingSystem
from herzschrittmacher.verification import VerificationEngine
from herzschrittmacher.execution_gate import ExecutionGate

# 1. Engine
pace = PacingSystem(mode=PacingMode.DDD)
pace.start()

# 2. Simulate physiology
pace.sense(Chamber.ATRIUM, 200)          # intrinsic A sense at t=200ms
decision = pace.check_escape(Chamber.VENTRICLE, 380)  # AV delay elapsed

# 3. Formal gate (Z3 proof before any pulse)
gate = ExecutionGate(VerificationEngine())
result = gate.release(decision, Chamber.VENTRICLE,
                      amplitude_ma=3.5, pulse_width_ms=0.5)

print(result.released)      # True only if all invariants proved UNSAT
print(result.verification.checks)  # per-invariant sat/unsat map
```

## CLI Reference

### `herzschrittmacher demo`

Runs the DDD pacing demo end to end: A-sense → AV delay → V escape check →
Z3 proof → gate verdict.

```bash
py -m herzschrittmacher demo
```

### `herzschrittmacher verify`

Reads a single JSON decision object on stdin and prints the full
verification result. Exit code: `0` released, `2` blocked.

```bash
'{"chamber":"ventricle","time_ms":1500,"amplitude_ma":3.5,"pulse_width_ms":0.5}' |
    py -m herzschrittmacher verify
```

JSON fields:

| Field | Type | Default |
|-------|------|---------|
| `chamber` | `"atrium"`/`"ventricle"` | `"ventricle"` |
| `time_ms` | int | `1000` |
| `amplitude_ma` | float | `3.5` |
| `pulse_width_ms` | float | `0.5` |

Example output:

```json
{
  "released": true,
  "reason": "proof obtained: all lethal invariants hold (all UNSAT)",
  "verification": {
    "safe": true,
    "all_invariants_hold": true,
    "blocked_invariants": [],
    "checks": {
      "no_pulse_in_refractory": "unsat",
      "no_pulse_in_blanking": "unsat",
      "rate_within_envelope": "unsat",
      "pulse_energy_within_envelope": "unsat",
      "no_sensed_pulse_in_blanking": "unsat"
    },
    "runtime_ms": 9.832,
    "detail": "all lethal invariants hold (all UNSAT)"
  },
  "decision_action": "pace",
  "decision_reason": "no prior event; escape interval 1000ms applies"
}
```

> Note: the stateless `verify` CLI operates on a fresh cardiac state, so a
> request with no prior events encodes `elapsed_ms=∞` and the rate invariant
> trivially holds. For stateful scenarios (e.g. a pulse *too soon after a
> ventricular sense*), use the `PacingSystem` API directly or extend the CLI
> driver — the blocked paths are covered by the test suite in
> `tests/test_verification.py`.

### `herzschrittmacher verify-symbolic`

Searches the programmable output envelope for a combination that violates
the energy invariant (critical state-space exploration).

```bash
py -m herzschrittmacher verify-symbolic --max-amplitude-ma 8.0 --max-width-ms 2.0
```

Output `envelope_violation: true` with a concrete `(amplitude_ma,
pulse_width_ms, energy_uj)` counterexample, or `false` when no combination
in the envelope can violate energy safety.

## Programmatic Integration

### Pacing engine lifecycle

```python
pace = PacingSystem(mode=PacingMode.VVI, cycles=TimingCycles(av_delay=150))
pace.start()
pace.stop()
```

All timing values are validated at construction against safety floors
(`MIN_AV_DELAY`, `MIN_PVARP`, `MIN_ESCAPE_INTERVAL_MS`).

### Decision API

`pace.sense(chamber, t)` → `PacingDecision` (usually `inhibit`).

`pace.check_escape(chamber, t)` → `PacingDecision`:

| Action | Meaning |
|--------|---------|
| `pace` | Escape interval elapsed; ready for formal verification |
| `inhibit` | Within escape interval, or mode cannot pace |
| `blocked` | Refractory — pacing refused regardless of timing |

`pace.request_pace(chamber, t, amplitude_ma, pulse_width_ms)` runs the
concrete invariant checks and updates state only on a clean pass.

### Gate policy (fail-closed)

`ExecutionGate.release(decision, chamber, A, W)`:

- decision action ≠ `pace` → **deny**
- decision has no path constraint → **deny**
- any invariant SAT or unknown → **deny**
- all invariants UNSAT → **release**

## Monitoring Modules

### Symbolic pulse analysis

```python
from herzschrittmacher.pulse_analysis import PulseAnalyzer, SymbolicPulse

analyzer = PulseAnalyzer()
pulse = SymbolicPulse(Chamber.VENTRICLE, 1000, 3.5, 0.5)
report = analyzer.analyze(pulse)      # charge_uc, energy_uj, over_limit
ledger = analyzer.analyze_ms_by_ms(pulse)  # per-millisecond ledger
```

### Taint audit

```python
from herzschrittmacher.taint import SensorStreamAuditor, TaintedValue, SensorSource

auditor = SensorStreamAuditor()
rate = TaintedValue(value=75.0, source=SensorSource.RATE_SENSOR)
record = auditor.audit_for_critical_use(rate, "rate_adaptation")  # allowed
telemetry = TaintedValue(value=5.0, source=SensorSource.TELEMETRY)
record = auditor.audit_for_critical_use(telemetry, "pulse_amplitude")  # blocked
```

### Temporal isolation

```python
from herzschrittmacher.isolation import IsolationModel

model = IsolationModel()
result = model.schedulability_check()   # feasible: bool, utilization breakdown
```

## Writing Tests

Tests are written with `pytest`; the package is importable through the
editable install (the `pyproject.toml` already adds `pythonpath = ["src"]`).

```bash
py -m pytest                    # full suite
py -m pytest -v                 # verbose
py -m pytest --cov=herzschrittmacher  # coverage