# OP-Roboter — Getting Started

A hands-on guide to building, testing, and running the OP-Roboter formally
verified surgical robot control stack.

> ⚠️ **Research tooling only.** This repository is for research, prototype, and
> open-source tooling for formal verification of surgical robot software — not
> a deployable medical device.

---

## 1. Prerequisites

- **Rust toolchain** ≥ 1.70 (edition 2021). Install via [rustup](https://rustup.rs).
- **Cargo** — comes with rustup.
- *(Optional)* **Z3** library for the `z3-solver` feature.

Check your toolchain:

```bash
rustc --version
cargo --version
```

---

## 2. Build

### Default build (mock solver — no Z3 required)

```bash
cargo build --workspace
```

### With real Z3 integration

```bash
cargo build --workspace --features op-verifier/z3-solver
```

Requires the Z3 library to be available to the linker (e.g. `z3` package /
Z3 shared library on `PATH`).

### Release build

```bash
cargo build --workspace --release
```

---

## 3. Run the tests

The workspace ships 47 unit tests across all 11 crates:

```bash
cargo test --workspace
```

See `docs/architecture.md` §6 for a map of what each crate's tests cover.

---

## 4. Run the 3D simulator CLI

The simulator (`op-sim`) wires the whole stack together: an RCM-style
surgical arm, a workspace with a tissue boundary, the trocar constraint, the
verifier, and the trustless execution gate.

### Status overview

```bash
cargo run -p op-sim -- status
```

Example output:

```
Tool tip: (0.250, 0.100, 0.300) m
Singularity: ok
RCM constraint: satisfied
Gate decision: release=true (all lethal invariants hold — command may release)
```

### Plan a motion

```bash
cargo run -p op-sim -- plan 0.25 0.16 0.30
```

This runs the full pipeline: inverse kinematics → motion path → boolean
constraint generation → verification → gate decision.

### Help

```bash
cargo run -p op-sim -- --help
```

---

## 5. Feature demonstrations per crate

Each crate is independently testable:

```bash
# Foundation math
cargo test -p op-math

# Kinematics (DH chain, FK/IK, Jacobian, singularities)
cargo test -p op-kinematics

# Workspace / paths / RCM
cargo test -p op-trajectory

# Constraint generation
cargo test -p op-constraints

# Verification engine (mock solver)
cargo test -p op-verifier

# Critical state-space exploration
cargo test -p op-state-space

# Symbolic force & energy analysis
cargo test -p op-energy

# Taint-coded sensor audit
cargo test -p op-taint

# Automated medical spec generation
cargo test -p op-spec

# Trustless execution gate
cargo test -p op-gate

# Simulator
cargo test -p op-sim
```

---

## 6. Quick API tour

### Model the arm (op-kinematics)

```rust
use op_kinematics::{Chain, DhLink, ForwardKinematics};

let arm = Chain {
    links: vec![
        DhLink::revolute(0.0, 0.30, 0.0, -std::f64::consts::FRAC_PI_2),
        DhLink::revolute(0.0, 0.0, 0.25, 0.0),
        DhLink::prismatic(0.0, 0.10, 0.0, 0.0),
        DhLink::revolute(0.0, 0.0, 0.0, 0.0),
    ],
};
let q = vec![0.0, 0.0, 0.0, 0.0];
let pose = arm.forward(&q)?;   // ToolPose { position, orientation }
```

### Constrain a path (op-trajectory + op-constraints)

```rust
use op_constraints::PathConstraintGenerator;
use op_trajectory::{MotionPath, Waypoint, Workspace, Volume, TissueBoundary, RcmConstraint};
use op_math::{Isometry3, Vector3};

// ... build workspace, rcm, start/end waypoints ...
let path = MotionPath::new(start_wp, end_wp);
let gen = PathConstraintGenerator::new(workspace, Some(rcm), 5.0, 0.1);
let set = gen.generate(&path, 8)?;   // ConstraintSet { formula, quantities }
```

### Verify (op-verifier)

```rust
use op_verifier::{Verifier, LethalInvariantSet, LethalInvariant, InvariantKind};

let invariants = LethalInvariantSet::new(vec![
    LethalInvariant {
        name: "depth_limit".into(),
        kind: InvariantKind::DepthLimit { max_depth: 0.1 },
    },
]);
let verifier = Verifier::new(invariants);
let result = verifier.verify(&set);
// result.outcome == ProofOutcome::Proved | Disproved | Unknown
```

### Gate the actuation (op-gate)

```rust
use op_gate::{ExecutionGate, ActuationCommand};

let mut gate = ExecutionGate::new(verifier);
let decision = gate.evaluate(&ActuationCommand {
    command: vec![0.0, 0.0, 0.0, 0.0],
    description: "insert tool".into(),
})?;
// decision.release == true only when verified Proved
```

---

## 7. Project layout

```
.
├── Cargo.toml                 # workspace manifest (11 members)
├── crates/
│   ├── op-math                # foundation math
│   ├── op-kinematics          # DH, FK/IK, Jacobian, singularities
│   ├── op-trajectory          # workspace, paths, RCM
│   ├── op-constraints         # boolean action-path constraints
│   ├── op-verifier            # SMT verification (mock / feature-gated Z3)
│   ├── op-state-space         # critical state exploration
│   ├── op-energy              # symbolic force & energy
│   ├── op-taint               # taint-coded sensor audit
│   ├── op-spec                # formal medical spec generation
│   ├── op-gate                # trustless execution gate
│   └── op-sim                 # 3D simulator + CLI
└── docs/                      # this documentation set + whitepaper
```

---

## 8. Next steps

- Read [`docs/architecture.md`](architecture.md) for the full system design.
- Read [`docs/formal-methods.md`](formal-methods.md) for the underlying
  formal-method foundations (SMT, NRA, invariants).
- Read [`docs/standards-and-safety.md`](standards-and-safety.md) for the
  medical-device standards context (IEC 80601-2-77, IEC 62304, ISO 14971).
- Read [`docs/whitepaper.md`](whitepaper.md) for the technical whitepaper.