"use strict";

/**
 * ClauseLens Stress-Korpus: Detektionslaeufe gegen src/analyze.js
 * =============================================================
 *
 * Fuehrt alle Korpus-Slices unter `test/stress/corpus/*.js` durch die Erkennungs-
 * regelmaschine und beantwortet zwei Fragen:
 *
 *   1. Werden taueschende Klauseln erkannt (Blindspot-Suche)?
 *   2. Bleiben harmlose, gesetzeskonforme Klauseln unbeanstandet (False-Positive-Suche)?
 *
 * Der Test schreibt `test/stress/report.md` (lesbarer Befund) und
 * `test/stress/report.json` (maschinenlesbar) neu. Beide Artefakte sind bewusst
 * deterministisch (kein Zeitstempel), damit ein Testlauf den Arbeitsbaum nicht
 * beschmutzt.
 *
 * Die Schwellen unten sind GEMESSENE Basiswerte dieses adversarialen Korpus und
 * dienen als Regressionszaun: sie verhindern Rueckschritte, behaupten aber
 * ausdruecklich nicht, dass die Erkennungsquote gut ist. Der Korpus enthaelt
 * absichtlich Evasionstechniken (Paraphrase, Nominalstil, Satztrennung,
 * ausgeschriebene Zahlen, Englisch, Typo-Evasion), die eine rein literale
 * Regex-Engine strukturell nicht abdecken kann. Wer die Detektoren haertet,
 * hebt diese Schwellen mit an.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import {
  EXPECTED_DETECTORS,
  validateCorpus,
  runCorpus,
  percent,
  blindspots,
  falsePositiveCases,
  formatReport,
  serializeReport,
} from "./harness.js";

const here = path.dirname(fileURLToPath(import.meta.url));
const corpusDir = path.join(here, "corpus");
const reportMdPath = path.join(here, "report.md");
const reportJsonPath = path.join(here, "report.json");

const FENCES = {
  minCases: 360,
  minDeceptiveCases: 280,
  minCompliantCases: 80,
  deceptiveDetectionRate: 0.985,
  deceptiveSeverityRate: 0.985,
  directDetectionRate: 0.95,
  compliantCleanRate: 1.0,
  maxRuntimeMs: 5000,
};

/**
 * Bekannte, dokumentierte Regelluecken bei unverschleierten (technique "direct")
 * Klauseln. Beide historischen Luecken sind inzwischen geschlossen:
 *
 *  - acc-direct-24 (vicarious_agents "für das Verschulden seiner Erfüllungsgehilfen
 *    … ist ausgeschlossen") feuert nach der Wave-A-Haertung.
 *  - bind-direct-32 (insolvency_termination, Insolvenzantrag vor Kündigungsrecht)
 *    feuert nach der Wave-A-Haertung.
 *
 * Der Test verlangt, dass jede neue Klartext-Luecke die Suite fallen laesst; die
 * Liste ist daher leer. Wird kuenftig eine Luecke bewusst akzeptiert, muss sie
 * hier mit Begruendung eingetragen werden.
 */
const KNOWN_DIRECT_GAPS = new Set([]);

async function loadSlices() {
  const files = readdirSync(corpusDir)
    .filter((f) => f.endsWith(".js"))
    .sort();
  assert.ok(files.length >= 8, `Erwartet mindestens 8 Korpus-Slices, gefunden: ${files.length}`);
  const slices = [];
  for (const file of files) {
    const mod = await import(pathToFileURL(path.join(corpusDir, file)).href);
    const cases = mod.CASES || mod.default;
    assert.ok(Array.isArray(cases), `${file}: muss CASES als Array exportieren`);
    slices.push({ name: file, cases });
  }
  return slices;
}

let runResult = null;
let caseCount = 0;

test("stress corpus: schema, Detektor-Abdeckung und Engine-Integration", async () => {
  const slices = await loadSlices();
  const { errors, cases } = validateCorpus(slices);
  caseCount = cases.length;

  assert.deepEqual(errors, [], `Korpus-Schemafehler:\n${errors.join("\n")}`);
  assert.ok(cases.length >= FENCES.minCases, `Korpus zu klein: ${cases.length} < ${FENCES.minCases}`);

  const deceptive = cases.filter((c) => c.variant === "deceptive");
  const compliant = cases.filter((c) => c.variant === "compliant");
  assert.ok(deceptive.length >= FENCES.minDeceptiveCases, `Zu wenige taueschende Faelle: ${deceptive.length}`);
  assert.ok(compliant.length >= FENCES.minCompliantCases, `Zu wenige harmlose Faelle: ${compliant.length}`);

  const covered = new Set(deceptive.map((c) => c.detector));
  const uncovered = EXPECTED_DETECTORS.filter((d) => !covered.has(d));
  assert.deepEqual(uncovered, [], `Detektoren ohne Testfall: ${uncovered.join(", ")}`);

  const t0 = Date.now();
  runResult = runCorpus(cases);
  const runtimeMs = Date.now() - t0;
  const stats = runResult.stats;

  assert.equal(stats.errors, 0, "Die Engine darf auf keinem Korpus-Fall eine Exception werfen");
  assert.ok(
    runtimeMs < FENCES.maxRuntimeMs,
    `Korpus-Lauf zu langsam: ${runtimeMs}ms >= ${FENCES.maxRuntimeMs}ms (ReDoS-/Performance-Regression?)`
  );

  assert.ok(
    stats.deceptive.detectionRate >= FENCES.deceptiveDetectionRate,
    `Detektionsquote gefallen: ${percent(stats.deceptive.detectionRate)} < ${percent(FENCES.deceptiveDetectionRate)}`
  );
  assert.ok(
    stats.deceptive.severityRate >= FENCES.deceptiveSeverityRate,
    `Schweregrad-Quote gefallen: ${percent(stats.deceptive.severityRate)} < ${percent(FENCES.deceptiveSeverityRate)}`
  );
  assert.ok(
    stats.directDeceptive.detectionRate >= FENCES.directDetectionRate,
    `Klartext-Fallen muessen erkannt werden: ${percent(stats.directDeceptive.detectionRate)} < ${percent(FENCES.directDetectionRate)}`
  );

  // Kein NEUER Klartext-Blindspot: nur die dokumentierten Regelluecken sind erlaubt.
  const newDirectGaps = runResult.results.filter(
    (r) => r.variant === "deceptive" && r.technique === "direct" && !r.detected && !KNOWN_DIRECT_GAPS.has(r.id)
  );
  assert.deepEqual(
    newDirectGaps.map((r) => `${r.id} (${r.detector}): ${r.reason}`),
    [],
    "Neue Klartext-Regelluecken aufgetreten"
  );
  const closedGaps = [...KNOWN_DIRECT_GAPS].filter(
    (id) => !runResult.results.some((r) => r.id === id && !r.detected)
  );
  if (closedGaps.length > 0) {
    console.log(`Hinweis: ${closedGaps.length} dokumentierte Regelluecke(n) inzwischen geschlossen (${closedGaps.join(", ")}) — KNOWN_DIRECT_GAPS kann verkleinert werden.`);
  }
  assert.ok(
    stats.compliant.cleanRate >= FENCES.compliantCleanRate,
    `Zu viele False Positives auf harmlosen Klauseln: sauber ${percent(stats.compliant.cleanRate)} < ${percent(FENCES.compliantCleanRate)}`
  );

  // Artefakte deterministisch neu schreiben
  writeFileSync(reportMdPath, formatReport(runResult), "utf8");
  writeFileSync(reportJsonPath, `${JSON.stringify(serializeReport(runResult), null, 2)}\n`, "utf8");

  // Konsolenbefund fuer den Testlauf
  console.log("");
  console.log(`Stress-Korpus: ${stats.total} Faelle (${deceptive.length} täuschend, ${compliant.length} harmlos) · ${runtimeMs}ms`);
  console.log(
    `  Erkennung: ${stats.deceptive.detected}/${deceptive.length} (${percent(stats.deceptive.detectionRate)}) · ` +
      `ausreichend scharf: ${stats.deceptive.severityOk} (${percent(stats.deceptive.severityRate)}) · ` +
      `stille Blindspots: ${stats.deceptive.silentMiss}`
  );
  console.log(
    `  Klartext-Kontrollgruppe: ${stats.directDeceptive.detected}/${stats.directDeceptive.total} (${percent(stats.directDeceptive.detectionRate)})`)
  ;
  console.log(
    `  Harmlose Klauseln sauber: ${stats.compliant.clean}/${compliant.length} (${percent(stats.compliant.cleanRate)}) · ` +
      `False-Positive-Faelle: ${stats.compliant.falsePositiveCases}`
  );
  console.log("  Schwaechste Detektoren:");
  for (const [key, d] of Object.entries(stats.byDetector)
    .sort((a, b) => b[1].failed - a[1].failed || a[0].localeCompare(b[0]))
    .slice(0, 8)) {
    console.log(`    ${String(d.failed).padStart(2)} Fehlschläge von ${String(d.total).padStart(2)}  ${key}`);
  }
  console.log("  Schwaechste Evasionstechniken:");
  for (const [key, t] of Object.entries(stats.byTechnique)
    .sort((a, b) => b[1].failed - a[1].failed || a[0].localeCompare(b[0]))
    .slice(0, 6)) {
    console.log(`    ${String(t.failed).padStart(2)} fehlend von ${String(t.total).padStart(2)}  ${key}`);
  }
  const worst = falsePositiveCases(runResult.results);
  if (worst.length > 0) {
    console.log(`  False Positives (harmlose Klauseln, ${worst.length}):`);
    for (const r of worst) console.log(`    ${r.id}  ${r.reason}`);
  }
  console.log(`  Blindspot-Liste: ${blindspots(runResult.results).length} Einträge in test/stress/report.md`);
  console.log("");
});

test("stress corpus: Berichtsartefakte sind vollständig und konsistent", () => {
  assert.ok(runResult, "Der Korpus-Lauf muss zuerst ausgefuehrt worden sein");

  const md = readFileSync(reportMdPath, "utf8");
  assert.match(md, /# ClauseLens Stress-Korpus/, "report.md braucht den Kopf");
  assert.match(md, /## Blindspots \(\d+\)/, "report.md braucht die Blindspot-Sektion");
  assert.match(md, /## False Positives auf harmlosen Klauseln \(\d+\)/, "report.md braucht die False-Positive-Sektion");

  const json = JSON.parse(readFileSync(reportJsonPath, "utf8"));
  assert.equal(json.schemaVersion, "1.0.0");
  assert.equal(json.results.length, caseCount, "report.json muss jeden Korpus-Fall enthalten");
  assert.equal(json.stats.total, caseCount);
  assert.equal(json.stats.passed + json.stats.failed, caseCount);

  const failing = json.results.filter((r) => !r.pass);
  assert.ok(failing.length > 0, "Ein adversarialer Korpus ohne jeden Fehlschlag misst nichts mehr");

  const documented = new Set(blindspots(runResult.results).map((r) => r.id));
  const inJson = new Set(json.results.filter((r) => !r.pass && r.variant === "deceptive").map((r) => r.id));
  assert.deepEqual([...documented].sort(), [...inJson].sort(), "report.json und Blindspot-Liste müssen übereinstimmen");
});
