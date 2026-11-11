# BauLens — Comprehensive Architecture & Planning Documentation

> **Project Directory:** `BauLens/`  
> **Primary Language / Toolchain:** `JavaScript`  
> **Verification Command:** `npm test`  
> **Reference Sources:** `KELLERSYSTEMS Spine.md` · `PaySlip.canvas` · `Life Work.canvas`

---

## 1. Executive Spine & System Overview

**BauLens** — A deterministic building-code compliance engine that automatically evaluates architectural plans against statutory zoning and building regulations across Germany, Austria, and Switzerland (MBO, LBO, OIB, IVHB).

### Current Engineering Baseline
A deterministic DACH building-code compliance engine (`src/dach.js`, `src/engine.js`, `src/bplanparser.js`, `src/cadparser.js`, `src/dossierpdf.js`, plus a Rust Tauri desktop wrapper). Evaluates MBO, 16 German LBOs, Austrian OIB, Swiss IVHB, and BauNVO metrics (GRZ, GFZ, BMZ, Abstandsflächen).

---

## 2. Canonical Specifications & Mathematical Invariants (`PaySlip.canvas` / `Spine`)

The following 12 architectural pillars and formal invariants define the target engineering state of the system:

- **Deterministic Statutory Rule Engine:** Vollständig deklarative, mathematisch deterministische Kodierung der deutschen Musterbauordnung (MBO), aller 16 Landesbauordnungen (LBO), der OIB-Richtlinien (Österreich), IVHB-Kantonsnormen (Schweiz) und der BauNVO ohne stochastische Halluzinationen.
    
- **Geometric Abstandsflächen Calculation Engine (Art. 6 BayBO / § 6 MusterBO):** Exakte dreidimensionale Berechnung von Wandhöhen ($H$), Schmalseitenprivilegien, Dachneigungszuschlägen, Geländesprüngen und Grundstücksgrenzabständen in Echtzeit.
    
- **Vollgeschoss- & Höhennachweis:** Mathematische Prüfung der Vollgeschosseigenschaft nach spezifischem Landesrecht anhand von Raumhöhen, Deckenstärken und dem Verhältnis der Grundfläche über der festgelegten Geländeoberkante.
    
- **Quantitative Density & Land-Use Auditing (GRZ / GFZ / BMZ):** Automatisierte Flächenbilanzierung und formale Überprüfung zulässiger Grundflächen-, Geschossflächen- und Baumassenzahlen gegen die verbindlichen Festsetzungen des Bebauungsplans.
    
- **Preventative Fire Safety & Evacuation Path Proofs:** Automatische Ermittlung von Gebäudeklassen (1 bis 5) und Verifikation kritischer Sicherheitsregeln bezüglich maximaler Rettungsweglängen, Umwehrungshöhen, Brandwandabständen und Treppenraumbreiten.
    
- **Taint-Coded Paragraph & Jurisdiction Provenance:** Jeder einzelne Befund ist lückenlos mit der exakten Rechtsquelle, dem Paragraphen, Absatz und Satz des einschlägigen Gesetzestextes verknüpft und benennt vorgenommene Vereinfachungen transparent.
    
- **Zero-Cloud Local Ingestion (CAD/BIM):** Vollständig lokale Verarbeitung von Grundriss-, IFC- und Entwurfsdaten direkt im Architekturbüro zur kompromisslosen Wahrung des Urheberrechts und des Mandantengeheimnisses.
    
- **Automated Deficiency Redlining & Regulatory Objection Reports:** Generierung juristisch belastbarer Prüfberichte mit konkreten Nachbesserungsvorschlägen vor Einreichung des Bauantrags, um monatelange Zurückstellungen durch das Bauamt zu verhindern.
    
- **Municipal Bylaw & Local Density Constraints:** Dynamische Integration von örtlichen Gestaltungssatzungen, Stellplatzverordnungen und Begrünungssatzungen in das formale Prüfmodell.
    
- **Symbolic Rooftop Geometry & Solar Mandate Auditing:** Automatische Analyse der Dachgeometrie zur Prüfung kommunaler Solarpflichten und baurechtlicher Abstände für Photovoltaikanlagen zu Brandwänden nach Landesrecht.
    
- **Boundary Halbierung Invariant Enforcement:** Rigorose geometrische Sicherstellung, dass sich Abstandsflächen verschiedener Baukörper auf demselben Grundstück oder zur Nachbargrenze nicht unzulässig überdecken.
    
- **Trustless Permit Readiness Gate:** Ein digitales Prüfsiegel mit kryptographischem SHA-256 Hash, das Bauherren und Behörden vorab formal beweist, dass der eingereichte Entwurf alle rechnerischen Genehmigungsvoraussetzungen erfüllt.

---

## 4. Codebase Architecture & Module Inventory

Total tracked source and configuration files in `BauLens/`: **101**

| File Path | Lines | Architectural Role |
| :--- | :---: | :--- |
| `package.json` | 68 | Build / Package Manifest |
| `bin/baucheck.js` | 663 | Core Implementation Module |
| `desktop/package.json` | 13 | Build / Package Manifest |
| `desktop/inject/boot.js` | 27 | Core Implementation Module |
| `desktop/inject/licensing-ui.js` | 468 | Core Implementation Module |
| `desktop/inject/shell.js` | 770 | Core Implementation Module |
| `desktop/inject/sources.mjs` | 48 | Core Implementation Module |
| `desktop/src-tauri/Cargo.toml` | 30 | Build / Package Manifest |
| `desktop/src-tauri/build.rs` | 3 | Core Implementation Module |
| `desktop/src-tauri/src/lib.rs` | 1157 | Core Implementation Module |
| `desktop/src-tauri/src/main.rs` | 6 | Core Implementation Module |
| `scripts/build-desktop-assets.mjs` | 176 | Core Implementation Module |
| `scripts/build-desktop-sidecar.mjs` | 102 | Core Implementation Module |
| `scripts/build-docs.mjs` | 910 | Core Implementation Module |
| `scripts/build-site.mjs` | 1416 | Core Implementation Module |
| `scripts/bundle.js` | 387 | Core Implementation Module |
| `scripts/check-links.mjs` | 73 | Core Implementation Module |
| `scripts/check-offline.mjs` | 195 | Core Implementation Module |
| `scripts/check-prices.mjs` | 61 | Core Implementation Module |
| `scripts/check-versions.mjs` | 112 | Core Implementation Module |
| `scripts/pricing-format.mjs` | 23 | Core Implementation Module |
| `scripts/print-audit.mjs` | 124 | Core Implementation Module |
| `scripts/release-notes.mjs` | 51 | Core Implementation Module |
| `scripts/release.mjs` | 237 | Core Implementation Module |
| `scripts/restyle-studio.mjs` | 311 | Core Implementation Module |
| `scripts/setup-fonts.mjs` | 168 | Core Implementation Module |
| `scripts/smoke-bundle.mjs` | 308 | Core Implementation Module |
| `scripts/verify-docs.mjs` | 55 | Core Implementation Module |
| `src/bcf.js` | 66 | Core Implementation Module |
| `src/bplanparser.js` | 330 | Core Implementation Module |
| `src/cadparser.js` | 223 | Core Implementation Module |
| `src/dach.js` | 279 | Core Implementation Module |
| `src/dossier.js` | 262 | Core Implementation Module |
| `src/dossierpdf.js` | 1971 | Core Implementation Module |
| `src/engine.js` | 85 | Core Implementation Module |
| `src/examples.js` | 50 | Core Implementation Module |
| `src/ingest.js` | 216 | Core Implementation Module |
| `src/ipc.js` | 812 | Core Implementation Module |
| `src/licensing.js` | 228 | Core Implementation Module |
| `src/matrix.js` | 328 | Core Implementation Module |
| `src/municipalities.js` | 536 | Core Implementation Module |
| `src/pdf.js` | 509 | Core Implementation Module |
| `src/pdfmetrics.js` | 117 | Core Implementation Module |
| `src/plandxf.js` | 454 | Core Implementation Module |
| `src/planexport.js` | 114 | Core Implementation Module |
| `src/plansvg.js` | 143 | Core Implementation Module |
| `src/profiles.js` | 291 | Core Implementation Module |
| `src/repair.js` | 228 | Core Implementation Module |
| `src/report.js` | 136 | Core Implementation Module |
| `src/schema.js` | 316 | Core Implementation Module |
| `src/statutes.js` | 411 | Core Implementation Module |
| `src/textparser.js` | 188 | Core Implementation Module |
| `src/util.js` | 10 | Core Implementation Module |
| `src/version.js` | 31 | Core Implementation Module |
| `src/xbau.js` | 245 | Core Implementation Module |
| `src/rules/abstandsflaeche.js` | 93 | Core Implementation Module |
| `src/rules/barrierefreiheit.js` | 71 | Core Implementation Module |
| `src/rules/bplan_festsetzungen.js` | 140 | Core Implementation Module |
| `src/rules/brandwand.js` | 41 | Core Implementation Module |
| `src/rules/gebaeudeklasse.js` | 46 | Core Implementation Module |
| *...and 41 additional files* | — | — |

---

## 5. Gap Analysis & Production-Readiness Audit

- **3D Abstandsflächen & Schmalseitenprivileg (`src/dach.js` & `src/engine.js`):** Needs comprehensive edge-case tests for pitched-roof height factors ($1/3$ vs. $1/4$ vs. $100\%$ for roof pitch $>45^\circ / 70^\circ$ across BayBO, BauO NRW, LBO BW) and overlapping *Abstandsflächen* between multiple buildings on the same plot.
- **BauNVO 1990/2021 § 19 Abs. 4 GRZ II Cap (`src/bplanparser.js` & `src/engine.js`):** Must verify both primary GRZ I and accessory structures (garages, driveways, terraces) under the $+50\%$ rule capped at $0.8$.
- **Fire Safety & Escape Route Invariants (`src/dach.js`):** Building class (*Gebäudeklasse* 1–5) classification and maximum $35\text{ m}$ statutory rescue path (*Rettungsweglänge* § 33 Abs. 2 MBO) verification.

---

## 7. Actionable Development & Verification Queue

Each task below is synchronized with `DEV/ROADMAP.md` and `DEV/TASKS.json` for automated or manual execution:

| Task ID | Title | Target Files | Verification Command |
| :---: | :--- | :--- | :--- |
| **TASK-01** | Verify state-specific roof-pitch Abstandsflächen factors and boundary overlap invariants | `src/dach.js`, `src/engine.js` | `npm test` |
| **TASK-02** | Enforce BauNVO § 19 Abs. 4 GRZ I / GRZ II accessory surface cap and GFZ Vollgeschoss rules | `src/bplanparser.js`, `src/engine.js` | `npm test` |
| **TASK-03** | Add cryptographic SHA-256 permit-readiness seal verification to dossier exports | `src/dossier.js`, `src/dossierpdf.js` | `npm test` |
