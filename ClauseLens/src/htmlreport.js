"use strict";

import { writeFileSync, mkdirSync } from "node:fs";
import { dirname } from "node:path";

function escapeHtml(str) {
  if (typeof str !== "string") return "";
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}

export function generateHtmlReport(result, meta = {}) {
  const sourceName = escapeHtml(meta.sourceName || meta.source || "Unbekannter Vertrag");
  const analyzedAt = escapeHtml(meta.analyzedAt || new Date().toISOString().slice(0, 19).replace("T", " "));
  const profile = escapeHtml(meta.profile || "default (BGB/AGB Standard)");
  const score = Math.max(0, Math.min(100, Number(result.riskScore) || 0));
  const level = (result.riskLevel || "niedrig").toLowerCase();
  const counts = result.counts || { hoch: 0, mittel: 0, niedrig: 0 };
  const findings = Array.isArray(result.findings) ? result.findings : [];

  // Circumference for r=54 is 2 * PI * 54 = ~339.292
  const circ = 2 * Math.PI * 54;
  const dashoffset = score >= 100 ? 0 : (circ * (100 - score)) / 100;
  const dashArray = score >= 100 ? `${(circ + 10).toFixed(2)} 0` : score <= 0 ? `0 ${circ.toFixed(2)}` : `${circ.toFixed(2)}`;
  const strokeColor = score >= 70 ? "#b8442c" : score >= 35 ? "#b8862c" : "#3f7a4a";

  const findingsJsonSafe = JSON.stringify(findings).replace(/</g, "\\u003c");

  return `<!doctype html>
<html lang="de">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>ClauseLens Risikobericht &mdash; ${sourceName}</title>
  <meta name="description" content="Autonomer Klauselpr&uuml;fungs- und Risikobericht f&uuml;r ${sourceName}">
  <style>
    :root {
      --bg: #f8f6f0;
      --surface: #ffffff;
      --surface-raised: #f3efe6;
      --border: #dfd8c8;
      --border-dark: #b8af9c;
      --text: #1d1b18;
      --text-muted: #6e675a;
      --red: #b8442c;
      --red-bg: #fcece9;
      --amber: #b8862c;
      --amber-bg: #fdf5e6;
      --green: #3f7a4a;
      --green-bg: #eaf3ec;
      --font-mono: 'IBM Plex Mono', 'JetBrains Mono', Consolas, monospace;
      --font-sans: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      --font-serif: 'Source Serif 4', Georgia, serif;
    }

    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      background: var(--bg);
      color: var(--text);
      font-family: var(--font-sans);
      font-size: 15px;
      line-height: 1.6;
      -webkit-font-smoothing: antialiased;
      padding: clamp(16px, 3vw, 36px) clamp(12px, 2.5vw, 24px);
    }

    .container {
      max-width: 1040px;
      margin: 0 auto;
    }

    /* Top Bar */
    header {
      background: var(--surface);
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: clamp(18px, 3vw, 28px);
      margin-bottom: 24px;
      box-shadow: 0 1px 3px rgba(0,0,0,0.03);
    }
    .kicker {
      font-family: var(--font-mono);
      font-size: 11px;
      letter-spacing: 0.15em;
      text-transform: uppercase;
      color: var(--red);
      font-weight: 600;
      margin-bottom: 6px;
    }
    .header-main {
      display: flex;
      justify-content: space-between;
      align-items: flex-start;
      gap: 20px;
      flex-wrap: wrap;
    }
    h1 {
      font-family: var(--font-serif);
      font-size: clamp(1.6rem, 3.5vw, 2.3rem);
      font-weight: 600;
      line-height: 1.2;
      color: var(--text);
      margin-bottom: 8px;
    }
    .file-meta {
      font-family: var(--font-mono);
      font-size: 12px;
      color: var(--text-muted);
      display: flex;
      flex-wrap: wrap;
      gap: 16px;
      margin-top: 4px;
    }
    .file-meta span strong {
      color: var(--text);
    }

    /* Overview Grid */
    .overview-grid {
      display: grid;
      grid-template-columns: 280px minmax(0, 1fr);
      gap: 24px;
      margin-bottom: 28px;
    }
    @media (max-width: 820px) {
      .overview-grid { grid-template-columns: 1fr; }
    }

    .card {
      background: var(--surface);
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 22px;
    }

    /* Gauge Card */
    .gauge-card {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      text-align: center;
    }
    .gauge-wrapper {
      position: relative;
      width: 140px;
      height: 140px;
      margin-bottom: 12px;
    }
    .gauge-score {
      font-family: var(--font-mono);
      font-size: 2.1rem;
      font-weight: 600;
      font-variant-numeric: tabular-nums;
      fill: var(--text);
    }
    .gauge-sub {
      font-family: var(--font-mono);
      font-size: 10px;
      fill: var(--text-muted);
      letter-spacing: 0.1em;
    }
    .risk-badge-large {
      display: inline-block;
      font-family: var(--font-mono);
      font-size: 11px;
      letter-spacing: 0.16em;
      text-transform: uppercase;
      font-weight: 600;
      padding: 4px 14px;
      border-radius: 20px;
      background: ${level === "hoch" ? "var(--red-bg)" : level === "mittel" ? "var(--amber-bg)" : "var(--green-bg)"};
      color: ${strokeColor};
      border: 1px solid ${strokeColor};
    }

    /* Summary Card */
    .summary-card {
      display: flex;
      flex-direction: column;
      justify-content: space-between;
    }
    .summary-title {
      font-family: var(--font-serif);
      font-size: 1.15rem;
      font-weight: 600;
      margin-bottom: 8px;
    }
    .summary-text {
      color: var(--text);
      font-size: 14.5px;
      line-height: 1.65;
      margin-bottom: 18px;
    }
    .stats-row {
      display: grid;
      grid-template-columns: repeat(3, 1fr);
      gap: 12px;
      border-top: 1px solid var(--border);
      padding-top: 16px;
    }
    .stat-box {
      font-family: var(--font-mono);
      text-align: center;
    }
    .stat-val {
      font-size: 1.5rem;
      font-weight: 600;
      font-variant-numeric: tabular-nums;
      line-height: 1.1;
    }
    .stat-val.red { color: var(--red); }
    .stat-val.amber { color: var(--amber); }
    .stat-val.green { color: var(--green); }
    .stat-lbl {
      font-size: 10px;
      text-transform: uppercase;
      color: var(--text-muted);
      margin-top: 4px;
      letter-spacing: 0.08em;
    }

    /* Controls Bar */
    .controls-bar {
      display: flex;
      justify-content: space-between;
      align-items: center;
      gap: 16px;
      flex-wrap: wrap;
      margin-bottom: 16px;
    }
    .filter-pills {
      display: flex;
      gap: 8px;
      flex-wrap: wrap;
    }
    .filter-btn {
      background: var(--surface);
      border: 1px solid var(--border);
      padding: 6px 14px;
      border-radius: 6px;
      font-family: var(--font-mono);
      font-size: 11.5px;
      cursor: pointer;
      color: var(--text-muted);
      transition: all 0.15s ease;
    }
    .filter-btn:hover {
      border-color: var(--text-muted);
      color: var(--text);
    }
    .filter-btn.active {
      background: var(--text);
      color: #fff;
      border-color: var(--text);
    }
    .search-box {
      position: relative;
    }
    .search-input {
      padding: 6px 14px;
      border: 1px solid var(--border);
      background: var(--surface);
      border-radius: 6px;
      font-size: 12.5px;
      font-family: var(--font-sans);
      width: 220px;
      color: var(--text);
    }
    .search-input:focus {
      outline: 2px solid var(--text);
      outline-offset: 1px;
    }

    /* Findings List */
    .findings-container {
      display: flex;
      flex-direction: column;
      gap: 16px;
    }
    .finding-card {
      background: var(--surface);
      border: 1px solid var(--border);
      border-left: 4px solid var(--text-muted);
      border-radius: 8px;
      padding: 20px;
      transition: box-shadow 0.15s ease;
    }
    .finding-card:hover {
      box-shadow: 0 2px 8px rgba(0,0,0,0.04);
    }
    .finding-card.sev-hoch {
      border-left-color: var(--red);
    }
    .finding-card.sev-mittel {
      border-left-color: var(--amber);
    }
    .finding-card.sev-niedrig {
      border-left-color: var(--green);
    }

    .finding-header {
      display: flex;
      justify-content: space-between;
      align-items: flex-start;
      gap: 14px;
      flex-wrap: wrap;
      margin-bottom: 10px;
    }
    .finding-title-group {
      display: flex;
      align-items: center;
      gap: 10px;
      flex-wrap: wrap;
    }
    .badge {
      font-family: var(--font-mono);
      font-size: 10px;
      font-weight: 600;
      letter-spacing: 0.1em;
      text-transform: uppercase;
      padding: 2px 8px;
      border-radius: 4px;
    }
    .badge-hoch { background: var(--red-bg); color: var(--red); border: 1px solid var(--red); }
    .badge-mittel { background: var(--amber-bg); color: var(--amber); border: 1px solid var(--amber); }
    .badge-niedrig { background: var(--green-bg); color: var(--green); border: 1px solid var(--green); }

    .finding-title {
      font-family: var(--font-serif);
      font-size: 1.15rem;
      font-weight: 600;
      color: var(--text);
    }
    .finding-loc {
      font-family: var(--font-mono);
      font-size: 11px;
      color: var(--text-muted);
    }
    .tags-list {
      display: flex;
      gap: 6px;
      margin-bottom: 12px;
      flex-wrap: wrap;
    }
    .tag-pill {
      font-family: var(--font-mono);
      font-size: 10px;
      background: var(--surface-raised);
      color: var(--text-muted);
      padding: 2px 7px;
      border-radius: 4px;
    }

    /* Finding Sections */
    .section-label {
      font-family: var(--font-mono);
      font-size: 10px;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.1em;
      color: var(--text-muted);
      margin-top: 10px;
      margin-bottom: 4px;
    }
    .quote-box {
      background: var(--surface-raised);
      border-left: 2px solid var(--border-dark);
      padding: 10px 14px;
      font-family: var(--font-serif);
      font-size: 14px;
      line-height: 1.6;
      color: #3b362e;
      border-radius: 0 4px 4px 0;
      margin-bottom: 12px;
    }
    .why-text {
      font-size: 14px;
      line-height: 1.6;
      color: var(--text);
      margin-bottom: 12px;
    }
    .suggestion-box {
      background: var(--green-bg);
      border: 1px solid #c2dfc8;
      border-radius: 6px;
      padding: 12px 14px;
      position: relative;
    }
    .suggestion-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 4px;
    }
    .suggestion-title {
      font-family: var(--font-mono);
      font-size: 10px;
      font-weight: 600;
      letter-spacing: 0.1em;
      text-transform: uppercase;
      color: var(--green);
    }
    .copy-btn {
      background: #fff;
      border: 1px solid #bad6bf;
      padding: 3px 8px;
      border-radius: 4px;
      font-family: var(--font-mono);
      font-size: 10px;
      cursor: pointer;
      color: var(--green);
      transition: background 0.15s ease;
    }
    .copy-btn:hover {
      background: #f0f8f1;
    }
    .suggestion-text {
      font-size: 13.5px;
      color: #214427;
      line-height: 1.55;
    }

    /* Print & Actions Bar */
    .actions-bar {
      margin-top: 28px;
      display: flex;
      justify-content: flex-end;
      gap: 12px;
    }
    .action-btn {
      background: var(--surface);
      border: 1px solid var(--border);
      padding: 8px 16px;
      border-radius: 6px;
      font-family: var(--font-mono);
      font-size: 12px;
      cursor: pointer;
      color: var(--text);
      transition: all 0.15s ease;
    }
    .action-btn:hover {
      border-color: var(--text);
    }

    /* Footer */
    footer {
      margin-top: 36px;
      padding-top: 20px;
      border-top: 1px solid var(--border);
      font-family: var(--font-mono);
      font-size: 11px;
      color: var(--text-muted);
      display: flex;
      justify-content: space-between;
      flex-wrap: wrap;
      gap: 12px;
    }
    footer a { color: var(--red); text-decoration: none; }

    /* Print Styles */
    @media print {
      body { background: #fff; color: #000; padding: 0; font-size: 12pt; }
      header, .card, .finding-card { box-shadow: none; border-color: #ccc; page-break-inside: avoid; }
      .controls-bar, .actions-bar { display: none; }
      .finding-card { margin-bottom: 12pt; }
    }
  </style>
</head>
<body>
<div class="container">
  <header>
    <div class="kicker">ClauseLens &middot; Deterministic Contract Risk Audit</div>
    <div class="header-main">
      <div>
        <h1>Risikoprüfungsbericht</h1>
        <div class="file-meta">
          <span>Datei: <strong>${sourceName}</strong></span>
          <span>Datum: <strong>${analyzedAt}</strong></span>
          <span>Profil: <strong>${profile}</strong></span>
          <span>Version: <strong>1.0.0 (ESM)</strong></span>
        </div>
      </div>
    </div>
  </header>

  <div class="overview-grid">
    <div class="card gauge-card">
      <div class="gauge-wrapper">
        <svg viewBox="0 0 140 140" width="140" height="140">
          <circle cx="70" cy="70" r="54" fill="none" stroke="#e6e1d4" stroke-width="11" />
          <circle cx="70" cy="70" r="54" fill="none" stroke="${strokeColor}" stroke-width="11"
            stroke-dasharray="${dashArray}" stroke-dashoffset="${dashoffset.toFixed(2)}"
            stroke-linecap="${score >= 100 || score <= 0 ? 'butt' : 'round'}" transform="rotate(-90 70 70)" />
          <text class="gauge-score" x="70" y="67" text-anchor="middle">${score}</text>
          <text class="gauge-sub" x="70" y="86" text-anchor="middle">VON 100</text>
        </svg>
      </div>
      <div class="risk-badge-large">RISIKO: ${level.toUpperCase()}</div>
    </div>

    <div class="card summary-card">
      <div>
        <h2 class="summary-title">Zusammenfassung der Risikobewertung</h2>
        <p class="summary-text">${escapeHtml(result.summary)}</p>
      </div>
      <div class="stats-row">
        <div class="stat-box">
          <div class="stat-val red">${counts.hoch}</div>
          <div class="stat-lbl">Kritisch (Hoch)</div>
        </div>
        <div class="stat-box">
          <div class="stat-val amber">${counts.mittel}</div>
          <div class="stat-lbl">Mittel</div>
        </div>
        <div class="stat-box">
          <div class="stat-val green">${counts.niedrig}</div>
          <div class="stat-lbl">Geringfügig</div>
        </div>
      </div>
    </div>
  </div>

  <div class="controls-bar">
    <div class="filter-pills" role="tablist">
      <button class="filter-btn active" data-filter="all">Alle (${findings.length})</button>
      <button class="filter-btn" data-filter="hoch">Hoch (${counts.hoch})</button>
      <button class="filter-btn" data-filter="mittel">Mittel (${counts.mittel})</button>
      <button class="filter-btn" data-filter="niedrig">Niedrig (${counts.niedrig})</button>
    </div>
    <div class="search-box">
      <input type="text" class="search-input" id="clauseSearch" placeholder="Klauseln filtern..." aria-label="Klauseln filtern">
    </div>
  </div>

  <div class="findings-container" id="findingsList">
    ${findings.length === 0 ? `
      <div class="card" style="text-align:center; padding: 40px; color: var(--text-muted);">
        <p>Keine auffälligen Klauseln oder Risiken nach den konfigurierten Kriterien gefunden.</p>
      </div>
    ` : findings.map((f, i) => {
      const sevClass = `sev-${f.severity}`;
      const badgeClass = `badge-${f.severity}`;
      const lines = Array.isArray(f.lineNumbers) ? (f.lineNumbers[0] === f.lineNumbers[1] ? `Zeile ${f.lineNumbers[0]}` : `Zeilen ${f.lineNumbers[0]}–${f.lineNumbers[1]}`) : "";
      const tags = Array.isArray(f.tags) ? f.tags : [];
      const tagsHtml = tags.map(t => `<span class="tag-pill">${escapeHtml(t)}</span>`).join("");

      const citationHtml = f.citation ? `<span class="tag-pill" style="background:#eef4fb;border-color:#b8d0eb;color:#1e3a5f;font-weight:600;">Norm: ${escapeHtml(f.citation)}</span>` : "";

      return `
      <div class="finding-card ${sevClass}" data-severity="${f.severity}" data-index="${i}">
        <div class="finding-header">
          <div class="finding-title-group">
            <span class="badge ${badgeClass}">${f.severity.toUpperCase()}</span>
            <h3 class="finding-title">${i + 1}. ${escapeHtml(f.titlePlainGerman)}</h3>
          </div>
          <span class="finding-loc">${lines}</span>
        </div>

        <div class="tags-list">${citationHtml}${tagsHtml}</div>

        <div class="section-label">Fundstelle (Auszug aus Vertrag):</div>
        <div class="quote-box">&bdquo;${escapeHtml(f.excerpt)}&ldquo;</div>

        <div class="section-label">Warum kritisch:</div>
        <p class="why-text">${escapeHtml(f.why)}</p>

        <div class="suggestion-box">
          <div class="suggestion-header">
            <span class="suggestion-title">Gegenvorschlag zur Nachverhandlung</span>
            <button class="copy-btn" onclick="copyRedline(${i})">Kopieren</button>
          </div>
          <div class="suggestion-text" id="suggestion-${i}">${escapeHtml(f.suggestion)}</div>
        </div>
      </div>
      `;
    }).join("\n")}
  </div>

  <div class="actions-bar">
    <button class="action-btn" onclick="window.print()">Als PDF drucken / exportieren</button>
  </div>

  <footer>
    <div>ClauseLens v1.0.0 &middot; Lokale, deterministische Vertragspr&uuml;fung</div>
    <div>Keine Rechtsberatung gem&auml;&szlig; RDG. Bei rechtlichen Streitigkeiten Rechtsanwalt konsultieren.</div>
  </footer>
</div>

<script>
  const findingsData = ${findingsJsonSafe};

  // Filter Pills
  const filterBtns = document.querySelectorAll('.filter-btn');
  filterBtns.forEach(btn => {
    btn.addEventListener('click', () => {
      filterBtns.forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      applyFilters();
    });
  });

  // Search Input
  const searchInput = document.getElementById('clauseSearch');
  searchInput.addEventListener('input', () => {
    applyFilters();
  });

  function applyFilters() {
    const activeFilter = document.querySelector('.filter-btn.active').dataset.filter;
    const query = searchInput.value.toLowerCase().trim();
    const cards = document.querySelectorAll('.finding-card');

    cards.forEach(card => {
      const sev = card.dataset.severity;
      const text = card.textContent.toLowerCase();
      const matchesFilter = (activeFilter === 'all' || sev === activeFilter);
      const matchesSearch = (!query || text.includes(query));
      card.style.display = (matchesFilter && matchesSearch) ? 'block' : 'none';
    });
  }

  function copyRedline(idx) {
    if (!findingsData[idx] || !findingsData[idx].suggestion) return;
    navigator.clipboard.writeText(findingsData[idx].suggestion).then(() => {
      const btn = event.target;
      const prev = btn.textContent;
      btn.textContent = 'Kopiert!';
      setTimeout(() => { btn.textContent = prev; }, 1800);
    });
  }
</script>
</body>
</html>`;
}

export function writeHtmlReport(htmlContent, outPath) {
  mkdirSync(dirname(outPath), { recursive: true });
  writeFileSync(outPath, htmlContent, "utf-8");
}
