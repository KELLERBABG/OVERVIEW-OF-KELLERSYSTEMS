import { readFile } from "node:fs/promises";
import { RULES, analyzeContract } from "../../src/analyze.js";

const NL = String.fromCharCode(10);
const CORPUS = new URL("../../Verträge/corpus/", import.meta.url);
const MISSES = ["service-003:16:deemed_acceptance","service-003:28:burden_of_proof","service-004:24:asymmetric_termination","service-004:32:force_majeure_asymmetry","service-005:16:excessive_survival","service-005:28:extended_retention_of_title","service-005:36:audit_log_manipulation","service-005:48:setoff_restriction","service-006:16:insolvency_termination","service-007:16:fictitious_receipt","service-007:28:agency_goodwill_waiver","service-007:36:mandatory_expert_determination","service-008:16:early_termination_charge","service-010:24:deemed_acceptance","service-010:36:burden_of_proof","service-011:32:asymmetric_termination","service-011:40:force_majeure_asymmetry","service-012:24:excessive_survival","service-012:36:extended_retention_of_title","service-013:16:audit_log_manipulation","service-013:24:insolvency_termination","service-014:24:fictitious_receipt","service-014:36:agency_goodwill_waiver","service-015:16:mandatory_expert_determination","service-015:24:early_termination_charge","service-017:32:deemed_acceptance","service-018:16:burden_of_proof","service-018:40:asymmetric_termination","service-018:44:as_is_software_disclaimer","service-019:20:force_majeure_asymmetry","service-019:32:excessive_survival","service-019:44:marketing_reference","service-020:16:extended_retention_of_title","service-020:24:audit_log_manipulation","service-020:32:insolvency_termination","service-020:48:unverbindliche_sla_falle","service-021:32:fictitious_receipt","service-021:44:agency_commission_exclusion","service-022:16:agency_goodwill_waiver","service-022:24:mandatory_expert_determination","service-022:32:early_termination_charge","service-023:44:service_credit_forfeiture","service-024:40:deemed_acceptance","service-024:44:maintenance_conditional_shutdown","service-025:24:burden_of_proof","service-026:20:asymmetric_termination","service-026:28:force_majeure_asymmetry","service-026:40:excessive_survival","service-027:24:extended_retention_of_title","service-027:32:audit_log_manipulation","service-027:40:insolvency_termination","service-028:40:fictitious_receipt","service-029:24:agency_goodwill_waiver","service-029:32:mandatory_expert_determination","service-029:40:early_termination_charge","service-029:44:setoff_restriction","service-032:20:deemed_acceptance","service-032:32:burden_of_proof"];
const proposal = JSON.parse(await readFile(new URL("./patterns_service.json", import.meta.url), "utf8"));
const slugs = Array.from({ length: 32 }, (_, i) => "service-" + String(i + 1).padStart(3, "0"));
const fails = [];
const bad = (m) => fails.push(m);
if (proposal.slice !== "service") bad("slice must be service");
if (!Array.isArray(proposal.patterns)) bad("patterns must be an array");
for (const p of proposal.patterns || []) {
  if (!RULES.some((r) => r.type === p.type)) bad("unknown type " + p.type);
  if (p.flags !== "i") bad("flags must be i: " + p.type);
  if (!Array.isArray(p.closes) || p.closes.length === 0) bad("empty closes: " + p.type + " " + p.evidence);
  if (typeof p.evidence !== "string" || !p.evidence) bad("evidence missing: " + p.type);
  if (p.risk !== "low" && p.risk !== "medium") bad("risk invalid: " + p.type);
  try { new RegExp(p.regex, p.flags); } catch (e) { bad("regex does not compile: " + p.type + " " + e.message); }
}
const text = {}; const labels = {};
for (const s of slugs) { text[s] = await readFile(new URL(s + ".txt", CORPUS), "utf8"); labels[s] = JSON.parse(await readFile(new URL("labels/" + s + ".json", CORPUS), "utf8")); }
const lineOfMarker = (t, m) => t.split(NL).findIndex((l) => l.includes(m)) + 1;
const paraOfLine = (t, ln) => { const L = t.split(NL); let s = ln - 1; while (s > 0 && L[s - 1].trim() !== "") s--; let e = ln - 1; while (e + 1 < L.length && L[e + 1].trim() !== "") e++; return L.slice(s, e + 1).join(NL); };
const hit = (r, ty, ln) => r.findings.some((f) => f.type === ty && f.lineNumbers.includes(ln));
const bLines = {}; const bParas = [];
for (const s of slugs) { bLines[s] = (labels[s].benign || []).map((b) => lineOfMarker(text[s], b.marker)); for (const b of labels[s].benign || []) bParas.push({ slug: s, para: paraOfLine(text[s], lineOfMarker(text[s], b.marker)) }); }
const miss = MISSES.map((m) => { const a = m.split(":"); return { slug: a[0], ln: Number(a[1]), type: a[2] }; });
const scan = () => { const res = {}; const hits = new Set(); for (const s of slugs) res[s] = analyzeContract(text[s]); for (const e of miss) if (hit(res[e.slug], e.type, e.ln)) hits.add(e.slug + ":" + e.ln); return { res, hits }; };
const before = scan();
const before2 = scan();
const det = JSON.stringify([...before.hits].sort().join("|")) === JSON.stringify([...before2.hits].sort().join("|")) && slugs.every((x) => JSON.stringify(before.res[x].findings.map((f) => f.type)) === JSON.stringify(before2.res[x].findings.map((f) => f.type)));
console.log("determinism (baseline scanned twice identical)=" + det);
for (const p of proposal.patterns) { const r = RULES.find((x) => x.type === p.type); if (p.type === "mandatory_expert_determination") r.patterns[0] = new RegExp("(?:" + r.patterns[0].source + ")|(?:" + p.regex + ")", r.patterns[0].flags); else r.patterns.push(new RegExp(p.regex, p.flags)); }
const after = scan();
let closesOK = 0;
for (const p of proposal.patterns) { const re = new RegExp(p.regex, p.flags); for (const c of p.closes) { const a = c.split(":"); const ln = Number(a[1]); if (!re.test(paraOfLine(text[a[0]], ln))) bad("pattern does not match close " + c + " (" + p.type + ")"); if (hit(after.res[a[0]], p.type, ln)) closesOK++; else bad("close not hit via analyzeContract " + c + " (" + p.type + ")"); } }
const propTypes = new Set(proposal.patterns.map((q) => q.type));
let fpBase = 0, fp = 0; const fpD = [];
for (const x of slugs) for (const f of after.res[x].findings) for (const ln of bLines[x]) if (f.lineNumbers.includes(ln)) { if (propTypes.has(f.type)) { fp++; fpD.push(x + ":L" + ln + " " + f.type + " " + f.excerpt.slice(0, 60)); } else fpBase++; }
console.log("benign FPs pre-existing (type not proposed)=" + fpBase);
let bpBase = 0, bp = 0; const bpD = [];
for (const b of bParas) { const r = analyzeContract(b.para); const mine = r.findings.filter((f) => propTypes.has(f.type)); if (mine.length) { bp++; bpD.push(b.slug + " " + mine.map((f) => f.type).join("+")); } else if (r.findings.length) bpBase++; }
console.log("benign paragraphs with pre-existing findings=" + bpBase);
let tb = 0, ta = 0, tn = 0;
for (const s of slugs) for (const t of labels[s].traps || []) { const ln = lineOfMarker(text[s], t.marker); tn++; if (hit(before.res[s], t.expectedType, ln)) tb++; if (hit(after.res[s], t.expectedType, ln)) ta++; }
console.log("patterns=" + proposal.patterns.length + " types=" + new Set(proposal.patterns.map((p) => p.type)).size);
console.log("misses before=" + before.hits.size + "/" + miss.length + " after=" + after.hits.size + "/" + miss.length);
console.log("closes verified=" + closesOK);
console.log("all-trap hits before=" + tb + "/" + tn + " after=" + ta + "/" + tn);
console.log("benign FPs=" + fp + (fp ? " :: " + fpD.join(" | ") : ""));
console.log("benign paragraphs with findings=" + bp + (bp ? " :: " + bpD.join(" | ") : ""));
for (const e of miss) if (!after.hits.has(e.slug + ":" + e.ln)) console.log("still open (not regex closable): " + e.slug + ":L" + e.ln + " " + e.type);
if (!det) fails.push("baseline findings are not deterministic across identical scans");
if (fails.length || fp || bp) { console.log("CHECK FAILED"); for (const f of fails) console.log(" - " + f); process.exitCode = 1; } else console.log("CHECK OK");
