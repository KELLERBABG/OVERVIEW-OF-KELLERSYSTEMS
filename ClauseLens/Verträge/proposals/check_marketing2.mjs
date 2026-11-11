import { pathToFileURL } from "node:url";
import fs from "node:fs";

const ROOT = "C:/Users/INTAL Admin/SYSTEMS & CREATIONS/ClauseLens";
const mod = await import(pathToFileURL(ROOT + "/src/analyze.js").href);
const { RULES, analyzeContract, flattenLineBreaks } = mod;

const TARGETS = [
  { slug: "marketing-008", line: 37, type: "burden_of_proof", foreign: "audit_log_manipulation" },
  { slug: "marketing-009", line: 25, type: "vicarious_agents", foreign: "liability" },
  { slug: "marketing-013", line: 37, type: "entire_agreement", foreign: "written_form_trap" },
  { slug: "marketing-021", line: 22, type: "data_grant", foreign: "data_lockin" },
];

const P = (type, re, evidence, risk) => ({ type, re, evidence, risk });
const PROPOSALS = [
  P("burden_of_proof", /aufzeichnungen\s+des\s+(?:anbieters|auftragnehmers|dienstleisters)[^.!?;:\n]{0,80}?(?:ma(?:ß|ss)geblich|verbindlich|entscheidend|entscheiden)/i, "Aufzeichnungen des Anbieters sind für die Nutzung maßgeblich", "medium"),
  P("vicarious_agents", /(?:haftet|haften)\s+(?:der\s+(?:anbieter|auftragnehmer|dienstleister)\s+)?nicht\s+f(?:ü|ue)r\s+handlungen\s+(?:und\s+unterlassungen\s+)?von\s+(?:nach|unter|sub)unternehmer/i, "Der Anbieter haftet nicht für Handlungen von Nachunternehmern", "medium"),
  P("entire_agreement", /nebenabreden[^.!?;:\n]{0,60}?(?:entfalten|entfaltet|haben|hat)\s+keine\s+(?:rechtliche\s+)?wirkung/i, "Nebenabreden entfalten keine Wirkung, ungeachtet ihres Zustands", "medium"),
  P("data_grant", /(?:erh(?:ä|ae)lt|erhalten|einger(?:ä|ae)umt)[^.!?;:\n]{0,60}?(?:unwiderruflich|unbefristet|unentgeltlich|weltweit|perpetual|irrevocable)[^.!?;:\n]{0,40}?(?:nutzungsrecht|licen[cs]e)/i, "Der Auftragnehmer erhält ein unwiderrufliches Nutzungsrecht an den erstellten Inhalten", "medium"),
];

const contracts = [];
for (let i = 1; i <= 32; i++) {
  const slug = "marketing-" + String(i).padStart(3, "0");
  contracts.push({ slug, text: fs.readFileSync(ROOT + "/Verträge/corpus/" + slug + ".txt", "utf8"), label: JSON.parse(fs.readFileSync(ROOT + "/Verträge/corpus/labels/" + slug + ".json", "utf8")) });
}

const norm = (s) => s.replace(/\s+/g, " ").trim().toLowerCase();
const flat = (s) => flattenLineBreaks(s);

function paragraphAt(text, line) {
  const lines = text.split(/\r?\n/);
  let a = line - 1, b = line - 1;
  while (a > 0 && lines[a - 1].trim()) a--;
  while (b < lines.length - 1 && lines[b + 1].trim()) b++;
  return { start: a + 1, end: b + 1, text: lines.slice(a, b + 1).join(" ") };
}

function findingsOn(text, type, start, end) {
  return analyzeContract(text).findings.filter((f) => f.type === type && f.lineNumbers[0] <= end && f.lineNumbers[1] >= start);
}

const benignKeys = (c, b, fs2) => fs2.filter((f) => norm(f.excerpt).includes(norm(b.marker))).map((f) => c.slug + "::" + b.marker + "::" + f.type);
const baseFindings = new Set();
const benignBase = [];
for (const c of contracts) {
  const fs2 = analyzeContract(c.text).findings;
  for (const f of fs2) baseFindings.add(c.slug + "::" + f.type + "::" + f.lineNumbers.join("-"));
  for (const b of c.label.benign) benignBase.push(...benignKeys(c, b, fs2));
}
console.log("baseline: benign FPs", benignBase.length, "| findings", baseFindings.size);

const byType = new Map();
for (const p of PROPOSALS) {
  if (!byType.has(p.type)) byType.set(p.type, []);
  byType.get(p.type).push(p);
}
const patchedTypes = [];
for (const rule of RULES) {
  const props = byType.get(rule.type);
  if (!props) continue;
  patchedTypes.push(rule.type);
  if (typeof rule.collect === "function") {
    const orig = rule.collect;
    rule.collect = function (sentences, text) {
      const hits = orig.call(this, sentences, text) || [];
      const seen = new Set(hits.map((h) => h.i));
      const extra = [];
      sentences.forEach((s, i) => {
        if (seen.has(i)) return;
        const t = flat(s.text);
        for (const p of props) if (p.re.test(t)) { extra.push({ i, severity: p.risk === "medium" ? "mittel" : p.risk }); break; }
      });
      return hits.concat(extra);
    };
  } else {
    for (const p of props) rule.patterns = (rule.patterns || []).concat([new RegExp(p.re.source, p.re.flags)]);
  }
}
console.log("patched rule types:", patchedTypes.join(","));

let regexOk = 0, firedBefore = 0, expectedFired = 0;
const perTarget = [];
for (const t of TARGETS) {
  const c = contracts.find((x) => x.slug === t.slug);
  const para = paragraphAt(c.text, t.line);
  const type = t.type;
  const props = byType.get(type) || [];
  const hit = props.find((p) => p.re.test(flat(para.text)));
  const before = analyzeContract(c.text).findings.filter((f) => f.lineNumbers[0] <= para.end && f.lineNumbers[1] >= para.start).map((f) => f.type);
  const both = findingsOn(c.text, type, para.start, para.end).length > 0;
  if (hit) regexOk++;
  if (before.includes(t.foreign)) firedBefore++;
  if (both) expectedFired++;
  perTarget.push({ key: t.slug + ":L" + t.line, type, regexMatches: !!hit, foreignFires: before.includes(t.foreign), foreignTypes: before, expectedFires: both, para: flat(para.text).slice(0, 90) });
}
console.log("regex matches target paragraph:", regexOk + "/" + TARGETS.length);
console.log("target expectedType fires after patch:", expectedFired + "/" + TARGETS.length);
for (const p of perTarget) console.log(" ", p.key, p.type, "| regex", p.regexMatches, "| foreign(" + p.foreignTypes.join(",") + ")", "| expected", p.expectedFires);

const proposalHits = [];
for (const c of contracts) for (const b of c.label.benign) for (const p of PROPOSALS) {
  if (p.re.test(flat(b.marker))) { proposalHits.push(p.type + " :: " + c.slug + " :: " + b.marker); break; }
}
const benignAfter = [];
const allNew = [];
for (const c of contracts) {
  const fs2 = analyzeContract(c.text).findings;
  for (const f of fs2) {
    const key = c.slug + "::" + f.type + "::" + f.lineNumbers.join("-");
    if (!baseFindings.has(key)) allNew.push(key);
  }
  for (const b of c.label.benign) benignAfter.push(...benignKeys(c, b, fs2));
}
const baseSet = new Set(benignBase);
const newFps = benignAfter.filter((k) => !baseSet.has(k));
console.log("proposal regex hits on benign markers:", proposalHits.length, "(must be 0)");
console.log("new benign findings vs baseline:", newFps.length, "(must be 0)");
console.log("all new findings after patch:", allNew.length);
console.log(JSON.stringify(allNew, null, 1));
if (proposalHits.length) console.log("FP-PROPOSAL", JSON.stringify(proposalHits, null, 1));
if (newFps.length) console.log("FP-NEW", JSON.stringify(newFps, null, 1));

const out = { slice: "marketing", patterns: [] };
for (const p of PROPOSALS) {
  const cl = TARGETS.filter((t) => t.type === p.type).map((t) => t.slug + ":L" + t.line);
  out.patterns.push({ type: p.type, regex: p.re.source, flags: p.re.flags, closes: cl, evidence: p.evidence, risk: p.risk });
}
fs.writeFileSync(ROOT + "/Verträge/proposals/patterns_marketing2.json", JSON.stringify(out, null, 2) + "\n");
console.log("wrote patterns_marketing2.json:", out.patterns.length);
if (regexOk !== TARGETS.length || expectedFired !== TARGETS.length || proposalHits.length || newFps.length) { console.log("CHECK-FAILED"); process.exit(1); }
console.log("CHECK-OK");
