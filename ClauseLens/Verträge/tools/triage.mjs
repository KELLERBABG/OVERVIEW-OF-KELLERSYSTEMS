import { readFileSync, writeFileSync, existsSync, mkdirSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, "..");
const REPORTS = path.join(ROOT, "reports");
const SCORE = path.join(REPORTS, "scorecard.json");
if (!existsSync(SCORE)) {
  console.error("run tools/score.mjs first");
  process.exit(1);
}
const S = JSON.parse(readFileSync(SCORE, "utf8"));
const pct = (a, b) => (b ? Math.round((a / b) * 1000) / 10 : 0);

const L = [];
L.push("# Red-Team Triage");
L.push("");
L.push(`Base: ${S.contracts} contracts, ${S.traps} traps, recall ${S.recall} %, ${S.falsePositives} false positives on ${S.benign} benign clauses (${S.fpr} %), ${(S.uncoveredMissed || []).length} uncovered clauses without any detector.`);
L.push("");

L.push("## P1 Real detection gaps (detector has a type, corpus proves it misses)");
L.push("");
L.push("| Detektor | erkannt | Quote | Beispiel einer verfehlten Falle |");
L.push("|---|---|---|---|");
const byType = Object.entries(S.perType).sort((a, b) => a[1].rate - b[1].rate || b[1].n - a[1].n);
for (const [ty, v] of byType) {
  if (v.hit === v.n) continue;
  const miss = (S.misses || []).find((m) => m.startsWith(ty + " "));
  L.push(`| ${ty} | ${v.hit}/${v.n} | ${v.rate} % | ${miss ? miss.split(":: ")[1] || "" : ""} |`);
}
if (byType.every(([, v]) => v.hit === v.n)) L.push("| keine | | | |");
L.push("");

L.push("## P2 False positives (detector fires on conforming clause)");
L.push("");
const fpTypes = Object.entries(S.perType).filter(([, v]) => v.fp > 0).sort((a, b) => b[1].fp - a[1].fp);
if (fpTypes.length) {
  L.push("| Detektor | Fehlalarme | Beispiel |");
  L.push("|---|---|---|");
  for (const [ty, v] of fpTypes) {
    const ex = (S.fps || []).find((f) => f.split(" ")[0].split(",").includes(ty)) || "";
    L.push(`| ${ty} | ${v.fp} | ${ex.split(":: ")[1] || ""} |`);
  }
} else L.push("keine");
L.push("");

L.push("## P3 Ungedeckte Klauseln (kein Detektortyp beschreibt sie)");
L.push("");
if ((S.uncoveredMissed || []).length) {
  L.push(`${S.uncoveredMissed} von ${S.uncovered} ungedeckten Klauseln blieben vollstaendig stumm. Beispiele:`);
  L.push("");
  L.push("~~~text");
  for (const u of S.uncoveredMissed.slice(0, 40)) L.push(u);
  L.push("~~~");
} else L.push("keine");
L.push("");

L.push("## P4 Vertraege ohne einen einzigen Befund");
L.push("");
const silent = (S.rows || []).filter((r) => r.findings === 0 && r.traps > 0);
const silentClean = (S.rows || []).filter((r) => r.findings === 0 && r.traps === 0);
if (silent.length) {
  L.push(`${silent.length} Vertraege mit geplanten Fallen und 0 Befunden: ${silent.slice(0, 40).map((r) => r.slug).join(", ")}`);
  L.push("");
  L.push("Ein Vertrag mit geplanten Fallen und 0 Befunden ist ein Totalausfall der Regelbasis fuer diese Klauselsprache. Pruefen, ob die Fallen auf Paraphrase beruhen.");
} else L.push("keine");
if (silentClean.length) L.push("", `Zusaetzlich ${silentClean.length} Vertraege ohne gelabelte Falle und ohne Befund (kein Defekt): ${silentClean.map((r) => r.slug).join(", ")}`);
L.push("");

L.push("## P5 Label-Fehler (Korpusdefekt, keine Engine-Schwaeche)");
L.push("");
L.push(S.labelIssues && S.labelIssues.length ? "~~~text\n" + S.labelIssues.slice(0, 60).join("\n") + "\n~~~" : "keine");
L.push("");

L.push("## P6 Abdeckung");
L.push("");
const uncoveredTypes = Object.keys(S.perType).length;
L.push(`${uncoveredTypes} der 47 Detektortypen sind im Korpus belegt.`);
L.push("");

mkdirSync(REPORTS, { recursive: true });
writeFileSync(path.join(REPORTS, "triage.md"), L.join("\n") + "\n", "utf8");

const worst = byType.filter(([, v]) => v.n >= 2 && v.hit < v.n).slice(0, 8);
console.log(`triage.md written. weakest detectors: ${worst.map(([t, v]) => t + " " + v.hit + "/" + v.n).join(", ") || "none"}`);
console.log(`false positive detectors: ${fpTypes.map(([t, v]) => t + ":" + v.fp).join(", ") || "none"}`);
