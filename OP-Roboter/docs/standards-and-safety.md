# OP-Roboter — Clinical Standards, Compliance & Safety

This document maps the OP-Roboter research toolchain to the relevant medical
device regulatory landscape. It is a **standards comprehension map**, not a
claim of certification.

> ⚠️ **Legal notice.** This project targets safety-critical medical systems
> requiring regulatory certification (FDA 510(k)/De Novo, EU MDR, IEC
> 80601-2-77). A solo developer **cannot legally certify or deploy** these
> systems. This repository is for research, prototype, and open-source tooling
> for formal verification of surgical robot software — not a deployable medical
> product.

---

## 1. Applicable Standards & Regulations

| Standard / Regulation | Title | Relevance to OP-Roboter |
|-----------------------|-------|------------------------|
| **IEC 80601-2-77** | Medical electrical equipment — Part 2-77: Particular requirements for the basic safety and essential performance of robotically assisted surgical equipment | Primary product standard for robotic surgery systems |
| **IEC 62304** | Medical device software — Software life cycle processes | Software development process, requirements, architecture, verification |
| **ISO 14971** | Application of risk management to medical devices | Risk analysis, hazard traceability, mitigations |
| **IEC 60601-1** | General requirements for basic safety and essential performance of medical electrical equipment | Electrical/mechanical safety of the physical system |
| **FDA** draft guidance, *Robotically-Assisted Surgical Devices* (2019/2020) | U.S. regulatory expectations | Intended-use claims, clinical data roadmap |
| **EU MDR 2017/745** | European medical device regulation | Classification rules, CE marking pathway |
| **FDA Safer Technologies Program (STeP)** | Expedited pathway for devices with a safety advantage | Positioning as a safety-enhancing technology |

---

## 2. Where OP-Roboter Sits in the Regulatory Landscape

OP-Roboter implements **open-source research tooling** that would sit *inside*
a certified system's verified software. In a regulated product, OP-Roboter's
verification engine would be a **software item** under IEC 62304, contributing
automated control functions that mitigate hazards identified under ISO 14971.

| Regulatory artifact | OP-Roboter counterpart |
|---------------------|------------------------|
| Software requirements specification | `docs/*.md` + `op-spec` generated formal requirements |
| Software architecture | `docs/architecture.md` |
| Software detail design | crate-level docs + source |
| Verification (unit/integration) | `cargo test` (47 tests) |
| Formal methods evidence | `op-verifier` proofs (SMT / mock) |
| Risk control measures | Lethal invariants, execution gate, fail-safe blocking |
| Hazard traceability | `op-state-space` edge cases + `op-spec` requirements |

---

## 3. Mapping OP-Roboter Components to IEC 80601-2-77 Functions

The draft standard focuses on robotically assisted surgical equipment's basic
safety and essential performance. OP-Roboter addresses the *software control*
aspects:

| IEC 80601-2-77 concern | OP-Roboter component |
|------------------------|----------------------|
| Motion range limits | `WorkspaceEnvelope` invariant |
| Unintended motion / force control | `ForceLimit` + `tissue_overload` (op-energy) |
| Single-fault safety (fail-safe) | `ExecutionGate` (block unless proof succeeds) |
| Tool/port (RCM) integrity | `RcmStay` invariant + `RcmConstraint` |
| Prevention of patient injury | `TissueBoundary` invariant |
| Software-provided hazards | `op-taint` (untrusted sensor data) + `op-verifier` |

---

## 4. Risk Management (ISO 14971) Connection

OP-Roboter's design implements the classic hazard-control loop:

```text
Hazard identification ──► Risk analysis ──► Risk control ──► Verification
        ▲                                                        │
        └──────────────── residual risk review ◄─────────────────┘
```

- **Hazard identification:** `op-state-space` enumerates fault events
  (patient motion, hemorrhage, instrument breakage, power loss, sensor dropout).
- **Risk control:** lethal invariants encode upper bounds
  (force/depth/speed/envelope/RCm). The execution gate enforces them
  per-command and blocks on any violation or unknown.
- **Verification of risk control:** each command is proved against the
  invariant conjunction before release.
- **Residual risk:** fail-safe blocking on `Unknown` leaves residual risk only
  where the verifier itself is unsound/incomplete — which is why event logging
  and `op-spec` archiving of generated proofs matter.

---

## 5. Software Life Cycle (IEC 62304) Position

OP-Roboter is a research scaffold; full IEC 62304 compliance would require:

1. Documented software requirements traceability.
2. Hazard-driven software architecture.
3. Code review + static analysis (clippy) in CI.
4. Full verification evidence (unit/integration/system).
5. Configuration management and change control.
6. Risk management file integration.

The scaffold lays the *architecture* and *verification* groundwork, which is
where formal methods provide the highest-value evidence.

---

## 6. Formal Verification as Regulatory Evidence

Regulators increasingly accept **formal methods** as high-assurance evidence.
OP-Roboter's approach maps to accepted techniques:

| Formal technique | OP-Roboter use | Regulatory relevance |
|------------------|----------------|----------------------|
| SMT solving (Z3) | `op-verifier/z3-solver` | Proves the negation of safety formula UNSAT |
| Invariant enforcement | `LethalInvariantSet` | Demonstrates hazard controls hold |
| Runtime verification | mock solver + gate | Per-command checks in the control loop |
| State-space exploration | `op-state-space` | Systematic hazard discovery |
| Taint / information flow analysis | `op-taint` | Demonstrates data integrity from sensors to actuators |
| Automated spec generation | `op-spec` | Machine-readable reference for certification |

---

## 7. Safety Concept: Fail-Safe by Construction

The single most important safety property is the **execution gate**:

```text
release(command) ⇔ verify(path) = Proved
```

Everything else follows from this:

- `Unknown` → blocked
- `Disproved` → blocked
- Unverified command (no proof available) → blocked
- Untrusted sensor data (not sanitized) → blocked (via `op-taint`)

---

## 8. Known Gaps on the Path to Certification

| Gap | Description |
|-----|-------------|
| Organizational structure | Certification requires a quality management system (ISO 13485), not possible for a solo developer |
| Clinical data | Regulatory submissions require clinical evidence |
| Hardware integration | HIL/testing with real servo/stepper systems required (see Phase 3 of `development_plan.md`) |
| Continuous formal methods | Hybrid automata / reachable-set analysis for continuous dynamics not yet implemented |
| Model validation | The kinematic/workspace model must be validated against the physical device |

---

## 9. Intended Use Statement (for research context)

> OP-Roboter is a **formal verification research toolchain** for robotic
> surgery software. It is not cleared/approved by any regulatory authority and
> must not be used to control a physical surgical device outside of a research
> simulation.

---

## 10. References

- IEC 80601-2-77:2020, Medical electrical equipment — Part 2-77
- IEC 62304:2006/AMD1:2015, Medical device software
- ISO 14971:2019, Medical devices — Application of risk management
- IEC 60601-1:2005/AMD1/AMD2, General requirements for safety
- FDA. *Robotically-Assisted Surgical Devices* — Draft Guidance (2019)
- Regulation (EU) 2017/745, Medical Devices Regulation (MDR)