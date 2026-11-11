# WashTrap Ledger (TradeShield) — Comprehensive Architecture & Planning Documentation

> **Project Directory:** `TradeShield/`  
> **Primary Language / Toolchain:** `TypeScript`  
> **Verification Command:** `npm test --prefix packages/api`  
> **Reference Sources:** `KELLERSYSTEMS Spine.md` · `PaySlip.canvas` · `Life Work.canvas`

---

## 1. Executive Spine & System Overview

**WashTrap Ledger (TradeShield)** — An IoT microplastic filter retrofit for commercial industrial laundry machines paired with a tamper-proof cryptographic ledger to certify captured microfibers and issue microplastic credits.

### Current Engineering Baseline
A TypeScript IoT microfiber capture and ledger monorepo (`packages/api`, `packages/shared`, `packages/web`) with 63 passing integration tests, ESP32 telemetry ingestion, tamper-evident hash-chained SQLite credit ledger, and a React dashboard.

---

## 2. Canonical Specifications & Mathematical Invariants (`PaySlip.canvas` / `Spine`)

The following 12 architectural pillars and formal invariants define the target engineering state of the system:

- **Mechanical Microfiber Hydrodynamic Interception (80 µm Polypropylene):** Nachrüstbarer hydrodynamischer Filter für gewerbliche Wäschereien, der pro 6-kg-Waschgang bis zu 700 mg synthetischer Mikroplastikfasern aus dem Abwasser mechanisch abscheidet.
    
- **ESP32 Inline Differential Pressure & Flow Telemetry:** Industrielle Sensorik erfasst Waschzyklen, Wassertemperatur, Durchflussvolumen und den Druckanstieg vor dem Filtergewebe zur exakten Bestimmung des Sättigungsgrads.
    
- **Empirical Wash Cycle Microplastic Mass Yield Equations:** Deterministische Algorithmen berechnen das zurückgehaltene Fasergewicht anhand von Textilart, Flottengröße, Laugentemperatur und Spülzyklusdauer.
    
- **Tamper-Evident SQLite Cryptographic Hash Chain:** Lokales, manipulationssicheres Hauptbuch auf SQLite-Basis, in dem jeder Waschgang kryptographisch mit dem vorherigen verkettet wird, ohne auf spekulative Blockchains angewiesen zu sein.
    
- **EU Ecodesign ESPR 2027 Regulatory Compliance Verification:** Automatisierte Erstellung rechtskonformer Auditberichte zur Erfüllung der kommenden EU-Ökodesign-Verordnung für nachhaltige Produkte und Abwasservorgaben.
    
- **Tradeable Digital Microplastic Offset Credit Issuance (€1/g):** Generierung digitaler, fälschungssicherer Zertifikate für Textilmarken und Modekonzerne zur Erfüllung verbindlicher CSRD- und ESG-Nachhaltigkeitsziele.
    
- **Flow-Rate Anomaly & Bypass Valve Manipulation Detection:** Mathematische Plausibilitätsprüfungen der Druckdifferenzkurve erkennen sofort, wenn das Bypassventil illegal geöffnet oder der Filter ohne Spülung betrieben wird.
    
- **Decentralized Commercial Laundry Fleet Coordination:** Zentrales Dashboard für Großwäschereien, Hotels und Krankenhäuser koordiniert Filterwechselintervalle, Entsorgungslogistik und Zertifikatsausschüttungen.
    
- **Dynamic Fiber Saturation & Automated Maintenance Alerts:** Frühzeitige Warnungen an das Bedienpersonal verhindern ein Zusetzen des Filters und garantieren unterbrechungsfreie Waschabläufe.
    
- **Third-Party Environmental Audit & Provenance Verification:** Externe Umweltgutachter können die gesamte Erfassungskette von den Rohdaten des Drucksensors bis zum ausgestellten Zertifikat mathematisch auditieren.
    
- **Offline Event Buffering & Industrial Mesh Ingestion:** Robuster Ringspeicher auf dem ESP32 puffert Betriebsdaten bei Netzwerkausfällen und synchronisiert sich nahtlos, sobald wieder eine Verbindung besteht.
    
- **Trustless Microplastic Abatement Verification Gate:** Ein kryptographisches Kontrollsystem, das den Transfer von Entsorgungs-Credits erst dann freigibt, wenn die physikalischen Sensordaten die reale Rückhalteabsaugung zweifelsfrei belegen.

---

## 4. Codebase Architecture & Module Inventory

Total tracked source and configuration files in `TradeShield/`: **50**

| File Path | Lines | Architectural Role |
| :--- | :---: | :--- |
| `package.json` | 18 | Build / Package Manifest |
| `packages/api/package.json` | 38 | Build / Package Manifest |
| `packages/api/vitest.config.ts` | 10 | Verification & Test Suite |
| `packages/api/scripts/batch-generate.ts` | 190 | Core Implementation Module |
| `packages/api/scripts/simulator.ts` | 506 | Core Implementation Module |
| `packages/api/src/app.ts` | 71 | Core Implementation Module |
| `packages/api/src/index.ts` | 52 | Core Implementation Module |
| `packages/api/src/db/connection.ts` | 68 | Core Implementation Module |
| `packages/api/src/db/migrate.ts` | 87 | Core Implementation Module |
| `packages/api/src/db/seed.ts` | 440 | Core Implementation Module |
| `packages/api/src/middleware/auth.ts` | 89 | Core Implementation Module |
| `packages/api/src/middleware/error-handler.ts` | 42 | Core Implementation Module |
| `packages/api/src/middleware/rate-limit.ts` | 78 | Core Implementation Module |
| `packages/api/src/middleware/request-id.ts` | 9 | Core Implementation Module |
| `packages/api/src/routes/auth.ts` | 114 | Core Implementation Module |
| `packages/api/src/routes/credits.ts` | 588 | Core Implementation Module |
| `packages/api/src/routes/dashboard.ts` | 348 | Core Implementation Module |
| `packages/api/src/routes/dev.ts` | 113 | Core Implementation Module |
| `packages/api/src/routes/devices.ts` | 279 | Core Implementation Module |
| `packages/api/src/routes/events.ts` | 285 | Core Implementation Module |
| `packages/api/tests/auth.test.ts` | 87 | Verification & Test Suite |
| `packages/api/tests/credits.test.ts` | 334 | Verification & Test Suite |
| `packages/api/tests/dashboard.test.ts` | 222 | Verification & Test Suite |
| `packages/api/tests/devices.test.ts` | 234 | Verification & Test Suite |
| `packages/api/tests/events.test.ts` | 268 | Verification & Test Suite |
| `packages/api/tests/helpers.ts` | 185 | Verification & Test Suite |
| `packages/shared/package.json` | 23 | Build / Package Manifest |
| `packages/shared/src/constants.ts` | 23 | Core Implementation Module |
| `packages/shared/src/index.ts` | 3 | Core Implementation Module |
| `packages/shared/src/types.ts` | 170 | Core Implementation Module |
| `packages/shared/src/validation.ts` | 102 | Core Implementation Module |
| `packages/web/package.json` | 28 | Build / Package Manifest |
| `packages/web/postcss.config.js` | 6 | Core Implementation Module |
| `packages/web/tailwind.config.js` | 31 | Core Implementation Module |
| `packages/web/vite.config.ts` | 15 | Core Implementation Module |
| `packages/web/src/App.tsx` | 37 | Core Implementation Module |
| `packages/web/src/main.tsx` | 13 | Core Implementation Module |
| `packages/web/src/components/charts/CaptureAreaChart.tsx` | 228 | Core Implementation Module |
| `packages/web/src/components/charts/DeviceBarChart.tsx` | 115 | Core Implementation Module |
| `packages/web/src/components/charts/SparkLine.tsx` | 41 | Core Implementation Module |
| `packages/web/src/components/charts/TrendIndicator.tsx` | 33 | Core Implementation Module |
| `packages/web/src/components/charts/index.ts` | 4 | Core Implementation Module |
| `packages/web/src/components/layout/Layout.tsx` | 70 | Core Implementation Module |
| `packages/web/src/lib/api.ts` | 102 | Core Implementation Module |
| `packages/web/src/pages/Credits.tsx` | 301 | Core Implementation Module |
| `packages/web/src/pages/Dashboard.tsx` | 380 | Core Implementation Module |
| `packages/web/src/pages/DeviceDetail.tsx` | 172 | Core Implementation Module |
| `packages/web/src/pages/Devices.tsx` | 107 | Core Implementation Module |
| `packages/web/src/pages/Events.tsx` | 225 | Core Implementation Module |
| `packages/web/src/pages/Login.tsx` | 91 | Core Implementation Module |

---

## 5. Gap Analysis & Production-Readiness Audit

- **Tamper-Evident Microfiber Ledger Hash Chain (`packages/api/src/routes/events.ts` & `credits.ts`):** Every recorded wash cycle event must compute $H_i = \text{SHA-256}(H_{i-1} \parallel \text{device\_id} \parallel \Delta P \parallel V_{\text{water}} \parallel m_{\text{microplastic}})$, proving that wash events cannot be inserted, deleted, or backdated.
- **Differential Pressure vs. Bypass Tamper Detection (`packages/api/src/routes/events.ts`):** If a commercial laundry operator illegally opens the filter bypass valve, water flows freely with zero differential pressure $\Delta P \approx 0$; the telemetry validator must detect this anomaly and freeze credit issuance.
- **EU Ecodesign ESPR 2027 Compliance Audit Export (`packages/api/src/routes/credits.ts`):** Must synthesize signed CSRD/ESPR regulatory proof certificates for textile brand offset purchasing.

---

## 7. Actionable Development & Verification Queue

Each task below is synchronized with `DEV/ROADMAP.md` and `DEV/TASKS.json` for automated or manual execution:

| Task ID | Title | Target Files | Verification Command |
| :---: | :--- | :--- | :--- |
| **TASK-01** | Harden cryptographic hash chain linkage in wash-cycle event ingestion | `packages/api/src/routes/events.ts`, `packages/api/tests/events.test.ts` | `npm test --prefix packages/api` |
| **TASK-02** | Implement filter bypass valve manipulation and sensor fraud detection | `packages/api/src/routes/events.ts`, `packages/api/tests/events.test.ts` | `npm test --prefix packages/api` |
| **TASK-03** | Implement EU Ecodesign ESPR 2027 CSRD compliance certificate generator | `packages/api/src/routes/credits.ts`, `packages/api/tests/credits.test.ts` | `npm test --prefix packages/api` |
