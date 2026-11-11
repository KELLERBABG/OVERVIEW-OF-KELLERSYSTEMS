import { readFileSync, writeFileSync, readdirSync } from "node:fs";
const ids = ["wkg-vob-abweichung-gewaehrleistung", "wkg-leistungseinstellung-ohne-mahnung", "dach-053", "sweep-308-nr-6-zugangsfiktion-portal-01", "sweep309-017", "vsr-033", "spt-003", "enb-017", "isb-041"];
const dir = "data/clauses";
const files = readdirSync(dir).filter((f) => f.endsWith(".jsonl"));
let total = 0;
for (const f of files) {
  const path = dir + "/" + f;
  const lines = readFileSync(path, "utf8").split(/\r?\n/);
  let changed = 0;
  for (let i = 0; i < lines.length; i++) {
    const hit = ids.some((id) => lines[i].includes('"id":"' + id + '"'));
    if (!hit) continue;
    const before = lines[i];
    lines[i] = lines[i].replace('"detectorStatus":"mapped_blind"', '"detectorStatus":"mapped"');
    if (lines[i] !== before) changed++;
  }
  if (changed) { writeFileSync(path, lines.join("\n")); console.log(f + " changed=" + changed); total += changed; }
}
console.log("total " + total);
