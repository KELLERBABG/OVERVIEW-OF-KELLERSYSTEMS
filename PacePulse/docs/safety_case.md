# Safety Case & Standards Mapping

This document maps the prototype's formal verification approach to the
applicable medical-device standards and the known pacemaker failure modes
catalogued from FDA/MAUDE recall analyses. It is a **research mapping**, not a
certification claim.

> **⚠️ No certification is claimed.** Implantable pacemaker certification
> (FDA PMA/510(k), EU MDR) requires a full Quality Management System, clinical
> trials, and organizational infrastructure that a solo developer cannot
> provide. This document is a traceability aid for the open-source research
> toolchain only.

## 1. Standards Traceability

| Standard | Scope | Prototype coverage |
|----------|-------|--------------------|
| **IEC 60601-2-31** | Implantable pacemakers — basic safety & essential performance | Pulse energy/rate envelopes as lethal invariants; refractory exclusion |
| **ISO 14708-6** | Implantable pacemakers — particular requirements | Timing-cycle model fidelity (PVARP, AV delay, blanking) |
| **IEC 62304** | Medical device software life cycle | Formal tier classification; verification artifacts per tier (research tier) |
| **ISO 14971** | Risk management | Failure-mode catalogue (below); each mapped to a lethal invariant or monitor |
| **IEEE 11073** | Personal health device communication | Not a focus in Phase 2; taint model reserves telemetry as untrusted |
| **FDA PMA/510(k)** | US market | Out of scope for solo research; roadmap Phase 4.5 |

## 2. Known Failure Modes → Controls

Mapped from the FDA/MAUDE recall catalogue described in the development plan
(oversensing failures, mode-switch bugs, rate-limit violations).

| Failure mode | Lethal invariant / control | Module |
|--------------|----------------------------|--------|
| Pulse delivered into refractory myocardium (tissue damage / arrhythmia induction) | `no_pulse_in_refractory` | `invariants.py`, engine gate |
| Oversensing → inappropriate pacing | `no_sensed_pulse_in_blanking` + blanking-state model | `invariants.py`, `pacing.py` |
| Rate-limit violation (runaway pacing) | `rate_within_envelope` (lower-rate floor) | `invariants.py` |
| Output-energy excursion (tissue damage) | `pulse_energy_within_envelope` (E = I²t/C) + symbolic envelope search | `invariants.py`, `verification.py` |
| Pacing into blanked sense window (sensing-model corruption) | `no_pulse_in_blanking` | `invariants.py` |
| Telemetry-driven manipulation of pacing parameters | `telemetry_never_critical` taint rule | `taint.py` |
| Sensor fault / implausible rate data | plausibility bounds in taint audit | `taint.py` |
| Monitoring/telemetry delaying critical pulse | temporal isolation asymmetry (monitoring never delays pulse) | `isolation.py` |
| Mode-switch bug (e.g. DDD→VVI during EMI) | mode-switch path constraints (modeled; SAT analysis pending Phase 2.7) | `constraints.py` |
| Sensor-initiated pulse during blanking (classic recall scenario) | `no_sensed_pulse_in_blanking` (cross-event blanking) | `invariants.py` |

## 3. Lethal Invariant → Standard Clause Mapping

| Invariant | IEC 60601-2-31 / ISO 14708-6 essence |
|-----------|----------------------------------------|
| `no_pulse_in_refractory` | Essential performance: pacing shall not deliver into refractory tissue |
| `rate_within_envelope` | Failure to limit pacing rate is a basic-safety concern (tachycardia induction) |
| `pulse_energy_within_envelope` | Basic safety: output energy bounded to avoid tissue damage |
| `no_sensed_pulse_in_blanking` | Sensing integrity: blanked events shall not drive decisions |
| `no_pulse_in_blanking` | Sensing-model integrity: pacing shall not occur in the sensing-blanking window |

## 4. How the Formal Proof Strengthens the Safety Case

For each pacing decision `P`, the toolchain produces a machine-checkable
certificate of the form:

```
P       — kinetic path constraint (what the engine believes)
∀i      — per-invariant Z3 result (sat/unsat/unknown)
safe(P) — ⋀ᵢ UNSAT(P ∧ ¬invariantᵢ)
```

This is exactly the *automated medical specification generation* facet: the
runtime behavior is transcribed into an incorruptible, machine-readable
mathematical reference that a certification body could audit. In a full
safety case this artifact would support the *software verification*
requirement of IEC 62304 and the *risk control measure verification*
requirement of ISO 14971.

## 5. Research-Grade Limitations (Explicit)

1. **Model fidelity** — the formal model captures a subset of DDD timing
   cycles with integer ms granularity; real physiology (refractory dynamics,
   capture threshold variation) is richer.
2. **Solver trust** — the proof is only as sound as Z3 itself; an external
   proof checker (e.g. SMT-LIB proof logging) is future work.
3. **No hardware-in-the-loop** — Phase 3.5 (HIL with ECG simulator + MCU)
   remains in the roadmap.
4. **No clinical data** — all timing constants are research defaults and
   must be replaced by a manufacturer's programmed parameters.
5. **No certification** — this document is traceability research, not a
   regulatory submission.

## 6. Verification Artifacts Produced

| Artifact | Where |
|----------|-------|
| Kinetic path certificates | `VerificationResult.as_dict()` / CLI `verify` |
| Energy-envelope exploration results | `verification.verify_symbolic_energy_envelope` |
| Gate history (every decision + proof) | `ExecutionGate.history` |
| Taint audit log | `SensorStreamAuditor.records` |
| Schedulability report | `IsolationModel.schedulability_check` |
| Test suite (101 tests) as behavioral spec | `tests/` |