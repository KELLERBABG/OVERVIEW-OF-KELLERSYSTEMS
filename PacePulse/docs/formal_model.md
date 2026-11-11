# Formal Model

This document defines the formal model of the heart's electrical conduction
states and the pacing engine's timing cycles that the toolchain verifies.
All timing values are **strict integers (milliseconds)** so the SMT encoding
stays decidable and the invariants exact.

## 1. Cardiac Chambers

```
Chamber ∈ { ATRIUM, VENTRICLE }
```

## 2. NBG Pacing Modes

Supported modes and their paced/sensed/response classification:

| Mode | Paced | Sensed | Response | Rate-adaptive |
|------|-------|--------|----------|---------------|
| `VVI` | Ventricle | Ventricle | Inhibited | no |
| `AAI` | Atrium | Atrium | Inhibited | no |
| `DDD` | Dual | Dual | Dual | no |
| `VDD` | Ventricle | Dual | Triggered | no |
| `DDDR` | Dual | Dual | Dual | **yes** |
| `OFF` | — | — | — | — |

Mode permissions are enforced by `PacingMode.can_pace(chamber)` and
`PacingMode.can_sense(chamber)`.

## 3. Timing Cycles

`TimingCycles` (defaults in ms):

| Cycle | Default | Meaning |
|-------|---------|---------|
| `av_delay` | 180 | Max A event → V pace interval (tracking) |
| `pvarp` | 350 | Post-ventricular atrial refractory period |
| `post_ventricular_blanking` | 40 | V-sense amplifier blanking after V event |
| `post_atrial_blanking` | 30 | A-sense amplifier blanking after A event |
| `lower_rate_interval` | 1000 | 60 bpm pacing floor |
| `upper_rate_interval` | 300 | 200 bpm ceiling |
| `ventricular_refractory` | 320 | Absolute V refractory after V event |
| `atrial_refractory` | 320 | Absolute A refractory after A event |

Hard parameter floors enforced at construction:

```
MIN_AV_DELAY           = 50
MIN_PVARP              = 150
MAX_PULSE_WIDTH_MS     = 2
MIN_ESCAPE_INTERVAL_MS = 150
```

## 4. Cardiac State

`CardiacState` is the formal electrical-conduction state:

```
time_ms                  : Int
last_atrial_sense_ms     : Int | None
last_ventricular_sense_ms: Int | None
last_atrial_pace_ms      : Int | None
last_ventricular_pace_ms : Int | None
pvarp_until_ms           : Int
ventricular_blanking_until_ms : Int
atrial_blanking_until_ms      : Int
ventricular_blanking_start_ms : Int | None
atrial_blanking_start_ms      : Int | None
```

### Refractoriness

```
is_refractory(VENTRICLE, t) := ∃last_v. t − last_v < ventricular_refractory
is_refractory(ATRIUM,   t) := (t < pvarp_until_ms) ∨ (t − last_a < atrial_refractory)
```

### Blanking

```
in_blanking(VENTRICLE, t) := t < ventricular_blanking_until_ms
in_blanking(ATRIUM,   t) := t < atrial_blanking_until_ms

blanking_started_by(chamber, t) := (blanking_start_ms == t)
```

`blanking_started_by` distinguishes a sense that **started** its own blanking
window (physiological, benign — the engine is expected to blank that
amplifier) from a sense that landed inside a blanking window started by a
*different* event (oversensing pre-condition, lethal candidate).

## 5. Escape Intervals

In a tracking mode (DDD/VDD), the ventricular escape is the AV delay while
the AV timer is active; otherwise it is the lower-rate interval:

```
escape_interval(VENTRICLE) := av_delay        if AV timer running
escape_interval(VENTRICLE) := lower_rate_interval   otherwise
escape_interval(ATRIUM)   := lower_rate_interval
```

Reference time for the escape countdown is the AV-timer start (ventricle,
tracking) or the last event in the chamber.

## 6. Lethal Invariants

The five unbreakable properties checked on **every** pulse request:

### 6.1 `no_pulse_in_refractory`
```
∀t. pulse(VENTRICLE, t) → ¬is_refractory(VENTRICLE, t)
```

### 6.2 `no_pulse_in_blanking`
```
∀t. pulse(chamber, t) → ¬in_blanking(chamber, t)
```

### 6.3 `rate_within_envelope`
```
∀t. pulse(chamber, t) → (t − last_event(chamber) ≥ lower_rate_interval)
```

### 6.4 `pulse_energy_within_envelope`
Capacitive-discharge pulse energy:

```
E = I² · t / C      [µJ, I in mA, t in ms, C in µF]
```

```
pulse(chamber, t, I, w) →
    w ≤ MAX_PULSE_WIDTH_MS
  ∧ w ≤ max_pulse_width_ms
  ∧ I ≤ max_amplitude_ma
  ∧ I²·w/C ≤ max_energy_uj
```

### 6.5 `no_sensed_pulse_in_blanking`
A sensed event inside a blanking window **not started by itself** shall not
drive a pacing decision (oversensing pre-condition):

```
sense(chamber, s) ∧ in_blanking(chamber, s) ∧ ¬blanking_started_by(chamber, s)
    → pulse decision from s is INVALID
```

## 7. Verification Semantics

For each invariant `i` and each pacing decision path `P`:

```
SAT(P ∧ ¬i)  →  a lethal violation is reachable along P  →  BLOCK
UNSAT(P ∧ ¬i) →  no violation is reachable               →  invariant holds

safe(P) := ∀i. UNSAT(P ∧ ¬i)
```

The pulse is released if and only if `safe(P)`.