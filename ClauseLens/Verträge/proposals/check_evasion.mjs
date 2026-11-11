// Evasion-Slice: Trefferquote (Zeilenzuordnung) und Falschtreffer auf harmlosen Klauseln.
// Aufruf: node "Vertraege/proposals/check_evasion.mjs" [--apply]
// --apply haengt die Muster aus patterns_evasion.json zur Laufzeit an die Regeln an
// (kein Schreibzugriff auf src/).
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { analyzeContract, RULES } from "../../src/analyze.js";

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, "../..");
const corpus = path.join(root, "Verträge", "corpus");
const labels = path.join(corpus, "labels");
const apply = process.argv.includes("--apply");

function loadPatterns() {
  const p = path.join(here, "patterns_evasion.json");
  if (!fs.existsSync(p)) return [];
  return JSON.parse(fs.readFileSync(p, "utf8")).patterns || [];
}

function installPatterns(list) {
  let n = 0;
  for (const p of list) {
    const rule = RULES.find((r) => r.type === p.type);
    if (!rule) throw new Error("unbekannter Detektor: " + p.type);
    const re = new RegExp(p.regex, p.flags || "i");
    rule.patterns = rule.patterns || [];
    rule.patterns.push(re);
    n++;
  }
  return n;
}

function lineOf(lines, marker) {
  for (let i = 0; i < lines.length; i++) if (lines[i].includes(marker)) return i + 1;
  const head = marker.slice(0, 40);
  for (let i = 0; i < lines.length; i++) if (lines[i].includes(head)) return i + 1;
  return -1;
}

function measure() {
  const slugs = [];
  for (let i = 1; i <= 32; i++) slugs.push("evasion-" + String(i).padStart(3, "0"));
  const res = { traps: 0, hits: 0, unmapped: 0, fp: 0, misses: [], fps: [] };
  for (const slug of slugs) {
    const text = fs.readFileSync(path.join(corpus, slug + ".txt"), "utf8");
    const lines = text.split("\n");
    const lab = JSON.parse(fs.readFileSync(path.join(labels, slug + ".json"), "utf8"));
    const findings = analyzeContract(text).findings;
    for (const t of lab.traps) {
      res.traps++;
      const ln = lineOf(lines, t.marker);
      if (ln < 0) { res.unmapped++; res.misses.push({ slug, line: -1, type: t.expectedType, why: "marker nicht gefunden" }); continue; }
      const hit = findings.some((f) => f.type === t.expectedType && f.lineNumbers[0] <= ln && f.lineNumbers[1] >= ln);
      if (hit) res.hits++;
      else {
        const other = findings.filter((f) => f.lineNumbers[0] <= ln && f.lineNumbers[1] >= ln).map((f) => f.type);
        res.misses.push({ slug, line: ln, type: t.expectedType, why: other.length ? "stattdessen " + [...new Set(other)].join("/") : "kein Befund" });
      }
    }
    for (const b of lab.benign) {
      const ln = lineOf(lines, b.marker);
      if (ln < 0) continue;
      const hit = findings.filter((f) => f.lineNumbers[0] <= ln && f.lineNumbers[1] >= ln);
      if (hit.length) { res.fp++; res.fps.push({ slug, line: ln, types: [...new Set(hit.map((f) => f.type))] }); }
    }
  }
  return res;
}

const list = apply ? loadPatterns() : loadPatterns();
if (apply) {
  const n = installPatterns(list);
  console.log("installiert: " + n + " Muster");
}
const r = measure();
const byType = {};
for (const m of r.misses) byType[m.type] = (byType[m.type] || 0) + 1;
console.log("evasion: " + r.hits + "/" + r.traps + " Treffer, " + r.misses.length + " Fehlschlaege, " + r.fp + " Falschtreffer, " + r.unmapped + " nicht zugeordnet");
console.log("Fehlschlag nach Typ: " + (Object.keys(byType).length ? JSON.stringify(byType) : "keine"));
for (const m of r.misses) console.log("  MISS " + m.slug + ":L" + m.line + " " + m.type + " (" + m.why + ")");
for (const f of r.fps) console.log("  FP " + f.slug + ":L" + f.line + " " + f.types.join("/"));
