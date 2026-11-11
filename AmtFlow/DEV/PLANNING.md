# AmtFlow (BriefHelfer) — Comprehensive Architecture & Planning Documentation

> **Project Directory:** `AmtFlow/`  
> **Primary Language / Toolchain:** `TypeScript`  
> **Verification Command:** `npm test`  
> **Reference Sources:** `KELLERSYSTEMS Spine.md` · `PaySlip.canvas` · `Life Work.canvas`

---

## 1. Executive Spine & System Overview

**AmtFlow (BriefHelfer)** — A 100% offline, privacy-first desktop application that scans German administrative letters (Behördenbriefe), extracts deadlines and file numbers, and translates complex bureaucratic German into clear, plain language.

### Current Engineering Baseline
A TypeScript/JavaScript offline German administrative letter assistant (`src/classifier.js`, `src/extractor.ts`, `src/reply_generator.ts`, `src/storage/db.ts`) with 30-language localization. The classifier has broad test coverage, but statutory deadline calculation (§ 70 VwGO, § 355 AO, § 87 SGG with § 193 BGB weekend/holiday shift), PII redaction, and financial table plausibility auditing need completion.

---

## 2. Canonical Specifications & Mathematical Invariants (`PaySlip.canvas` / `Spine`)

The following 12 architectural pillars and formal invariants define the target engineering state of the system:

- **100% On-Device WASM/Tesseract OCR Pipeline:** Vollständig gerätelokale optische Zeichenerkennung für Dokumentenscans, die direkt im WebAssembly-Thread des Browsers oder Smartphones ausgeführt wird, ohne dass hochsensible Bescheide jemals an externe Cloud-Server übertragen werden.
    
- **Deterministic AST & Legal Entity Extraction:** Regelbasierte syntaktische Analyse und Parser-Grammatiken zur fehlerfreien Extraktion von Aktenzeichen, Ausstellungsdatum, erlassender Behörde, Handlungsaufforderungen, Zahlungszielen und konkreten Bescheid-Tenoren.
    
- **Statutory Deadline Calculator (§ 70 VwGO, § 355 AO, § 87 SGG):** Formale mathematische Modellierung des deutschen Fristenrechts inklusive Bekanntgabefiktion (§ 41 VwVfG / § 122 AO) sowie exakter Feiertags- und Wochenendberechnung nach §§ 187 ff., 193 BGB.
    
- **DIN 5008 Vector Document Synthesizer:** Algorithmische Generierung formvollendeter, juristisch präzise formulierter PDF-Schriftsätze für Fristverlängerung, Widerspruch, Akteneinsicht nach § 25 SGB X und Unterlagennachreichung per Knopfdruck.
    
- **Multi-Lingual Plain Language Mapping:** Deterministische Übersetzung komplexer behördlicher Schachtelsätze (*Beamtendeutsch*) in verständliche Leichte Sprache sowie 30 Fremdsprachen für barrierefreie Rechtsdurchsetzung von sozial benachteiligten Bürgern und Zugewanderten.
    
- **Encrypted Local Calendar ICS Generation:** Automatische Synthese lokaler `.ics`-Kalenderdateien mit gestaffelten Vorwarnungen (z. B. 7 Tage, 3 Tage und 24 Stunden vor Fristablauf) zur direkten Integration in das Smartphone-Betriebssystem.
    
- **Zero-Telemetry Privacy Guarantee:** Vollständiger Verzicht auf Tracker, externe Fonts, Web-Analytics oder Telemetrie-Endpunkte, wodurch das Sozialgeheimnis nach § 35 SGB I und strengste Vorgaben der DSGVO kompromisslos gewahrt bleiben.
    
- **PII Sanitization & Automatic Redaction Layer:** Vor dem Drucken oder Weiterleiten von Dokumenten werden hochsensible Identifikatoren (wie Steuer-ID, IBAN, Rentenversicherungsnummer) automatisch isoliert, maskiert oder auf Wunsch unumkehrbar geschwärzt.
    
- **Legal Aid Cost Calculation & Voucher Verification:** Integrierter Berechnungsalgorithmus für Beratungshilfe nach BerHG und Prozesskostenhilfe nach ZPO zur sofortigen Feststellung, ob ein Anspruch auf kostenlose anwaltliche Unterstützung besteht.
    
- **Form-Field Validation Invariants & Plausibility Audit:** Mathematische Konsistenzprüfung von behördlichen Geldforderungen gegen beiliegende Berechnungsbögen, um typische Rechenfehler von Jobcentern oder Finanzämtern sofort aufzudecken.
    
- **Asynchronous Dead-Drop PDF Vault:** Lokale, passwortverschlüsselte Ablage aller eingelesenen Bescheide und erstellten Schriftsätze im isolierten Gerätespeicher mit automatischer Löschfrist nach Ablauf der Aufbewahrungsnotwendigkeit.
    
- **Trustless Citizen Advocacy Gate:** Ein dezentrales System, das generierte Widerspruchsschreiben mit einem kryptographischen Zeitstempel versieht, der als verlässlicher Nachweis für die fristgerechte Absendung vor Gericht vorgelegt werden kann.

---

## 4. Codebase Architecture & Module Inventory

Total tracked source and configuration files in `AmtFlow/`: **18**

| File Path | Lines | Architectural Role |
| :--- | :---: | :--- |
| `capacitor.config.ts` | 27 | Core Implementation Module |
| `package.json` | 42 | Build / Package Manifest |
| `server.js` | 214 | Core Implementation Module |
| `sw.js` | 30 | Core Implementation Module |
| `test_catalog_all.py` | 195 | Verification & Test Suite |
| `vite.config.ts` | 22 | Core Implementation Module |
| `src/classifier.js` | 485 | Core Implementation Module |
| `src/classifier.test.js` | 1606 | Verification & Test Suite |
| `src/extractor.test.ts` | 73 | Verification & Test Suite |
| `src/extractor.ts` | 210 | Core Implementation Module |
| `src/index.ts` | 3 | Core Implementation Module |
| `src/languages.ts` | 34 | Core Implementation Module |
| `src/main.ts` | 25 | Core Implementation Module |
| `src/reply_generator.ts` | 27 | Core Implementation Module |
| `src/engine/camera.ts` | 38 | Core Implementation Module |
| `src/engine/classifier.ts` | 20 | Core Implementation Module |
| `src/engine/i18n.ts` | 41 | Core Implementation Module |
| `src/storage/db.ts` | 117 | Core Implementation Module |

---

## 5. Gap Analysis & Production-Readiness Audit

- **Statutory Deadline Calculation Engine (`src/extractor.ts`):** Needs exact implementation of the 3-day postal notification fiction (*Bekanntgabefiktion*, § 41 Abs. 2 VwVfG / § 122 Abs. 2 AO / § 37 Abs. 2 SGB X) and weekend/public-holiday rollover to the next business day per § 193 BGB.
- **PII Sanitization & Redaction Layer (`src/extractor.ts` / `src/reply_generator.ts`):** Needs deterministic masking of German Steuer-ID (11 digits), IBAN (`DE\d{20}`), Rentenversicherungsnummer, and BG-Aktenzeichen before export or sharing.
- **Bescheid Arithmetic Plausibility Auditor (`src/extractor.ts`):** Needs line-item summation verification for Jobcenter (*Bürgergeld* SGB II) and tax notice tables to automatically flag calculation discrepancies.
- **DIN 5008 Objection & Legal Aid Generator (`src/reply_generator.ts`):** Currently 27 lines; needs structured templates for *Fristwahrender Widerspruch*, *Antrag auf Akteneinsicht (§ 25 SGB X)*, and *Beratungshilfe-Prüfung*.

---

## 7. Actionable Development & Verification Queue

Each task below is synchronized with `DEV/ROADMAP.md` and `DEV/TASKS.json` for automated or manual execution:

| Task ID | Title | Target Files | Verification Command |
| :---: | :--- | :--- | :--- |
| **TASK-01** | Implement German statutory deadline calculator with Bekanntgabefiktion and § 193 BGB rollover | `src/extractor.ts`, `src/extractor.test.ts` | `npm test` |
| **TASK-02** | Expand DIN 5008 objection, Akteneinsicht, and Ratenzahlung generator | `src/reply_generator.ts`, `src/extractor.test.ts` | `npm test` |
| **TASK-03** | Implement deterministic PII redaction and sanitization engine | `src/extractor.ts`, `src/extractor.test.ts` | `npm test` |
| **TASK-04** | Add Bescheid calculation table plausibility auditor and ICS calendar reminder export | `src/extractor.ts`, `src/reply_generator.ts`, `src/extractor.test.ts` | `npm test` |
