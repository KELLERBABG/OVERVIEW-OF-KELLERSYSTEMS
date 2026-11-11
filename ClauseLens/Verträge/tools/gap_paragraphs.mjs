import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { analyzeContract } from "../../src/analyze.js";

const prefix = process.argv[2] || "evasion";
const root = dirname(dirname(fileURLToPath(import.meta.url)));
const corpus = join(root, "corpus");
const labelsDir = join(corpus, "labels");

function hitsIn(text, type, lo, hi) {
  return analyzeContract(text).findings.filter((f) => f.type === type && f.lineNumbers[0] <= hi && f.lineNumbers[1] >= lo);
}
function lineOf(text, marker) {
  const idx = text.indexOf(marker);
  if (idx < 0) return null;
  const lo = text.slice(0, idx).split("\n").length;
  return [lo, lo + marker.split("\n").length - 1];
}

const slugs = readdirSync(corpus).filter((f) => f.startsWith(prefix + "-") && f.endsWith(".txt")).map((f) => f.replace(/\.txt$/, "")).sort();
const out = [];
for (const slug of slugs) {
  const text = readFileSync(join(corpus, slug + ".txt"), "utf8");
  const label = JSON.parse(readFileSync(join(labelsDir, slug + ".json"), "utf8"));
  for (const trap of label.traps || []) {
    if (!(typeof trap.note === "string" && trap.note.startsWith("GAP:"))) continue;
    const lines = lineOf(text, trap.marker);
    const overlap = lines && hitsIn(text, trap.expectedType, lines[0], lines[1]).length > 0;
    if (overlap) continue;
    const para = text.split(/\n\s*\n/).find((p) => p.includes(trap.marker)) || trap.marker;
    out.push({ slug, type: trap.expectedType, line: lines ? lines[0] : 0, tech: trap.note.slice(4).trim(), para: para.replace(/\s+/g, " ").trim() });
  }
}
writeFileSync(join(root, "reports", "gap_paragraphs_" + prefix + ".json"), JSON.stringify(out, null, 1));
console.log("missing with overlap check: " + out.length);
