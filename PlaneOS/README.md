# Plane OS — Formally Verified Aerospace Flight Control Prototype

> **Research / open-source tooling only — NOT certified avionics.** This crate
> is not deployable and is not certified to DO-178C (DAL A), EASA CS-25,
> ARP4754A, or any other airworthiness standard. A solo developer cannot
> legally certify or deploy safety-critical aviation systems; this project
> exists for research, education, and open formal-methods tooling (see
> [development_plan.md](development_plan.md) and the
> [whitepaper](docs/whitepaper.md)).

Plane OS is a formally verified real-time flight-control **prototype** that
implements the core verification chain from the development plan:

1. **Mathematical trajectory mapping** — every flight-surface/thrust command
   is translated into formal expressions over the canonical control-cycle
   state.
2. **Kinetic path constraint generation** — hazards are expanded over a
   prediction horizon (`his.*_k` variables).
3. **Critical state-space exploration** — the SMT-style checker evaluates
   every hazard against the current and predicted states.
4. **Lethal invariant enforcement** — envelope (V-N, Mach, altitude, terrain)
   hazards must be proven UNSAT on every cycle.
5. **Symbolic thrust analysis** — N1/N2/EGT/oil/reverser limits excluded in
   advance.
6. **Taint-coded sensor stream audit** — pitot/IMU/GPS/ADS-B/radar data flow
   is tracked to actuation channels.
7. **Trustless execution gate** — a command reaches the dynamics integrator
   **only if every hazard formula is proven UNSAT**; otherwise the aircraft
   freezes (fail-safe).
8. **Automated specification generation** — every verdict is recorded in a
   hash-chained, machine-readable flight-recorder spec.

## Project layout

```
Cargo.toml                 Crate manifest (optional `z3` feature)
src/
  lib.rs                   Crate root + module map
  formula.rs               Symbolic NRA DSL (Expr / Formula / comparison ops)
  solver.rs                SMTBackend trait: StubSolver + optional Z3Backend
  dynamics.rs              6-DoF state, quaternions, ISA atmosphere, terrain,
                           mock integrator
  state.rs                 ControlSnapshot -> canonical env (state.*, cmd.*, mag.*)
  envelope.rs              V-N / Mach / altitude / terrain hazard formulas
  thrust.rs                Engine limit hazard formulas + reverser policy
  sensors.rs               Taint-coded sensor stream audit
  controller.rs            Prototype fly-by-wire control law
  gate.rs                  TRUSTLESS EXECUTION GATE (the safety core)
  invariants.rs            Lethal invariant set + run verification + margins
  constraints.rs           Kinetic path constraint generation (horizon expansion)
  spec.rs                  Hash-chained flight-recorder formal spec
  simulator.rs             Integrated 6-DoF mock simulator
  main.rs                  Demo binary (cruise -> overspeed block -> export)
tests/
  verification_chain.rs    End-to-end integration tests
docs/
  architecture.md          Deep dive on the verification chain
  whitepaper.md            Formal-methods whitepaper (research paper draft)
```

## Quick start

```sh
# Build (dependency-light; StubSolver only)
cargo build

# Run the full verification suite (63 tests)
cargo test

# Run the end-to-end demo
cargo run
```

Optional real SMT-NRA backend via Z3 (requires the Z3 native library):

```sh
cargo build --features z3
```

## What the demo shows

```
Phase 1: Normal cruise (10 cycles)
  every command approved, ~30-140 µs per verification cycle
Phase 2: Overspeed disturbance (sea level, 354 kt IAS)
  envelope_escape -> HAZARD_SAT -> command BLOCKED, aircraft frozen
Phase 3: Flight-recorder spec
  hash chain valid, 1563 canonical spec lines exported
```

## Verification backends

| Backend        | Requires     | Description |
|----------------|--------------|-------------|
| `StubSolver`   | none (default) | Sound ground evaluation of the concrete control-cycle state — dependency-free, always available |
| `Z3Backend`    | `--features z3` + Z3 native lib | Full SMT-NRA through Z3 with the ground state asserted, so nonlinear constraints are checked symbolically on the exact snapshot |

Both implement [`SMTBackend`](src/solver.rs) and are **conservative**: a real
satisfiability is never reported as UNSAT (`Unknown` is treated as unsafe).

## Standard safety cases (checked every cycle)

| Case                       | Hazard formula (SAT = danger) |
|----------------------------|-------------------------------|
| `envelope_escape`          | Any single envelope limit violated |
| `vn_maneuver_limit`        | `nz` exceeds speed-scaled structural limit |
| `thrust_hazard`            | N1/N2/EGT/oil exceeded or reverser overspeed |
| `taint_hazard`             | Command routed through a failed/tainted channel |
| `throttle_on_failed_engine`| Throttle applied to an unauthorized engine |

## Canonical variable namespaces

| Namespace | Prefix   | Meaning |
|-----------|----------|---------|
| `state.*` | `state.` | Concrete control-cycle state / derived quantities |
| `cmd.*`   | `cmd.`   | Pending (not yet committed) actuator command |
| `his.*_k` | `his.`   | Predicted horizon state at step `k` |
| `mag.*`   | `mag.`   | Magic constants (envelope limits, engine limits, terrain, aircraft) |
| `sym.*`   | `sym.`   | Free symbolic inputs (for offline analysis) |

## License

Apache-2.0 (per Phase 4 of the development plan). See
`development_plan.md`.