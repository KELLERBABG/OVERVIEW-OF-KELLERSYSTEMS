import { readFileSync, readdirSync, existsSync, mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { analyzeContract, RULES } from "../../src/analyze.js";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, "..");
const REPORTS = path.join(ROOT, "reports");
const KNOWN = new Set(RULES.map((r) => r.type));

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

const slices = [];
for (const rootName of ["corpus", "public"]) {
  const root = path.join(ROOT, rootName);
  if (!existsSync(root)) continue;
  for (const file of readdirSync(root).filter((f) => f.endsWith(".txt")).sort()) {
    const slug = file.replace(/\.txt$/, "");
    const labelPath = path.join(root, "labels", slug + ".json");
    if (!existsSync(labelPath)) continue;
    const text = readFileSync(path.join(root, file), "utf8");
    const labels = JSON.parse(readFileSync(labelPath, "utf8"));
    slices.push({ slug, root: rootName, domain: labels.domain || "", source: labels.source || "", text, labels });
  }
}

const perType = new Map();
const fpByType = new Map();
const rows = [];
const labelIssues = [];
const missList = [];
const fpList = [];
const uncoveredMissed = [];
const uncoveredFired = [];
let totalTraps = 0, totalHits = 0, totalBenign = 0, totalFalsePositives = 0;
let totalUncovered = 0, totalUncoveredMissed = 0, totalUncoveredFired = 0;

for (const s of slices) {
  const res = analyzeContract(s.text);
  const traps = s.labels.traps || [];
  const benign = s.labels.benign || [];
  const uncovered = s.labels.uncovered || [];
  let hits = 0;
  const localMiss = [];
  const localFp = [];

  for (const t of traps) {
    if (!KNOWN.has(t.expectedType)) { labelIssues.push(`${s.slug}: unknown expectedType ${t.expectedType}`); continue; }
    const range = markerRange(s.text, t.marker);
    if (!range) { labelIssues.push(`${s.slug}: trap marker unresolved: ${t.marker.slice(0, 50)}`); continue; }
    const hit = res.findings.some((f) => f.type === t.expectedType && covers(f, range));
    const bucket = perType.get(t.expectedType) || { hit: 0, n: 0 };
    bucket.n++;
    if (hit) { bucket.hit++; hits++; }
    else {
      const other = [...new Set(res.findings.filter((f) => covers(f, range)).map((f) => f.type))];
      localMiss.push(`${t.expectedType} @${s.slug}:L${range.start} [${other.length ? "stattdessen " + other.join(",") : "stumm"}] :: ${t.marker.slice(0, 70)}`);
    }
    perType.set(t.expectedType, bucket);
  }

  for (const b of benign) {
    const range = markerRange(s.text, b.marker);
    if (!range) { labelIssues.push(`${s.slug}: benign marker unresolved: ${b.marker.slice(0, 50)}`); continue; }
    const fired = res.findings.filter((f) => covers(f, range));
    if (fired.length) {
      const types = [...new Set(fired.map((f) => f.type))];
      totalFalsePositives++;
      localFp.push(`${types.join(",")} @${s.slug}:L${range.start} :: ${b.marker.slice(0, 60)}`);
      for (const ty of types) fpByType.set(ty, (fpByType.get(ty) || 0) + 1);
    }
  }

  for (const u of uncovered) {
    const range = markerRange(s.text, u.marker);
    if (!range) { labelIssues.push(`${s.slug}: uncovered marker unresolved: ${u.marker.slice(0, 50)}`); continue; }
    const fired = res.findings.filter((f) => covers(f, range));
    totalUncovered++;
    if (fired.length) {
      totalUncoveredFired++;
      uncoveredFired.push(`${[...new Set(fired.map((f) => f.type))].join(",")} @${s.slug}:L${range.start} :: ${u.marker.slice(0, 60)}`);
    } else {
      totalUncoveredMissed++;
      uncoveredMissed.push(`${s.slug}:L${range.start} :: ${u.marker.slice(0, 70)}`);
    }
  }

  totalTraps += traps.length;
  totalHits += hits;
  totalBenign += benign.length;
  missList.push(...localMiss);
  fpList.push(...localFp);
  rows.push({ slug: s.slug, root: s.root, domain: s.domain, findings: res.findings.length, score: res.riskScore, hits, traps: traps.length, benign: benign.length, falsePositives: localFp.length, uncovered: uncovered.length });
}

const pct = (a, b) => (b ? Math.round((a / b) * 1000) / 10 : 0);
const L = [];
L.push("# ClauseLens Red-Team Scorecard");
L.push("");
L.push(`Corpus: ${rows.length} contracts, ${totalTraps} planted traps, ${totalBenign} benign controls, ${totalUncovered} uncovered clauses.`);
L.push(`A trap is a hit only when a finding of exactly the expected detector type overlaps the marked lines. Higher recall and lower false positive count are better.`);
L.push("");
L.push(`| Kennzahl | Wert |`);
L.push(`|---|---|`);
L.push(`| Verträge | ${rows.length} |`);
L.push(`| Fallen erkannt | ${totalHits}/${totalTraps} (${pct(totalHits, totalTraps)} %) |`);
L.push(`| Fehlalarme auf harmlosen Klauseln | ${totalFalsePositives}/${totalBenign} (${pct(totalFalsePositives, totalBenign)} %) |`);
L.push(`| Ungedeckte Fallen | ${totalUncoveredMissed} still, ${totalUncoveredFired} von einem anderen Detektor erfasst |`);
L.push(`| Label-Fehler | ${labelIssues.length} |`);
L.push("");
L.push("## Trefferquote je Detektor (schlechteste zuerst)");
L.push("");
L.push("| Detektor | erkannt | Quote | Fehlalarme |");
L.push("|---|---|---|---|");
const typeRows = [...perType.entries()].sort((a, b) => a[1].hit / a[1].n - b[1].hit / b[1].n || b[1].n - a[1].n);
for (const [ty, b] of typeRows) L.push(`| ${ty} | ${b.hit}/${b.n} | ${pct(b.hit, b.n)} % | ${fpByType.get(ty) || 0} |`);
L.push("");
L.push("## Nicht abgedeckte Detektortypen in diesem Korpus");
L.push("");
const missing = [...KNOWN].filter((ty) => !perType.has(ty));
L.push(missing.length ? missing.join(", ") : "keine, alle 47 Typen sind belegt");
L.push("");
L.push("## Fehlalarme (Detektor feuerte auf harmloser Klausel)");
L.push("");
if (fpList.length) { L.push("~~~text"); for (const f of fpList.slice(0, 400)) L.push(f); L.push("~~~"); }
else L.push("keine");
L.push("");
L.push("## Verfehlte Fallen (nicht erkannt)");
L.push("");
if (missList.length) { L.push("~~~text"); for (const m of missList.slice(0, 600)) L.push(m); L.push("~~~"); }
else L.push("keine");
L.push("");
L.push("## Ungedeckte Klauseln ohne Detektor");
L.push("");
if (uncoveredMissed.length) { L.push("### still geblieben (Gap)"); L.push("~~~text"); for (const u of uncoveredMissed.slice(0, 400)) L.push(u); L.push("~~~"); }
if (uncoveredFired.length) { L.push("### von einem Detektor erfasst (Overreach-Kandidat)"); L.push("~~~text"); for (const u of uncoveredFired.slice(0, 400)) L.push(u); L.push("~~~"); }
L.push("");
L.push("## Korpus je Vertrag");
L.push("");
L.push("| Vertrag | Wurzel | Domäne | Befunde | Score | erkannt | Fehlalarme | ungedeckt |");
L.push("|---|---|---|---|---|---|---|---|");
for (const r of rows) L.push(`| ${r.slug} | ${r.root} | ${r.domain} | ${r.findings} | ${r.score} | ${r.hits}/${r.traps} | ${r.falsePositives} | ${r.uncovered} |`);
if (labelIssues.length) {
  L.push("");
  L.push("## Label-Fehler (bitte in den Labels beheben)");
  L.push("");
  L.push("~~~text");
  for (const i of labelIssues.slice(0, 400)) L.push(i);
  L.push("~~~");
}

mkdirSync(REPORTS, { recursive: true });
writeFileSync(path.join(REPORTS, "scorecard.md"), L.join("\n") + "\n", "utf8");
writeFileSync(path.join(REPORTS, "scorecard.json"), JSON.stringify({
  contracts: rows.length,
  traps: totalTraps, hits: totalHits, recall: pct(totalHits, totalTraps),
  benign: totalBenign, falsePositives: totalFalsePositives, fpr: pct(totalFalsePositives, totalBenign),
  uncovered: totalUncovered, uncoveredMissed: totalUncoveredMissed, uncoveredFired: totalUncoveredFired,
  labelIssues, perType: Object.fromEntries([...perType].map(([k, v]) => [k, { ...v, rate: pct(v.hit, v.n), fp: fpByType.get(k) || 0 }])),
  rows, misses: missList, fps: fpList, uncoveredMissed, uncoveredFired,
}, null, 2), "utf8");

console.log(`contracts=${rows.length} traps=${totalTraps} hits=${totalHits} recall=${pct(totalHits, totalTraps)}% benign=${totalBenign} fp=${totalFalsePositives} fpr=${pct(totalFalsePositives, totalBenign)}% uncoveredMissed=${totalUncoveredMissed} uncoveredFired=${totalUncoveredFired} labelIssues=${labelIssues.length}`);
process.exit(labelIssues.length ? 1 : 0);
