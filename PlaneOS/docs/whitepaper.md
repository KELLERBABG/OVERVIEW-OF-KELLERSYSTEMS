# Plane OS: A Trustless Execution Gate for Formally Verified Flight Envelope Protection

**Draft — research/whitepaper version 0.1**
*Plane OS Contributors — open-source formal verification tooling for avionics*

---

## Abstract

We present **Plane OS**, an open-source prototype of a formally verified
flight-envelope protection architecture. Plane OS introduces a **trustless
execution gate**: a hardware-philosophy-independent software component that
blocks any flight-surface or thrust command unless a mathematical proof of
its safety exists within the control cycle. Safety is expressed as
**lethal invariants** — formulas over a canonical control-cycle environment
that must be unsatisfiable for the command to proceed. The prototype
implements the gate, a symbolic nonlinear-real-arithmetic (NRA) formula DSL,
predictive **kinetic path constraints** over a horizon, **taint-coded sensor
auditing**, symbolic thrust limits, and a hash-chained flight-recorder
specification. We report 63 automated tests covering end-to-end behavior and
a demo where an overspeed condition at 354 kt IAS is provably excluded and
the command is blocked before actuation.

**Scope and disclaimer.** Plane OS is a *research and educational* artifact.
It is not certified to DO-178C (DAL A), ARP4754A, or EASA CS-25, and it must
not be deployed in aircraft. The aerodynamic model is a mock; the
verification architecture is the contribution.

---

## 1. Introduction

### 1.1 Motivation

Modern fly-by-wire aircraft implement envelope protection in software (e.g.,
stall prevention, V-N limiting, alpha floor). The certification standards
(DO-178C, ARP4754A) require showing — through process, review, and test —
that such software never issues unsafe commands. A recurring theme in
aviation accidents (e.g., high-speed upsets, MCAS-type sensor-trust
failures) is that a **single untrusted data source or an unvalidated command
can reach the control surfaces**.

We propose a defense-in-depth architectural pattern: **make the command
path itself verify its own safety as a mathematical obligation**, and record
every verdict in a tamper-evident chain. The control loop remains
conventional; a smallest-possible gate component sits in front of the
actuators and computes a proof obligation in real time.

### 1.2 Contributions

1. A **canonical environment** abstraction that makes hazard formulas
   auditable and reusable across backend solvers.
2. A **trustless execution gate** with conservative (fail-safe) semantics:
   *any* inability to prove safety blocks the command.
3. **Kinetic path constraints** that lift single-cycle verification to a
   prediction horizon, approximating critical state-space exploration.
4. A **taint-coded sensor audit** integrated as a *lethal* invariant rather
   than an advisory warning.
5. A **hash-chained flight recorder** producing machine-readable formal
   specifications for every cycle (tamper-evident).
6. An open-source implementation with 63 unit/integration tests.

---

## 2. Background and related work

| Line of work | Relevance | Difference |
|---|---|---|
| Runtime verification (RV) | Monitoring safety properties online | We make monitoring *gating*: RV typically alerts; the gate *blocks* |
| Hybrid-system reachability (Flow\*, JuliaReach, CORA) | Verified continuous dynamics | Our mock uses ground evaluation; reachability is future work |
| SMT-based verification (Z3, dReal) | Decision procedures for NRA | Our `StubSolver` is a sound ground-evaluation proxy; `Z3Backend` is provided |
| ARINC 653 partitioning | Temporal isolation | Currently scheduling is out of scope; the architecture reserves it |
| Model-based development (SCADE) | Verified code generation | Our DSL + gate is a lightweight alternative for prototyping |
| Trusted execution environments (TEEs) | Hardware trust roots | "Trustless" here means *no software trust assumed*: safety is proven, not inherited |

The term *trustless* is borrowed from blockchain/TEE literature: no party
(sensor, pilot input, control law, or even the simulator) is trusted — only
the proof obligation matters.

---

## 3. Formal model

### 3.1 Control cycle

A control cycle is a tuple:

```
C = (S, D, A, M, Q)
```

where:

- `S` is the concrete flight state (position, body velocity, quaternion,
  angular velocity, time);
- `D` are derived quantities (IAS, Mach, alpha, nz, altitude, terrain
  clearance, climb rate);
- `A = (throttle, elevator, aileron, rudder)` is the **candidate** command;
- `M` are magic constants parameterizing limits (envelope, engine, terrain,
  aircraft);
- `Q` is the taint/quality state of sensors and channels.

`ControlSnapshot::env()` maps `(S, D, A, M, Q)` into a ground environment
`E : Var → ℝ` where variables live in canonical namespaces
(`state.*`, `cmd.*`, `his.*_k`, `mag.*`, `sym.*`).

### 3.2 Hazard formulas

A **hazard** `H` is a formula of NRA over `Var`:

```
H ::= ⊤ | ⊥ | (e1 ≺ e2) | ¬H | H1 ∧ H2 | H1 ∨ H2
e ::= c | x | −e | e1+e2 | e1−e2 | e1·e2 | e1/e2 | e^e | |e| | min | max
```

`H` is *satisfiable* (SAT) on environment `E` iff there is an assignment to
the free variables extending `E` that makes `H` true; unsatisfiable (UNSAT)
means the hazard cannot occur given the current, fully-specified cycle.

### 3.3 Gate predicate

The gate evaluates a finite set of hazards `{H1, …, Hk}` and admits the
command iff:

```
∀ i · Verdict(E, Hi) = UNSAT
```

where `Verdict` is a **conservative** decision procedure:

```
Verdict(E, H) = UNSAT   only if H is unsatisfiable on E
```

Equivalently, the gate may return `SAT` or `UNKNOWN` for a formula that is
actually satisfiable, but must never return `UNSAT` for a satisfiable one.
This is precisely the safety condition of a one-sided decision procedure.

When the gate admits, the command is committed; otherwise the aircraft
freezes (fail-safe).

### 3.4 Kinematic horizon lifting

For horizon `N`, define the **lifted hazard**:

```
L_H^N = H[state ↦ state] ∨ ⋁_{k=1..N} H[state ↦ his_k]
```

where substitution `H[state ↦ his_k]` replaces `state.x` with `his.x_k`. The
simulator produces the `his_k` environments by integrating the candidate
command forward with the mock dynamics. With `L_H^N` UNSAT, the envelope is
provably respected for the entire predicted path — not merely the current
instant.

---

## 4. Architecture

### 4.1 Component diagram

```
Pilot input ─▶ Control law ─▶ Candidate A
                                   │
                                   ▼
                          ControlSnapshot (canonical env)
                                   │
                                   ▼
        ┌─────────────────────────────────────────────┐
        │  Trustless Execution Gate                   │
        │  ┌─ envelope_escape (V-N, Mach, alt, terr)  │
        │  ├─ vn_maneuver_limit                       │
        │  ├─ thrust_hazard (N1/N2/EGT/oil/reverser)  │
        │  ├─ taint_hazard (channel_ok ∧ cmd ≠ 0)     │
        │  └─ throttle_on_failed_engine               │
        │     decision: ∀i Hi ≡ UNSAT?                │
        └─────────────────┬───────────────────────────┘
                          │ approved?
                  yes ────┴──── no
                  apply            freeze
                  │                │
                  ▼                ▼
            dynamics mock    recorder records
                  │              BLOCKED verdict
                  ▼              (+ hash chain)
            recorder records
            APPROVED verdict
```

### 4.2 The fatal-invariant set

The default gate checks five hazards. Three represent continuous physical
limits (`envelope_escape`, `vn_maneuver_limit`, `thrust_hazard`), one
represents the *sensor trust* limit (`taint_hazard`), and one represents
the *command authorization* limit (`throttle_on_failed_engine`). All five
share the same shape:

```
hazard is TRUE  ⇒  the aircraft is unsafe or the command is unauthorized
```

The disjunction ordering matters only for reporting; the gate's decision is
the conjunction of all pairwise UNSAT verdicts.

### 4.3 Canonical namespaces

Using namespaced variables (`state.ias_knots`, `mag.envelope.vne_knots`,
`his.ias_knots_3`) makes every formula self-describing: a human auditor can
read a formula and see exactly which physical quantities and which limit
constants constrain a decision. This directly supports the "machine-readable
mathematical reference" goal from the development plan.

### 4.4 Flight recorder

Each cycle appends a `SpecRecord` containing:

- cycle index and simulation time;
- gate decision and the first blocking case;
- for every case: verdict and canonical formula text;
- the full environment `E`;
- `prev_hash` and a hash over the canonical serialization.

`validate()` replays the chain from the genesis record, so any
post-hoc tampering is detected. The hash is FNV-1a 64-bit for
dependency-free determinism; a production artifact would use SHA-256.

---

## 5. Soundness and completeness

### 5.1 Soundness (no unsafe command)

Given a conservative `Verdict`, the gate **never** admits a command when some
hazard is satisfiable on the cycle:

```
admit(C)  ⇒  ∀ i · H_i is UNSAT on E(C)
```

Since each `H_i` is a necessary condition for a particular unsafe event
(envelope escape, V-N corner, engine limit, tainted channel, unauthorized
throttle), admission implies none of those events is possible on the
canonical state.

The `StubSolver` is sound for the ground state for two independent reasons:

1. **Exactness:** evaluation of the formula tree on the concrete
   environment has no abstraction; if it returns `UNSAT`, the formula is
   literally false under that assignment.
2. **Conservativeness:** missing variables and domain errors (division by
   zero, invalid `pow`) yield `UNKNOWN`, which blocks.

### 5.2 Completeness (liveness of the prototype)

Completeness — every safe command is eventually admitted — is *not*
guaranteed and is not required for safety. It is, however, necessary for a
usable autopilot. In the prototype, completeness is limited by:

- the fidelity of `FlightDynamics` (a coarse mock; alpha/nz are approximate);
- the horizon model's prediction error accumulating over `N` steps;
- the use of a single density-compensated IAS formula rather than a Mach
  schedule.

These limitations are explicitly documented; they do not affect soundness.

### 5.3 The `Unknown`-blocking policy

`UNKNOWN` is treated as dangerous. This is the classical *fail-safe* choice:
the gate may stall the aircraft (false positive) but never flies it into a
hazard (false negative). The recorder preserves the `UNKNOWN` verdicts so
that a human can review why safety could not be proven.

---

## 6. Threat model

Plane OS is not a security product, but it formalizes the trust boundaries
that safety analysis must respect:

1. **Sensor compromise / spoofing.** Tainted data (pitot, GPS, ADS-B, radar)
   is represented as taint on the quantities it feeds. Any command on a
   channel whose dependent quantity is untrusted is blocked. A single
   failing IRU degrades the elevator channel in the demo test.
2. **Control-law fault.** The control law is an *untrusted candidate
   generator*: a software fault that produces an aggressive command cannot
   bypass the gate.
3. **Numerical / floating-point error.** Every limit check is encoded as NRA
   and evaluated symbolically on the concrete state; a domain error is
   conservative.
4. **Post-hoc tampering.** The hash-chained recorder makes silent alteration
   of the verification history detectable.

Plane OS does **not** currently model: side-channel timing leakage
(temporal-isolation asymmetry from the plan), multi-lane dissimilar
redundancy, or cyber-physical attacks on the actuators themselves. These are
documented as future work.

---

## 7. Experimental evaluation

### 7.1 Correctness tests (63 passing)

| Suite | Count | Coverage |
|---|---|---|
| Library unit tests | 56 | DSL evaluation, solver verdicts, envelope hazards, thrust limits, taint propagation, gate semantics, horizon expansion, hash-chain validation, snapshot env completeness |
| Integration tests | 7 | 20-cycle cruise approval, engine overheat block, terrain conflict block, future-horizon overspeed block, taint-channel block, hazard variable completeness, snapshot decidable on all standard cases |

### 7.2 Demo scenario

- **Phase 1 — Cruise** (3000 m, 150 m/s): 10/10 commands approved,
  ~23–142 µs verification time per cycle, recorder chain valid.
- **Phase 2 — Overspeed** (sea level, `vel_body = 182 m/s` → IAS ≈ 354 kt):
  `envelope_escape` returns `HAZARD_SAT`; the command is blocked and the
  aircraft freezes.
- **Phase 3 — Recorder**: 1,563 canonical spec lines exported; chain valid.

The demo also documents a subtle feature of the density-corrected model: the
*same* true airspeed (182 m/s) is inside the envelope at 3000 m (≈301 kt
IAS) but outside at sea level (≈354 kt IAS). This motivates careful IAS-vs-
TAS modeling in envelope protection.

### 7.3 Verification cost

The `StubSolver` gate evaluates a handful of formula trees per cycle on a
complete ground environment. The demo reports ~30–100 µs per complete cycle
on common developer hardware — well within the mock control-rate budget.
A real Z3-backed run would be heavier and is supplied behind the `z3`
feature for offline/one-shot checks.

---

## 8. Limitations and future work

1. **Mock dynamics.** The continuous model is point-mass-plus-attitude with
   constant coefficients. Reachability analysis (Flow\*, CORA, JuliaReach)
   on the *actual* differential equations is the natural next step.
2. **Single decision procedure.** The default ground evaluator is exact but
   not "SMT": it checks the current snapshot, not symbolically-unbounded
   models. The `Z3Backend` provides real NRA for richer offline proofs.
3. **Horizon fidelity.** The predicted path assumes the candidate command
   persists unchanged. Adaptive-horizon (receding-horizon) verification with
   controller-in-the-loop would improve liveness.
4. **No real-time scheduling.** ARINC 653-style partition isolation with a
   verified schedule and WCET analysis is future work (Phase 3 of the plan).
5. **No HIL.** The plan lists servo/actuator hardware-in-the-loop as a later
   phase.
6. **Hash function.** FNV-1a is not cryptographic; swap for SHA-256 in any
   real recorder artifact.

---

## 9. Conclusion

Plane OS demonstrates that a formally verifiable flight-envelope protection
architecture can be expressed as: a canonical environment, a set of NRA
hazard formulas, and a conservative gate that blocks anything it cannot
prove safe. The prototype keeps the verification machinery independent of
the (mock) flight dynamics, ships a tamper-evident recorder, and enforces
the taint audit as a lethal invariant. It is a stepping stone toward the
long-term goal in `development_plan.md`: an open-source toolchain for
research and education on formally verified envelope protection for
aviation.

---

## Acknowledgments

This document is a research draft. Special thanks to the formal-verification
and flight-control communities whose public tooling (Z3, SAT/SMT, reachability
libraries) makes such prototypes possible.

---

## References

1. RTCA DO-178C — *Software Considerations in Airborne Systems and Equipment
   Certification*.
2. ARP4754A — *Development of Civil Aircraft and Systems*.
3. ARP4761 — *Guidelines and Methods for Conducting the Safety Assessment
   Process on Civil Airborne Systems and Equipment*.
4. EASA CS-25 / FAA 14 CFR Part 25 — *Airworthiness Standards: Transport
   Category Airplanes*.
5. de Moura, L., Bjørner, N. — *Z3: An Efficient SMT Solver* (TACAS 2008).
6. Chen, X., Ábrahám, E., Sankaranarayanan, S. — *Flow\*: An Analyzer for
   Non-Linear Hybrid Systems* (CAV 2013).
7. Bogomolov, S. et al. — *JuliaReach: A Toolbox for Set-Based
   Reachability in Hybrid Dynamical Systems* (CAV 2019).
8. Althoff, M. — *CORA 2.0: A Toolbox for Reachability Analysis of Hybrid
   Dynamical Systems* (HSCC 2021).
9. ARINC 653 — *Avionics Application Software Standard Interface*.
10. *development_plan.md* — Plane OS solo-developer development plan
    (Phase 1–4).