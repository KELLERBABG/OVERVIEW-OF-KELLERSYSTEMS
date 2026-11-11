# ADR-004: SMT Solver Architecture

## Status
Accepted

## Context
The platform requires SMT solving for three capabilities:
1. Path constraint routing (capacity, fuel, border time feasibility)
2. Kinetic invariant enforcement (perishability, cold chain)
3. Supply chain space exploration (disruption scenario analysis)

Full Z3 integration requires cmake + Z3 system libraries to be installed,
which may not be available during development.

## Decision
Use a **two-tier solver architecture**:

### Tier 1: `ExactSolver` (default, no external deps)
- In-process, **sound and complete** decision procedure for the
  `ConstraintExpr` language; no Z3, no cmake
- Every leaf of `ConstraintExpr` is a unary atom `var <op> const`, and
  constraints never relate two variables, so each variable is an independent
  real. Satisfiability is decided by enumerating, per variable, the atom
  truth-vectors reachable on the regions cut by that variable's constants, and
  searching the product for one satisfying the boolean structure
- `minimize`/`maximize` return exact bounds (a real infimum/supremum), not a
  binary-search approximation
- Variables can be pinned to observed values with `.with_var(..)`, e.g. a
  sensor reading
- Implements the `Solver` trait

### Tier 2: Z3 backend (future, behind a `z3-backend` feature)
- Translates constraints to SMT-LIB 2.0 format via the `smt_lib` module
  (already implemented)
- Will invoke Z3 via the `z3` Rust crate bindings once an external Z3
  installation and cmake are available
- Full SMT solving, model enumeration and optimisation for constraint
  languages beyond the current unary-atom subset

> History: tier 1 was previously called `MockSolver` and assigned a *single
> shared value* to every variable while trying seven fixed starting points. It
> therefore reported feasible paths such as `capacity >= 100 AND temperature <= 8`
> as unsatisfiable — a false negative in every path verification. The name
> `MockSolver` survives only as a deprecated alias for `ExactSolver`.

### Core Capabilities Built on the Solver Trait
- `PathVerifier` — checks all edge constraints along a path
- `KineticInvariantEngine` — defines and verifies logistical invariants
- `SpaceExplorer` — tests disruption scenarios, finds alternative routes
- `smt_lib` — generates SMT-LIB 2.0 strings for any solver

## Consequences
+ Zero external dependencies for default builds
+ Clean `Solver` trait allows swapping implementations
+ SMT-LIB generation is ready for Z3 integration when cmake is available
+ Tier 1 decisions are exact, so path verification no longer has false negatives
- Tier 1 decides only the unary-atom language the types express today; richer
  constraints (relations between variables, non-linear terms) require tier 2
- The exact search is exponential in the number of *variables*, so a very large
  constraint set would exhaust `MAX_COMBINATIONS` and report "cannot decide"
  rather than guess — acceptable because real corridor constraint sets are small
- Z3 is not wired up; `z3`/`cmake` are not present in the default environment

## Related
sca-smt crate, Z3 documentation, SMT-LIB 2.0 standard