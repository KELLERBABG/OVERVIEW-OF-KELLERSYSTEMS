# Herzschrittmacher OS — Documentation

**Formally Verified Pacemaker Real-Time OS — Research Toolchain**

This documentation describes the research/open-source toolchain for formally
verifying pacemaker pacing logic via SMT-based safety verification (Z3),
symbolic pulse analysis, lethal invariant enforcement, and a trustless
execution gate.

> **⚠️ NOT A MEDICAL DEVICE.** This project targets safety-critical medical
> devices requiring regulatory certification (FDA PMA/510(k), EU MDR,
> IEC 60601-2-31, ISO 14708-6). This is a **research prototype for formal
> verification tooling only**. No pacing pulse is ever delivered by this
> software, and it is not deployable as an implantable product.

## Document Map

| Document | Purpose |
|----------|---------|
| [`architecture.md`](architecture.md) | Layered architecture, module responsibilities, data flow |
| [`formal_model.md`](formal_model.md) | The formal cardiac/pacing model: timing cycles, modes, lethal invariants |
| [`verification.md`](verification.md) | Z3 SMT encoding, SAT/UNSAT semantics, symbolic envelope search |
| [`usage.md`](usage.md) | Installation, quickstart, CLI reference, integration examples |
| [`api.md`](api.md) | Public API reference |
| [`safety_case.md`](safety_case.md) | Standards mapping (IEC 60601-2-31, ISO 14708-6, IEC 62304, ISO 14971) and failure-mode analysis |
| [`whitepaper.md`](whitepaper.md) | Research whitepaper: mathematical trajectory mapping, kinetic path constraints, critical state-space exploration, trustless gate |
| [`../development_plan.md`](../development_plan.md) | The 56-month solo-developer roadmap the scaffold implements (Phase 2 core) |

## Quick Start

```bash
pip install -e .[dev]

# DDD pacing demo with the formal gate
py -m herzschrittmacher demo

# Verify one decision from JSON on stdin
'{"chamber":"ventricle","time_ms":1500,"amplitude_ma":3.5,"pulse_width_ms":0.5}' |
    py -m herzschrittmacher verify

# Search the output envelope for an energy-invariant violation
py -m herzschrittmacher verify-symbolic
```

## Verification Pipeline (One Paragraph)

Every pacing decision (sense → inhibit/trigger → pulse) is translated by the
pacing engine into a **kinetic path constraint** — a boolean expression of the
physiological and logical conditions of the decision. The Z3 SMT solver then
asks, per lethal invariant, whether the path is satisfiable together with a
*violation* of the invariant. Any **SAT** answer means a lethal violation is
reachable along that path and the pulse is **blocked** by the trustless
execution gate. Only the all-**UNSAT** case releases the pulse — and it is
released *because* a mathematical proof of safety exists, not because a
heuristic approved it:

```
pacing_decision ∧ ∨(¬lethal_invariant_i)   --SAT-->   BLOCKED
∀i: pacing_decision ∧ ¬lethal_invariant_i   --UNSAT--> ALLOWED
```

## Project Status

**Phase 2 prototype (Verification Engine)** of the development plan. All 101
pytest tests pass (85% coverage; execution gate and taint auditor at 100%).
APIs are expected to evolve.

## References

- IEC 60601-2-31 — implantable pacemakers (basic safety & essential performance)
- ISO 14708-6 — implantable pacemakers (particular requirements)
- IEEE 11073 — personal health device communication
- IEC 62304 — medical device software life cycle
- ISO 14971 — risk management
- UPPAAL DDD-mode case study (Bengtsson et al., timing-cycle verification)