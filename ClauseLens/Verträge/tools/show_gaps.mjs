import { readFileSync } from "node:fs";
const d = JSON.parse(readFileSync("Verträge/reports/gap_paragraphs_service.json", "utf8"));
const seen = new Set();
for (const e of d) {
  const key = e.type + "|" + (e.tech || "").replace(/^engine miss\.?\s*/i, "").slice(0, 55);
  if (seen.has(key)) continue;
  seen.add(key);
  console.log("[" + e.type + "] tech=" + (e.tech || "").slice(0, 80));
  console.log("   " + e.para.slice(0, 190));
}
console.log("unique groups " + seen.size + " of " + d.length);
