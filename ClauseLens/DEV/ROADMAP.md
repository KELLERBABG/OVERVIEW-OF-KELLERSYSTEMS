# ClauseLens — Development Roadmap & Action Tasks

> This file is the authoritative internal development roadmap and task sequencing plan.
> **Language:** `JavaScript`
> **Summary:** A mature v1.0.0 zero-dependency Pure-Node B2B contract & AGB auditing engine (`src/analyze.js`, `src/clausedb.js`, `src/compare.js`, `src/pdfextract.js`, `src/docxextract.js`, `src/pdfreport.js`) with 47+ deterministic BGB §§ 305–310 detectors, SARIF 2.1.0 output, and extensive stress/red-team test corpora.

## Task Queue

- [ ] **TASK-01: Harden multi-paragraph BGB § 309 Nr. 7 liability carve-out resolution in analyze.js**
  - **Target Files:** `src/analyze.js`, `test/analyze.test.js`
  - **Context Files:** `src/clausedb.js`
  - **Verify:** `node --test test/analyze.test.js`
  - **Spec:** Verify and harden cross-sentence / cross-paragraph context window resolution in `src/analyze.js` so that liability caps that validly exempt *Vorsatz*, *grobe Fahrlässigkeit*, *Kardinalpflichten*, and *Verletzung von Leben, Körper, Gesundheit* (§ 309 Nr. 7a/b BGB) in an adjacent sentence are not false-flagged, while caps missing any mandatory carve-out are deterministically flagged.

1. Add targeted regression tests in `test/analyze.test.js`.

- [ ] **TASK-02: Verify negotiation delta risk-score regression tracking in compare.js**
  - **Target Files:** `src/compare.js`, `test/compare.test.js`
  - **Context Files:** `src/analyze.js`
  - **Verify:** `node --test test/compare.test.js`
  - **Spec:** Ensure `src/compare.js` computes a deterministic `NegotiationGateVerdict` (`APPROVED`, `REQUIRES_REDLINE`, `REJECTED_NEW_HIGH_RISK`) when comparing an original contract against a counterparty redline, explicitly listing resolved findings, persistent blockers, and newly introduced risk clauses.

1. Add unit tests in `test/compare.test.js`.
