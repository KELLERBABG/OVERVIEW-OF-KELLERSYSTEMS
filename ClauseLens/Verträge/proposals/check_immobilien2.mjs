// Scratch checker for patterns_immobilien2.json. Patches RULES in memory only; never edits src/.
import fs from "node:fs";
import path from "node:path";

const ROOT = "C:/Users/INTAL Admin/SYSTEMS & CREATIONS/ClauseLens";
const { analyzeContract, RULES } = await import("file:///" + path.join(ROOT, "src/analyze.js").replace(/ /g, "%20"));
const CORPUS = path.join(ROOT, "Verträge", "corpus");
const doc = JSON.parse(fs.readFileSync(path.join(ROOT, "Verträge", "proposals", "patterns_immobilien2.json"), "utf8"));

const TARGETS = [
  ["immobilien-002", 40, "third_party_indemnity"],
  ["immobilien-006", 24, "third_party_indemnity"],
  ["immobilien-016", 24, "unilateral_convenience_termination"],
  ["immobilien-016", 40, "third_party_indemnity"],
  ["immobilien-026", 36, "excessive_survival"],
  ["immobilien-028", 28, "tax_indemnity_shift"],
  ["immobilien-029", 20, "unilateral_change"],
  ["immobilien-032", 40, "third_party_indemnity"],
];

const norm = (s) => s.replace(/\s+/g, " ").trim();
function labels(slug) {
  const p = path.join(CORPUS, "labels", slug + ".json");
  return fs.existsSync(p) ? JSON.parse(fs.readFileSync(p, "utf8")) : null;
}
function benignLines(slug, text) {
  const lab = labels(slug);
  const lines = text.split("\n");
  const out = new Set();
  for (const b of (lab && lab.benign) || []) {
    const m = norm(b.marker || "");
    if (!m) continue;
    const i = lines.findIndex((l) => norm(l).includes(m));
    if (i >= 0) out.add(i + 1);
  }
  return out;
}
function sliceFiles(prefix) {
  return fs.readdirSync(CORPUS).filter((f) => f.startsWith(prefix + "-") && f.endsWith(".txt"));
}
function prefixes() {
  return [...new Set(fs.readdirSync(CORPUS).filter((f) => f.endsWith(".txt")).map((f) => f.replace(/-\d+\.txt$/, "")))];
}
function fpScan(prefix) {
  let fp = 0;
  const detail = [];
  for (const f of sliceFiles(prefix)) {
    const slug = f.replace(/\.txt$/, "");
    const text = fs.readFileSync(path.join(CORPUS, f), "utf8");
    const ben = benignLines(slug, text);
    for (const fi of analyzeContract(text).findings) {
      for (const ln of fi.lineNumbers || []) {
        if (ben.has(ln)) { fp++; detail.push(slug + ":L" + ln + " " + fi.type); }
      }
    }
  }
  return { fp, detail };
}

// --- patch (idempotent: same source removed first)
const byType = new Map();
for (const e of doc.patterns) {
  if (!byType.has(e.type)) byType.set(e.type, new Set());
  byType.get(e.type).add(new RegExp(e.regex, e.flags || "i").source);
}
fmt: {
  const baselineImm = fpScan("immobilien");
  const baselineAll = {};
  for (const p of prefixes()) baselineAll[p] = fpScan(p).fp;
  console.log("BASELINE (ohne Proposals): immobilien benign FP = " + baselineImm.fp + ", Gesamt = " + Object.values(baselineAll).reduce((a, b) => a + b, 0));

  for (const rule of RULES) {
    const s = byType.get(rule.type);
    if (!s) continue;
    rule.patterns = (rule.patterns || []).filter((p) => !s.has(p.source)).concat([...s].map((src) => new RegExp(src, "i")));
  }

  console.log("\nZIELE");
  let closed = 0;
  for (const [slug, ln, type] of TARGETS) {
    const text = fs.readFileSync(path.join(CORPUS, slug + ".txt"), "utf8");
    const para = text.split("\n")[ln - 1].trim();
    const types = analyzeContract(para).findings.map((f) => f.type);
    const ok = types.includes(type);
    if (ok) closed++;
    console.log((ok ? "HIT  " : "MISS ") + slug + ":L" + ln + " expected=" + type + " actual=" + JSON.stringify(types));
  }
  console.log("geschlossen: " + closed + "/" + TARGETS.length);

  console.log("\nPRO PATTERN (Regex-Treffer / Ziel-schliesst)");
  for (const e of doc.patterns) {
    const re = new RegExp(e.regex, e.flags || "i");
    const rows = (e.closes || []).map((c) => {
      const [slug, l] = c.split(":L");
      const para = fs.readFileSync(path.join(CORPUS, slug + ".txt"), "utf8").split("\n")[Number(l) - 1].trim();
      const fires = analyzeContract(para).findings.map((f) => f.type);
      return c + (re.test(para) ? " re+" : " re-") + (fires.includes(e.type) ? " ok" : " NO(" + fires.join(",") + ")");
    });
    console.log(e.type.padEnd(35) + (rows.join("  ") || "(keine closes)"));
  }

  console.log("\nBENIGN FP");
  const imm = fpScan("immobilien");
  console.log("immobilien: " + baselineImm.fp + " -> " + imm.fp + (imm.detail.length ? " | " + imm.detail.join(", ") : ""));
  const per = {};
  const added = [];
  for (const p of prefixes()) {
    const now = fpScan(p);
    per[p] = now.fp;
    if (now.fp !== baselineAll[p]) added.push(p + ": " + baselineAll[p] + " -> " + now.fp + " | " + now.detail.join(", "));
  }
  console.log("Gesamt: " + Object.values(baselineAll).reduce((a, b) => a + b, 0) + " -> " + Object.values(per).reduce((a, b) => a + b, 0));
  console.log(added.length ? "neue FP je Slice:\n" + added.join("\n") : "keine neue FP in irgendeinem Slice");
}
