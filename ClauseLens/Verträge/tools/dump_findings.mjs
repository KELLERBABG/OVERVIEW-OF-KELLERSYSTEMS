import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { analyzeContract } from "../../src/analyze.js";
const root = dirname(dirname(fileURLToPath(import.meta.url)));
const slug = process.argv[2];
const text = readFileSync(join(root, "corpus", slug + ".txt"), "utf8");
const label = JSON.parse(readFileSync(join(root, "corpus", "labels", slug + ".json"), "utf8"));
const res = analyzeContract(text);
console.log("slug " + slug + " findings " + res.findings.length);
for (const f of res.findings) console.log("  L" + f.lineNumbers.join("-") + " " + f.type);
console.log("--- label traps");
for (const t of label.traps || []) {
  const idx = text.indexOf(t.marker);
  const lo = text.slice(0, idx).split("\n").length;
  console.log("  L" + lo + " " + t.expectedType + " " + (t.note||"").slice(0, 40));
}
