# OP-Roboter — Engineering Risk Register & Mitigations

This document records the principal engineering risks to the OP-Roboter
control-stack design, with the mitigations implemented (or planned) for each.
It complements the whitepaper's limitations section with a concrete engineering
response per risk.

> ⚠️ **Research tooling only.** This project is research/prototype/open-source
> tooling for formal verification of surgical robot software — not a deployable
> medical device.

---

## Risk 1 — Sampling Discretization Gap

**Issue.** Kinetic action paths are evaluated via dense discrete spatial
samples (`s ∈ path`). A motion trajectory between sample points `sᵢ` and
`sᵢ₊₁` could theoretically cross a thin tissue boundary or violate an RCM
tolerance when the sample density is too low or the tool velocity is high.

**Mitigation (implemented).** `op-trajectory` now provides **exact continuous
segment checks** that close the gap for piecewise-linear tool motion:

- `Volume::contains_segment(p0, p1)` — an axis-aligned box is convex, so a
  segment lies inside *iff* both endpoints do. Exact, no sampling.
- `TissueBoundary::segment_signed_min_distance(p0, p1)` — the signed distance
  to a plane is *linear* in the interpolation parameter `t`, so its minimum
  over the whole segment is `min(d(p0), d(p1))`. Exact.
- `TissueBoundary::segment_protected_with_margin(p0, p1, margin)` — a boolean
  that is **true iff every point of the segment** keeps the margin.
- `Workspace::contains_segment_with_margin(p0, p1, margin)` — combines the
  two exact tests into a per-control-step continuous check.

**Residual / future.** Tool motion between control steps is assumed
piecewise-linear. For curved/spline-driven trajectories, or when RCM cone
constraints need *continuous* verification, we plan interval-arithmetic swept
volumes and hybrid-automata reachability (Flow\*, JuliaReach), as noted in the
whitepaper roadmap.

---

## Risk 2 — Static World & Tissue Deformation

**Issue.** In actual surgical environments (IEC 80601-2-77), soft tissue
deforms, organs shift, and patient breathing introduces dynamic movement. The
initial model uses fixed axis-aligned bounding boxes and static planes.

**Mitigation (implemented).** `op-trajectory` supports **sensor-updated,
swept barrier functions**:

- `TissueBoundary::expanded(distance)` returns a boundary shifted `distance`
  metres into the protected half-space.
- Conservatively expanding every boundary by the *maximum expected tissue
  displacement since the last sensor update* (from `op-state-space` fault
  magnitudes or clinical tolerances) and then applying
  `contains_segment_with_margin` yields a path check that holds even when the
  tissue moves — with no re-sampling.

**Residual / future.** The barrier is conservative (worst-case shift), not a
fully dynamic polyhedral model. Future work extends the workspace to dynamic
convex polyhedra and to continuous barrier-function updates driven by the
sensor stream (`op-taint` provenance guarantees the updates come from trusted,
sanitized sources).

---

## Risk 3 — SMT Performance & Real-Time Constraints

**Issue.** QF_NRA solving in Z3 (nlsat) is EXPTIME-complete in the worst case.
Online per-command SMT solving risks unpredictable latency spikes that cause
control-loop deadline misses in RTOS environments.

**Mitigation (implemented).** `op-verifier/src/z3.rs` enforces a **hard
verification deadline**:

- `DEFAULT_SOLVER_TIMEOUT_MS = 50` — a verification deadline budget.
- `Solver::set_timeout(timeout_ms)` — Z3's internal timeout.
- An **external `Instant` deadline guard** — if the process is still solving
  at the deadline, the call returns `Z3Outcome::Unknown`.
- `Unknown` propagates to `ProofOutcome::Unknown` → `ExecutionGate` **blocks**
  (fail-safe by construction: a late proof is a blocked command).

**Residual / future.** The 50 ms budget is a starting point; a hardened design
would:

1. Profile on the target RTOS and set per-cycle WCET-safe budgets.
2. Use **offline proof certificates** — pre-computed parameterised symbolic
   proofs checked in microseconds online (SMT-lib certificates / UNSAT cores).
3. Replace/augment solver calls with **R2U2-style runtime monitors** that
   evaluate streamed invariants in bounded time per tick.
4. Run the solver in a **priority-inverted shadow**: the control loop uses a
   cached/certified result while the solver keeps up asynchronously.

---

## Risk Register Summary

| # | Risk | Severity | Mitigation | Status |
|----|------|----------|-----------|--------|
| 1 | Sampling discretization gap | High | Exact continuous segment checks (`contains_segment`, `segment_signed_min_distance`) | **Implemented**; curved-path reachability planned |
| 2 | Static world / tissue deformation | High | Swept `expanded` barrier functions + segment checks; dynamic polyhedra planned | **Implemented** (conservative); dynamic model planned |
| 3 | SMT latency / RT deadline misses | High | Hard Z3 timeout + external deadline guard → `Unknown` → gate blocks | **Implemented**; offline certificates + R2U2 monitors planned |

---

## Interaction with the Rest of the Stack

- **Risk 1** is enforced by the new `Workspace::contains_segment_with_margin`
  which `op-constraints` generators should use per control step.
- **Risk 2** relies on `op-state-space` to bound *expected tissue displacement*
  and on `op-taint` to ensure displacement updates come from sanitized,
  trusted sensor sources.
- **Risk 3** makes the `z3-solver` feature fail-safe and RT-compatible; the
  default mock solver remains deterministic and bounded for simulator use.

See `docs/formal-methods.md` for the formal context and `docs/whitepaper.md`
for the overall system argument.