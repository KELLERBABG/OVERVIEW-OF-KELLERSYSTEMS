# Verification

This document describes the SMT-based verification engine: how kinetic path
constraints are encoded into Z3, what SAT/UNSAT means for safety, and how the
symbolic energy-envelope search performs critical state-space exploration.

## 1. Decision → Logic

Each pacing decision from the engine is a `PathConstraint` — an ordered tuple
of boolean predicates. Examples from a real DDD ventricular escape check:

```
mode=DDD
can_pace(ventricle)=true
refractory(ventricle)=false
blanking(ventricle)=false
elapsed_ms=900
elapsed>=escape_interval=false
```

The `PathEncoder` parses each predicate into Z3 symbols:

| Predicate | Z3 symbol | Sort |
|-----------|-----------|------|
| `refractory(ventricle)=b` | `refractory_v` | `Bool` |
| `blanking(ventricle)=b` | `blanking_v` | `Bool` |
| `can_pace(ventricle)=b` | `can_pace_v` | `Bool` |
| `can_sense(ventricle)=b` | `can_sense_v` | `Bool` |
| `sense(ventricle)=b` | `sense_v` | `Bool` |
| `elapsed_ms=n` | `elapsed_ms` | `Int` |
| `time_ms=n` | `time_ms` | `Int` |
| `elapsed>=escape_interval=b` | `escape_elapsed` | `Bool` |
| `elapsed_ms=∞` | `elapsed_ms == 10⁹` | `Int` |

Informational components (`mode=DDD`, `trigger=av_delay_elapsed`,
`inhibited_by_intrinsic=false`, `reason=…`) carry no SMT semantics and are
ignored by the encoder — they exist for the audit trail and automated
specification generation.

## 2. Invariant Encodings

For a decision to pace chamber `c` with amplitude `A`, width `W`, the engine
constructs five negation formulas (each is `path ∧ ¬invariant`):

### 2.1 no_pulse_in_refractory
```
path ∧ refractory_c == True
```

### 2.2 no_pulse_in_blanking
```
path ∧ blanking_c == True
```

### 2.3 rate_within_envelope
```
path ∧ elapsed_ms < lower_rate_interval_ms
```

### 2.4 pulse_energy_within_envelope
```
path ∧ A == amplitude ∧ W == pulse_width ∧ C == capacitance
    ∧ (A²·W/C > max_energy ∨ A > max_amplitude ∨ W > max_pulse_width)
```

### 2.5 no_sensed_pulse_in_blanking
```
path ∧ sense_c == True ∧ blanking_c == True
```

## 3. SAT/UNSAT Semantics

Each formula is checked with a fresh Z3 solver (50 ms timeout):

| Result | Meaning | Gate decision |
|--------|---------|----------------|
| `SAT` | There exists an assignment where the decision path permits a lethal invariant violation | **BLOCK** |
| `UNSAT` | No assignment violates the invariant along this path | invariant holds |
| `unknown` | Solver did not decide in the time budget | **BLOCK** (fail-closed) |

The decision is `safe` iff **all five** checks are `UNSAT`:

```
safe(P) := ⋀ᵢ UNSAT(P ∧ ¬invariantᵢ)
```

The trustless execution gate releases the pulse **iff** `safe(P)`.

## 4. Why UNSAT Means "Conditionally Safe"

The check `SAT(P ∧ ¬invariantᵢ)` searches for a *counterexample*: an
assignment to the symbolic variables consistent with the pacing engine's own
description of its decision path that violates invariant `i`. If Z3 finds
none, then *within the model captured by the path*, violating invariant `i`
is impossible — the decision is conditionally safe with respect to the formal
model. The soundness disclaimer is the usual SMT one: the proof is only as
strong as the model's fidelity to the real physiology/hardware.

## 5. Critical State-Space Exploration (Energy Envelope)

`VerificationEngine.verify_symbolic_energy_envelope()` does not verify a
single concrete pulse — it asks Z3 to **search the entire programmable output
envelope** for a violation:

```
A ∈ [a_lo, a_hi]
W ∈ [w_lo, w_hi]
C == capacitance
A²·W/C > max_energy
```

- `SAT` → Z3 produces an actual `(A, W)` pair inside the programmed envelope
  that violates the energy invariant (a discovered edge case).
- `UNSAT` → no combination in the envelope can violate energy safety.

This is the critical state-space exploration facet: instead of enumerating
thousands of amplitude/width combinations, Z3 systematically excludes the
whole envelope in a single proof query.

## 6. Worked Example (from the demo)

Pacing decision: DDD, V-pace at t=380 ms after an A-sense at t=200 ms
(escape = AV delay 180 ms elapsed).

```
path = mode=DDD ∧ can_pace(ventricle)=true ∧ refractory(ventricle)=false
     ∧ blanking(ventricle)=false ∧ elapsed_ms=∞
     ∧ elapsed>=escape_interval=true
```

Z3 checks with amplitude 3.5 mA, width 0.5 ms:

```
no_pulse_in_refractory          → UNSAT
no_pulse_in_blanking            → UNSAT
rate_within_envelope            → UNSAT
pulse_energy_within_envelope    → UNSAT
no_sensed_pulse_in_blanking     → UNSAT
```

All-UNSAT → `safe=True` → gate releases. Measured end-to-end solver
runtime ≈ 11 ms (well inside the 200 µs verification budget target is a
future-hardening item — the timeout is configurable).

## 7. Real-Time Integration Constraint

The verification budget is bounded by the temporal isolation model: the SMT
proof must fit inside the 250 µs pulse window with 200 µs budget. In the
research prototype the solver runs on the host; the RT target integration
(Zephyr/FreeRTOS + Z3-compiled-to-C or an external proof oracle) is Phase 3
of the development plan.