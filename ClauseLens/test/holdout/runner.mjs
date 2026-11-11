import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { analyzeContract } from "../../src/analyze.js";

/**
 * Held-out evaluation runner.
 *
 * Every corpus slice pairs a contract text with a label manifest:
 *   traps  [{ marker, expectedType, note }]  - planted deceptive clauses
 *   benign [{ marker, note }]                - fair clauses that must NOT fire
 *   uncovered [{ marker, note }]             - deceptive clauses no detector type describes
 *
 * A marker is a verbatim phrase from the contract. A label counts as hit when a finding
 * of exactly `expectedType` overlaps the marker's line range. Benign clauses count as a
 * false positive when any finding overlaps them.
 *
 * Usage: node test/holdout/runner.mjs [--no-write]
 */

const HERE = path.dirname(fileURLToPath(import.meta.url));
const CONTRACTS = path.join(HERE, "contracts");
const LABELS = path.join(HERE, "labels");

function lineOfIndex(text, index) {
  let line = 1;
  for (let i = 0; i < index && i < text.length; i++) if (text[i] === "\n") line++;
  return line;
}

function markerRange(text, marker) {
  const idx = text.indexOf(marker);
  if (idx === -1) return null;
  return { start: lineOfIndex(text, idx), end: lineOfIndex(text, idx + marker.length - 1) };
}

function covers(finding, range) {
  const [a, b] = finding.lineNumbers;
  return a <= range.end && b >= range.start;
}

const slices = readdirSync(CONTRACTS).filter((f) => f.endsWith(".txt")).sort();
const report = [];
const perType = new Map();
let totalTraps = 0;
let totalHits = 0;
let totalBenign = 0;
let totalFalsePositives = 0;
let totalUncovered = 0;

for (const file of slices) {
  const slug = file.replace(/\.txt$/, "");
  const text = readFileSync(path.join(CONTRACTS, file), "utf8");
  const labels = JSON.parse(readFileSync(path.join(LABELS, slug + ".json"), "utf8"));
  const res = analyzeContract(text);
  const traps = labels.traps || [];
  const benign = labels.benign || [];
  const uncovered = labels.uncovered || [];
  const notFound = [];
  const misses = [];
  const falsePositives = [];

  let hits = 0;
  for (const t of traps) {
    const range = markerRange(text, t.marker);
    if (!range) {
      notFound.push("traps: " + t.marker);
      continue;
    }
    const hit = res.findings.some((f) => f.type === t.expectedType && covers(f, range));
    const bucket = perType.get(t.expectedType) || { hit: 0, n: 0 };
    bucket.n++;
    if (hit) {
      bucket.hit++;
      hits++;
    } else {
      misses.push(`${t.expectedType} @L${range.start} :: ${t.marker.slice(0, 70)}`);
      const other = res.findings.filter((f) => covers(f, range)).map((f) => f.type);
      if (other.length) misses[misses.length - 1] += `  [stattdessen: ${[...new Set(other)].join(",")}]`;
    }
    perType.set(t.expectedType, bucket);
  }

  for (const b of benign) {
    const range = markerRange(text, b.marker);
    if (!range) {
      notFound.push("benign: " + b.marker);
      continue;
    }
    const fired = res.findings.filter((f) => covers(f, range));
    if (fired.length) falsePositives.push(`${[...new Set(fired.map((f) => f.type))].join(",")} @L${range.start} :: ${b.marker.slice(0, 60)}`);
  }

  // Ein unaufgeloester Marker messbar zu machen ist Pflicht: sonst zaehlt ein Label, das auf
  // gar keinen Text zeigt, stillschweigend als "ungedeckt" mit, ohne etwas zu messen.
  const uncoveredFired = [];
  for (const u of uncovered) {
    const range = markerRange(text, u.marker);
    if (!range) {
      notFound.push("uncovered: " + u.marker);
      continue;
    }
    if (res.findings.some((f) => covers(f, range))) uncoveredFired.push(u.marker.slice(0, 60));
  }

  totalTraps += traps.length;
  totalHits += hits;
  totalBenign += benign.length;
  totalFalsePositives += falsePositives.length;
  totalUncovered += uncovered.length;

  report.push({
    slug,
    domain: labels.domain || "",
    source: labels.source || "",
    findings: res.findings.length,
    score: res.riskScore,
    hits,
    traps: traps.length,
    benign: benign.length,
    falsePositives,
    uncovered: uncovered.length,
    uncoveredFired,
    misses,
    notFound,
  });
}

const lines = [];
lines.push("# Held-out Scorecard");
lines.push("");
lines.push("Evaluation corpus: generated contracts plus published clause wordings. A trap counts only when a finding");
lines.push("of the expected detector type overlaps its marked lines. The eight formerly uncovered clauses were used to design these new detectors, so this post-change score is an iteration result—not an independent holdout estimate; see scorecard-baseline.md for the pre-change score.");
lines.push("");
lines.push("| Korpus | Herkunft | Befunde | Score | erkannt | Trefferquote | Fehlalarme (harmlos) | ungedeckt |");
lines.push("|---|---|---|---|---|---|---|---|");
for (const r of report) {
  const q = r.traps ? Math.round((r.hits / r.traps) * 1000) / 10 : 0;
  lines.push(
    `| ${r.slug} | ${r.source} | ${r.findings} | ${r.score} | ${r.hits}/${r.traps} | ${q}% | ${r.falsePositives.length}/${r.benign} | ${r.uncovered} |`
  );
}
lines.push(
  `| **Summe** | | | | **${totalHits}/${totalTraps}** | **${Math.round((totalHits / totalTraps) * 1000) / 10}%** | **${totalFalsePositives}/${totalBenign}** | **${totalUncovered}** |`
);
lines.push("");
lines.push("## Trefferquote je Detektor (schlechteste zuerst)");
lines.push("");
lines.push("| Detektor | erkannt | Quote |");
lines.push("|---|---|---|");
for (const [type, b] of [...perType.entries()].sort((a, b) => a[1].hit / a[1].n - b[1].hit / b[1].n || b[1].n - a[1].n)) {
  lines.push(`| ${type} | ${b.hit}/${b.n} | ${Math.round((b.hit / b.n) * 100)}% |`);
}
for (const r of report) {
  lines.push("");
  lines.push(`## ${r.slug} (${r.domain})`);
  lines.push("");
  if (r.notFound.length) {
    lines.push(`- Marker nicht gefunden (Korpusfehler): ${r.notFound.length}`);
    for (const m of r.notFound.slice(0, 5)) lines.push(`  - ${m.slice(0, 120)}`);
  }
  if (r.misses.length) {
    lines.push("");
    lines.push("Nicht erkannt:");
    lines.push("");
    for (const m of r.misses) lines.push(`- ${m}`);
  }
  if (r.falsePositives.length) {
    lines.push("");
    lines.push("Fehlalarme auf harmlosen Klauseln:");
    lines.push("");
    for (const m of r.falsePositives) lines.push(`- ${m}`);
  }
  if (r.uncoveredFired.length) {
    lines.push("");
    lines.push(`Ungedeckte Klauseln, die doch feuern: ${r.uncoveredFired.length}`);
  }
}
lines.push("");

const out = path.join(HERE, "scorecard.md");
if (!process.argv.includes("--no-write")) writeFileSync(out, lines.join("\n"), "utf8");

for (const r of report) {
  console.log(r.slug.padEnd(34) + " " + r.hits + "/" + r.traps + " traps  " + r.falsePositives.length + "/" + r.benign + " FP  findings=" + r.findings + "  score=" + r.score + (r.notFound.length ? "  marker-not-found=" + r.notFound.length : ""));
}
console.log("TOTAL " + totalHits + "/" + totalTraps + " (" + (Math.round((totalHits / totalTraps) * 1000) / 10) + "%)  FP=" + totalFalsePositives + "/" + totalBenign + "  uncovered=" + totalUncovered);
console.log("scorecard: " + out);

// Ein Korpusfehler ist kein Messergebnis, sondern ein kaputtes Label: unaufgeloeste Marker
// muessen den Lauf rot machen, sonst zaehlt ein Label stillschweigend mit, das nichts trifft.
const broken = report.filter((r) => r.notFound.length > 0);
if (broken.length) {
  console.error("Korpusfehler: " + broken.reduce((n, r) => n + r.notFound.length, 0) + " Marker ohne Textstelle");
  for (const r of broken) for (const m of r.notFound) console.error("  " + r.slug + ": " + m.slice(0, 120));
  process.exitCode = 1;
}
