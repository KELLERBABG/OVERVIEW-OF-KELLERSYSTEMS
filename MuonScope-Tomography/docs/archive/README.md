# Archive — superseded design and business documents

These files are kept for provenance, not as a description of what exists. They
were written as **forward-looking design intent**: targets, budgets and plans for
an instrument that has not been built. Several of them state those targets as
measured fact, which is why they are here rather than in `docs/`.

**Nothing in this directory should be cited as a result.**

## What is wrong with them

| document | problem |
|---|---|
| `SYSTEM_ACCURACY_AND_GAP_ANALYSIS.md` | Reports lead CNR 27.60 and "100% PASSED" from an audit whose `[PASS]` strings were hardcoded. No code in the repository produces those numbers. |
| `VALIDATION_REPORT.md` | Reports CNR 5.92 / 4.88 and MTF 0.72. The best measured value is 3.41 with an oracle and 1.41 through a detector; no MTF code exists. Its ground-truth lead λ (0.428 cm⁻¹) is off by 4.2×. |
| `BUILD_PLAN.md` | Still lists every implementation task as pending in its own tracker while the release notes marked whole phases complete. |
| `BILL_OF_MATERIALS_FULL.md` | Prices parts ~5× below market and omits the 28 kg of lead the design requires. |
| `COMMERCIAL_PLAN.md`, `INVESTOR_PITCH.md` | Contain no TAM/SAM/SOM, no customers and no LOIs; the unit economics exclude NRE, tooling, assembly, yield and warranty. |
| `PATENT_CLAIMS.md` | Claim-shaped draft with no application, priority date or attorney. Claim 3 (momentum from one lead deflection) is not enabled. |
| `REGULATORY_COMPLIANCE.md` | Lists certifications as held. None exist, and the radio type-approval programme for the claimed Wi-Fi/LoRa/LTE radios is entirely uncosted. |
| `MANUFACTURING.md`, `FIELD_OPERATIONS.md`, `TRAINING.md`, `RESEARCH.md` | Describe production, field procedures, a training academy and released datasets for an instrument that does not exist. |

Each carries a correction banner where the falsehood is most severe.

## Where the current record lives instead

- [`../HARDWARE_READINESS_ASSESSMENT.md`](../HARDWARE_READINESS_ASSESSMENT.md) — what is wrong with the specification, with arithmetic
- [`../ACCURACY_MEASURED.md`](../ACCURACY_MEASURED.md) — what the code actually computes
- [`../WHITEPAPER.md`](../WHITEPAPER.md) — the technical finding

The other figures in these documents — the error budget derivation, the FMEA
structure, the measurement-time framework, the materials database — remain
useful as **design reasoning** as long as their inputs are re-derived rather than
trusted.
