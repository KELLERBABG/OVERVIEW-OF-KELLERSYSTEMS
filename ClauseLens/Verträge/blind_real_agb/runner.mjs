import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { analyzeContract } from "../../src/analyze.js";

/**
 * Blind Real-AGB Evaluation Runner (Engine Frozen).
 *
 * Evaluates 25 real-world German AGB contracts whose labels were written BEFORE
 * running `analyzeContract`. Reports both:
 *  - Strict type-exact recall (finding of exact `expectedType` overlaps marker)
 *  - Clause-level risk detection recall (any finding overlaps the trap marker)
 *  - False positive rate on benign compliant control clauses
 *  - Uncovered clauses (legal defects outside the 47 detector types)
 */

const HERE = path.dirname(fileURLToPath(import.meta.url));
const CONTRACTS = path.join(HERE, "contracts");
const LABELS = path.join(HERE, "labels");

function lineOfIndex(text, index) {
  let line = 1;
  for (let i = 0; i < index && i < text.length; i++) if (text[i] === "\n") line++;
  return line;
}

function paragraphRange(text, marker) {
  const idx = text.indexOf(marker);
  if (idx === -1) return null;
  // Find paragraph boundaries (\n\n) so a finding anywhere in the same clause paragraph counts
  let pStart = text.lastIndexOf("\n\n", idx);
  pStart = pStart === -1 ? 0 : pStart + 2;
  let pEnd = text.indexOf("\n\n", idx + marker.length);
  pEnd = pEnd === -1 ? text.length - 1 : pEnd - 1;
  return {
    start: lineOfIndex(text, pStart),
    end: lineOfIndex(text, pEnd),
    markerStart: lineOfIndex(text, idx),
    paragraphText: text.slice(pStart, pEnd + 1).trim(),
  };
}

function covers(finding, range) {
  const [a, b] = finding.lineNumbers;
  return a <= range.end && b >= range.start;
}

const files = readdirSync(CONTRACTS).filter((f) => f.endsWith(".txt")).sort();
const perContract = [];
const perType = new Map();
const allMisses = [];
const allFalsePositives = [];
const allUncovered = [];
const allMatchedFindings = [];

let totalTraps = 0;
let exactHits = 0;
let anyHits = 0;
let totalBenign = 0;
let totalFP = 0;
let totalUncovered = 0;
let uncoveredHitCount = 0;
let markerErrors = 0;

for (const file of files) {
  const slug = file.replace(/\.txt$/, "");
  const text = readFileSync(path.join(CONTRACTS, file), "utf8");
  const label = JSON.parse(readFileSync(path.join(LABELS, slug + ".json"), "utf8"));
  const res = analyzeContract(text);

  const traps = label.traps || [];
  const benign = label.benign || [];
  const uncovered = label.uncovered || [];

  let cExact = 0;
  let cAny = 0;
  let cFP = 0;
  const contractMisses = [];
  const contractFPs = [];

  for (const t of traps) {
    const range = paragraphRange(text, t.marker);
    if (!range) {
      markerErrors++;
      continue;
    }
    const overlapping = res.findings.filter((f) => covers(f, range));
    const exactMatch = overlapping.find((f) => f.type === t.expectedType);
    const anyMatch = overlapping.length > 0;

    const bucket = perType.get(t.expectedType) || { n: 0, exact: 0, any: 0 };
    bucket.n++;
    if (exactMatch) {
      bucket.exact++;
      bucket.any++;
      cExact++;
      cAny++;
      allMatchedFindings.push({
        slug,
        domain: label.domain,
        expectedType: t.expectedType,
        statutoryBasisLabel: t.statutoryBasis || "",
        labelNote: t.note || "",
        finding: exactMatch,
        paragraphText: range.paragraphText,
      });
    } else if (anyMatch) {
      bucket.any++;
      cAny++;
      const firedTypes = [...new Set(overlapping.map((f) => f.type))];
      contractMisses.push({
        slug,
        domain: label.domain,
        expectedType: t.expectedType,
        firedInstead: firedTypes,
        line: range.markerStart,
        marker: t.marker,
        paragraphText: range.paragraphText,
        statutoryBasis: t.statutoryBasis || "",
        note: t.note || "",
      });
    } else {
      contractMisses.push({
        slug,
        domain: label.domain,
        expectedType: t.expectedType,
        firedInstead: [],
        line: range.markerStart,
        marker: t.marker,
        paragraphText: range.paragraphText,
        statutoryBasis: t.statutoryBasis || "",
        note: t.note || "",
      });
    }
    perType.set(t.expectedType, bucket);
  }

  for (const b of benign) {
    const range = paragraphRange(text, b.marker);
    if (!range) {
      markerErrors++;
      continue;
    }
    const fired = res.findings.filter((f) => covers(f, range));
    if (fired.length > 0) {
      cFP++;
      const fpEntry = {
        slug,
        domain: label.domain,
        line: range.markerStart,
        marker: b.marker,
        paragraphText: range.paragraphText,
        note: b.note || "",
        firedTypes: [...new Set(fired.map((f) => f.type))],
        findings: fired,
      };
      contractFPs.push(fpEntry);
      allFalsePositives.push(fpEntry);
    }
  }

  for (const u of uncovered) {
    const range = paragraphRange(text, u.marker);
    if (!range) {
      markerErrors++;
      continue;
    }
    const fired = res.findings.filter((f) => covers(f, range));
    if (fired.length > 0) uncoveredHitCount++;
    allUncovered.push({
      slug,
      domain: label.domain,
      line: range.markerStart,
      marker: u.marker,
      paragraphText: range.paragraphText,
      statutoryBasis: u.statutoryBasis || "",
      note: u.note || "",
      firedTypes: [...new Set(fired.map((f) => f.type))],
    });
  }

  totalTraps += traps.length;
  exactHits += cExact;
  anyHits += cAny;
  totalBenign += benign.length;
  totalFP += cFP;
  totalUncovered += uncovered.length;
  allMisses.push(...contractMisses);

  perContract.push({
    slug,
    domain: label.domain,
    traps: traps.length,
    exactHits: cExact,
    anyHits: cAny,
    benign: benign.length,
    falsePositives: cFP,
    uncovered: uncovered.length,
    riskScore: res.riskScore,
    totalFindings: res.findings.length,
  });
}

const exactRecallPct = totalTraps ? ((exactHits / totalTraps) * 100).toFixed(1) : "0.0";
const anyRecallPct = totalTraps ? ((anyHits / totalTraps) * 100).toFixed(1) : "0.0";
const fprPct = totalBenign ? ((totalFP / totalBenign) * 100).toFixed(1) : "0.0";

const summary = {
  evaluatedAt: new Date().toISOString().slice(0, 10),
  engineFrozen: true,
  contracts: files.length,
  totalTraps,
  exactHits,
  exactRecallPct: Number(exactRecallPct),
  anyHits,
  anyRecallPct: Number(anyRecallPct),
  strictMisses: totalTraps - exactHits,
  silentMisses: totalTraps - anyHits,
  typeCollisions: anyHits - exactHits,
  totalBenign,
  totalFalsePositives: totalFP,
  falsePositiveRatePct: Number(fprPct),
  totalUncovered,
  uncoveredFlaggedByOtherRule: uncoveredHitCount,
  markerErrors,
  perContract,
  perType: Object.fromEntries(
    [...perType.entries()].sort((a, b) => a[0].localeCompare(b[0]))
  ),
  misses: allMisses,
  falsePositives: allFalsePositives,
  uncoveredClauses: allUncovered,
  sampleMatchedFindings: allMatchedFindings.slice(0, 45),
};

writeFileSync(path.join(HERE, "scorecard.json"), JSON.stringify(summary, null, 2), "utf8");

const md = [
  "# Blind Real-AGB Benchmark Scorecard (Frozen Engine)",
  "",
  `- **Contracts evaluated:** ${summary.contracts} real-world German AGBs across 25 industry domains`,
  `- **Engine state:** Frozen (zero regex edits after label authoring)`,
  `- **Label errors:** ${summary.markerErrors}`,
  `- **Type-Exact Recall:** **${summary.exactHits} / ${summary.totalTraps} (${exactRecallPct}%)**`,
  `- **Clause-Level Risk Recall (any detector fired on trap):** **${summary.anyHits} / ${summary.totalTraps} (${anyRecallPct}%)**`,
  `- **Silent Misses (no detector fired at all):** ${summary.silentMisses} (${((summary.silentMisses / summary.totalTraps) * 100).toFixed(1)}%)`,
  `- **Type Collisions (flagged as risk under a neighbouring detector):** ${summary.typeCollisions}`,
  `- **Benign Control Clauses (False Positives):** **${summary.totalFalsePositives} / ${summary.totalBenign} (${fprPct}%)**`,
  `- **Uncovered Legal Defects (outside 47 detector types):** ${summary.totalUncovered} (${summary.uncoveredFlaggedByOtherRule} caught by adjacent rules)`,
  "",
  "## Per-Contract Breakdown",
  "",
  "| # | Contract Slug | Domain | Traps | Exact Hits | Any Hit | Benign | FP | Score |",
  "|---|---|---|---|---|---|---|---|---|",
  ...perContract.map(
    (c, i) =>
      `| ${i + 1} | \`${c.slug}\` | ${c.domain} | ${c.traps} | ${c.exactHits}/${c.traps} | ${c.anyHits}/${c.traps} | ${c.benign} | ${c.falsePositives} | ${c.riskScore} |`
  ),
  "",
  "## Silent Misses (Unflagged Traps -> Product Roadmap)",
  "",
  ...allMisses
    .filter((m) => m.firedInstead.length === 0)
    .map(
      (m, i) =>
        `${i + 1}. **\`${m.expectedType}\`** in \`${m.slug}\` (L${m.line}, ${m.statutoryBasis}): „${m.marker}“ — *${m.note}*`
    ),
  "",
  "## Type Collisions (Flagged by Adjacent Detector)",
  "",
  ...allMisses
    .filter((m) => m.firedInstead.length > 0)
    .map(
      (m, i) =>
        `${i + 1}. Expected **\`${m.expectedType}\`**, fired **\`${m.firedInstead.join(", ")}\`** in \`${m.slug}\` (L${m.line}): „${m.marker}“`
    ),
  "",
  "## False Positives on Benign Clauses",
  "",
  ...(allFalsePositives.length === 0
    ? ["None (0 false positives)."]
    : allFalsePositives.map(
        (fp, i) =>
          `${i + 1}. **\`${fp.firedTypes.join(", ")}\`** in \`${fp.slug}\` (L${fp.line}): „${fp.marker}“ — *${fp.note}*`
      )),
  "",
  "## Uncovered Clauses (Outside the 47 Detector Types -> Product Roadmap)",
  "",
  ...allUncovered.map(
    (u, i) =>
      `${i + 1}. **${u.statutoryBasis}** in \`${u.slug}\` (L${u.line}): „${u.marker}“ — *${u.note}*${
        u.firedTypes.length ? ` *(incidentally flagged by: ${u.firedTypes.join(", ")})*` : ""
      }`
  ),
  "",
].join("\n");

writeFileSync(path.join(HERE, "scorecard.md"), md, "utf8");

console.log(
  `Blind Real-AGB Benchmark: contracts=${summary.contracts} traps=${summary.totalTraps} exact=${summary.exactHits} (${exactRecallPct}%) any=${summary.anyHits} (${anyRecallPct}%) benign=${summary.totalBenign} fp=${summary.totalFalsePositives} (${fprPct}%) uncovered=${summary.totalUncovered} markerErrors=${summary.markerErrors}`
);
