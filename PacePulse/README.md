# Herzschrittmacher OS

**Formally Verified Pacemaker Real-Time OS — Research Toolchain**

A research/open-source framework for formally verifying pacemaker pacing logic using SMT-based safety verification (Z3), symbolic pulse analysis, lethal invariant enforcement, and a trustless execution gate.

> **⚠️ NOT A MEDICAL DEVICE.** This project targets safety-critical medical devices requiring regulatory certification (FDA PMA/510(k), EU MDR, IEC 60601-2-31, ISO 14708-6). This is a **research prototype for formal verification tooling only** — it is not deployable as an implantable product, and no pacing pulse is ever delivered by this software.

## Documentation

Full documentation lives in [`docs/`](docs/index.md):

| Document | Purpose |
|----------|---------|
| [Documentation Index](docs/index.md) | Start here — document map, quick start, pipeline summary |
| [Architecture](docs/architecture.md) | Layered architecture, module responsibilities, data flow |
| [Formal Model](docs/formal_model.md) | Timing cycles, NBG modes, cardiac state, lethal invariants |
| [Verification](docs/verification.md) | Z3 SMT encoding, SAT/UNSAT semantics, symbolic envelope search |
| [Usage](docs/usage.md) | Installation, quickstart, CLI reference, integration |
| [API Reference](docs/api.md) | Complete public API |
| [Safety Case](docs/safety_case.md) | Standards mapping (IEC 60601-2-31, ISO 14708-6, IEC 62304, ISO 14971) and failure-mode analysis |
| [Whitepaper](docs/whitepaper.md) | Research whitepaper — decision-as-formula, trustless gate, symbolic exploration |

## Scope

The prototype focuses on the **mathematically verifiable core** of DDD-mode pacing:

- **Formal cardiac model** — refractory periods (PVARP, blanking), AV delay, pacing modes (VVI, DDD, DDDR, AAI, VDD), mode switching
- **Kinetic path constraint generation** — boolean path conditions per pacing decision (sense → inhibit/trigger → pulse)
- **Lethal invariant enforcement** — invariants proving no pulse into a refractory period, no rate-limit violation, no energy-envelope excursion
- **Symbolic pulse analysis** — symbolic charge/energy computation per millisecond (amplitude × pulse width, capacitor state)
- **Taint-coded sensor audit** — tracks data flow from untrusted sources (EGM electrodes, rate sensors, telemetry) into critical pulse decisions
- **Temporal isolation asymmetry** — scheduling model preventing telemetry/diagnostics from delaying critical pulse delivery
- **Trustless execution gate** — blocks a pulse command unless Z3 proves the action safe

## Installation (development)

```bash
pip install -e .[dev]
```

## Usage

```python
from herzschrittmacher.pacing import PacingSystem
from herzschrittmacher.modes import PacingMode, Chamber

pace = PacingSystem(mode=PacingMode.DDD)
pace.start()

# Simulate an atrial sense event at t=200ms
pace.sense(Chamber.ATRIUM, 200)
# Simulate a ventricular sense event at t=380ms
pace.sense(Chamber.VENTRICLE, 380)
```

CLI:

```bash
# Run the demo verification script
herzschrittmacher demo

# Verify a pacing decision (JSON event log on stdin)
'{"chamber":"ventricle","time_ms":1500,"amplitude_ma":3.5,"pulse_width_ms":0.5}' |
    herzschrittmacher verify
```

## Project Layout

```
src/herzschrittmacher/
    modes.py            # NBG pacing modes, chambers, cardiac state model
    pacing.py           # Timing-cycle state machine (AV delay, PVARP, blanking)
    constraints.py      # Kinetic path constraint generation
    invariants.py       # Lethal invariant definitions
    verification.py     # Z3 SAT/UNSAT checks
    pulse_analysis.py   # Symbolic pulse energy / charge computation
    taint.py            # Taint-coded sensor stream audit
    isolation.py        # Temporal isolation asymmetry scheduling model
    execution_gate.py   # Trustless execution gate
    cli.py              # CLI entrypoint
    __main__.py         # python -m herzschrittmacher
docs/                   # Documentation + research whitepaper
tests/                  # pytest suite (101 tests)
```

## Formal Model Summary

DDD timing-cycle core (subset verified in this prototype):

| Cycle | Definition |
|-------|------------|
| `AV-Delay` | Max interval between A-sense/pacing and V-pacing |
| `PVARP` | Post-ventricular atrial refractory period — A-senses ignored |
| `VA-Interval` | Lower-rate limit at atrial level |
| `Blanking` | Post-pace refractory for the paced chamber |
| `LRL` | Lower rate limit (pacing floor, no faster than e.g. 30 bpm) |

## Key Lethal Invariants (prototype)

1. `no_pulse_in_refractory` — never deliver a pulse while the target chamber is refractory
2. `rate_within_envelope` — escape interval never below the programmed lower-rate limit and never above the upper rate limit
3. `pulse_energy_within_envelope` — symbolic energy `E = I² · t / C` never exceeds the programmed tissue-safety envelope
4. `no_sensed_pulse_in_blanking` — sensed events during blanking never trigger pacing

## Verification Approach

Each pacing decision generates a boolean path condition (`kinetic path constraint`). The Z3 solver checks satisfiability of the decision combined with the lethal invariants. If the conjunction is `SAT` — the decision is **blocked** by the trustless execution gate. Only `UNSAT` (i.e. no assignment satisfies both the path and the negation of safety) lets a pulse through.

```
pacing_decision ∧ ∨(¬lethal_invariant_i)   --SAT--> BLOCKED
∀i: pacing_decision ∧ ¬lethal_invariant_i   --UNSAT--> ALLOWED
```

## Testing

```bash
py -m pytest                    # 101 tests
py -m pytest --cov=herzschrittmacher
```

## Licensing

Apache-2.0 — research / open-source tooling. Not a medical device. See `LICENSE`.

## Status

**Phase 2 prototype (Verification Engine).** Expect rapidly changing APIs. Tested with pytest against Z3 solver integration.

## References

- IEC 60601-2-31 — implantable pacemakers (basic safety & essential performance)
- ISO 14708-6 — implantable pacemakers (particular requirements)
- IEEE 11073 — personal health device communication
- IEC 62304 — medical device software life cycle
- ISO 14971 — risk management
- UPPAAL DDD-mode case study (Bengtsson et al., timing-cycle verification)