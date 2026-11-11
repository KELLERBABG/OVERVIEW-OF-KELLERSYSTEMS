"use strict";

import { readdirSync, statSync, writeFileSync, mkdirSync } from "node:fs";
import path, { dirname } from "node:path";
import { extractText } from "./textio.js";
import { analyzeContract } from "./analyze.js";
import { loadConfig } from "./config.js";

function escapeHtml(str) {
  if (typeof str !== "string") return "";
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}

export function collectFiles(pathsOrDir) {
  const targets = Array.isArray(pathsOrDir) ? pathsOrDir : [pathsOrDir];
  const files = [];

  for (const target of targets) {
    if (!target) continue;
    try {
      const st = statSync(target);
      if (st.isDirectory()) {
        const entries = readdirSync(target);
        for (const entry of entries) {
          const fullPath = path.join(target, entry);
          const ext = path.extname(entry).toLowerCase();
          if ([".txt", ".pdf", ".docx"].includes(ext)) {
            files.push(fullPath);
          }
        }
      } else if (st.isFile()) {
        files.push(target);
      }
    } catch {
      // Ignored for nonexistent files during traversal
    }
  }

  return files;
}

/**
 * Runs batch analysis across multiple contracts.
 */
export function analyzeBatch(filePaths, options = {}) {
  const config = options.config || loadConfig(options.configPath);
  const files = collectFiles(filePaths);

  if (files.length === 0) {
    throw new Error("Keine analysierbaren Vertragsdateien (.txt, .pdf, .docx) gefunden.");
  }

  const docResults = [];
  const riskCountsByType = new Map();

  for (const file of files) {
    const baseName = path.basename(file);
    let extracted;
    try {
      extracted = extractText(file, options.format || "auto");
    } catch (err) {
      docResults.push({
        file: baseName,
        filePath: file,
        error: err.message,
        riskScore: 0,
        riskLevel: "fehler",
        counts: { hoch: 0, mittel: 0, niedrig: 0 },
        findingsCount: 0,
        findings: [],
      });
      continue;
    }

    const res = analyzeContract(extracted.text, { config, profile: options.profile });
    docResults.push({
      file: baseName,
      filePath: file,
      format: extracted.format,
      riskScore: res.riskScore,
      riskLevel: res.riskLevel,
      counts: res.counts,
      findingsCount: res.findings.length,
      findings: res.findings,
      summary: res.summary,
    });

    for (const f of res.findings) {
      if (!riskCountsByType.has(f.type)) {
        riskCountsByType.set(f.type, {
          type: f.type,
          title: f.titlePlainGerman,
          severity: f.severity,
          count: 0,
          affectedDocuments: new Set(),
        });
      }
      const entry = riskCountsByType.get(f.type);
      entry.count++;
      entry.affectedDocuments.add(baseName);
    }
  }

  const validDocs = docResults.filter((d) => !d.error);
  const totalDocs = validDocs.length;
  const highRiskDocs = validDocs.filter((d) => d.riskLevel === "hoch").length;
  const mediumRiskDocs = validDocs.filter((d) => d.riskLevel === "mittel").length;
  const lowRiskDocs = validDocs.filter((d) => d.riskLevel === "niedrig").length;
  const avgScore =
    totalDocs > 0
      ? Math.round((validDocs.reduce((acc, d) => acc + d.riskScore, 0) / totalDocs) * 10) / 10
      : 0;
  const totalFindings = validDocs.reduce((acc, d) => acc + d.findingsCount, 0);

  const topRisks = Array.from(riskCountsByType.values())
    .map((r) => ({
      type: r.type,
      title: r.title,
      severity: r.severity,
      occurrences: r.count,
      affectedDocuments: Array.from(r.affectedDocuments),
    }))
    .sort((a, b) => b.occurrences - a.occurrences);

  // Sort documents by highest risk score first
  docResults.sort((a, b) => b.riskScore - a.riskScore);

  const summary = `Portfolio-Analyse über ${totalDocs} Verträge: ${highRiskDocs} Verträge mit hohem Risiko, ${mediumRiskDocs} mit mittlerem Risiko, ${lowRiskDocs} unkritisch. Durchschnittlicher Portfolio-Risikoscore: ${avgScore}/100.`;

  return {
    schemaVersion: "1.0.0",
    analyzedAt: new Date().toISOString(),
    summary,
    portfolioStats: {
      totalDocuments: totalDocs,
      highRiskDocuments: highRiskDocs,
      mediumRiskDocuments: mediumRiskDocs,
      lowRiskDocuments: lowRiskDocs,
      averageScore: avgScore,
      totalFindings,
    },
    documents: docResults,
    topRisksAcrossPortfolio: topRisks,
  };
}

/**
 * Renders batch portfolio audit report as Markdown.
 */
export function renderBatchMarkdown(batchResult) {
  const stats = batchResult.portfolioStats;
  const lines = [];

  lines.push("# ClauseLens Portfolio-Risikobericht");
  lines.push("");
  lines.push(`**Analysierte Verträge:** ${stats.totalDocuments}`);
  lines.push(`**Durchschnittlicher Risikoscore:** ${stats.averageScore}/100`);
  lines.push(`**Datum:** ${batchResult.analyzedAt}`);
  lines.push("");
  lines.push("## Portfolio-Zusammenfassung");
  lines.push("");
  lines.push(batchResult.summary);
  lines.push("");
  lines.push("## Vertragsübersicht (nach Risiko sortiert)");
  lines.push("");
  lines.push("| Vertrag | Format | Risikoscore | Einstufung | Befunde (H/M/N) |");
  lines.push("|---|---|---|---|---|");

  for (const doc of batchResult.documents) {
    if (doc.error) {
      lines.push(`| ${doc.file} | - | FEHLER | nicht lesbar | ${doc.error} |`);
    } else {
      const countsStr = `${doc.counts.hoch}/${doc.counts.mittel}/${doc.counts.niedrig}`;
      lines.push(
        `| ${doc.file} | ${doc.format} | **${doc.riskScore}/100** | ${doc.riskLevel.toUpperCase()} | ${countsStr} |`
      );
    }
  }

  lines.push("");
  lines.push("## Häufigste Risikoklauseln im Gesamtbestand");
  lines.push("");
  for (const risk of batchResult.topRisksAcrossPortfolio) {
    lines.push(
      `- **${risk.title}** (${risk.severity.toUpperCase()}): ${risk.occurrences}x in ${risk.affectedDocuments.length} Verträgen (${risk.affectedDocuments.join(", ")})`
    );
  }

  lines.push("");
  lines.push("---");
  lines.push("*Automatisierter Portfoliobericht von ClauseLens. Keine Rechtsberatung.*");

  return lines.join("\n");
}

/**
 * Generates interactive self-contained HTML portfolio dashboard.
 */
export function generateBatchHtml(batchResult) {
  const stats = batchResult.portfolioStats;
  const docs = batchResult.documents;
  const topRisks = batchResult.topRisksAcrossPortfolio;

  return `<!doctype html>
<html lang="de">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>ClauseLens &mdash; Portfolio Risikobericht</title>
  <style>
    :root {
      --bg: #f8f6f0;
      --surface: #ffffff;
      --border: #dfd8c8;
      --text: #1d1b18;
      --text-muted: #6e675a;
      --red: #b8442c;
      --red-bg: #fcece9;
      --amber: #b8862c;
      --amber-bg: #fdf5e6;
      --green: #3f7a4a;
      --green-bg: #eaf3ec;
      --font-mono: 'IBM Plex Mono', 'JetBrains Mono', Consolas, monospace;
      --font-sans: 'Inter', -apple-system, sans-serif;
      --font-serif: 'Source Serif 4', Georgia, serif;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body { background: var(--bg); color: var(--text); font-family: var(--font-sans); padding: 32px 20px; font-size: 15px; }
    .container { max-width: 1040px; margin: 0 auto; }
    header { background: var(--surface); border: 1px solid var(--border); border-radius: 8px; padding: 24px; margin-bottom: 24px; }
    .kicker { font-family: var(--font-mono); font-size: 11px; text-transform: uppercase; color: var(--red); font-weight: 600; letter-spacing: 0.15em; margin-bottom: 6px; }
    h1 { font-family: var(--font-serif); font-size: 2.1rem; font-weight: 600; margin-bottom: 6px; }
    .subtitle { color: var(--text-muted); font-size: 14px; }
    .stats-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 16px; margin-bottom: 24px; }
    @media (max-width: 768px) { .stats-grid { grid-template-columns: repeat(2, 1fr); } }
    .stat-card { background: var(--surface); border: 1px solid var(--border); border-radius: 8px; padding: 18px; text-align: center; }
    .stat-num { font-family: var(--font-mono); font-size: 2rem; font-weight: 600; font-variant-numeric: tabular-nums; }
    .stat-num.red { color: var(--red); }
    .stat-num.amber { color: var(--amber); }
    .stat-num.green { color: var(--green); }
    .stat-desc { font-size: 11px; text-transform: uppercase; color: var(--text-muted); font-family: var(--font-mono); margin-top: 4px; }
    .table-card { background: var(--surface); border: 1px solid var(--border); border-radius: 8px; padding: 20px; margin-bottom: 24px; overflow-x: auto; }
    h2 { font-family: var(--font-serif); font-size: 1.25rem; font-weight: 600; margin-bottom: 14px; }
    table { width: 100%; border-collapse: collapse; font-size: 14px; text-align: left; }
    th { font-family: var(--font-mono); font-size: 10.5px; text-transform: uppercase; color: var(--text-muted); padding: 10px 12px; border-bottom: 2px solid var(--border); }
    td { padding: 12px; border-bottom: 1px solid var(--border); vertical-align: middle; }
    tr:hover td { background: #faf9f6; }
    .score-badge { display: inline-block; font-family: var(--font-mono); font-size: 11.5px; font-weight: 600; padding: 2px 8px; border-radius: 4px; }
    .score-badge.hoch { background: var(--red-bg); color: var(--red); border: 1px solid var(--red); }
    .score-badge.mittel { background: var(--amber-bg); color: var(--amber); border: 1px solid var(--amber); }
    .score-badge.niedrig { background: var(--green-bg); color: var(--green); border: 1px solid var(--green); }
    footer { border-top: 1px solid var(--border); padding-top: 18px; font-family: var(--font-mono); font-size: 11px; color: var(--text-muted); display: flex; justify-content: space-between; }
  </style>
</head>
<body>
<div class="container">
  <header>
    <div class="kicker">Portfolio-Audit &middot; ClauseLens</div>
    <h1>Vertragsbestand Risikobericht</h1>
    <p class="subtitle">${escapeHtml(batchResult.summary)}</p>
  </header>

  <div class="stats-grid">
    <div class="stat-card">
      <div class="stat-num">${stats.totalDocuments}</div>
      <div class="stat-desc">Verträge gesamt</div>
    </div>
    <div class="stat-card">
      <div class="stat-num red">${stats.highRiskDocuments}</div>
      <div class="stat-desc">Hohes Risiko (&ge; 70)</div>
    </div>
    <div class="stat-card">
      <div class="stat-num amber">${stats.mediumRiskDocuments}</div>
      <div class="stat-desc">Mittleres Risiko</div>
    </div>
    <div class="stat-card">
      <div class="stat-num">${stats.averageScore}</div>
      <div class="stat-desc">&empty; Portfolio-Score</div>
    </div>
  </div>

  <div class="table-card">
    <h2>Vertragsübersicht</h2>
    <table>
      <thead>
        <tr>
          <th>Vertragsdokument</th>
          <th>Format</th>
          <th>Risiko-Score</th>
          <th>Einstufung</th>
          <th>Kritische Klauseln (H/M/N)</th>
        </tr>
      </thead>
      <tbody>
        ${docs.map(d => `
          <tr>
            <td><strong>${escapeHtml(d.file)}</strong></td>
            <td><code>${escapeHtml(d.format || '-')}</code></td>
            <td><span class="score-badge ${d.riskLevel}">${d.riskScore}/100</span></td>
            <td><strong>${escapeHtml(d.riskLevel.toUpperCase())}</strong></td>
            <td>${d.counts ? `${d.counts.hoch} / ${d.counts.mittel} / ${d.counts.niedrig}` : '-'}</td>
          </tr>
        `).join("")}
      </tbody>
    </table>
  </div>

  <div class="table-card">
    <h2>Häufigste Risikomuster im Bestand</h2>
    <table>
      <thead>
        <tr>
          <th>Klauseltyp</th>
          <th>Schwere</th>
          <th>Vorkommen</th>
          <th>Betroffene Verträge</th>
        </tr>
      </thead>
      <tbody>
        ${topRisks.map(r => `
          <tr>
            <td><strong>${escapeHtml(r.title)}</strong></td>
            <td><span class="score-badge ${r.severity}">${r.severity.toUpperCase()}</span></td>
            <td><strong>${r.occurrences}x</strong></td>
            <td>${escapeHtml(r.affectedDocuments.join(", "))}</td>
          </tr>
        `).join("")}
      </tbody>
    </table>
  </div>

  <footer>
    <div>ClauseLens v1.0.0 &middot; Automatisierte Portfolioanalyse</div>
    <div>Keine Rechtsberatung gem&auml;&szlig; RDG.</div>
  </footer>
</div>
</body>
</html>`;
}

export function writeBatchReport(content, outPath) {
  mkdirSync(dirname(outPath), { recursive: true });
  writeFileSync(outPath, content, "utf-8");
}
