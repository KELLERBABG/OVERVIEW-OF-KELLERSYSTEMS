# op-verifier

SMT verification engine for the OP-Roboter formally verified surgical robot
control stack.

- **Default build** uses a built-in mock solver (no Z3 required) that evaluates
  the constraint formula numerically and reports a verification verdict.
- **`z3-solver` feature** integrates the real Z3 SMT solver for real
  satisfiability checks of the lethal-invariant constraints.

> Research tooling only — not a deployable medical device.

## Build

```bash
# Mock solver (default)
cargo build -p op-verifier

# Real Z3 integration (requires Z3 library available to the linker)
cargo build -p op-verifier --features z3-solver
```
