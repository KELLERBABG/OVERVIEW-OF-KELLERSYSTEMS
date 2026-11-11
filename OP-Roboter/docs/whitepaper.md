# OP-Roboter

## Proof-Gated Actuation for Surgical Robot Safety via Satisfiability Checking

**Technical Whitepaper — v0.1 (Research Scaffold)**

> ⚠️ **Research tooling only.** This describes an open-source research
> toolchain. It is not a certified medical device and must not control a
> physical surgical system outside of research simulation.

---

## Abstract

Surgical robots couple an actuator directly to a patient, which places an
extraordinary burden on the software that commands them: a single bug can
translate a logical error into physical injury. Standard defensive programming
and testing reduce risk but cannot *exclude* hazards. This paper describes
**OP-Roboter**, a formally guided control architecture whose central claim is
that **no actuator command is released without a mathematical proof that its
path cannot violate a lethal invariant**. The design translates every planned
motion into a boolean constraint formula, asks an SMT solver (Z3) whether the
*negated* safety formula — the fault state — is satisfiable, and blocks the
command unless the proof succeeds. We present the crate architecture, the
constraint pipeline, the lethal-invariant model, the trustless execution gate,
and the 47-test verification scaffold. We then discuss the formal-method
foundations (QF_NRA over the geometric Jacobian), the safety concept, and the
roadmap toward regulatory-grade formal verification.

---

## 1. Introduction

### 1.1 The problem

Robotically assisted surgery (IEC 80601-2-77 domain) creates a closed loop:
sensors → software → actuator → patient. Safety-critical failures surface as
uncontrolled motion, tissue damage, or collisions. Traditional assurance
performs extensive testing, code review, and hazard analysis (ISO 14971,
IEC 62304), but testing samples the behavior space; it does not prove its
absence of hazards.

### 1.2 The opportunity

Modern SMT solvers can decide the satisfiability of constraint formulas over
real arithmetic quickly enough to be used *online*. This raises the possibility
of **per-command mathematical verification**: instead of asking "did we test
this path?", ask "can this path be proven to avoid every lethal invariant?".

### 1.3 Thesis

> Every actuator command (incision depth, retraction force, tool orientation)
> is **mathematically proven safe before it is released**.

OP-Roboter operationalizes this with four stages — **Model, Constrain,
Verify, Execute** — connected by a trustless execution gate.

---

## 2. System Model

### 2.1 Kinematics (op-math, op-kinematics)

The arm is a serial chain of Denavit–Hartenberg links. Forward kinematics
compose per-link isometries into a tool pose `T(q)`. Inverse kinematics solve
`IK(target, q₀, tol)` by damped least squares over the geometric Jacobian
`J(q): q̇ ↦ [v; ω]`.

### 2.2 Workspace (op-trajectory)

The surgical environment is modeled as:

- A reachable axis-aligned volume `W`
- Protected tissue planes `{b: n·p ≥ d}`
- A remote-center-of-motion (trocar) point `x_rcm` the tool shaft must pass through
- A tool shaft length and RCM tolerance

### 2.3 Quantities

All constraints compare **named scalar quantities** (positions, clearances,
forces, depths, RCM deviations). This makes every clause trivially liftable to
an SMT variable.

---

## 3. Constrain: Kinetic Path Constraints (op-constraints)

A candidate `MotionPath` is sampled densely. Each sample `s` emits a clause
conjunction:

```text
C(s) =  inside_workspace(s)
      ∧ clearance(s) ≥ margin
      ∧ off_axis_rcm(s) ≤ tol
      ∧ force(s) ≤ f_max
      ∧ depth(s) ≤ d_max
```

The path is safe iff all samples satisfy:

```text
safe(path) = ∧_s C(s)
```

This formula is the **kinetic action path** — a boolean description of the
whole motion.

---

## 4. Verify: SMT and the Negative (op-verifier)

The verifier asks the solver the *dual* question:

```text
check-sat: ¬safe(path)
```

- **UNSAT** → no fault state is reachable → **Proved**.
- **SAT** → a fault state is reachable → **Disproved**.
- **UNKNOWN** → fail-safe: block.

Two backends:

- **Mock solver (default):** deterministic numeric evaluation against
  invariant bounds. Dependency-free; suitable for the simulator and CI.
- **Z3 (`z3-solver` feature):** real SMT solving over nonlinear real
  arithmetic. The `ConstraintExpr` AST is translated into Z3 reals and
  assertions, enabling genuine proof obligations.

**Lethal invariants** (`LethalInvariantSet`) currently include:
`WorkspaceEnvelope`, `TissueBoundary`, `ForceLimit`, `DepthLimit`, `RcmStay`.
Verification enforces their conjunction.

---

## 5. Execute: Trustless Gate (op-gate)

The gate holds no trust in the command source. It re-verifies the path
against the invariant conjunction and releases the physical command **only**
on `Proved`:

```text
release(command) ⇔ verify(path) = Proved
```

`Disproved` and `Unknown` block. This is **fail-safe by construction**: the
default action is to not move.

---

## 6. Supporting Assurance Layers

### 6.1 Critical state-space exploration (op-state-space)

Fault events (patient motion, hemorrhage, instrument breakage, power loss,
sensor dropout) are perturbed at multiple magnitudes and classified against
safety limits, producing an `EdgeCase` set for the specification and risk
register.

### 6.2 Symbolic energy & force analysis (op-energy)

Per-cycle symbolic torque/energy expressions (`τ = z·( (p_tip−p)×f + n )`)
enable tissue-overload and hardware-failure checks before actuation.

### 6.3 Taint-coded sensor audit (op-taint)

Untrusted sensor provenance is tracked to actuator commands. A command is
approved only when every untrusted source has been cleared by a registered
sanitizer — the verification gate itself.

### 6.4 Automated specification generation (op-spec)

Proved invariants and behavior are archived as machine-readable formal
requirements (JSON), intended as a certification reference under IEC 62304 /
ISO 14971.

---

## 7. Formal Foundation

### 7.1 QF_NRA

The safety formulas are quantifier-free formulas over nonlinear real
arithmetic:

- Workspace box: linear inequalities in `x,y,z`
- Tissue clearance: `n·p ≥ d` (linear)
- RCM: `‖(x_rcm − p)×ẑ‖ ≤ tol` (quadratic)
- Force/depth: linear bounds

These are decidable fragments; Z3 solves them with nlsat.

### 7.2 Why negation

Checking `safe` directly is vacuous unless we search for a counterexample.
`¬safe` *is* the counterexample search: a satisfying assignment is a concrete
fault trajectory. This is the standard SMT reachability framing used in
software model checking (e.g., Bounded Model Checking).

### 7.3 Compositionality

The strict bottom-up crate DAG means each layer's guarantees compose. If
`op-math` rotations are orthonormal, `op-kinematics` FK is exact, and
`op-verifier` proves the invariant conjunction, then the gate's release
decision is sound *modulo* the modeling assumptions (discrete sampling, exact
DH parameters, sensor truth).

---

## 8. Safety Concept

The fail-safe gate is the keystone:

| Situation | Gate behavior |
|-----------|---------------|
| Proof succeeds (`Proved`) | Release |
| Fault state found (`Disproved`) | Block |
| Solver timeout/unknown | Block |
| No proof requested | Block |
| Untrusted sensor taint uncleared | Block (op-taint) |

Residual risk is confined to modeling inaccuracies and solver unsoundness,
both external to the per-command control decision.

---

## 9. Evaluation (Scaffold)

The scaffold builds and passes 47 tests across 11 crates:

```bash
cargo build --workspace            # mock solver, no Z3
cargo test --workspace             # 47 tests, 0 failures
cargo run -p op-sim -- status
```

Example simulator report:

```
Tool tip: (0.250, 0.100, 0.300) m
Singularity: ok
RCM constraint: satisfied
Gate decision: release=true (all lethal invariants hold — command may release)
```

Notable engineering results in the scaffold:

- Correct row-major `Matrix3` multiplication enabling orthonormal rotation
  identity checks.
- Robust Jacobian singular-value estimation (power iteration on the Gram
  matrix) with a direct determinant ground-truth test.
- Self-consistent RCM scenario geometry (trocar placed on the computed tool
  axis).

---

## 10. Limitations

1. **Mock solver:** numeric evaluation, not a general proof. Real guarantees
   require `z3-solver`.
2. **Sampled paths:** constraints are checked on a discrete sample grid; the
   guarantee is as dense as the sample count.
3. **Static environment:** workspace/tissue models are static during a plan;
   dynamic tissue deformation is future work.
4. **No continuous dynamics:** hybrid automata / reachable-set analysis
   (Flow*, JuliaReach) are future phases.
5. **Model validation:** DH parameters and tissue boundaries must be
   validated against a physical system.
6. **Regulatory:** certification requires organizational QMS (ISO 13485),
   clinical evidence, and HIL testing — out of scope for a solo researcher.

---

## 11. Roadmap

| Phase | Work |
|-------|------|
| 1 (current) | Kinematics, trajectory, simulator, mock SMT scaffold |
| 2 | Real Z3 integration hardening, invariant expansion, validation against incident reports |
| 3 | RTOS integration, temporal isolation, HIL on embedded servo targets |
| 4 | Open-source release (Apache-2.0), publication, academic collaboration |

Detailed phases in `../development_plan.md`.

---

## 12. Related Work

- **SMT verification of robotic plans:** recent work applies Z3 to motion
  planning reachability.
- **Runtime verification monitors (R2U2):** stream monitors for
  safety-critical aerospace software; a similar approach is applicable to
  per-command surgical checks.
- **Hybrid automata analysis (Flow*, JuliaReach):** continuous reachable-set
  analysis complements our discrete proof obligations.
- **Medical device software assurance:** IEC 62304 documents the life-cycle
  framing into which this toolchain fits.

---

## 13. Conclusion

OP-Roboter demonstrates a practical architecture for **proof-gated
actuation**: every command is first translated into a boolean path formula,
its negation is checked for satisfiability, and the gate refuses to move unless
the proof succeeds. The scaffold compiles, tests, and demonstrates the full
pipeline in simulation. Remaining work is to harden the SMT backend, model
environment dynamics, and pursue the multi-year path toward regulatory-grade
formal verification of surgical robot software.

---

## A. Appendix: Constructed Files

- `Cargo.toml` — workspace manifest (11 crates)
- `crates/op-math` … `crates/op-sim` — layered implementation
- `docs/architecture.md` — system architecture
- `docs/getting-started.md` — build/run/test guide
- `docs/formal-methods.md` — SMT/invariant foundations
- `docs/standards-and-safety.md` — regulatory context
- `docs/whitepaper.md` — this document