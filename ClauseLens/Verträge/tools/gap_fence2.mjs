import { readFileSync } from "node:fs";
import { loadDatabase, verifyMappings } from "../../src/clausedb.js";
import { analyzeContract } from "../../src/analyze.js";
const src = readFileSync("src/analyze.js", "utf8");
const start = src.indexOf("const PATTERN_SUPPLEMENTS = {");
const end = src.indexOf("\n};", start);
const block = src.slice(start + "const PATTERN_SUPPLEMENTS = ".length, end + 2);
const SUPP = eval("(" + block + ")");
const db = loadDatabase("C:/Users/INTAL Admin/SYSTEMS & CREATIONS/ClauseLens");
const records = db.records || db;
const v = verifyMappings(records);
const byId = new Map(records.map((r) => [r.id, r]));
let n = 0;
for (const item of v.gapHits) {
  const rec = byId.get(item.id);
  const text = rec ? rec.clauseText : "";
  const mine = [];
  for (const found of item.found) {
    const res = SUPP[found];
    if (res && res.some((re) => re.test(text))) mine.push(found);
  }
  if (mine.length) { n++; console.log(item.id + " " + item.clauseType + " via " + mine.join(",")); }
}
console.log("gapHits " + v.gapHits.length + ", caused by supplements: " + n);
