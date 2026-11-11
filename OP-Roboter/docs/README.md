# OP-Roboter — Documentation

Comprehensive technical documentation and whitepaper for the **OP-Roboter**
formally verified surgical robot control stack.

> ⚠️ **Research tooling only.** This project is research/prototype/open-source
> tooling for formal verification of surgical robot software — not a deployable
> medical device.

---

## Document Index

| Document | Purpose |
|----------|---------|
| [`getting-started.md`](getting-started.md) | Build, test, and run the workspace and the simulator CLI. |
| [`architecture.md`](architecture.md) | System design: crate responsibilities, dependency DAG, runtime data flow, verification object, correctness guarantees. |
| [`formal-methods.md`](formal-methods.md) | Formal-method foundations: SMT, QF_NRA, lethal invariants, constraint pipeline, kinematics, limitations. |
| [`risks-and-mitigations.md`](risks-and-mitigations.md) | Engineering risk register: sampling gap, tissue deformation, SMT latency — with implemented mitigations. |
| [`standards-and-safety.md`](standards-and-safety.md) | Regulatory context: IEC 80601-2-77, IEC 62304, ISO 14971, FDA/MDR; safety concept and certification gaps. |
| [`whitepaper.md`](whitepaper.md) | Technical whitepaper: abstract, system model, constraint pipeline, proof-gated actuation, evaluation, roadmap. |

---

## Quick Navigation

### If you are new to the project

Start with **[`getting-started.md`](getting-started.md)** — build the workspace,
run the 47-test suite, and try the `op-sim` CLI.

### If you want to understand the design

Read **[`architecture.md`](architecture.md)** — the layered crate DAG and the
Model → Constrain → Verify → Execute pipeline.

### If you want the mathematical foundations

Read **[`formal-methods.md`](formal-methods.md)** — SMT / nonlinear real
arithmetic, lethal invariants, the negation trick
(`check-sat: ¬safe(path)`).

### If you want to understand the engineering risks and how they are handled

Read **[`risks-and-mitigations.md`](risks-and-mitigations.md)** — the sampling
discretization gap, tissue deformation, and SMT real-time latency, each with a
concrete mitigation implemented in `op-trajectory` and `op-verifier`.

### If you care about regulatory context

Read **[`standards-and-safety.md`](standards-and-safety.md)** — how the
toolchain maps to IEC 80601-2-77, IEC 62304, ISO 14971, and the FDA / EU MDR
landscape.

### If you want the one-page technical argument

Read **[`whitepaper.md`](whitepaper.md)**.

---

## Related

- [`../README.md`](../README.md) — project overview and build commands
- [`../development_plan.md`](../development_plan.md) — multi-year solo
  development plan