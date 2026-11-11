# Architecture

Herzschrittmacher OS is layered: a formal cardiac model at the bottom,
a timing-cycle pacing engine above it, and the Z3 verification engine +
trustless execution gate at the decision boundary. Monitoring concerns
(sensor taint, temporal isolation) run alongside but can never delay or
influence the pulse-decision path except through the gate.

## Module Map

```
┌──────────────────────────────────────────────────────────────┐
│ Presentation                                                   │
│   cli.py          console script + JSON verify interface       │
│   __main__.py     python -m herzschrittmacher                  │
├──────────────────────────────────────────────────────────────┤
│ Decision boundary (safety-critical)                            │
│   execution_gate.py   trustless execution gate (fail-closed)   │
│   verification.py     Z3 SMT encoder + SAT/UNSAT engine        │
├──────────────────────────────────────────────────────────────┤
│ Engine (timing cycles)                                         │
│   pacing.py          sense → inhibit/trigger → pulse FSM       │
│   constraints.py     kinetic path constraint generation        │
├──────────────────────────────────────────────────────────────┤
│ Formal model (domain)                                          │
│   modes.py           NBG modes, chambers, timing cycles,       │
│                      CardiacState (PVARP, blanking, AV)       │
├──────────────────────────────────────────────────────────────┤
│ Monitors (secondary, isolated)                                 │
│   pulse_analysis.py   symbolic charge/energy per ms            │
│   taint.py            sensor-stream taint audit                │
│   isolation.py        temporal isolation asymmetry model       │
└──────────────────────────────────────────────────────────────┘
```

## Data Flow (One Pulse Decision)

```
                    ┌──────────────┐
   sense/pace event │  pacing.py   │
   ─────────────────▶  (engine)    │
                    └──────┬───────┘
                           │ emits
                           ▼
                    ┌──────────────┐        ┌──────────────────┐
                    │ constraints  │ path   │  verification.py │
                    │ .py          │───────▶│  (Z3)            │
                    │ PathConstraint│        │  5 SAT/UNSAT     │
                    └──────────────┘        │  checks          │
                                            └────────┬─────────┘
                                                     │ safe?
                                                     ▼
                                            ┌──────────────────┐
                                            │ execution_gate   │
                                            │ .py (fail-closed)│
                                            └────────┬─────────┘
                                                     │ released?
                                                     ▼
                                            ┌──────────────────┐
                                            │  PULSE ENABLED   │
                                            │  (research: log) │
                                            └──────────────────┘
```

Key formal property of the flow: **the pulse command reaches the outside
world only if the gate returns `released=True`, which requires `safe=True`
from the verification engine, which requires all five lethal invariants to
be UNSAT under the decision's path constraint.** There is no fallback path.

## Module Responsibilities

### `modes.py` — formal cardiac model
- NBG pacing modes (VVI, AAI, DDD, VDD, DDDR, OFF) with paced/sensed
  polarity and response mode.
- `TimingCycles`: AV delay, PVARP, blanking windows, lower/upper rate
  intervals, refractory periods — validated against hard floors in
  `__post_init__`.
- `CardiacState`: the formal electrical-conduction state — last sense/pace
  per chamber, PVARP end, blanking windows. Methods: `is_refractory`,
  `in_blanking`, `blanking_started_by`, `can_sense`.

### `constraints.py` — kinetic path constraint generation
Every pacing decision is turned into a `PathConstraint`: an ordered tuple of
boolean predicates (`mode=DDD`, `refractory(ventricle)=false`,
`blanking(ventricle)=false`, `elapsed_ms=900`, `elapsed>=escape_interval=false`,
…). Lowercase booleans keep the Z3 encoding exact.

### `pacing.py` — timing-cycle state machine
- `sense()` updates state, starts PVARP/blanking, restarts escape timing.
- `check_escape()` implements the AV-delay and lower-rate escape logic;
  refractory always takes precedence and returns `blocked`.
- `request_pace()` runs the concrete invariant check and updates state only
  on a clean pass.

### `invariants.py` — lethal invariant enforcement
Five unbreakable properties:
1. `no_pulse_in_refractory`
2. `no_pulse_in_blanking`
3. `rate_within_envelope` (lower-rate floor)
4. `pulse_energy_within_envelope` (E = I²t/C)
5. `no_sensed_pulse_in_blanking` (oversensing pre-condition)

### `verification.py` — Z3 SMT integration
- `PathEncoder`: parses path predicates into Z3 `Bool`/`Int` symbols.
- `VerificationEngine.check_decision()`: per-invariant SAT/UNSAT check of
  `path ∧ ¬invariant`. SAT → blocked. All UNSAT → safe.
- `verify_symbolic_energy_envelope()`: critical state-space exploration —
  Z3 searches the whole programmable amplitude/width envelope for an
  energy violation.

### `execution_gate.py` — trustless execution gate
Fail-closed: denies when the engine did not authorize a pace, when the path
is missing, or when any invariant check returns SAT/unknown. Releases only
on a complete all-UNSAT proof.

### Monitors (cannot delay critical path)
- `pulse_analysis.py`: exact charge/energy, per-millisecond ledger.
- `taint.py`: provenance-tracks sensor values into critical decisions;
  telemetry is never allowed into the critical path.
- `isolation.py`: asymmetric schedule — monitoring budget is capped (5% CPU),
  verification budget fits inside the pulse window, and monitoring can never
  delay a pulse.

## Temporal Isolation Asymmetry

| Class | Can delay a pulse? | Budget |
|-------|--------------------|--------|
| CRITICAL (pulse) | never (it *is* the pulse) | 250µs window / 1000µs period |
| VERIFICATION (Z3) | yes — bounded, inside pulse window | 200µs budget |
| MONITORING (telemetry/diagnostics) | **never** | 50ms/s (5% CPU) |

The schedulability check verifies `u_critical + u_verification < 1` and that
the verification budget fits inside the pulse window.