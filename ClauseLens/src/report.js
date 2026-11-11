"use strict";

import { writeFileSync, mkdirSync } from "node:fs";
import { dirname } from "node:path";

const BADGE = { hoch: "HOCH", mittel: "MITTEL", niedrig: "NIEDRIG" };

function wrapBlockquote(text, width = 96) {
  const words = text.split(/\s+/);
  const rows = [];
  for (const word of words) {
    const last = rows.length ? rows[rows.length - 1] : null;
    if (last && last.length + 1 + word.length <= width) {
      rows[rows.length - 1] = `${last} ${word}`;
    } else {
      rows.push(word);
    }
  }
  return rows.map((row) => `> ${row}`);
}

export function renderMarkdownReport(result, meta = {}) {
  const lines = [];
  lines.push("# ClauseLens Risikobericht");
  lines.push("");
  if (meta.sourceName) lines.push(`**Datei:** ${meta.sourceName}`);
  if (meta.analyzedAt) lines.push(`**Datum:** ${meta.analyzedAt}`);
  lines.push(`**Risiko-Score:** ${result.riskScore}/100 (${result.riskLevel})`);
  lines.push("");
  lines.push("## Zusammenfassung");
  lines.push("");
  lines.push(result.summary);
  lines.push("");
  lines.push("## Befunde");
  lines.push("");

  if (result.findings.length === 0) {
    lines.push("Keine auff\u00e4lligen Klauseln gefunden.");
    lines.push("");
  }

  result.findings.forEach((finding, idx) => {
    const [from, to] = finding.lineNumbers;
    lines.push(
      `### ${idx + 1}. ${finding.titlePlainGerman} \u2014 Schwere: ${BADGE[finding.severity]}`
    );
    lines.push("");
    lines.push(`**Klauseltyp:** \`${finding.type}\``);
    lines.push("");
    lines.push(`**Zeilen:** ${from}\u2013${to}`);
    lines.push("");
    lines.push("**Warum kritisch:**");
    lines.push("");
    lines.push(finding.why);
    lines.push("");
    lines.push("**Fundstelle (Auszug):**");
    lines.push("");
    lines.push(...wrapBlockquote(finding.excerpt));
    lines.push("");
    lines.push("**Empfehlung:**");
    lines.push("");
    lines.push(finding.suggestion);
    lines.push("");
  });

  lines.push("---");
  lines.push("");
  lines.push(
    "*Dieser Bericht wurde automatisch erstellt und stellt keine Rechtsberatung dar. Im Zweifel eine Rechtsanwältin oder einen Rechtsanwalt fragen.*"
  );
  lines.push("");
  return lines.join("\n");
}

export function dumpJson(obj, outPath) {
  mkdirSync(dirname(outPath), { recursive: true });
  writeFileSync(outPath, `${JSON.stringify(obj, null, 2)}\n`, "utf-8");
}

export function writeReport(mdText, outPath) {
  mkdirSync(dirname(outPath), { recursive: true });
  writeFileSync(outPath, mdText, "utf-8");
}
