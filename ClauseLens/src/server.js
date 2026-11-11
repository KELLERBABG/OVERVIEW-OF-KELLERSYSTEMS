"use strict";

import { createServer } from "node:http";
import { readFileSync, existsSync } from "node:fs";
import path from "node:path";
import { analyzeContract, RULES } from "./analyze.js";
import { POLICY_PROFILES, mergeConfig } from "./config.js";
import { TIERS, TIER_ENTITLEMENTS } from "./licensing.js";
import { generateHtmlReport } from "./htmlreport.js";
import { generatePdfReport } from "./pdfreport.js";
import { generateSarif } from "./sarif.js";
import { extractTextFromBuffer } from "./textio.js";
import { getOpenApiSpec, renderApiDocsHtml } from "./openapi.js";
import { handleCheckoutWebhook } from "./webhook.js";
import {
  verifySeatOnline,
  loadStoredAccount,
  saveStoredAccount,
  clearStoredAccount,
  getMachineFingerprint,
  resolveActiveLicense,
} from "./account.js";

const MIME_TYPES = {
  ".html": "text/html; charset=utf-8",
  ".svg": "image/svg+xml",
  ".css": "text/css; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".txt": "text/plain; charset=utf-8",
};

function readBody(req) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    req.on("data", (chunk) => chunks.push(chunk));
    req.on("end", () => resolve(Buffer.concat(chunks).toString("utf-8")));
    req.on("error", reject);
  });
}

function sendJson(res, statusCode, data) {
  const payload = JSON.stringify(data, null, 2);
  res.writeHead(statusCode, {
    "Content-Type": "application/json; charset=utf-8",
    "Access-Control-Allow-Origin": "*",
    "Access-Control-Allow-Headers": "Content-Type, Authorization, X-License-Key",
    "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
  });
  res.end(payload);
}

export function createClauseLensServer(options = {}) {
  const rootDir = options.rootDir || process.cwd();
  const startTime = Date.now();

  const server = createServer(async (req, res) => {
    // CORS Preflight
    if (req.method === "OPTIONS") {
      res.writeHead(204, {
        "Access-Control-Allow-Origin": "*",
        "Access-Control-Allow-Headers": "Content-Type, Authorization, X-License-Key",
        "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
      });
      res.end();
      return;
    }

    const url = new URL(req.url, `http://${req.headers.host || "localhost"}`);
    const pathname = url.pathname;

    // --- API ROUTES ---

    // Health
    if (pathname === "/api/v1/health" && req.method === "GET") {
      sendJson(res, 200, {
        status: "ok",
        engine: "ClauseLens",
        version: "1.0.0",
        uptimeSeconds: Math.floor((Date.now() - startTime) / 1000),
      });
      return;
    }

    // OpenAPI Spec
    if (pathname === "/api/v1/openapi.json" && req.method === "GET") {
      sendJson(res, 200, getOpenApiSpec());
      return;
    }

    // Interactive API Documentation
    if ((pathname === "/docs" || pathname === "/api/docs") && req.method === "GET") {
      res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
      res.end(renderApiDocsHtml());
      return;
    }

    // Rules Catalog
    if (pathname === "/api/v1/rules" && req.method === "GET") {
      const catalog = RULES.map((r) => ({
        type: r.type,
        title: r.title,
        severity: typeof r.severity === "string" ? r.severity : "dynamisch",
        why: typeof r.why === "string" ? r.why : "dynamischer Kontext",
        suggestion: r.suggestion,
        tags: r.tags || [],
      }));
      sendJson(res, 200, {
        count: catalog.length,
        profiles: Object.keys(POLICY_PROFILES),
        rules: catalog,
      });
      return;
    }

    // License Check
    if (pathname === "/api/v1/license" && req.method === "GET") {
      const licKey = req.headers["x-license-key"] || url.searchParams.get("key");
      let lic = resolveActiveLicense();
      if (licKey) {
        const stored = loadStoredAccount();
        const seat = await verifySeatOnline({
          email: (stored && stored.email) || "license-check@clauselens.local",
          licenseKey: licKey,
        });
        lic = seat.valid
          ? {
              valid: true,
              tier: seat.tier,
              licensee: seat.email,
              expiresAt: seat.expiresAt,
              error: null,
              entitlements: TIER_ENTITLEMENTS[seat.tier] || TIER_ENTITLEMENTS[TIERS.COMMUNITY],
            }
          : {
              valid: false,
              tier: TIERS.COMMUNITY,
              licensee: null,
              expiresAt: null,
              error: seat.error,
              entitlements: TIER_ENTITLEMENTS[TIERS.COMMUNITY],
            };
      }
      sendJson(res, 200, lic);
      return;
    }

    // Account & Seat Status
    if (pathname === "/api/v1/account/status" && req.method === "GET") {
      const acc = loadStoredAccount();
      sendJson(res, 200, {
        authenticated: !!acc,
        account: acc || null,
        machineFingerprint: getMachineFingerprint(),
      });
      return;
    }

    // Account & Seat Login
    if (pathname === "/api/v1/account/login" && req.method === "POST") {
      try {
        const rawBody = await readBody(req);
        const body = JSON.parse(rawBody || "{}");
        const resSeat = await verifySeatOnline({
          email: body.email,
          licenseKey: body.licenseKey,
          authServer: body.authServer,
        });
        if (!resSeat.valid) {
          sendJson(res, 401, { error: resSeat.error });
          return;
        }
        saveStoredAccount(resSeat);
        sendJson(res, 200, { success: true, account: resSeat });
      } catch (err) {
        sendJson(res, 500, { error: err.message });
      }
      return;
    }

    // Account Logout
    if (pathname === "/api/v1/account/logout" && req.method === "POST") {
      clearStoredAccount();
      sendJson(res, 200, { success: true, message: "Seat auf diesem Gerät freigegeben." });
      return;
    }

    // Document Text Extraction (Native Zero-Dependency for PDF, DOCX, TXT)
    if (pathname === "/api/v1/extract" && req.method === "POST") {
      try {
        const rawBody = await readBody(req);
        const body = JSON.parse(rawBody || "{}");
        let buf;
        if (body.fileBase64) {
          buf = Buffer.from(body.fileBase64, "base64");
        } else if (body.text) {
          buf = Buffer.from(body.text, "utf-8");
        } else {
          sendJson(res, 400, { error: 'Feld "fileBase64" (Base64-String) oder "text" ist erforderlich.' });
          return;
        }
        const filename = body.filename || "upload.bin";
        const format = body.format || "auto";
        const extracted = extractTextFromBuffer(buf, format, filename);
        sendJson(res, 200, {
          text: extracted.text,
          format: extracted.format,
          charCount: extracted.text.length,
          lineCount: extracted.text.split("\n").length,
        });
      } catch (err) {
        sendJson(res, 400, { error: `Extraktionsfehler: ${err.message}` });
      }
      return;
    }

    // Analyze Single Document
    if (pathname === "/api/v1/analyze" && req.method === "POST") {
      try {
        const rawBody = await readBody(req);
        const body = JSON.parse(rawBody || "{}");
        if (!body.text || typeof body.text !== "string") {
          sendJson(res, 400, { error: 'Feld "text" (String) ist erforderlich.' });
          return;
        }

        const profileName = body.profile || "default";
        const result = analyzeContract(body.text, { profile: profileName });
        const meta = {
          source: body.filename || "api_input.txt",
          analyzedAt: new Date().toISOString(),
          profile: profileName,
        };

        if (body.format === "html") {
          const html = generateHtmlReport(result, meta);
          res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
          res.end(html);
          return;
        }

        if (body.format === "pdf") {
          const pdfBuf = generatePdfReport(result, meta);
          res.writeHead(200, {
            "Content-Type": "application/pdf",
            "Content-Disposition": 'attachment; filename="clauselens-audit.pdf"',
          });
          res.end(pdfBuf);
          return;
        }

        if (body.format === "sarif") {
          const sarif = generateSarif(result, meta);
          res.writeHead(200, { "Content-Type": "application/sarif+json; charset=utf-8" });
          res.end(sarif);
          return;
        }

        sendJson(res, 200, { ...result, meta });
      } catch (err) {
        sendJson(res, 500, { error: `Analysefehler: ${err.message}` });
      }
      return;
    }

    // Checkout Webhook (Stripe & Polar integration)
    if (
      (pathname === "/api/v1/webhook/checkout" ||
        pathname === "/api/v1/webhook/stripe" ||
        pathname === "/api/v1/webhook/polar") &&
      req.method === "POST"
    ) {
      try {
        const rawBody = await readBody(req);
        const provisioned = handleCheckoutWebhook(rawBody);
        sendJson(res, 200, provisioned);
      } catch (err) {
        sendJson(res, 400, { error: `Webhook-Fehler: ${err.message}` });
      }
      return;
    }

    // Compare Two Documents
    if (pathname === "/api/v1/compare" && req.method === "POST") {
      try {
        const rawBody = await readBody(req);
        const body = JSON.parse(rawBody || "{}");
        if (!body.text1 || !body.text2) {
          sendJson(res, 400, { error: 'Felder "text1" und "text2" sind erforderlich.' });
          return;
        }
        const res1 = analyzeContract(body.text1, { profile: body.profile });
        const res2 = analyzeContract(body.text2, { profile: body.profile });
        const deltaScore = res2.riskScore - res1.riskScore;

        sendJson(res, 200, {
          v1: { riskScore: res1.riskScore, riskLevel: res1.riskLevel, counts: res1.counts },
          v2: { riskScore: res2.riskScore, riskLevel: res2.riskLevel, counts: res2.counts },
          deltaScore,
          improvement: deltaScore < 0,
        });
      } catch (err) {
        sendJson(res, 500, { error: `Vergleichsfehler: ${err.message}` });
      }
      return;
    }

    // Batch Audit Endpoint
    if (pathname === "/api/v1/batch" && req.method === "POST") {
      try {
        const rawBody = await readBody(req);
        const body = JSON.parse(rawBody || "{}");
        const items = Array.isArray(body.documents) ? body.documents : [];
        if (items.length === 0) {
          sendJson(res, 400, { error: 'Array "documents" darf nicht leer sein.' });
          return;
        }

        const profileName = body.profile || "default";
        const results = items.map((doc, idx) => {
          const resDoc = analyzeContract(doc.text || "", { profile: profileName });
          return {
            filename: doc.filename || `doc_${idx + 1}.txt`,
            riskScore: resDoc.riskScore,
            riskLevel: resDoc.riskLevel,
            findingsCount: resDoc.findings.length,
            findings: resDoc.findings,
          };
        });

        const totalScore = results.reduce((sum, r) => sum + r.riskScore, 0);
        sendJson(res, 200, {
          totalDocuments: results.length,
          averageScore: Math.round(totalScore / results.length),
          documents: results,
        });
      } catch (err) {
        sendJson(res, 500, { error: `Batch-Fehler: ${err.message}` });
      }
      return;
    }

    // --- STATIC FILES & WEB STUDIO ---

    // Root serves Landingpage index.html
    if (pathname === "/" || pathname === "/index.html") {
      const indexPath = path.join(rootDir, "index.html");
      if (existsSync(indexPath)) {
        res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
        res.end(readFileSync(indexPath));
        return;
      }
    }

    // /app & /studio serves ClauseLens Studio (Web UI with Drag&Drop & Redlines)
    if (pathname === "/app" || pathname === "/studio" || pathname === "/studio.html") {
      const studioPath = path.join(rootDir, "studio.html");
      if (existsSync(studioPath)) {
        res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
        res.end(readFileSync(studioPath));
        return;
      }
      res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
      res.end(renderStudioHtml());
      return;
    }

    // /pricing serves Pricing & Licensing page
    if (pathname === "/pricing" || pathname === "/pricing.html") {
      const pricingPath = path.join(rootDir, "pricing.html");
      if (existsSync(pricingPath)) {
        res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
        res.end(readFileSync(pricingPath));
        return;
      }
    }

    // Legal Pages (Impressum, Datenschutz, EULA & AGB)
    if (pathname === "/impressum" || pathname === "/impressum.html") {
      const p = path.join(rootDir, "impressum.html");
      if (existsSync(p)) {
        res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
        res.end(readFileSync(p));
        return;
      }
    }
    if (pathname === "/privacy" || pathname === "/privacy.html" || pathname === "/datenschutz") {
      const p = path.join(rootDir, "privacy.html");
      if (existsSync(p)) {
        res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
        res.end(readFileSync(p));
        return;
      }
    }
    if (pathname === "/terms" || pathname === "/terms.html" || pathname === "/agb") {
      const p = path.join(rootDir, "terms.html");
      if (existsSync(p)) {
        res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
        res.end(readFileSync(p));
        return;
      }
    }

    // Static Assets
    const safePath = path.normalize(pathname).replace(/^(\.\.[\/\\])+/, "");
    const filePath = path.join(rootDir, safePath);
    if (existsSync(filePath)) {
      const ext = path.extname(filePath);
      const mime = MIME_TYPES[ext] || "application/octet-stream";
      res.writeHead(200, { "Content-Type": mime });
      res.end(readFileSync(filePath));
      return;
    }

    // 404
    sendJson(res, 404, { error: `Ressource ${pathname} nicht gefunden.` });
  });

  return server;
}

function renderStudioHtml() {
  return `<!doctype html>
<html lang="de">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>ClauseLens Studio &mdash; AGB &amp; Vertrags-Auditing</title>
  <style>
    :root {
      --bg: #121110;
      --surface: #1a1918;
      --surface-raised: #242220;
      --border: #33312e;
      --text: #eae6df;
      --text-muted: #9c9689;
      --red: #d1563c;
      --red-bg: #2d1814;
      --amber: #d99b38;
      --green: #48a05b;
      --green-bg: #14281a;
      --font-mono: 'IBM Plex Mono', 'JetBrains Mono', Consolas, monospace;
      --font-sans: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      --font-serif: 'Source Serif 4', Georgia, serif;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body { background: var(--bg); color: var(--text); font-family: var(--font-sans); padding: 24px; font-size: 14.5px; }
    .wrap { max-width: 1100px; margin: 0 auto; }
    header { display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid var(--border); padding-bottom: 16px; margin-bottom: 24px; flex-wrap: wrap; gap: 12px; }
    .brand { display: flex; align-items: center; gap: 12px; }
    .brand img { width: 36px; height: 36px; }
    .brand h1 { font-family: var(--font-serif); font-size: 1.6rem; color: #fff; }
    .nav-link { color: var(--text-muted); text-decoration: none; font-family: var(--font-mono); font-size: 12px; border: 1px solid var(--border); padding: 6px 12px; border-radius: 4px; cursor: pointer; transition: all 0.15s ease; }
    .nav-link:hover { color: #fff; border-color: var(--text-muted); }
    .modal-overlay { display: none; position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0,0,0,0.75); z-index: 1000; align-items: center; justify-content: center; }
    .modal-card { background: var(--surface); border: 1px solid var(--border); border-radius: 8px; width: 90%; max-width: 500px; padding: 24px; }
    .panel { background: var(--surface); border: 1px solid var(--border); border-radius: 8px; padding: 20px; margin-bottom: 20px; }
    h2 { font-family: var(--font-serif); font-size: 1.25rem; margin-bottom: 12px; color: #fff; }
    
    /* Drop Zone */
    .drop-zone { border: 2px dashed var(--border); border-radius: 8px; padding: 18px; text-align: center; margin-bottom: 14px; cursor: pointer; transition: all 0.2s ease; background: rgba(255,255,255,0.01); }
    .drop-zone:hover, .drop-zone.dragover { border-color: var(--red); background: rgba(209, 86, 60, 0.05); }
    .drop-zone-text { font-family: var(--font-mono); font-size: 12.5px; color: var(--text-muted); }
    
    textarea { width: 100%; height: 210px; background: var(--surface-raised); border: 1px solid var(--border); color: #fff; padding: 12px; font-family: var(--font-mono); font-size: 13px; border-radius: 6px; resize: vertical; }
    textarea:focus { outline: 2px solid var(--red); }
    .controls { display: flex; justify-content: space-between; align-items: center; gap: 12px; margin-top: 14px; flex-wrap: wrap; }
    select, button { padding: 9px 16px; border-radius: 6px; font-family: var(--font-mono); font-size: 12.5px; cursor: pointer; }
    select { background: var(--surface-raised); border: 1px solid var(--border); color: #fff; }
    button.btn-primary { background: #b8442c; color: #fff; border: 1px solid #e0694e; font-weight: 600; }
    button.btn-primary:hover { background: #cf4f33; }
    button.btn-sec { background: var(--surface-raised); border: 1px solid var(--border); color: #fff; }
    button.btn-sec:hover { border-color: var(--text); }
    #resultArea { display: none; }
    .res-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; flex-wrap: wrap; gap: 12px; }
    .score-badge { font-family: var(--font-mono); font-size: 1.3rem; font-weight: 600; padding: 4px 14px; border-radius: 6px; }
    .badge-hoch { background: var(--red-bg); color: var(--red); border: 1px solid var(--red); }
    .badge-mittel { background: #2f2514; color: var(--amber); border: 1px solid var(--amber); }
    .badge-niedrig { background: #172b1b; color: var(--green); border: 1px solid var(--green); }
    .finding-item { background: var(--surface-raised); border: 1px solid var(--border); border-radius: 6px; padding: 16px; margin-bottom: 14px; }
    .finding-title { font-weight: 600; color: #fff; margin-bottom: 8px; font-family: var(--font-serif); font-size: 1.05rem; display: flex; justify-content: space-between; align-items: center; }
    .finding-quote { font-style: italic; color: var(--text-muted); font-size: 13px; margin: 8px 0; border-left: 2px solid var(--border); padding-left: 10px; }
    .finding-why { font-size: 13px; color: #d0c9bc; margin: 6px 0; }
    .finding-sugg { color: var(--green); font-size: 13px; margin-top: 8px; background: rgba(72, 160, 91, 0.1); padding: 10px; border-radius: 4px; border: 1px solid rgba(72,160,91,0.25); }
    
    /* Redline Diff */
    .redline-box { margin-top: 10px; padding: 10px; background: #0e0d0c; border: 1px solid #282624; border-radius: 4px; font-family: var(--font-mono); font-size: 12.5px; line-height: 1.6; }
    del.redline-del { color: #f27360; background: rgba(209, 86, 60, 0.2); text-decoration: line-through; padding: 2px 4px; border-radius: 3px; }
    ins.redline-ins { color: #56c270; background: rgba(72, 160, 91, 0.2); text-decoration: none; padding: 2px 4px; border-radius: 3px; font-weight: 600; }
    .copy-redline-btn { font-size: 11px; padding: 3px 8px; margin-left: 8px; background: #1f1d1b; border: 1px solid #3d3a36; color: var(--green); border-radius: 3px; cursor: pointer; }
    .copy-redline-btn:hover { background: #2b2825; }
  </style>
</head>
<body>
<div class="wrap">
  <header>
    <div class="brand">
      <img src="assets/icon.svg" alt="ClauseLens Icon">
      <h1>ClauseLens Studio</h1>
    </div>
    <div style="display:flex; gap: 8px; align-items: center; flex-wrap: wrap;">
      <span id="licenseBadge" class="nav-link" style="color:var(--amber);" onclick="openLicenseModal()">Lizenz: Community</span>
      <a href="/docs" target="_blank" class="nav-link">API Dokumentation</a>
      <a href="/" class="nav-link">&larr; Zur Landingpage</a>
    </div>
  </header>

  <!-- License Modal -->
  <div id="licenseModal" class="modal-overlay" onclick="closeLicenseModal(event)">
    <div class="modal-card" onclick="event.stopPropagation()">
      <h2>ClauseLens Lizenzaktivierung</h2>
      <p style="color: var(--text-muted); font-size: 13px; margin-bottom: 14px;">
        Geben Sie Ihren Offline-Lizenzschlüssel (z. B. CL-PRO-... oder CL-ENTERPRISE-...) ein:
      </p>
      <input type="text" id="licenseKeyInput" placeholder="CL-PRO-eyJ0aWVyIj..." style="width: 100%; padding: 10px; background: var(--surface-raised); border: 1px solid var(--border); color: #fff; border-radius: 4px; font-family: var(--font-mono); font-size: 12px; margin-bottom: 14px;">
      <div id="licenseFeedback" style="font-family: var(--font-mono); font-size: 12px; margin-bottom: 14px;"></div>
      <div style="display: flex; justify-content: flex-end; gap: 8px;">
        <button class="btn-sec" onclick="closeLicenseModal()">Schließen</button>
        <button class="btn-primary" onclick="activateLicense()">Aktivieren</button>
      </div>
    </div>
  </div>

  <div class="panel">
    <h2>Vertragstext oder AGB zur Prüfung einfügen</h2>
    
    <!-- Drag & Drop Zone -->
    <div id="dropZone" class="drop-zone" onclick="document.getElementById('fileUpload').click()">
      <div class="drop-zone-text" id="dropZoneText">
        📄 Dokument hier ablegen (PDF, DOCX, TXT) oder klicken zum Auswählen
      </div>
      <input type="file" id="fileUpload" accept=".pdf,.docx,.txt" style="display:none" onchange="handleFileUpload(this.files[0])">
    </div>

    <textarea id="contractInput" placeholder="Fügen Sie hier Ihren Vertragstext ein (z. B. Klauseln zur Kündigung, Haftung oder automatischen Verlängerung)...">§ 12 Laufzeit und Kündigung
Der Vertrag hat eine Mindestvertragslaufzeit von zwei Jahren und verlängert sich nach Ablauf automatisch um jeweils weitere 24 Monate, sofern er nicht mit einer Frist von drei Monaten gekündigt wird.

§ 15 Haftungsbegrenzung
Die Haftung des Dienstleisters ist ausgeschlossen für sämtliche Schäden, auch bei grober Fahrlässigkeit.

§ 18 Gerichtsstand
Ausschließlicher Gerichtsstand für alle Streitigkeiten ist der Sitz des Dienstleisters.</textarea>

    <div class="controls">
      <div>
        <label style="font-family: var(--font-mono); font-size: 11.5px; color: var(--text-muted); margin-right: 8px;">Prüfprofil:</label>
        <select id="profileSelect">
          <option value="default">Standard BGB/AGB</option>
          <option value="b2b_saas">SaaS & Cloud Software</option>
          <option value="freelance_consulting">Freelancer & Beratung</option>
          <option value="procurement_strict">Einkauf & Großkunden</option>
        </select>
      </div>
      <div style="display: flex; gap: 8px; flex-wrap: wrap;">
        <button class="btn-sec" onclick="loadSampleDirty()">Beispiel-AGB laden</button>
        <button class="btn-primary" onclick="runAudit()">Audit starten &rarr;</button>
      </div>
    </div>
  </div>

  <div class="panel" id="resultArea">
    <div class="res-header">
      <div>
        <h2>Prüfergebnis</h2>
        <p id="summaryText" style="color: var(--text-muted); margin-top: 4px;"></p>
      </div>
      <div style="display: flex; gap: 8px; align-items: center; flex-wrap: wrap;">
        <div id="scoreBadge" class="score-badge"></div>
        <button class="btn-sec" onclick="downloadReport('html')">HTML-Export</button>
        <button class="btn-sec" onclick="downloadReport('pdf')">Vektor-PDF</button>
        <button class="btn-sec" onclick="downloadReport('sarif')">SARIF Export</button>
      </div>
    </div>
    <div id="findingsContainer"></div>
  </div>
</div>

<script>
  let activeLicenseKey = localStorage.getItem('clauselens_license') || '';

  window.addEventListener('DOMContentLoaded', () => {
    checkLicense(activeLicenseKey);
    setupDropZone();
  });

  function setupDropZone() {
    const dz = document.getElementById('dropZone');
    ['dragenter', 'dragover'].forEach(name => {
      dz.addEventListener(name, (e) => { e.preventDefault(); dz.classList.add('dragover'); }, false);
    });
    ['dragleave', 'drop'].forEach(name => {
      dz.addEventListener(name, (e) => { e.preventDefault(); dz.classList.remove('dragover'); }, false);
    });
    dz.addEventListener('drop', (e) => {
      if (e.dataTransfer.files && e.dataTransfer.files[0]) {
        handleFileUpload(e.dataTransfer.files[0]);
      }
    });
  }

  async function handleFileUpload(file) {
    if (!file) return;
    const dropText = document.getElementById('dropZoneText');
    dropText.textContent = 'Lese Datei: ' + file.name + '...';

    if (file.name.endsWith('.txt')) {
      const reader = new FileReader();
      reader.onload = (e) => {
        document.getElementById('contractInput').value = e.target.result;
        dropText.textContent = 'Geladen: ' + file.name;
        runAudit();
      };
      reader.readAsText(file);
      return;
    }

    // Binary PDF or DOCX: Send Base64 to /api/v1/extract
    const reader = new FileReader();
    reader.onload = async (e) => {
      const base64 = e.target.result.split(',')[1];
      try {
        const resp = await fetch('/api/v1/extract', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ fileBase64: base64, filename: file.name })
        });
        const data = await resp.json();
        if (data.text) {
          document.getElementById('contractInput').value = data.text;
          dropText.textContent = 'Geladen: ' + file.name + ' (' + data.lineCount + ' Zeilen)';
          runAudit();
        } else {
          dropText.textContent = 'Fehler: ' + (data.error || 'Extraktion fehlgeschlagen.');
        }
      } catch (err) {
        dropText.textContent = 'Fehler: ' + err.message;
      }
    };
    reader.readAsDataURL(file);
  }

  function openLicenseModal() {
    document.getElementById('licenseModal').style.display = 'flex';
    document.getElementById('licenseKeyInput').value = activeLicenseKey;
  }
  function closeLicenseModal() {
    document.getElementById('licenseModal').style.display = 'none';
  }

  async function checkLicense(key) {
    if (!key) {
      document.getElementById('licenseBadge').textContent = 'Lizenz: Community (Kostenlos)';
      return;
    }
    try {
      const res = await fetch('/api/v1/license?key=' + encodeURIComponent(key));
      const data = await res.json();
      if (data.valid) {
        document.getElementById('licenseBadge').textContent = 'Lizenz: ' + data.tier.toUpperCase() + ' (' + data.licensee + ')';
        document.getElementById('licenseBadge').style.color = 'var(--green)';
      } else {
        document.getElementById('licenseBadge').textContent = 'Lizenz: Ungültig';
        document.getElementById('licenseBadge').style.color = 'var(--red)';
      }
    } catch {
      document.getElementById('licenseBadge').textContent = 'Lizenz: Offline';
    }
  }

  async function activateLicense() {
    const key = document.getElementById('licenseKeyInput').value.trim();
    const fb = document.getElementById('licenseFeedback');
    fb.textContent = 'Prüfe...';
    try {
      const res = await fetch('/api/v1/license?key=' + encodeURIComponent(key));
      const data = await res.json();
      if (data.valid) {
        activeLicenseKey = key;
        localStorage.setItem('clauselens_license', key);
        fb.style.color = 'var(--green)';
        fb.textContent = 'Gültig: ' + data.tier.toUpperCase() + ' bis ' + (data.expiresAt || 'unbegrenzt');
        checkLicense(key);
        setTimeout(closeLicenseModal, 1200);
      } else {
        fb.style.color = 'var(--red)';
        fb.textContent = data.error || 'Ungültiger Lizenzschlüssel.';
      }
    } catch (err) {
      fb.style.color = 'var(--red)';
      fb.textContent = 'Fehler: ' + err.message;
    }
  }

  function copyToClipboard(text) {
    navigator.clipboard.writeText(text).then(() => {
      alert('In Zwischenablage kopiert!');
    });
  }

  async function runAudit() {
    const text = document.getElementById('contractInput').value;
    const profile = document.getElementById('profileSelect').value;
    const resArea = document.getElementById('resultArea');
    const badge = document.getElementById('scoreBadge');
    const summary = document.getElementById('summaryText');
    const list = document.getElementById('findingsContainer');

    try {
      const resp = await fetch('/api/v1/analyze', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-License-Key': activeLicenseKey
        },
        body: JSON.stringify({ text, profile })
      });
      const data = await resp.json();

      resArea.style.display = 'block';
      badge.textContent = 'Score: ' + data.riskScore + '/100 [' + data.riskLevel.toUpperCase() + ']';
      badge.className = 'score-badge badge-' + data.riskLevel;
      summary.textContent = data.summary;

      list.innerHTML = data.findings.map((f, i) => \`
        <div class="finding-item">
          <div class="finding-title">
            <span>\${i + 1}. \${f.titlePlainGerman} [\${f.severity.toUpperCase()}]</span>
            <button class="copy-redline-btn" onclick="copyToClipboard('\${f.suggestion.replace(/'/g, "\\\\'")}')">Gegenentwurf kopieren</button>
          </div>
          <div class="finding-quote">&bdquo;\${f.excerpt}&ldquo;</div>
          <div class="finding-why">\${f.why}</div>
          <div class="finding-sugg"><strong>Gegenvorschlag:</strong> \${f.suggestion}</div>
          <div class="redline-box">
            <strong>Redline-Vergleich:</strong><br>
            <del class="redline-del">\${f.excerpt}</del><br>
            <ins class="redline-ins">\${f.suggestion}</ins>
          </div>
        </div>
      \`).join('');
    } catch (err) {
      alert('Fehler bei der Analyse: ' + err.message);
    }
  }

  async function downloadReport(fmt) {
    const text = document.getElementById('contractInput').value;
    const profile = document.getElementById('profileSelect').value;
    const resp = await fetch('/api/v1/analyze', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-License-Key': activeLicenseKey
      },
      body: JSON.stringify({ text, profile, format: fmt })
    });
    const blob = await resp.blob();
    const url = window.URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = fmt === 'pdf' ? 'clauselens-audit.pdf' : fmt === 'sarif' ? 'clauselens-audit.sarif' : 'clauselens-audit.html';
    document.body.appendChild(a);
    a.click();
    a.remove();
  }

  function loadSampleDirty() {
    document.getElementById('contractInput').value = \`§ 12 Verlängerung
Der Vertrag verlängert sich nach Ablauf automatisch um weitere 24 Monate.

§ 8 Haftung
Die Haftung des Anbieters ist ausgeschlossen auch für grobe Fahrlässigkeit.

§ 14 Gerichtsstand
Ausschließlicher Gerichtsstand ist der Sitz des Anbieters.\`;
  }
</script>
</body>
</html>`;
}

export function startServer(port = (parseInt(process.env.PORT, 10) || 3000), rootDir = process.cwd(), host = process.env.HOST || "0.0.0.0") {
  const srv = createClauseLensServer({ rootDir });
  srv.listen(port, host, () => {
    const displayHost = host === "0.0.0.0" ? "localhost" : host;
    console.log(`ClauseLens Server läuft unter http://${displayHost}:${port}`);
    console.log(`  - Landing Page: http://${displayHost}:${port}/`);
    console.log(`  - Web Studio:   http://${displayHost}:${port}/app`);
    console.log(`  - REST API:     http://${displayHost}:${port}/api/v1/analyze`);
    console.log(`  - API Docs:     http://${displayHost}:${port}/docs`);
  });
  return srv;
}
