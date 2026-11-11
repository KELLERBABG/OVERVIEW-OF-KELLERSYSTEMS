import { readFileSync, readdirSync } from "node:fs";
import { analyzeContract, RULES } from "../../src/analyze.js";
const ROOT = "Verträge/corpus";
const want = new Set(["subcontractor_uncontrolled", "excessive_survival", "marketing_reference", "notice_period", "liability"]);
const samples = {};
for (const f of readdirSync(ROOT).filter((x) => x.endsWith(".txt"))) {
  const slug = f.replace(/\.txt$/, "");
  let label;
  try { label = JSON.parse(readFileSync(ROOT + "/labels/" + slug + ".json", "utf8")); } catch { continue; }
  const text = readFileSync(ROOT + "/" + f, "utf8");
  const lines = text.split(/\n/);
  const res = analyzeContract(text);
  for (const b of label.benign || []) {
    const ml = new Set();
    for (let i = 0; i < lines.length; i++) if (lines[i].includes(b.marker.slice(0, 40))) ml.add(i + 1);
    if (!ml.size) continue;
    for (const fd of res.findings) {
      if (!want.has(fd.type)) continue;
      const [a, z] = fd.lineNumbers;
      let ov = false;
      for (let l = a; l <= z; l++) if (ml.has(l)) ov = true;
      if (!ov) continue;
      const k = fd.type;
      samples[k] = samples[k] || [];
      if (samples[k].length < 4 && !samples[k].includes(fd.excerpt)) samples[k].push(fd.excerpt);
    }
  }
}
for (const [k, v] of Object.entries(samples)) {
  console.log("=== " + k + " (" + v.length + " samples)");
  for (const e of v) console.log("  EX " + e.replace(/\s+/g, " ").slice(0, 230));
}
for (const t of ["subcontractor_uncontrolled", "excessive_survival", "marketing_reference"]) {
  const r = RULES.find((x) => x.type === t);
  console.log("--- patterns for " + t + " (" + r.patterns.length + ")");
  r.patterns.slice(-4).forEach((p, i) => console.log("  P" + (r.patterns.length - 4 + i) + " " + String(p).slice(0, 320)));
}
