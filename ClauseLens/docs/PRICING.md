# ClauseLens — Pricing & Licensing Model

ClauseLens is offered as a tiered commercial software product tailored to freelancers, growing SMBs, legal tech consultants, and enterprise procurement teams.

---

## 1. Plan Overview & Pricing Matrix

| Feature | Starter (Trial) | Professional (Solo) | Team & Kanzlei | Enterprise |
|---|---|---|---|---|
| **Target User** | Evaluators, Single Reviews | Freelancers, Founders, Consultants | Boutique Law Firms, SMB Teams (3–5) | Corporate Legal Ops, Procurement, IT-Security |
| **Price (Monthly)** | **€0** (14 Tage) | **€49** net | **€149** net | **Auf Anfrage** |
| **Price (Annual / Discount)** | **€0** | **€490 / Jahr** | **€1.490 / Jahr** | **Auf Anfrage** |
| **Supported File Formats** | TXT, PDF, DOCX | TXT, PDF, DOCX | TXT, PDF, DOCX | TXT, PDF, DOCX |
| **Analysis Engine** | 47 BGB Detectors | 47 BGB Detectors | 47 BGB Detectors | 47 BGB Detectors + Custom Rules |
| **Output Formats** | CLI Output, Markdown (`.md`) | CLI, Markdown, JSON, **HTML, DIN A4 Vektor-PDF** | CLI, Markdown, JSON, HTML, PDF, **Branded Reports** | CLI, Markdown, JSON, HTML, PDF, **OASIS SARIF, REST API** |
| **Batch / Portfolio Audits** | 1 Document at a time | Up to 50 Documents per Batch | **Up to 250 Documents per Batch** | **Unlimited Documents** |
| **Version Comparison (Diff)** | Basic | **Full Delta & Redline Diff** | **Full Delta & Redline Diff** | **Full Delta & Redline Diff** |
| **Policy Profiles** | Default Only | Standard, SaaS, Freelance, Procurement | All Profiles + Custom Policy Config | All Profiles + Custom Policy Config |
| **Deployment Mode** | Local CLI | Local CLI / Studio UI | Local CLI, Web Studio UI | Local CLI, Web UI, Headless Docker Server |
| **Account & Seat Licensing** | 1 Trial Seat (14 Tage) | **1 Hardware-gebundener Seat** | **5 Hardware-gebundene Seats** | **Unbegrenzte Seats / Flottenlizenz** |
| **Support & Updates** | Community / GitHub Issues | Email Support (48h SLA) | Priority Support (24h SLA), Team-Onboarding | Dedicated Support (12h SLA), Custom Setup |

---

## 2. ROI & Value Proposition

### The Cost of Contract Blindness
- **Hourly rate of German commercial attorney:** €200 – €450 / hour.
- **Average cost for simple AGB review:** €150 – €350 per agreement.
- **Cost of an unspotted 24-month automatic renewal trap:** Often €5,000 – €25,000 in unwanted recurring vendor fees.
- **ClauseLens cost per audit:** **< €0.50 per contract**.

By deploying ClauseLens, SMBs identify trap clauses in seconds before signing, saving thousands in external legal spend while protecting commercial flexibility.

---

## 3. Account-Bindung & Seat-Aktivierung

ClauseLens führt alle Vertragsprüfungen zu 100% lokal on-premise im Speicher aus (DSGVO-konform, kein Dokumententransfer ins Internet). Um Seats pro Nutzer und Arbeitsplatz sicherzustellen, erfordert die Software einen registrierten Nutzeraccount und eine Online-Seat-Validierung über die Lizenz-API `https://license.kellersystems.dev` (CLI, Browser und GitHub Action); der Server prüft den Polar-Schlüssel und signiert die gerätegebundene Sitzung.

### 3.1 Einen Pro- oder Team-Seat online aktivieren
```bash
node bin/clauselens.js login --email nutzer@kanzlei.de --key "CLA-PRO-..."
```

Der Lizenzserver prüft den Schlüssel und stellt ein 7 Tage gültiges, an Schlüssel und Gerät gebundenes Sitzungstoken aus. Lizenzschlüssel werden ausschließlich vom Lizenzserver ausgestellt; die CLI kann keine eigenen erzeugen.

### 3.2 Status und Geräte-Fingerprint prüfen
```bash
node bin/clauselens.js whoami
```

---

## 4. Payment & Checkout Integration

ClauseLens integrates directly with standard payment providers (Stripe, Lemon Squeezy, Paddle).
When a customer purchases a subscription:
1. Webhook receives `checkout.session.completed`.
2. Server calls `generateLicense({ tier, licensee: customer.name, expiresAt })`.
3. Customer receives their activation key via confirmation email and onboarding dashboard.
