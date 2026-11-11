import { readFileSync, readdirSync, existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { RULES } from "../../src/analyze.js";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, "..");
const KNOWN = new Set(RULES.map((r) => r.type));
const prefix = process.argv[2] || "";

function dirsWithLabels() {
  const out = [];
  for (const root of ["corpus", "public"]) {
    const labels = path.join(ROOT, root, "labels");
    if (existsSync(labels)) out.push({ root: path.join(ROOT, root), labels });
  }
  return out;
}

let files = 0;
let issues = 0;
let traps = 0;

for (const { root, labels } of dirsWithLabels()) {
  for (const file of readdirSync(root).filter((f) => f.endsWith(".txt")).sort()) {
    const slug = file.replace(/\.txt$/, "");
    if (prefix && !slug.startsWith(prefix)) continue;
    files++;
    const text = readFileSync(path.join(root, file), "utf8");
    const labelPath = path.join(labels, slug + ".json");
    const fail = (m) => { issues++; console.log(`FAIL ${slug}: ${m}`); };
    if (!existsSync(labelPath)) { fail("missing label file"); continue; }
    let L;
    try { L = JSON.parse(readFileSync(labelPath, "utf8")); }
    catch (e) { fail("invalid JSON: " + e.message); continue; }
    if (L.slug && L.slug !== slug) fail(`slug mismatch: ${L.slug}`);
    if (prefix && !String(L.slug || slug).startsWith(prefix)) fail("slug escapes prefix");
    const trapsList = L.traps || [];
    const benign = L.benign || [];
    const uncovered = L.uncovered || [];
    traps += trapsList.length;
    // Reale oeffentliche Vertragstexte tragen oft wenige oder keine Registerklauseln.
    const minCounts = !path.basename(root).startsWith("public");
    if (minCounts && trapsList.length < 6) fail(`traps=${trapsList.length} (min 6)`);
    if (minCounts && benign.length < 5) fail(`benign=${benign.length} (min 5)`);
    const seen = new Set();
    for (const [kind, list] of [["traps", trapsList], ["benign", benign], ["uncovered", uncovered]]) {
      for (const item of list) {
        const marker = item.marker;
        if (typeof marker !== "string" || marker.length < 25 || marker.length > 100) {
          fail(`${kind} marker length ${typeof marker === "string" ? marker.length : "n/a"}: ${String(marker).slice(0, 40)}`);
          continue;
        }
        const first = text.indexOf(marker);
        if (first === -1) { fail(`${kind} marker not in text: ${marker.slice(0, 60)}`); continue; }
        if (text.indexOf(marker, first + 1) !== -1) fail(`${kind} marker not unique: ${marker.slice(0, 60)}`);
        if (/\n\s*\n/.test(marker)) fail(`${kind} marker spans paragraphs: ${marker.slice(0, 40)}`);
        if (kind === "traps") {
          if (!KNOWN.has(item.expectedType)) fail(`unknown expectedType "${item.expectedType}" for ${marker.slice(0, 40)}`);
          if (seen.has(marker)) fail(`duplicate trap marker: ${marker.slice(0, 40)}`);
          seen.add(marker);
        }
        if (!item.note) fail(`${kind} missing note: ${marker.slice(0, 40)}`);
      }
    }
    if (/[\u2013\u2014]/.test(text)) fail("text contains en or em dash");
  }
}

console.log(`${prefix ? "prefix " + prefix + ": " : ""}${files} files, ${traps} traps, ${issues} issues`);
console.log(issues === 0 ? "OK" : "ISSUES");
process.exit(issues === 0 ? 0 : 1);
