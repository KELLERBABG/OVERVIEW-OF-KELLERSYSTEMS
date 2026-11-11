import { readFileSync } from "node:fs";
const s = JSON.parse(readFileSync("Verträge/reports/scorecard.json", "utf8"));
const fps = s.fps || [];
console.log("fps " + fps.length);
for (const f of fps.slice(0, 12)) console.log("  " + String(f).slice(0, 170));
