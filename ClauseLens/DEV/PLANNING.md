# ClauseLens (B2B Contract Audit) — Comprehensive Architecture & Planning Documentation

> **Project Directory:** `ClauseLens/`  
> **Primary Language / Toolchain:** `JavaScript`  
> **Verification Command:** `node --test test/*.test.js`  
> **Reference Sources:** `KELLERSYSTEMS Spine.md` · `PaySlip.canvas` · `Life Work.canvas`

---

## 1. Executive Spine & System Overview

**ClauseLens (B2B Contract Audit)** — A zero-dependency, 100% offline contract audit tool for European SMEs to parse PDF/DOCX agreements and detect legally invalid clauses, renewal traps, and unfair liability terms under German BGB §§ 305–310.

### Current Engineering Baseline
A mature v1.0.0 zero-dependency Pure-Node B2B contract & AGB auditing engine (`src/analyze.js`, `src/clausedb.js`, `src/compare.js`, `src/pdfextract.js`, `src/docxextract.js`, `src/pdfreport.js`) with 47+ deterministic BGB §§ 305–310 detectors, SARIF 2.1.0 output, and extensive stress/red-team test corpora.

---

## 2. Canonical Specifications & Mathematical Invariants (`PaySlip.canvas` / `Spine`)

The following 12 architectural pillars and formal invariants define the target engineering state of the system:

- **Zero-Dependency Pure-Node Architecture:** Vollständige Ausführung auf Node.js nativen Modulen (`node:fs`, `node:crypto`, `node:http`, `node:zlib`) ohne ein einziges externes npm-Paket zur Eliminierung von Supply-Chain-Angriffen und Abhängigkeitskonflikten.
    
- **Deterministic BGB Invariant Checking (§§ 305–310 BGB):** 47 spezialisierte deterministische Klauseldetektoren zur Identifikation unzulässiger Klauselverbote mit und ohne Wertungsmöglichkeit im gewerblichen und Endverbraucher-Vertragsrecht.
    
- **Auto-Renewal & Asymmetric Termination Detectors:** Algorithmische Erkennung versteckter automatischer Vertragsverlängerungen, asymmetrischer Kündigungsfristen, Sonderkündigungsbeschränkungen und unzulässiger Schriftformerfordernisse nach § 309 Nr. 13 BGB.
    
- **Liability & Penalty Limit Auditing:** Präzise Identifikation einseitiger Haftungsausschlüsse, ungedeckter Folgeschadensklauseln, ungerechtfertigter Freistellungsverpflichtungen und überhöhter Vertragsstrafen nach deutschem AGB-Recht.
    
- **Native In-Memory Pure-Node PDF/DOCX Document Parsers:** Eigene Low-Level-Parser zur Extraktion von Text und Formatierungsbäumen aus PDF (FlateDecode, WinAnsi, UTF-16, TJ-Kerning) und DOCX (ZIP-Struktur und XML-Entity-Decoding) direkt im Arbeitsspeicher.
    
- **Air-Gapped DSGVO Privacy Architecture:** 100 % lokale Dokumentenanalyse im flüchtigen RAM ohne jegliche Datenübertragung an Cloud-LLMs zur absoluten Geheimhaltung strategischer Unternehmensverträge.
    
- **Automated Redline Alternative Clause Synthesizer:** Generierung von rechtssicheren Ersatzklauseln, pragmatischen Entschärfungsvorschlägen und Verhandlungsargumenten für den Einkauf und Vertrieb per Mausklick.
    
- **OASIS SARIF 2.1.0 & Sealed PDF Reports:** Standardisierter SARIF-Export für automatische CI/CD-Pipelines sowie Erstellung fälschungssicherer, vektorisierter DIN-A4-Auditberichte mit digitalem SHA-256 Integritätsstempel ohne Headless Chrome.
    
- **Unilateral Service Modification & Price Escalation Trap Detection:** Aufspüren von Klauseln, die dem Vertragspartner das einseitige Recht einräumen, Leistungsumfänge, Preisanpassungen oder SLAs ohne Kündigungsrecht nachzujustieren.
    
- **Jurisdiction & Choice of Law Risk Assessment:** Automatische Analyse ausländischer Gerichtsstandsklauseln und Schiedsgerichtsbarkeiten zur Minimierung von Prozesskostenrisiken bei internationalen Handelsverträgen.
    
- **Cross-Border Contract Delta Verification:** Integrierter Versionsvergleich zur mathematischen Ermittlung von Risikodifferenzen zwischen der Erstfassung und dem verhandelten Gegenentwurf.
    
- **Trustless B2B Negotiation Gate:** Automatisches Freigabesystem, das den Abschluss eines Vertrags erst dann empfiehlt, wenn alle identifizierten Hochrisikoklauseln nachweislich entschärft oder vom Vorstand freigezeichnet wurden.

---

## 4. Codebase Architecture & Module Inventory

Total tracked source and configuration files in `ClauseLens/`: **89**

| File Path | Lines | Architectural Role |
| :--- | :---: | :--- |
| `package.json` | 72 | Build / Package Manifest |
| `bin/archil-persist-check.js` | 75 | Core Implementation Module |
| `bin/clause-db.js` | 197 | Core Implementation Module |
| `bin/clauselens.js` | 472 | Core Implementation Module |
| `bin/mcp.js` | 6 | Core Implementation Module |
| `bin/server.js` | 17 | Core Implementation Module |
| `src/account.js` | 245 | Core Implementation Module |
| `src/analyze.js` | 3246 | Core Implementation Module |
| `src/archil.js` | 113 | Core Implementation Module |
| `src/batch.js` | 349 | Core Implementation Module |
| `src/clausedb.js` | 764 | Core Implementation Module |
| `src/compare.js` | 258 | Core Implementation Module |
| `src/config.js` | 135 | Core Implementation Module |
| `src/docxextract.js` | 131 | Core Implementation Module |
| `src/htmlreport.js` | 644 | Core Implementation Module |
| `src/licence-token.js` | 84 | Core Implementation Module |
| `src/licensing.js` | 218 | Core Implementation Module |
| `src/mcp.js` | 435 | Core Implementation Module |
| `src/openapi.js` | 371 | Core Implementation Module |
| `src/pdfextract.js` | 310 | Core Implementation Module |
| `src/pdfreport.js` | 416 | Core Implementation Module |
| `src/polar.js` | 129 | Core Implementation Module |
| `src/report.js` | 83 | Core Implementation Module |
| `src/sarif.js` | 98 | Core Implementation Module |
| `src/server.js` | 792 | Core Implementation Module |
| `src/textio.js` | 402 | Core Implementation Module |
| `src/webhook.js` | 74 | Core Implementation Module |
| `test/account.test.js` | 279 | Verification & Test Suite |
| `test/analyze.test.js` | 177 | Verification & Test Suite |
| `test/batch.test.js` | 31 | Verification & Test Suite |
| `test/binary.test.js` | 299 | Verification & Test Suite |
| `test/brain_corpus.test.js` | 76 | Verification & Test Suite |
| `test/clause_db.test.js` | 379 | Verification & Test Suite |
| `test/cli.test.js` | 205 | Verification & Test Suite |
| `test/compare.test.js` | 47 | Verification & Test Suite |
| `test/config.test.js` | 58 | Verification & Test Suite |
| `test/enterprise_benchmark.test.js` | 110 | Verification & Test Suite |
| `test/evasion_swarm_hardened.test.js` | 90 | Verification & Test Suite |
| `test/evidence.test.js` | 107 | Verification & Test Suite |
| `test/extract_api.test.js` | 63 | Verification & Test Suite |
| `test/hardening_stress.test.js` | 134 | Verification & Test Suite |
| `test/htmlreport.test.js` | 42 | Verification & Test Suite |
| `test/licence-token.test.js` | 67 | Verification & Test Suite |
| `test/license_ui.test.js` | 38 | Verification & Test Suite |
| `test/licensing.test.js` | 142 | Verification & Test Suite |
| `test/mcp.test.js` | 169 | Verification & Test Suite |
| `test/mdr_audit.test.js` | 111 | Verification & Test Suite |
| `test/new_detectors.test.js` | 178 | Verification & Test Suite |
| `test/nexusscale_audit.test.js` | 103 | Verification & Test Suite |
| `test/openapi.test.js` | 63 | Verification & Test Suite |
| `test/outreach.test.js` | 369 | Verification & Test Suite |
| `test/pdfreport.test.js` | 108 | Verification & Test Suite |
| `test/polar.test.js` | 123 | Verification & Test Suite |
| `test/redteam_ip_runner.js` | 142 | Verification & Test Suite |
| `test/redteam_payment_runner.js` | 142 | Verification & Test Suite |
| `test/redteam_saas.test.js` | 130 | Verification & Test Suite |
| `test/redteam_stress.js` | 122 | Verification & Test Suite |
| `test/sarif.test.js` | 56 | Verification & Test Suite |
| `test/segmentation.test.js` | 68 | Verification & Test Suite |
| `test/server.test.js` | 99 | Verification & Test Suite |
| *...and 29 additional files* | — | — |

---

## 5. Gap Analysis & Production-Readiness Audit

- **Cross-Reference & Multi-Paragraph Evasion Resilience (`src/analyze.js`):** Adversarial contracts often split a liability cap and its carve-out (e.g. § 309 Nr. 7a/b BGB injury to life/body/health and gross negligence) across non-adjacent paragraphs.
- **Contract Version Delta Risk Scoring (`src/compare.js`):** Needs quantitative risk-score delta verification proving whether a counter-draft actually reduced high-severity BGB §§ 307–309 findings or introduced new hidden traps.
- **Trustless Negotiation Gate Seal (`src/pdfreport.js` & `src/report.js`):** Ensure every exported SARIF and PDF report carries a reproducible SHA-256 document + ruleset hash and explicit gate verdict (`PASS`, `CONDITIONAL_REVIEW`, `BLOCK_EXECUTION`).

---

## 7. Actionable Development & Verification Queue

Each task below is synchronized with `DEV/ROADMAP.md` and `DEV/TASKS.json` for automated or manual execution:

| Task ID | Title | Target Files | Verification Command |
| :---: | :--- | :--- | :--- |
| **TASK-01** | Harden multi-paragraph BGB § 309 Nr. 7 liability carve-out resolution in analyze.js | `src/analyze.js`, `test/analyze.test.js` | `node --test test/analyze.test.js` |
| **TASK-02** | Verify negotiation delta risk-score regression tracking in compare.js | `src/compare.js`, `test/compare.test.js` | `node --test test/compare.test.js` |
