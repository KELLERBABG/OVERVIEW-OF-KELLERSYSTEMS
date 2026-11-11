import { readFileSync, writeFileSync, readdirSync } from "node:fs";
const SRC = "src/analyze.js";
const DIR = "Verträge/proposals";
let text = readFileSync(SRC, "utf8");
const files = readdirSync(DIR).filter((f) => /^patterns_.*\.json$/.test(f));
const byType = new Map();
const seen = new Set();
for (const f of files) {
  const j = JSON.parse(readFileSync(DIR + "/" + f, "utf8"));
  for (const p of j.patterns || []) {
    if (!p.regex || !p.type) continue;
    const key = p.type + "\u0000" + p.regex;
    if (seen.has(key)) continue;
    seen.add(key);
    const list = byType.get(p.type) || [];
    list.push(p);
    byType.set(p.type, list);
  }
}
// Geminete Muster, die saubere Kontrollklauseln treffen, werden hier entschaerft.
const FIXUPS = [
  { type: "excessive_survival", find: "|unver(?:ä|ae)ndert", replace: "" },
];
for (const f of FIXUPS) {
  for (const p of byType.get(f.type) || []) p.regex = p.regex.split(f.find).join(f.replace);
}
const lines = ["// Muster aus dem Red-Team-Korpus, gemergt aus Verträge/proposals/patterns_*.json.", "const MINED_SUPPLEMENTS = {"];
for (const [type, list] of [...byType].sort()) {
  lines.push("  " + type + ": [");
  for (const p of list) lines.push("    new RegExp(" + JSON.stringify(p.regex) + ", " + JSON.stringify(p.flags || "i") + "),");
  lines.push("  ],");
}
lines.push("};");
const block = lines.join("\n");
const startMark = "// >>> mined patterns";
const endMark = "// <<< mined patterns";
const wrapped = startMark + "\n" + block + "\n" + endMark;
if (text.includes(startMark)) {
  const a = text.indexOf(startMark);
  const b = text.indexOf(endMark, a) + endMark.length;
  text = text.slice(0, a) + wrapped + text.slice(b);
} else {
  const anchor = "};\nfor (const rule of RULES) {\n  const extra = PATTERN_SUPPLEMENTS[rule.type];\n  if (extra) rule.patterns = (rule.patterns || []).concat(extra);\n}";
  if (!text.includes(anchor)) throw new Error("anchor missing");
  text = text.replace(anchor, "};\n" + wrapped + "\nfor (const rule of RULES) {\n  const extra = (PATTERN_SUPPLEMENTS[rule.type] || []).concat(MINED_SUPPLEMENTS[rule.type] || []);\n  if (extra.length) rule.patterns = (rule.patterns || []).concat(extra);\n}");
}
writeFileSync(SRC, text);
const total = [...byType.values()].reduce((a, b) => a + b.length, 0);
console.log("merged " + files.join(",") + " types=" + byType.size + " patterns=" + total);
