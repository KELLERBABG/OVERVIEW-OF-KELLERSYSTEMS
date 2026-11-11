"use strict";

import path, { dirname } from "node:path";
import { writeFileSync, mkdirSync } from "node:fs";
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

export function compareContracts(file1Path, file2Path, options = {}) {
  const config = options.config || loadConfig(options.configPath);

  const doc1 = extractText(file1Path, options.format || "auto");
  const doc2 = extractText(file2Path, options.format || "auto");

  const res1 = analyzeContract(doc1.text, { config, profile: options.profile });
  const res2 = analyzeContract(doc2.text, { config, profile: options.profile });

  const f1ByType = new Map();
  for (const f of res1.findings) {
    if (!f1ByType.has(f.type)) f1ByType.set(f.type, []);
    f1ByType.get(f.type).push(f);
  }

  const f2ByType = new Map();
  for (const f of res2.findings) {
    if (!f2ByType.has(f.type)) f2ByType.set(f.type, []);
    f2ByType.get(f.type).push(f);
  }

  const resolved = [];
  const introduced = [];
  const unchanged = [];
  const modified = [];

  // Check items in v1
  for (const [type, list1] of f1ByType.entries()) {
    const list2 = f2ByType.get(type) || [];
    if (list2.length === 0) {
      resolved.push(...list1);
    } else {
      const minLen = Math.min(list1.length, list2.length);
      for (let i = 0; i < minLen; i++) {
        const item1 = list1[i];
        const item2 = list2[i];
        if (item1.severity === item2.severity) {
          unchanged.push({ v1: item1, v2: item2 });
        } else {
          modified.push({ v1: item1, v2: item2 });
        }
      }
      if (list1.length > list2.length) {
        resolved.push(...list1.slice(minLen));
      }
    }
  }

  // Check items in v2 not in v1
  for (const [type, list2] of f2ByType.entries()) {
    const list1 = f1ByType.get(type) || [];
    if (list1.length === 0) {
      introduced.push(...list2);
    } else if (list2.length > list1.length) {
      introduced.push(...list2.slice(list1.length));
    }
  }

  const deltaScore = res2.riskScore - res1.riskScore;
  const scoreImprovement = deltaScore < 0;

  return {
    schemaVersion: "1.0.0",
    v1: {
      file: path.basename(file1Path),
      riskScore: res1.riskScore,
      riskLevel: res1.riskLevel,
      counts: res1.counts,
      findingsCount: res1.findings.length,
    },
    v2: {
      file: path.basename(file2Path),
      riskScore: res2.riskScore,
      riskLevel: res2.riskLevel,
      counts: res2.counts,
      findingsCount: res2.findings.length,
    },
    delta: {
      deltaScore,
      scoreImprovement,
      resolvedCount: resolved.length,
      introducedCount: introduced.length,
      modifiedCount: modified.length,
      unchangedCount: unchanged.length,
    },
    changes: {
      resolved,
      introduced,
      modified,
      unchanged,
    },
  };
}

export function renderCompareMarkdown(diff) {
  const lines = [];
  lines.push("# ClauseLens Versionsvergleich & Risikodelta");
  lines.push("");
  lines.push(`**Ursprüngliche Fassung (V1):** ${diff.v1.file} (${diff.v1.riskScore}/100, ${diff.v1.riskLevel})`);
  lines.push(`**Neue Fassung (V2):** ${diff.v2.file} (${diff.v2.riskScore}/100, ${diff.v2.riskLevel})`);
  lines.push("");
  const deltaSymbol = diff.delta.deltaScore > 0 ? `+${diff.delta.deltaScore}` : `${diff.delta.deltaScore}`;
  lines.push(
    `**Risikoscore-Delta:** ${deltaSymbol} Punkte (${diff.delta.scoreImprovement ? "Verbesserung" : diff.delta.deltaScore === 0 ? "Unverändert" : "Verschlechterung"})`
  );
  lines.push("");
  lines.push(
    `**Statistik:** ${diff.delta.resolvedCount} behoben, ${diff.delta.introducedCount} neu hinzugekommen, ${diff.delta.modifiedCount} verändert, ${diff.delta.unchangedCount} unverändert.`
  );
  lines.push("");

  if (diff.changes.resolved.length > 0) {
    lines.push("## Erfreulich: Behobene / entschärfte Risiken");
    for (const r of diff.changes.resolved) {
      lines.push(`- [BEHOBEN] **${r.titlePlainGerman}** (${r.severity}): Zeilen ${r.lineNumbers.join("-")} in V1`);
    }
    lines.push("");
  }

  if (diff.changes.introduced.length > 0) {
    lines.push("## Achtung: Neu hinzugekommene Risiken");
    for (const intro of diff.changes.introduced) {
      lines.push(
        `- [NEU!] **${intro.titlePlainGerman}** (${intro.severity}): Zeilen ${intro.lineNumbers.join("-")} in V2`
      );
      lines.push(`  > ${intro.excerpt}`);
    }
    lines.push("");
  }

  if (diff.changes.modified.length > 0) {
    lines.push("## Geänderte Risikobewertungen");
    for (const mod of diff.changes.modified) {
      lines.push(
        `- **${mod.v1.titlePlainGerman}**: Schwere geändert von ${mod.v1.severity} auf ${mod.v2.severity}`
      );
    }
    lines.push("");
  }

  lines.push("---");
  lines.push("*Automatisierter Risikovergleich von ClauseLens. Keine Rechtsberatung.*");
  return lines.join("\n");
}

export function generateCompareHtml(diff) {
  const deltaSymbol = diff.delta.deltaScore > 0 ? `+${diff.delta.deltaScore}` : `${diff.delta.deltaScore}`;
  const deltaClass = diff.delta.scoreImprovement ? "green" : diff.delta.deltaScore === 0 ? "neutral" : "red";

  return `<!doctype html>
<html lang="de">
<head>
  <meta charset="utf-8">
  <title>ClauseLens &mdash; Risikovergleich</title>
  <style>
    :root {
      --bg: #f8f6f0; --surface: #ffffff; --border: #dfd8c8;
      --text: #1d1b18; --text-muted: #6e675a;
      --red: #b8442c; --green: #3f7a4a; --amber: #b8862c;
      --font-mono: 'IBM Plex Mono', monospace;
      --font-serif: 'Source Serif 4', Georgia, serif;
      --font-sans: 'Inter', system-ui, sans-serif;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body { background: var(--bg); color: var(--text); font-family: var(--font-sans); padding: 32px 20px; font-size: 15px; }
    .container { max-width: 980px; margin: 0 auto; }
    header { background: var(--surface); border: 1px solid var(--border); border-radius: 8px; padding: 24px; margin-bottom: 24px; }
    .kicker { font-family: var(--font-mono); font-size: 11px; text-transform: uppercase; color: var(--red); font-weight: 600; letter-spacing: 0.15em; margin-bottom: 6px; }
    h1 { font-family: var(--font-serif); font-size: 2.1rem; margin-bottom: 8px; }
    .delta-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 16px; margin-bottom: 24px; }
    .card { background: var(--surface); border: 1px solid var(--border); border-radius: 8px; padding: 20px; text-align: center; }
    .card-val { font-family: var(--font-mono); font-size: 2.2rem; font-weight: 600; }
    .card-val.red { color: var(--red); }
    .card-val.green { color: var(--green); }
    .card-lbl { font-size: 11px; text-transform: uppercase; color: var(--text-muted); font-family: var(--font-mono); margin-top: 4px; }
    .section-card { background: var(--surface); border: 1px solid var(--border); border-radius: 8px; padding: 20px; margin-bottom: 20px; }
    h2 { font-family: var(--font-serif); font-size: 1.25rem; margin-bottom: 14px; }
    .item-resolved { border-left: 3px solid var(--green); padding: 8px 12px; margin-bottom: 8px; background: #eaf3ec; border-radius: 0 4px 4px 0; }
    .item-introduced { border-left: 3px solid var(--red); padding: 8px 12px; margin-bottom: 8px; background: #fcece9; border-radius: 0 4px 4px 0; }
    footer { border-top: 1px solid var(--border); padding-top: 18px; font-family: var(--font-mono); font-size: 11px; color: var(--text-muted); }
  </style>
</head>
<body>
<div class="container">
  <header>
    <div class="kicker">ClauseLens &middot; Vertragsvergleich</div>
    <h1>Risikodelta nach Nachverhandlung</h1>
    <p>Vergleich zwischen <strong>${escapeHtml(diff.v1.file)}</strong> und <strong>${escapeHtml(diff.v2.file)}</strong></p>
  </header>

  <div class="delta-grid">
    <div class="card">
      <div class="card-val">${diff.v1.riskScore} &rarr; ${diff.v2.riskScore}</div>
      <div class="card-lbl">Score V1 &rarr; V2</div>
    </div>
    <div class="card">
      <div class="card-val ${deltaClass}">${deltaSymbol}</div>
      <div class="card-lbl">Risikoscore-Delta</div>
    </div>
    <div class="card">
      <div class="card-val green">${diff.delta.resolvedCount}</div>
      <div class="card-lbl">Behobene Risiken</div>
    </div>
  </div>

  ${diff.changes.resolved.length > 0 ? `
    <div class="section-card">
      <h2 style="color: var(--green);">&check; Erfolgreich behobene Risiken</h2>
      ${diff.changes.resolved.map(r => `
        <div class="item-resolved">
          <strong>${escapeHtml(r.titlePlainGerman)}</strong> (${r.severity.toUpperCase()}) &mdash; in neuer Fassung gestrichen / entschärft.
        </div>
      `).join("")}
    </div>
  ` : ""}

  ${diff.changes.introduced.length > 0 ? `
    <div class="section-card">
      <h2 style="color: var(--red);">&excl; Neu eingeführte Risiken</h2>
      ${diff.changes.introduced.map(r => `
        <div class="item-introduced">
          <strong>${escapeHtml(r.titlePlainGerman)}</strong> (${r.severity.toUpperCase()})
          <div style="font-family: var(--font-serif); font-size: 13.5px; margin-top: 4px;">&bdquo;${escapeHtml(r.excerpt)}&ldquo;</div>
        </div>
      `).join("")}
    </div>
  ` : ""}

  <footer>
    ClauseLens v1.0.0 &middot; Automatisierter Vertragsvergleich &middot; Keine Rechtsberatung
  </footer>
</div>
</body>
</html>`;
}

export function writeCompareReport(content, outPath) {
  mkdirSync(dirname(outPath), { recursive: true });
  writeFileSync(outPath, content, "utf-8");
}
