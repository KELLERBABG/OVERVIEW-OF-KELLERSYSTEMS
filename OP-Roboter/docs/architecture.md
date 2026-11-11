# OP-Roboter — System Architecture

This document describes the logical architecture of the **OP-Roboter** formally
verified surgical robot control stack: crate responsibilities, the strict
bottom-up dependency rule, and the runtime data flow from sensor input to
proof-gated actuation.

> ⚠️ **Research tooling only.** This project is a research/prototype/
> open-source toolchain for formal verification of surgical robot software. It
> is not a deployable medical device and cannot be certified by a solo
> developer. See `docs/standards-and-safety.md`.

---

## 1. Design Principles

1. **Layers depend only on layers below them.** The crate graph is a strict
   DAG. A crate may only depend on crates listed earlier in the workspace
   layout. This makes the formal-method guarantees compositional: if every
   layer's safety property holds and the dependencies hold, the composition
   holds.
2. **Every actuator command is mathematically proven safe before release.**
   The pipeline is *model → constrain → verify → execute*. No command reaches
   the actuator without a proof.
3. **Plain data, no hidden state.** All mathematical values flow as plain
   `f64`-backed, `serde`-serializable structs so they can be embedded into SMT
   constraints, serialized into verification logs, and archived as formal
   specifications without lossy conversions.
4. **Fail-safe by construction.** The execution gate defaults to *block* — a
   command is released only when the proof explicitly succeeds.

---

## 2. Workspace Layout and Dependency Order

```
┌─────────────────────────────────────────────┐
│ op-sim        (3D simulator + CLI)          │  ↑ top-level application
├─────────────────────────────────────────────┤
│ op-gate       (trustless execution gate)    │
├─────────────────────────────────────────────┤
│ op-spec       (medical spec generation)     │
│ op-taint      (sensor stream audit)         │
│ op-energy     (force & energy analysis)     │
│ op-state-space(critical state exploration)  │
│ op-verifier   (SMT verification engine)     │  ← depends on op-constraints
├─────────────────────────────────────────────┤
│ op-constraints(kinetic path constraints)    │  ← depends on op-trajectory
├─────────────────────────────────────────────┤
│ op-trajectory (3D workspace, paths, RCM)    │  ← depends on op-kinematics
├─────────────────────────────────────────────┤
│ op-kinematics (DH, FK/IK, Jacobian)         │  ← depends on op-math
├─────────────────────────────────────────────┤
│ op-math       (vectors, matrices, quats)    │  ← foundation
└─────────────────────────────────────────────┘
```

Dependencies always point **downward** in this diagram.

### Crate responsibilities

| Crate | Responsibility | Key exports |
|-------|----------------|-------------|
| `op-math` | 3D vector/matrix/quaternion/isometry math | `Vector3`, `Matrix3`, `Quaternion`, `Isometry3`, `Scalar`, `EPSILON` |
| `op-kinematics` | Surgical-arm kinematics | `Chain`, `DhLink`, `ForwardKinematics`, `InverseKinematics`, `Jacobian`, `SingularityReport` |
| `op-trajectory` | Workspace + motion paths + RCM | `Workspace`, `Volume`, `TissueBoundary`, `MotionPath`, `RcmConstraint` |
| `op-constraints` | Boolean action-path constraints | `ConstraintExpr`, `ConstraintSet`, `PathConstraintGenerator` |
| `op-verifier` | Z3/mock SMT verification + invariants | `Verifier`, `LethalInvariantSet`, `ProofOutcome`, `VerificationResult` |
| `op-state-space` | Critical edge-case exploration | `StateExplorer`, `SystemState`, `FaultEvent`, `EdgeCase` |
| `op-energy` | Symbolic force & energy | `EnergyAnalyzer`, `SymbolicExpr`, `ToolWrench`, `EnergyReport` |
| `op-taint` | Taint-coded sensor audit | `TaintTracker`, `ActuatorCommand`, `TrustLevel`, `SanitizeResult` |
| `op-spec` | Formal spec generation | `SpecGenerator`, `FormalRequirement`, `Specification` |
| `op-gate` | Proof-gated actuation | `ExecutionGate`, `ActuationCommand`, `GateDecision` |
| `op-sim` | 3D simulator + CLI | `Simulator`, `rcm_arm()`, `render_summary()` |

---

## 3. Runtime Data Flow

```
                ┌──────────────────────────────┐
 Sensors (untrusted):                          │
  • force/torque   │ op-taint  (source trust)  │
  • joint encoders │                           │
  • endoscope      ▼                           │
  ───────────────────────────┐                 │
                             │                 │
 Control loop                ▼                 │
 ┌─────────────────────────────────────────┐   │
 │ 1. op-trajectory: plan motion path      │   │
 │    (tool tip, incision depth, force)    │   │
 │ 2. op-constraints: path → boolean       │   │
 │    formula (kinetic action path)        │   │
 │ 3. op-verifier: check ¬formula against  │   │
 │    lethal invariants (SMT / mock)       │   │
 │ 4. op-gate: release only if Proved      │   │
 └─────────────────────────────────────────┘   │
     │            │                            │
     ▼            ▼                            ▼
 ┌─────────────┐ ┌────────────────────────────────────┐
 │ Physical    │ │ op-spec: archive proved invariants  │
 │ actuator    │ │ + behaviour as formal spec (JSON)   │
 └─────────────┘ └────────────────────────────────────┘
```

1. **Model** — `op-kinematics` models the arm as a DH serial chain; `op-trajectory`
   models the workspace (reachable volume), tissue boundaries, and the RCM
   (trocar) constraint.
2. **Constrain** — `op-trajectory` produces a candidate `MotionPath`;
   `op-constraints` converts the path into a boolean `ConstraintExpr`
   (workspace box, tissue clearances, RCM off-axis distance, force and depth
   limits).
3. **Verify** — `op-verifier` evaluates whether the *negation* of the safety
   formula is satisfiable using either the feature-gated Z3 solver
   (`z3-solver`) or the built-in mock solver (default).
4. **Execute** — `op-gate` blocks the physical `ActuationCommand` unless the
   verification outcome is `Proved`.

---

## 4. The Verification Object

The verification engine reasons about a **boolean constraint formula**:

```text
safe(path) =
    (∀ sample s ∈ path:
        ws_min ≤ s.position ≤ ws_max           — reachable envelope
      ∧ ∀ boundary b: clearance_b(s) ≥ margin  — tissue boundary
      ∧ off_axis_rcm(s) ≤ rcm_tol              — trocar (RCM) constraint
      ∧ force(s) ≤ force_limit                 — force limit
      ∧ depth(s) ≤ depth_limit)                — depth limit
```

A *fault state* is reachable iff `¬safe(path)` is satisfiable. `op-verifier`
therefore asks the solver:

```text
check-sat: ¬safe(path)
```

- **UNSAT** → the path cannot reach a fault state → **Proved** → gate releases.
- **SAT** → a fault state exists → **Disproved** → gate blocks.
- **Unknown** → fail-safe: gate blocks.

---

## 5. Correctness Guarantees (per layer)

| Crate | Property |
|-------|----------|
| `op-math` | Rotation matrices are orthonormal (`is_rotation`); transforms invert exactly; quaternions normalize. |
| `op-kinematics` | FK accumulates DH transforms deterministically; IK converges to tolerance with damped least squares; `analyze` flags rank-deficient Jacobians. |
| `op-trajectory` | Path samples are dense interpolations; `is_safe_along` checks every sample against the workspace; RCM `check` enforces the trocar invariant. |
| `op-constraints` | Every clause is a comparison of named scalar quantities; the generated formula covers workspace, tissue, RCM, force, depth. |
| `op-verifier` | A `Proved` outcome implies all invariants hold in the evaluated environment. |
| `op-taint` | Untrusted sensor values cannot reach an actuator command unless cleared by a registered sanitizer (verification gate). |
| `op-gate` | Only `Proved` commands are released; `Unknown` and `Disproved` are blocked (fail-safe). |

---

## 6. Testing

The workspace ships 47 unit tests covering every crate. Run:

```bash
cargo test --workspace
```

Key scenarios covered:

- `op-math`: matrix multiplication correctness, quaternion rotation,
  transform round-trips.
- `op-kinematics`: Jacobian velocity mapping, full-extension singularity,
  non-singular generic pose (with a direct Gram-determinant ground truth).
- `op-constraints`: formula generation for a safe path, zero-length rejection.
- `op-verifier`: proved vs. disproved outcomes.
- `op-gate`: command release and empty-command rejection.
- `op-taint`: untrusted rejection, sanitizer approval, unknown source error.
- `op-sim`: default scenario validity, RCM satisfaction, planning, gate release.

---

## 7. Feature Flags

| Flag | Effect |
|------|--------|
| *(default)* | Built-in mock solver. No external dependency. |
| `op-verifier/z3-solver` | Enables real Z3 SMT solver integration. Requires the Z3 library available to the linker. |

```bash
cargo build --workspace
cargo build --workspace --features op-verifier/z3-solver