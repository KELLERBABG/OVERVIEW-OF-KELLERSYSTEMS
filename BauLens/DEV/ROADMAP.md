# BauLens — Development Roadmap & Action Tasks

> This file is the authoritative internal development roadmap and task sequencing plan.
> **Language:** `JavaScript`
> **Summary:** A deterministic DACH building-code compliance engine (`src/dach.js`, `src/engine.js`, `src/bplanparser.js`, `src/cadparser.js`, `src/dossierpdf.js`, plus a Rust Tauri desktop wrapper). Evaluates MBO, 16 German LBOs, Austrian OIB, Swiss IVHB, and BauNVO metrics (GRZ, GFZ, BMZ, Abstandsflächen).

## Task Queue

- [ ] **TASK-01: Verify state-specific roof-pitch Abstandsflächen factors and boundary overlap invariants**
  - **Target Files:** `src/dach.js`, `src/engine.js`
  - **Context Files:** `src/cadparser.js`, `src/bplanparser.js`
  - **Verify:** `npm test`
  - **Spec:** Ensure `src/dach.js` and `src/engine.js` deterministically calculate wall height $H = H_{\text{wand}} + f(\alpha) \cdot H_{\text{dach}}$ and setback depth $T = \max(T_{\text{min}}, c_{\text{land}} \cdot H)$ with full paragraph provenance.

1. Verify state rules for Bavaria (BayBO Art. 6: $0.4 H$, $\min 3.0\text{ m}$), NRW (BauO NRW § 6: $0.4 H$), and Baden-Württemberg (LBO BW § 5: $0.4 H$, $\min 2.5\text{ m}$), plus Austrian OIB RL 2/3 and Swiss IVHB.
2. Detect illegal *Abstandsflächen* overlap between opposing exterior walls on the same parcel.
3. Add automated tests verifying exact setback distances and statutory citations.

- [ ] **TASK-02: Enforce BauNVO § 19 Abs. 4 GRZ I / GRZ II accessory surface cap and GFZ Vollgeschoss rules**
  - **Target Files:** `src/bplanparser.js`, `src/engine.js`
  - **Context Files:** `src/dach.js`, `src/dossier.js`
  - **Verify:** `npm test`
  - **Spec:** Harden `src/bplanparser.js` and `src/engine.js` to audit both main building footprint (GRZ I) and sealed accessory surfaces (GRZ II: garages, access roads, subterranean structures per § 19 Abs. 4 BauNVO).

1. Verify that $\text{GRZ}_{\text{II}} \le \min(1.5 \times \text{GRZ}_{\text{max}}, 0.80)$ unless an explicit *Bebauungsplan* exemption is present.
2. Add tests covering compliant and non-compliant GRZ I / GRZ II / GFZ / BMZ configurations.

- [ ] **TASK-03: Add cryptographic SHA-256 permit-readiness seal verification to dossier exports**
  - **Target Files:** `src/dossier.js`, `src/dossierpdf.js`
  - **Context Files:** `src/engine.js`
  - **Verify:** `npm test`
  - **Spec:** Ensure `src/dossier.js` computes a deterministic canonical SHA-256 digest over the input project geometry, active jurisdiction rule version, and all rule verdicts, embedding it into the audit dossier so any post-export manipulation of findings is detectable.
