// Scratch checker for the immobilien slice: parses the miss dossier, applies
// proposed patterns in memory and reports recall, declared-closes coverage and
// benign false positives.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(here, "..", "..");
const { analyzeContract, RULES } = await import(pathToFileURL(path.join(ROOT, "src", "analyze.js")).href);

const DOSSIER = path.join(here, "misses_immobilien.txt");
const PATTERNS = path.join(here, "patterns_immobilien.json");
const CORPUS = path.join(ROOT, "Verträge", "corpus");
const LABELS = path.join(CORPUS, "labels");

export function parseDossier() {
  const raw = fs.readFileSync(DOSSIER, "utf8");
  return raw.split(/^### /m).slice(1).map((b) => {
    const lines = b.split("\n");
    const m = lines[0].match(/^([^:]+):L(\d+)\s+type=(\S+)/);
    const noteIdx = lines.findIndex((l) => l.startsWith("note:"));
    const body = lines.slice(noteIdx + 1).filter((l) => l.trim()).join(" ");
    return { slug: m[1], line: Number(m[2]), type: m[3], para: body.replace(/\s+/g, " ").trim() };
  });
}

const patched = [];
function applyProposals(entries) {
  for (const e of entries) {
    const rule = RULES.find((r) => r.type === e.type);
    if (!rule) throw new Error("unknown type " + e.type);
    const re = new RegExp(e.regex, e.flags || "i");
    rule.patterns = (rule.patterns || []).filter((p) => p.source !== re.source);
    if (e.type === "mandatory_expert_determination") {
      rule.patterns[0] = new RegExp("(?:" + rule.patterns[0].source + ")|(?:" + re.source + ")", e.flags || "i");
    } else {
      rule.patterns = (rule.patterns || []).concat([re]);
    }
    patched.push({ ...e, re });
  }
}

function report() {
  const blocks = parseDossier();
  const bySlug = new Map();
  for (const b of blocks) {
    if (!bySlug.has(b.slug)) bySlug.set(b.slug, []);
    bySlug.get(b.slug).push(b);
  }
  const found = new Map();
  for (const b of blocks) found.set(b.slug + ":" + b.line, analyzeContract(b.para).findings.map((f) => f.type));
  const covered = blocks.filter((b) => found.get(b.slug + ":" + b.line).includes(b.type)).length;

  console.log("recall: " + covered + " / " + blocks.length + " miss paragraphs now hit (" + (100 * covered / blocks.length).toFixed(1) + "%)");
  console.log("-- per type --");
  const perType = new Map();
  for (const b of blocks) {
    if (!perType.has(b.type)) perType.set(b.type, { total: 0, covered: 0 });
    perType.get(b.type).total++;
    if (found.get(b.slug + ":" + b.line).includes(b.type)) perType.get(b.type).covered++;
  }
  for (const [t, v] of [...perType].sort((a, b) => (b[1].total - b[1].covered) - (a[1].total - a[1].covered))) {
    console.log("  " + (v.covered === v.total ? "ok  " : "MISS") + " " + t + ": " + v.covered + "/" + v.total);
  }

  console.log("-- declared closes --");
  let declared = 0, declaredOk = 0;
  for (const p of patched) {
    const bad = [];
    for (const cl of p.closes || []) {
      declared++;
      const [slug, ln] = cl.split(":L");
      const b = blocks.find((x) => x.slug === slug && x.line === Number(ln));
      if (!b) { bad.push(cl + " [not in dossier]"); continue; }
      const types = found.get(slug + ":" + Number(ln)) || [];
      const rx = p.re.test(b.para);
      if (types.includes(p.type)) { declaredOk++; continue; }
      bad.push(cl + (rx ? " [regex matches, analyzeContract emits: " + (types.join(",") || "nothing") + "]" : " [regex does NOT match]"));
    }
    if (bad.length) console.log("  " + p.type + " :: " + bad.join("; "));
  }
  console.log("declared closes hit: " + declaredOk + " / " + declared);

  console.log("-- remaining misses --");
  for (const b of blocks) if (!(found.get(b.slug + ":" + b.line) || []).includes(b.type)) console.log("  " + b.slug + ":L" + b.line + " " + b.type + " :: " + b.para);

  let benignLines = 0, fp = 0;
  const fpDetail = [];
  for (const slug of [...bySlug.keys()].sort()) {
    const text = fs.readFileSync(path.join(CORPUS, slug + ".txt"), "utf8");
    const lab = JSON.parse(fs.readFileSync(path.join(LABELS, slug + ".json"), "utf8"));
    const marks = new Set();
    for (const ben of lab.benign || []) {
      const flat = ben.marker.replace(/\s+/g, " ").trim();
      const lines = text.split("\n");
      for (let i = 0; i < lines.length; i++) if (lines[i].replace(/\s+/g, " ").includes(flat)) { marks.add(i + 1); benignLines++; break; }
    }
    for (const f of analyzeContract(text).findings) {
      for (const ln of f.lineNumbers || []) if (marks.has(ln)) { fp++; fpDetail.push(slug + ":L" + ln + " " + f.type); break; }
    }
  }
  console.log("benign marker lines checked=" + benignLines + "  findings overlapping benign lines (FP)=" + fp);
  if (fpDetail.length) console.log("FP detail: " + fpDetail.join(", "));
}

if (process.argv[1] && process.argv[1].endsWith("check_immobilien.mjs")) {
  const entries = JSON.parse(fs.readFileSync(PATTERNS, "utf8")).patterns || [];
  applyProposals(entries);
  console.log("proposals=" + entries.length + " patched=" + patched.length);
  report();
}
