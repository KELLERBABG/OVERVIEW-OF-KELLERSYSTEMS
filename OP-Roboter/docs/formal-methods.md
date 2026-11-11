# OP-Roboter — Formal Methods Foundations

This document explains the formal-method foundations underpinning OP-Roboter:
satisfiability modulo theories (SMT), the nonlinear real arithmetic fragment,
lethal invariants, and how each maps to the codebase.

> ⚠️ **Research tooling only.** This project is research/prototype/open-source
> tooling for formal verification of surgical robot software — not a deployable
> medical device.

---

## 1. The Core Question

Before any actuator command is released, the control system must answer:

> *Can this motion path reach a state that violates a lethal invariant?*

This is a **reachability** question expressed as a **satisfiability** question:

```
check-sat: ¬safe(path)
```

- If `¬safe(path)` is **UNSAT** (unsatisfiable), the path can never reach a
  fault state → the command is **proved safe**.
- If it is **SAT** (satisfiable), a fault state exists → the command is
  **blocked**.
- If the solver returns **UNKNOWN**, the command is **blocked** (fail-safe).

---

## 2. Satisfiability Modulo Theories (SMT)

An **SMT solver** decides the satisfiability of first-order formulas over
background theories. OP-Roboter uses the fragment that matters for kinematics
and geometry:

### 2.1 Quantifier-free nonlinear real arithmetic (QF_NRA)

Kinematic and geometric constraints are polynomial equalities/inequalities
over real-valued variables:

| Constraint | Formula |
|------------|---------|
| Workspace box | `ws_min ≤ x ≤ ws_max` ∧ `ws_min ≤ y ≤ ws_max` ∧ `ws_min ≤ z ≤ ws_max` |
| Tissue clearance | `n · (p − p₀) ≥ margin` |
| RCM (trocar) | `‖(p_rcm − p_tip) × ẑ‖ ≤ tolerance` |
| Force limit | `‖f‖ ≤ f_max` |
| Depth limit | `depth ≤ depth_max` |

### 2.2 Z3

The `op-verifier/z3-solver` feature integrates the Z3 SMT solver via the
`z3` crate. The `ConstraintExpr` AST is translated into Z3 real-valued
variables and assertions, and the solver checks the negated safety formula.

### 2.3 The mock solver

The default build ships a **mock solver** that numerically evaluates the
constraint formula against concrete invariant bounds. It is deterministic,
dependency-free, and sufficient for the simulator and tests. It serves as the
trusted fallback when Z3 is unavailable.

---

## 3. Lethal Invariants

A **lethal invariant** is a property that must hold at every instant of the
surgical procedure. OP-Roboter currently models five kinds
(`op-verifier::InvariantKind`):

| Invariant | Meaning |
|-----------|---------|
| `WorkspaceEnvelope` | The tool tip never leaves the reachable axis-aligned box. |
| `TissueBoundary` | The tool tip stays on the protected side of every tissue plane by a safety margin. |
| `ForceLimit` | The reported/commanded tool-tip force never exceeds a bound. |
| `DepthLimit` | The incision depth never exceeds a bound. |
| `RcmStay` | The tool shaft always passes through the trocar point within tolerance. |

Verification enforces the **conjunction** of all invariants:

```text
safe(path) ⟺ ∧ᵢ invariantᵢ(path)
```

---

## 4. The Constraint Pipeline

```
MotionPath (waypoints)
   │
   ▼
PathConstraintGenerator        [op-constraints]
   │  samples the path densely
   │  emits ConstraintExpr over named scalar quantities
   ▼
ConstraintSet { formula, quantities }
   │
   ▼
Verifier                       [op-verifier]
   │  1. build numeric environment from invariant bounds
   │  2. evaluate formula with the solver (mock or Z3)
   │  3. negate: is a fault state reachable?
   ▼
VerificationResult { Proved | Disproved | Unknown }
   │
   ▼
ExecutionGate                  [op-gate]
   │  release ⇔ Proved
   ▼
Actuator command (or block)
```

---

## 5. Kinematic Foundations

### 5.1 Denavit–Hartenberg (DH) model

Each link is described by `(θ, d, a, α)`. The forward transform is:

```text
Tᵢ = RotZ(θᵢ) · TransZ(dᵢ) · TransX(aᵢ) · RotX(αᵢ)
```

The tool pose is the composition `T₀₁ · T₁₂ · … · Tₙ₋₁,ₙ`
(`op-kinematics::ForwardKinematics`).

### 5.2 Inverse kinematics

`op-kinematics::InverseKinematics` solves `IK(target, q₀, tol)` with damped
least squares on the geometric Jacobian:

```text
dq = Jᵀ (J Jᵀ + λ²I)⁻¹ e
```

### 5.3 Jacobian and singularities

The geometric Jacobian maps joint velocities to the spatial twist:

```text
[v; ω] = J(q) · q̇
```

`op-kinematics::Jacobian::analyze` flags configurations where the Gram matrix
`G = J Jᵀ` loses rank (smallest singular value → 0), which is the classic
definition of a kinematic singularity.

---

## 6. State Space Exploration

Beyond per-command verification, OP-Roboter explores *off-nominal* states to
discover rare edge cases *before* surgery exposes them
(`op-state-space`). Fault events:

- Patient motion
- Hemorrhage / pressure change
- Instrument breakage
- Power loss
- Sensor dropout

Each fault is applied at mild/moderate/severe magnitudes; states that violate
limits are surfaced as `EdgeCase`s for the specification and risk register.

---

## 7. Taint Analysis

`op-taint` tracks the provenance of values from **untrusted** sensors to
**critical** actuator commands. A command is only approved when every
untrusted source feeding it has passed through a registered sanitizer — the
verification gate clears the taint.

---

## 8. The Trustless Execution Gate

`op-gate`:

- **Holds no trust** in the command source.
- Re-verifies the path against lethal invariants.
- Releases a physical command **only** when the proof succeeds
  (`ProofOutcome::Proved`).
- Blocks on `Disproved` and `Unknown` (fail-safe).

---

## 9. Documented Limitations

- The mock solver performs **numerical evaluation**, not a general SMT proof.
  Real proof guarantees require the `z3-solver` feature.
- The current workspace/tissue models are axis-aligned boxes and planes;
  general convex polyhedra are future work.
- DH chains are serial; parallel/continuum kinematics are future work.
- Runtime verification of *continuous* dynamics (hybrid automata, reachable
  sets) is not yet implemented; the current per-command SMT check is discrete
  in the control cycle.