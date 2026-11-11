import { readFileSync } from "node:fs";
const s = JSON.parse(readFileSync("Verträge/reports/scorecard.json", "utf8"));
const misses = s.misses || [];
const bySlice = {}, byType = {}, byKind = {};
const rows = {};
for (const m of misses) {
  const mm = String(m).match(/^([a-z_]+) @([a-z0-9-]+):L(\d+)(?:-(\d+))? \[(\w+)\]/);
  if (!mm) { byKind["unparsed"] = (byKind.unparsed || 0) + 1; continue; }
  const type = mm[1], slug = mm[2], kind = mm[5];
  const slice = slug.split("-")[0];
  bySlice[slice] = (bySlice[slice] || 0) + 1;
  byType[type] = (byType[type] || 0) + 1;
  byKind[kind] = (byKind[kind] || 0) + 1;
  (rows[slice] = rows[slice] || []).push({ type, slug, line: +mm[3] });
}
console.log("BY SLICE " + Object.entries(bySlice).sort((a, b) => b[1] - a[1]).map(([k, v]) => k + "=" + v).join(" "));
console.log("BY KIND " + JSON.stringify(byKind));
console.log("BY TYPE " + Object.entries(byType).sort((a, b) => b[1] - a[1]).map(([k, v]) => k + "=" + v).join(" "));
