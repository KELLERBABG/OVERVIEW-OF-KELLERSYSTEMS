# ClauseLens — Architecture

**Version:** 1.0.0 · ESM · Node ≥ 18 · zero npm dependencies

## Module Map

```
bin/
  clauselens.js          Unified CLI entry (analyze, batch, compare, serve, login, whoami, logout)
  server.js              Direct local web server & API launcher

src/
  textio.js              File dispatch, decoding, line mapping, sentence segmentation
    ├─ pdfextract.js     PDF → text (FlateDecode, Tj/TJ, WinAnsi/UTF-16BE, scan diagnostics)
    └─ docxextract.js    DOCX → text (ZIP central directory, document.xml)
  analyze.js             Deterministic rule engine: 47 BGB detectors, dedup, risk scoring
  config.js              Policy profiles (SaaS, Freelance, Procurement) & custom rules
  account.js             Hardware fingerprinting, per-seat online validation & account gating
  licensing.js           Licence keys, RS256 session tokens (public key only), seat gating
  licence-token.js       Browser-safe token verifier (WebCrypto, no node:crypto) shared by CLI & pages
  polar.js               Licence-server client for the pages: seat activation + local token check
  pdfreport.js           Multi-page DIN A4 vector PDF engine with SHA-256 seal
  htmlreport.js          Self-contained interactive HTML risk report generator
  sarif.js               OASIS SARIF 2.1.0 generator for GitHub Code Scanning
  openapi.js             OpenAPI 3.0.3 specification & interactive Swagger docs
  webhook.js             Stripe & Polar checkout webhook handler
  batch.js               Multi-document portfolio aggregator & risk ranker
  compare.js             Contract version diffing & risk delta engine
  server.js              Zero-dependency HTTP server, REST API & ClauseLens Studio
  report.js              Markdown & JSON report renderers

worker/
  index.js               Cloudflare Worker: license API (seats/verify, Polar webhook, RS256 tokens)
  wrangler.toml          Custom domain license.kellersystems.dev + workers.dev fallback route

schema/
  findings.schema.json   JSON Schema v2020-12 for contract audit results
  config.schema.json     JSON Schema v2020-12 for clauselens.config.json
  batch.schema.json      JSON Schema v2020-12 for portfolio batch audits

test/
  account.test.js        Hardware fingerprinting & online seat validation tests
  analyze.test.js        Detector behavior, deduplication, scoring
  segmentation.test.js   Sentence boundary rules and offset invariant
  binary.test.js         PDF/DOCX extraction with self-generating fixtures
  cli.test.js            CLI smoke tests (analyze, batch, compare, whoami, account gate)
  licensing.test.js      Offline license signing, expiry, and entitlement tests
  worker.test.js         License worker routes, signatures, Polar webhook
  polar.test.js          Browser licence-server client: fallback, forged tokens, seat errors
  licence-token.test.js  WebCrypto signature, expiry and machine-binding checks
  license_ui.test.js     Both pages must verify a signed session, never trust localStorage
  pdfreport.test.js      Multi-page vector DIN A4 PDF generation tests
  sarif.test.js          OASIS SARIF 2.1.0 schema conformance tests
  openapi.test.js        OpenAPI 3.0.3 spec and docs endpoint tests
  webhook.test.js        Stripe & Polar checkout webhook tests
  config.test.js         Policy profile overrides and custom rule compilation
  htmlreport.test.js     Interactive HTML report generator tests
  batch.test.js          Multi-contract portfolio aggregation tests
  compare.test.js        Contract version diffing and risk delta tests
  server.test.js         REST API endpoints, extraction, and intact index.html static serving
  evidence.test.js       Golden contract corpus and false-positive QA harness
  detectors_new.test.js  Verification of all newly expanded BGB trap detectors
  hardening.test.js      Adversarial stress-testing, soft-hyphen evasion, ReDoS benchmarks
  adversarial_enterprise.test.js Disguised trap detection in complex enterprise agreements
```

## Data Flow

```
┌────────────────────────────────────────────────────────┐
│                   Input Document(s)                    │
│                 (PDF, DOCX, TXT)                       │
└──────────────────────────┬─────────────────────────────┘
                           │
                           ▼
┌────────────────────────────────────────────────────────┐
│               loadDocument(path, format)               │
│        Sniff magic bytes (%PDF-, PK) or extension      │
│         → pdfToText() | docxToText() | decodeBuffer()  │
└──────────────────────────┬─────────────────────────────┘
                           │
                           ▼
┌────────────────────────────────────────────────────────┐
│                 segmentSentences(text)                 │
│        Boundaries: [.!?;:] + whitespace                │
│        Guards: abbreviations (z.B., Abs., Nr., vgl.)   │
│        Invariant: text.slice(start, end) === text      │
└──────────────────────────┬─────────────────────────────┘
                           │
                           ▼
┌────────────────────────────────────────────────────────┐
│           analyzeContract(text, { config, profile })   │
│   1) Collect candidates per sentence                   │
│   2) Dedup: merge same type within same paragraph      │
│   3) Construct structured Finding objects:             │
│      id, type, severity, confidence, lineNumbers,      │
│      excerpt, why, suggestion, tags                    │
│   4) Calculate riskScore according to active profile   │
└──────────────────────────┬─────────────────────────────┘
                           │
                           ▼
┌────────────────────────────────────────────────────────┐
│                   Output Dispatch                      │
│   - Terminal: Formatted summary                        │
│   - JSON: schema/findings.schema.json envelope         │
│   - Markdown: renderMarkdownReport()                   │
│   - HTML: generateHtmlReport() (self-contained SVG/CSS)│
└────────────────────────────────────────────────────────┘
```

## Policy Profiles & Risk Scoring

```
weight = activeProfile.weights (default: niedrig=2, mittel=7, hoch=15)
riskScore = min(100, Σ weight(severity(f)) for f in findings)
riskLevel = hoch if ≥ activeProfile.thresholds.hoch,
            mittel if ≥ activeProfile.thresholds.mittel,
            else niedrig
```

Built-in profiles:
- `default`: Balanced general commercial terms (§§ 305–310 BGB).
- `b2b_saas`: Cloud software focus (stricter on unilateral changes and IP transfer).
- `freelance_consulting`: Freelancer focus (stricter on non-compete and penalties).
- `procurement_strict`: Buyer focus (stricter on liability and default interest).

## Exit Codes (CLI)

| Code | Meaning |
|---|---|
| 0 | Analysis or operation completed successfully |
| 1 | Usage error / unknown command / missing file argument |
| 2 | Input file unreadable or configuration load error |
| 3 | Output file could not be written |
