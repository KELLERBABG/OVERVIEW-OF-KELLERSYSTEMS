# Plane OS — Architecture Reference

This document describes the internal architecture of the Plane OS prototype:
the control-cycle pipeline, the canonical variable environment, the trustless
execution gate semantics, soundness/conservativeness arguments, and the
demo scenario. It is written to be readable by researchers and engineers new
to formal verification of flight-control software.

> **Disclaimer:** reference/educational material for a **research prototype**,
> not a deployment or certification artifact (DO-178C / ARP4754A / CS-25).

---

## 1. Control-cycle pipeline

A single simulation cycle (`PlaneSimulator::step`) performs five stages:

```
┌──────────────┐     ┌──────────────────┐     ┌──────────────────────┐
│ 1. Controller│ ──▶ │ 2. Snapshot env  │ ──▶ │ 3. Execution gate    │
│  FlyByWire   │     │  ControlSnapshot │     │  hazards → SAT/UNSAT │
└──────────────┘     └──────────────────┘     └──────────┬───────────┘
                                                         │
                                  ┌──────────────────────┴──────────┐
                                  │ approved?                        │
                                  ▼                                  ▼
                     ┌────────────────────────┐        ┌────────────────────────┐
                     │ 4a. Apply command to   │        │ 4b. Block: aircraft    │
                     │     FlightDynamics     │        │     freezes (fail-safe) │
                     └────────────────────────┘        └────────────────────────┘
                                  │
                                  ▼
                     ┌────────────────────────┐
                     │ 5. Append verdict to   │
                     │     SpecChain (hash)   │
                     └────────────────────────┘
```

### Stage 1 — Candidate command

`FlyByWire::compute(derived, quat, input)` produces a normalized
`ActuatorCommand { throttle, elevator, aileron, rudder }` in `[-1, 1]` per
channel from pilot input and simple feedback laws (attitude hold, roll
tracking, yaw damping, auto-throttle, and a load-factor limiter).

The command is **candidate only** — it is not applied until the gate proves
safety.

### Stage 2 — Canonical snapshot environment

`ControlSnapshot` freezes the flight state, derived quantities, command,
configuration constants, engine parameters, and channel statuses. Its
`env()` method materializes a `BTreeMap<String, f64>` — the **canonical
ground environment** — that every hazard formula is evaluated against.

The environment has four namespaces (see §3) plus `mag.*` magic constants:

- `mag.envelope.*` — V_NE, V_MO, V_S, V_S_flaps, M_MO, n_z,max, n_z,min,
  alpha_max, max altitude, min terrain clearance, surface-rate limits.
- `mag.engine.*` — N1_max, N2_max, N1_reverse_max, EGT_max, oil pressure min,
  throttle rate max.
- `mag.terrain.*` — base elevation and plane slopes.
- `mag.aircraft.*` — wing area, mass, max thrust, stall alpha.

### Stage 3 — Trustless execution gate

`ExecutionGate::evaluate(env, cycle)` runs every `SafetyCase`:

```
case verdict = StubSolver::check(case.formula, env):
  Unsat   → ProvenSafe
  Sat     → HazardSat   (danger)
  Unknown → Unproven    (danger, conservative)
```

If **any** case is not `ProvenSafe`, the gate blocks the command
(`approved = false`) and records the first violating case name.

### Stage 4 — Commit / freeze

- **Approved**: the candidate is clamped and fed to `FlightDynamics::step`,
  which integrates the 6-DoF mock state for one control cycle.
- **Blocked**: `last_command = None`; the aircraft state is frozen. Fail-safe
  by default.

### Stage 5 — Flight recorder

`SpecChain::push(report, t, env)` appends a `SpecRecord` containing the gate
verdict, canonical formulas for every case, and the environment, chained via
an FNV-1a hash of the previous record. `validate()` replays the chain and
detects any tampering.

---

## 2. The formula DSL (`formula.rs`)

A **hazard** is a `Formula` (boolean tree) over `Expr` (real-arithmetic
tree):

```
Formula ::= True | False | Atom(Comparison) | Not | And | Or
Expr    ::= Const | Var | Neg | Add | Sub | Mul | Div | Pow | Abs | Min | Max
Comparison ::= lhs <rhs  |  lhs <= rhs  |  lhs > rhs  |  lhs >= rhs  |
               lhs = rhs |  lhs != rhs
```

All formulas are constructed over **canonical namespaced variables**, so a
formula is *auditable*: a reader can see exactly which state/command/magic
quantities feed a hazard without tracing code.

Key operations:

- `Expr::free_vars` / `Formula::free_vars` — collect referenced variables.
- `Expr::substitute` / `Formula::substitute` — replace variables with other
  expressions (used for horizon expansion).
- `Expr::eval` / `Formula::eval` — ground evaluation against an environment,
  returning `Option<bool>`; `None` indicates an unknown variable or a domain
  error (e.g. division by zero, `pow` of a negative base with a non-integer
  exponent). `None` is conservative: it becomes `Unknown` in the solver.

The DSL deliberately exposes `lt/le/gt/ge/eq` but **not** `ne`, because
`Expr` implements `PartialEq` whose `ne(&self, &Expr) -> bool` would shadow a
consuming `Formula`-returning `ne`. Non-zero comparisons are expressed as
`abs(x) > 0`.

---

## 3. Canonical variable environment (`state.rs`)

| Namespace | Example         | Meaning |
|-----------|-----------------|---------|
| `state.*` | `state.ias_knots` | Flight state + derived quantities (IAS, alpha, n_z, Mach, altitude, terrain clearance, climb rate, quaternion, omega, engine N1/N2/EGT/oil, channel-OK flags, reverser/gear/flap) |
| `cmd.*`   | `cmd.elevator`   | Pending actuator command (candidate, not yet applied) |
| `his.*_k` | `his.ias_knots_3`| Predicted state at horizon step `k` (kinetic path constraint) |
| `mag.*`   | `mag.envelope.vne_knots` | Magic constants that parameterize limits (envelope/engine/terrain/aircraft) |
| `sym.*`   | `sym.nz`         | Free symbolic inputs reserved for offline analysis |

Channel-OK flags are embedded as 0/1 reals (`state.aileron_ok`, …) so the
taint audit result is itself a first-class NRA quantity.

---

## 4. Standard safety cases (`gate.rs::StandardCases`)

Checked every cycle, in declaration order:

1. **`envelope_escape`** — disjunction of all individual envelope hazards:

   ```
   ias_knots > vne_knots
      ∨ ias_knots < vs_clean_knots
      ∨ alpha_deg > alpha_max_deg
      ∨ nz > nz_max
      ∨ nz < nz_min
      ∨ mach > mmo
      ∨ altitude > max_altitude
      ∨ terrain_clearance < min_terrain_clearance
   ```

2. **`vn_maneuver_limit`** — V-N corner approximation:

   ```
   nz > nz_max · min(1, (ias_knots / (vne_knots · 0.7))²)
   ```

3. **`thrust_hazard`** — N1/N2/EGT exceedance, oil pressure below minimum,
   reverser overspeed.

4. **`taint_hazard`** — a non-zero command on a failed channel:
   `channel_ok < 0.5 ∧ abs(cmd) > 0`. This makes the sensor audit a *lethal*
   property rather than a warning.

5. **`throttle_on_failed_engine`** — throttle applied when
   `state.throttle_ok < 0.5`.

---

## 5. Kinetic path constraints (`constraints.rs`)

`PathConstraints::expand_over_horizon(hazard, horizon=N)` builds

```
hazard(state_0) ∨ hazard(his_1) ∨ … ∨ hazard(his_N)
```

where each `hazard(his_k)` is the formula with all `state.*` variables
substituted by `his.*_k`. The simulator seeds the horizon environment by
integrating the candidate command forward `N` cycles with the mock dynamics.
If any predicted step violates the envelope, the gate blocks the command
*before* it is applied — critical state-space exploration in advance.

---

## 6. Taint-coded sensor audit (`sensors.rs`)

`SensorTaintGraph` tracks which flight-path quantities depend on which
sensors:

| Quantity          | Contributing sensors |
|-------------------|----------------------|
| Airspeed          | Pitot L/R/C, static port |
| Altitude          | Static port, GPS      |
| Attitude          | IRU                   |
| Position          | GPS, ADS-B in         |
| Angle of attack   | AoA vane, IRU         |
| Vertical speed    | Static port, IRU      |
| Heading           | IRU, GPS              |
| Weather           | Weather radar         |

Per-quantity taint = worst taint among available contributors; if no
contributor is available, the quantity is considered **untrusted** (biased
safe). The audit maps quantities to actuator channels and, in the simulator,
writes the resulting channel status into the snapshot; the gate's
`taint_hazard` then blocks any command on a failed channel.

---

## 7. Soundness arguments

1. **Lethal-invariant enforcement is conservative.** A hazard formula that is
   SAT (or cannot be proven UNSAT) blocks the command. `Unknown` is never
   treated as safe.

2. **Ground evaluation is exact on the snapshot.** The `StubSolver`
   evaluates the formula tree directly on the concrete control-cycle state —
   there is no abstraction loss. Missing variables or domain errors yield
   `Unknown`, which blocks.

3. **Hash chaining detects tampering.** Each `SpecRecord` commits to the
   previous record's hash; `validate()` replays the chain. Note: the
   prototype uses FNV-1a 64-bit for determinism without dependencies — a
   real DO-178C artifact would use a cryptographic hash (SHA-256).

4. **The mock dynamics are clearly labeled.** The aerodynamics use constant
   coefficient approximations. The *verification machinery* is what the
   prototype demonstrates, not certified flight-dynamics fidelity.

---

## 8. Demo walkthrough (`main.rs`)

1. **Cruise** (3000 m, 150 m/s): 10 cycles, all commands approved,
   ~23–142 µs per verification cycle, spec chain valid.
2. **Overspeed disturbance**: dive to sea level and set
   `vel_body = 182 m/s` → IAS ≈ 354 kt > V_NE = 340 kt. The gate reports
   `envelope_escape → HAZARD_SAT` and the aircraft freezes at 354 kt.
3. **Recorder export**: the hash-chained spec validates and exports 1,563
   canonical lines for certification-style review.

> **Physics note:** IAS is density-compensated. 182 m/s at 3000 m is only
> ~301 kt IAS (inside the envelope), while the same TAS at sea level is
> ~354 kt (outside). The demo dives to sea level to make the hazard real —
> a nice property of the density-corrected model.

---

## 9. Module dependency map (crate-level)

```
formula ─┬─ solver ──────────────┐
         ├─ envelope ────────────┤
         ├─ thrust               ├──▶ gate ◀── StandardCases
         ├─ sensors ─────────────┘
dynamics ─┬─ state ── build_snapshot/env
          └─ controller ─────────┐
constraints ─────────────────────┤
spec ────────────────────────────┴──▶ simulator ◀── main
```

- `gate` depends on `envelope`, `thrust`, `sensors`, `solver`.
- `simulator` depends on `gate`, `controller`, `dynamics`, `state`,
  `constraints`, `spec`, `sensors`.
- `state` depends on `dynamics`, `envelope`, `thrust`.
- Nothing depends on `simulator` except the demo binary, keeping the
  verification core reusable.

---

## 10. Future work (see development_plan.md)

- Real reachability analysis (Flow*, JuliaReach, CORA) for hybrid-system
  verification of the continuous dynamics.
- Real Z3 NRA backend exercised in CI and a soundness audit of the
  `Expr::Pow` encoding.
- True ARINC 653 partition scheduling with worst-case-execution-time
  analysis for the verification deadlines.
- Hardware-in-the-loop with servo/actuator mocks.
- DO-178C artifact roadmap (Phase 4.5 of the development plan).