# Herzschrittmacher OS: Formally Verified Pacemaker Pacing via Satisfiability-Modulo-Theories — A Research Whitepaper

**Version 0.1 — Phase 2 Prototype (Verification Engine)**

> **Status:** Research / open-source tooling. **Not a medical device.** No
> electrical impulse is ever delivered to the myocardium by this software.
> Deployment in an implantable pacemaker requires regulatory certification
> (FDA PMA/510(k), EU MDR, IEC 60601-2-31, ISO 14708-6) and organizational
> infrastructure beyond a solo developer. This whitepaper documents the
> formal-verification methodology, not a product.

---

## Abstract

Implantable cardiac pacemakers are life-sustaining cyber-physical systems
whose failure modes — oversensing, pacing into refractory tissue, runaway
rate, output-energy excursions — can be lethal. Classical development relies
on extensive testing and review, but testing can only demonstrate the
absence of *known* faults. This work presents **Herzschrittmacher OS**, an
SMT-based architecture that makes pacing safety *mathematically enforced at
decision time*: every pacing decision is converted into a quantifier-free
logical formula (a **kinetic path constraint**), handed to the Z3
satisfiability-modulo-theories solver, and checked against **lethal
invariants** — unbreakable properties such as *no pulse into refractory
tissue*, *rate within the programmed envelope*, and *energy within the tissue
safety bound*. Only if Z3 proves unsatisfiable every formula expressing *"this
decision violates a lethal invariant"* does a **trustless execution gate**
release the pulse command. We additionally present symbolic energy-envelope
exploration (critical state-space search in one solver query), taint-coded
sensor audit, and temporal isolation asymmetry. The prototype implements the
DDD timing-cycle core with five lethal invariants; 101 automated tests
exercise the full pipeline including the gate's fail-closed behaviour.

---

## 1. Motivation

Pacemaker recalls historically cluster around a small number of
reproducible logic faults: sensing algorithms that mis-handle blanked
events, rate-limiting bypasses, and mode-switch edge cases. These are
exactly the class of defects that *model checking and SMT* are designed to
eliminate: they are reachability questions over a finite-state timing
system. The cardiology literature already contains the canonical result
(Bengtsson et al.) that DDD pacing modes can be verified with timed
automata (UPPAAL). Our contribution is to move verification from *offline
design-time analysis* toward *online, per-decision checking*: the device
carries a decision procedure that vet each individual pacing command before
it can reach the output stage.

## 2. Architecture Overview

The system is layered; the critical-path ordering is strict:

```
physiology/events → Pacing Engine (timing FSM)
                 → Kinetic Path Constraint Generation
                 → Z3 Verification (5 × SAT/UNSAT checks)
                 → Trustless Execution Gate (fail-closed)
                 → pulse decision released ONLY if all-UNSAT proof exists
```

Secondary monitors (symbolic pulse analysis, taint audit, temporal-isolation
scheduler) execute independently and cannot delay or alter the critical
path except by feeding the gate's inputs.

```
        ┌──────────────────────────────────────────────┐
        │                Pacing Engine                 │  sense → inhibit/trigger → pulse
        └──────────────────────┬───────────────────────┘
                               │ PathConstraint (boolean predicates)
                               ▼
        ┌──────────────────────────────────────────────┐
        │         Z3: for each lethal invariant i      │
        │   check SAT( path ∧ ¬i )                     │
        └──────────────────────┬───────────────────────┘
                               │ any SAT?
                    ┌──────────┴──────────┐
                    ▼                     ▼
              BLOCK (fail-closed)    all UNSAT
                    │                     │
                    └──────────┬──────────┘
                               ▼
        ┌──────────────────────────────────────────────┐
        │        Trustless Execution Gate              │
        │   pulse released ⟺ safe(P) proved            │
        └──────────────────────────────────────────────┘
```

**Fig. 1** — The verification pipeline. There is no path around the gate.

## 3. Mathematical Foundation

### 3.1 Decision-as-Formula (Kinetic Path Constraints)

Let a pacing decision be the pair `(C, t)` — a command to pace chamber
`C ∈ {A, V}` at time `t ∈ ℤ` (ms). The engine captures its full decision
context as a conjunction of predicates:

```
P(C, t) ≡  mode = M
        ∧ can_pace(C)
        ∧ refractory(C, t) = b_refr
        ∧ blanking(C, t)   = b_blk
        ∧ elapsed_ms       = t − last_event(C)
        ∧ elapsed ≥ escape_interval
        ∧ (trigger semantics / inhibition semantics)
```

`P` is the **kinetic path constraint**: it records the *physiological and
logical conditions the engine believes* — the audit trail of *why* this
decision was made. `elapsed_ms=∞` encodes "no prior cardiac event" for
fresh-state initial pacing.

### 3.2 Lethal Invariants

Define five safety predicates `I₁…I₅` that must hold for every released pulse:

| i | Invariant | Definition (int, ms / µJ units) |
|---|-----------|--------------------------------|
| 1 | `no_pulse_in_refractory` | `¬is_refractory(C, t)` |
| 2 | `no_pulse_in_blanking` | `¬in_blanking(C, t)` |
| 3 | `rate_within_envelope` | `t − last_event(C) ≥ LRL` (lower-rate floor) |
| 4 | `pulse_energy_within_envelope` | `W ≤ W_max ∧ A ≤ A_max ∧ A²W/C ≤ E_max` |
| 5 | `no_sensed_pulse_in_blanking` | no sense in a foreign blanking window drove the decision |

where `LRL = lower_rate_interval`, `A` = current amplitude (mA),
`W` = pulse width (ms), `C` = output capacitance (µF), `E_max` = max energy
(µJ).

**Invariant 4 derivation.** A capacitive-discharge pacemaker delivers
energy `E = ∫ P dt = ∫ i²R dt`; with constant-current approximation over the
pulse width, `E = I²·t/C` — energy in µJ when `I` is in mA, `t` in ms, `C`
in µF. The invariant bounds this symbolic expression plus the raw
amplitude/width hardware limits.

### 3.3 The Verification Criterion

For each invariant `i`, the engine asks Z3:

```
SAT( P(C,t) ∧ ¬Iᵢ ) ?
```

- **SAT**   ⇔ there exists an assignment consistent with the engine's own
  decision path under which lethal invariant `i` is violated. The decision
  is **blocked**.
- **UNSAT** ⇔ within the model captured by `P`, violating `i` is
  *mathematically impossible*. Invariant `i` holds.

The decision is **safe** iff all five are UNSAT:

```
safe(P) := ⋀ᵢ₌₁⁵ UNSAT( P ∧ ¬Iᵢ )
```

**Correctness intuition.** Verification is counterexample search: we never
prove "this pulse is good" directly — we prove "no way to make it bad
exists," relative to the formal model.

### 3.4 Soundness and the Trustless Gate

The execution gate enforces, by construction:

```
release(P) ⟹ safe(P)
```

because every release path requires `safe=True` from the solver, and every
non-`safe` result (including `unknown` on timeout) is fail-closed. This is
**trustless** in the sense that no heuristic, no author bias, and no
configuration flag can release a pulse without a valid proof: the gate
*only* trusts the mathematical certificate.

## 4. Symbolic Pulse Analysis

Beyond single-decision checking, the toolchain computes — symbolically — the
exact charge/energy delivery of any pulse:

```
Q = I·W           [µC]
E = I²·W/C        [µJ]
```

`PulseAnalyzer` additionally produces a **per-millisecond ledger**
(`analyze_ms_by_ms`) so that a marginal fault in any millisecond of the
pulse is independently checked — excluding hardware overload, tissue-damage
thresholds, or functional failure *in advance*, per the specification.

## 5. Critical State-Space Exploration

Instead of enumerating the programmable output envelope cell-by-cell, Z3
searches it symbolically:

```
A ∈ [A_low, A_high], W ∈ [W_low, W_high], C = C₀,  A²W/C > E_max   ?
```

- `SAT`  → Z3 *returns a concrete violating* `(A, W)` — an automatically
  discovered edge case.
- `UNSAT` → the whole envelope is proven safe with respect to energy in a
  single query.

This is the *critical state space exploration* facet: lethal edge cases are
discovered by the solver, not by luck or exhaustive simulation.

## 6. Taint-Coded Sensor Stream Audit

Pacemaker telemetry is a documented attack surface (e.g., wireless
reprogramming exploits). `SensorStreamAuditor` attaches provenance to every
value entering the decision path. Policy:

1. **Telemetry never feeds a critical path** (parameter or pulse-decision
   altering).
2. Propagation depth beyond `max_propagation_depth` → provenance lost →
   **suspect**.
3. Values outside physiological plausibility bounds → **blocked**.

Every audit crossing is logged — an incorruptible lineage of all
sensor-derived influences on life-sustaining decisions.

## 7. Temporal Isolation Asymmetry

Secondary processes (telemetry transmission, diagnostics, episode logging)
must never (a) delay critical pulse delivery, or (b) leak information via
CPU-cycle measurement. The model enforces:

| Task class | May delay pulse? | Budget |
|------------|------------------|--------|
| CRITICAL (pulse) | (is the pulse) | 250 µs / 1000 µs period |
| VERIFICATION (Z3) | yes, bounded, inside window | 200 µs / period |
| MONITORING | **never** | 5% CPU cap |

Schedulability is checked via `u_crit + u_verif < 1` and
`budget_verif ≤ pulse_window`. Monitoring's 5% cap bounds the timing
side-channel that telemetry could otherwise create.

## 8. Automated Medical Specification Generation

Every decision produces a structured certificate:

```json
{
  "path": "mode=DDD ∧ can_pace(ventricle)=true ∧ …",
  "checks": {
    "no_pulse_in_refractory": "unsat",
    "no_pulse_in_blanking": "unsat",
    "rate_within_envelope": "unsat",
    "pulse_energy_within_envelope": "unsat",
    "no_sensed_pulse_in_blanking": "unsat"
  },
  "safe": true,
  "runtime_ms": 10.9
}
```

Accumulated, these certificates form a machine-readable formal
specification of the observed runtime behavior — an incorruptible,
auditable mathematical reference for medical certification bodies
(automated medical specification generation).

## 9. Experimental Validation

### 9.1 Test Suite

101 automated tests (pytest) cover: mode semantics, timing-cycle
refractoriness/blanking, path constraint generation, all five invariant
engines, the Z3 encoder, the SAT/UNSAT semantics (safe release vs.
refractory/rate/energy blocking), the symbolic envelope search, the gate's
fail-closed branches, taint policies, and schedulability. Coverage: 85%
overall; execution gate and taint auditor at 100%.

### 9.2 End-to-End Demo (DDD)

```
A-sense @200ms                    → inhibit (intrinsic sense)
V check @200ms (AV window)        → inhibit (escape 180ms not elapsed)
V check @380ms (AV elapsed)       → pace
Gate(3.5mA, 0.5ms) → 5 × UNSAT → RELEASED   (solver ≈ 11ms)
```

### 9.3 Known Lethal-Case Blocking (from tests)

| Scenario | Check behaviour |
|----------|-----------------|
| V-pace 100ms after V-sense | `no_pulse_in_refractory` → SAT → blocked |
| V-pace 400ms after V-sense (LRL=1000ms) | `rate_within_envelope` → SAT → blocked |
| 25mA / 3.0ms pulse | `pulse_energy_within_envelope` → SAT → blocked |
| Oversense inside foreign blanking window | `no_sensed_pulse_in_blanking` → blocked |

## 10. Limitations & Future Work

1. **Model fidelity** — integer-ms subset of DDD timing. Real refractory
   dispersion, capture-threshold dynamics, and lead impedance behavior
   require richer models (Phase 2.4 roadmap).
2. **Solver trust** — proof may be logged in SMT-LIB and cross-checked by an
   independent proof checker.
3. **Real-time budget** — host solver ≈11 ms per decision; embedded target
   requires Z3-compiled-to-C or an incremental proof oracle inside the
   200 µs verification budget (Phase 3.2).
4. **Hardware-in-the-loop** — ECG simulator + low-power MCU integration
   (Phase 3.5).
5. **Recall validation** — reproduce known FDA/MAUDE failure modes with the
   engine (Phase 2.7 validation milestone).
6. **Mode-switch analysis** — mode-switch path constraints are modelled;
   automated SAT analysis of the full mode-transition graph is pending.

## 11. Conclusion

Herzschrittmacher OS demonstrates that pacing safety can be expressed as a
decision-time satisfiability problem and enforced by a trustless gate: no
pulse without a proof. The contributions — kinetic path constraints as a
runtime audit formalism, five machine-checked lethal invariants, symbolic
envelope exploration, taint provenance, and temporal isolation asymmetry —
form a coherent methodology for advancing toward formally verified
implantable pacing software. The prototype is deliberately research-grade
and open-source; clinical deployment remains, and must remain, the domain
of regulated medical-device organizations.

## References

- Bengtsson, J., et al. — Automated analysis of an audio control protocol
  / DDD pacing timing verification with UPPAAL (classic case study).
- IEC 60601-2-31 — Medical electrical equipment, Part 2-31: Particular
  requirements for implantable pacemakers.
- ISO 14708-6 — Implants for surgery, Active implantable medical devices,
  Part 6: Particular requirements for pacemakers.
- IEC 62304 — Medical device software — Software life cycle processes.
- ISO 14971 — Medical devices — Application of risk management.
- de Moura, L., Bjørner, N. — Z3: An Efficient SMT Solver (TACAS 2008).

---

*Herzschrittmacher OS — research toolchain. Apache-2.0. Not a medical device.*