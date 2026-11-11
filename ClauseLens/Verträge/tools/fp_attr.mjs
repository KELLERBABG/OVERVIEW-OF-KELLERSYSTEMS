import { readFileSync, readdirSync } from "node:fs";
import { analyzeContract, RULES } from "../../src/analyze.js";
const ROOT = "Verträge/corpus";
const files = readdirSync(ROOT).filter((f) => f.endsWith(".txt"));
const agg = new Map();
let fps = 0;
for (const f of files) {
  const slug = f.replace(/\.txt$/, "");
  let label;
  try { label = JSON.parse(readFileSync(ROOT + "/labels/" + slug + ".json", "utf8")); } catch { continue; }
  const text = readFileSync(ROOT + "/" + f, "utf8");
  const res = analyzeContract(text);
  const lines = text.split(/\n/);
  for (const b of label.benign || []) {
    const markerLines = new Set();
    for (let i = 0; i < lines.length; i++) if (lines[i].includes(b.marker.slice(0, 40))) markerLines.add(i + 1);
    if (!markerLines.size) continue;
    for (const fd of res.findings) {
      const [a, z] = fd.lineNumbers;
      let overlap = false;
      for (let l = a; l <= z; l++) if (markerLines.has(l)) overlap = true;
      if (!overlap) continue;
      fps++;
      const rule = RULES.find((r) => r.type === fd.type);
      const pats = (rule.patterns || []).map((re, i) => (re.test(fd.excerpt) ? i : -1)).filter((i) => i >= 0);
      for (const i of pats) {
        const key = fd.type + " :: " + String(rule.patterns[i]).slice(0, 110);
        agg.set(key, (agg.get(key) || 0) + 1);
      }
      if (!pats.length) agg.set(fd.type + " :: <no direct pattern hit>", (agg.get(fd.type + " :: <no direct pattern hit>") || 0) + 1);
    }
  }
}
console.log("fp findings " + fps);
for (const [k, v] of [...agg].sort((a, b) => b[1] - a[1]).slice(0, 24)) console.log(v + "x " + k);
