# OP-Roboter — Development Roadmap & Action Tasks

> This file is the authoritative internal development roadmap and task sequencing plan.
> **Language:** `Rust`
> **Summary:** A 7-crate Rust surgical robotics safety workspace (`op-math`, `op-kinematics`, `op-constraints`, `op-energy`, `op-gate`, `op-solver`, `op-telemetry`) implementing Denavit-Hartenberg forward/inverse kinematics, Jacobian velocity mapping, virtual fixture anatomical keep-out zones, and an execution safety gate.

## Task Queue

- [ ] **TASK-01: Wire anatomical virtual-fixture keep-out spheres and planes into op-gate**
  - **Target Files:** `crates/op-gate/src/lib.rs`, `crates/op-constraints/src/generate.rs`
  - **Context Files:** `crates/op-kinematics/src/forward.rs`, `crates/op-gate/tests/stress_tests.rs`
  - **Verify:** `cargo test -p op-gate --quiet`
  - **Spec:** Ensure `crates/op-gate/src/lib.rs` computes the forward-kinematics end-effector position $p(\theta + \Delta\theta)$ and worst-case stopping trajectory $p_{\text{stop}} = p + v \cdot t_{\text{brake}} + \frac{1}{2} a_{\max} t_{\text{brake}}^2$ for every commanded joint increment.

1. Block the command (`GateVerdict::VetoedAnatomicalBreach`) if any point along the stopping segment intersects a registered anatomical keep-out sphere/half-space (e.g. carotid artery or optic nerve boundary).
2. Add unit tests in `crates/op-gate/tests/stress_tests.rs` proving that commands aimed into a keep-out zone are vetoed before reaching the boundary.

- [ ] **TASK-02: Enforce Jacobian manipulability singularity guard and kinetic energy cap**
  - **Target Files:** `crates/op-kinematics/src/jacobian.rs`, `crates/op-energy/src/lib.rs`
  - **Context Files:** `crates/op-gate/src/lib.rs`
  - **Verify:** `cargo test -p op-kinematics -p op-energy --quiet`
  - **Spec:** Enhance `crates/op-kinematics/src/jacobian.rs` and `crates/op-energy/src/lib.rs`.

1. Compute Yoshikawa manipulability index $w(\theta) = \sqrt{\det(J(\theta) J(\theta)^T)}$ and trigger damped least-squares (DLS) velocity scaling or a hard gate veto when $w(\theta) < w_{\min}$.
2. In `op-energy`, enforce that tissue-contact kinetic energy $\frac{1}{2} \dot{\theta}^T M(\theta) \dot{\theta} \le E_{\text{safe}}(d)$, where $E_{\text{safe}}(d)$ scales down linearly with distance $d$ to the nearest anatomical boundary.
3. Add unit tests for singularity damping and proximity energy clamping.
