import { readFileSync } from "node:fs";
const s = JSON.parse(readFileSync("Verträge/reports/scorecard.json", "utf8"));
const refRe = /(?:anlage|anhang|annex|ziffer|abschnitt|gem(?:äß|aess)|siehe|verweis|geregelt\s+in|nach\s+Ma(?:ß|ss)gabe)/i;
let n = 0, short = 0;
const ex = [];
for (const m of s.misses || []) {
  const mm = String(m).match(/^([a-z_]+) @([a-z0-9-]+):L(\d+)/);
  if (!mm) continue;
  const [, type, slug, line] = mm;
  const text = readFileSync("Verträge/corpus/" + slug + ".txt", "utf8");
  const paras = text.split(/\n\s*\n/);
  let cursor = 0, para = "";
  for (const p of paras) {
    const c = p.split(/\n/).length;
    if (+line >= cursor + 1 && +line < cursor + 1 + c) { para = p.replace(/\s+/g, " ").trim(); break; }
    cursor += c + 1;
  }
  if (refRe.test(para)) { n++; if (para.length < 190) { short++; if (ex.length < 12) ex.push(slug + ":L" + line + " " + type + " :: " + para.slice(0, 150)); } }
}
console.log("misses containing reference language: " + n + " of which short (<190 chars): " + short);
console.log(ex.join("\n"));
