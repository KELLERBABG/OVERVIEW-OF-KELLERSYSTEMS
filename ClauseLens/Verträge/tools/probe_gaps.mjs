import { readFileSync, readdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { analyzeContract } from "../../src/analyze.js";
import { normalizeText } from "../../src/textio.js";

const prefix = process.argv[2] || "evasion";
const root = dirname(dirname(fileURLToPath(import.meta.url)));
const corpus = join(root, "corpus");
const labelsDir = join(corpus, "labels");

function hitsFor(text, type) {
  const res = analyzeContract(text);
  return res.findings.filter((f) => f.type === type);
}
function lineRangeOf(text, marker) {
  const idx = text.indexOf(marker);
  if (idx < 0) return null;
  const before = text.slice(0, idx);
  return before.split("\n").length;
}

const slugs = readdirSync(corpus).filter((f) => f.startsWith(prefix + "-") && f.endsWith(".txt")).map((f) => f.replace(/\.txt$/, ""));
const rows = [];
for (const slug of slugs.sort()) {
  const text = readFileSync(join(corpus, slug + ".txt"), "utf8");
  const label = JSON.parse(readFileSync(join(labelsDir, slug + ".json"), "utf8"));
  for (const trap of label.traps || []) {
    const isGap = typeof trap.note === "string" && trap.note.startsWith("GAP:");
    if (!isGap) continue;
    const full = hitsFor(text, trap.expectedType).length > 0;
    const para = text.split(/\n\s*\n/).find((p) => p.includes(trap.marker));
    const isolated = para ? hitsFor(para, trap.expectedType).length > 0 : false;
    const normalized = para ? hitsFor(normalizeText(para), trap.expectedType).length > 0 : false;
    rows.push({ slug, type: trap.expectedType, line: lineRangeOf(text, trap.marker), full, isolated, normalized, kind: para ? "para" : "missing" });
  }
}
const byCause = { contextOnly: 0, wording: 0, okNow: 0, missingPara: 0 };
for (const r of rows) {
  if (r.kind === "missing") byCause.missingPara++;
  else if (r.full) byCause.okNow++;
  else if (r.isolated) byCause.contextOnly++;
  else byCause.wording++;
}
console.log("prefix " + prefix + ": " + slugs.length + " contracts, " + rows.length + " GAP traps");
console.log(JSON.stringify(byCause));
for (const r of rows) {
  console.log([r.slug, r.type, "L" + r.line, r.full ? "FULL" : "-", r.isolated ? "ISO" : "-", r.kind].join("\t"));
}
