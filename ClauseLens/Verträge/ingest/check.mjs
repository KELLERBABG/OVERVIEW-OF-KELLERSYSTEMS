"use strict";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { extractTextFromBuffer } from "../../src/textio.js";
import { analyzeContract } from "../../src/analyze.js";

const here = dirname(fileURLToPath(import.meta.url));
const cases = JSON.parse(readFileSync(join(here, "cases.json"), "utf8")).cases;

const rows = [];
const failures = [];

for (const c of cases) {
  const abs = join(here, c.file);
  const row = { id: c.id, file: c.file, format: c.format, result: "PASS", detail: "", types: [], defects: c.defect ? [c.defect] : [] };
  if (!existsSync(abs)) {
    row.result = "FAIL";
    row.detail = "fixture missing, run node Verträge/ingest/build.mjs";
    failures.push({ ...row, expected: "fixture exists", actual: "missing", classification: "bad fixture" });
    rows.push(row);
    continue;
  }

  const buf = readFileSync(abs);
  let text = null;
  let err = null;
  try {
    text = extractTextFromBuffer(buf, c.format, c.file).text;
  } catch (e) {
    err = e;
  }

  if (c.expectedError || c.expectedErrorIncludes) {
    const want = c.expectedError || c.expectedErrorIncludes;
    const okMsg = err && (c.expectedError ? err.message === want : err.message.includes(want));
    const okClass = !c.expectedErrorClass || (err && err.constructor.name === c.expectedErrorClass);
    if (okMsg && okClass) {
      row.detail = err.message;
    } else {
      row.result = "FAIL";
      row.detail = err ? err.message : "no error thrown, text: " + JSON.stringify(String(text).slice(0, 120));
      failures.push({ ...row, expected: (c.expectedErrorClass ? c.expectedErrorClass + ": " : "") + want, actual: err ? err.constructor.name + ": " + err.message : "text of length " + String(text).length, classification: "unclassified" });
    }
    rows.push(row);
    continue;
  }

  if (err) {
    row.result = "FAIL";
    row.detail = err.constructor.name + ": " + err.message;
    failures.push({ ...row, expected: "extracted text", actual: err.constructor.name + ": " + err.message, classification: "unclassified" });
    rows.push(row);
    continue;
  }

  const problems = [];
  if (c.expectedText !== undefined && text !== c.expectedText) problems.push("text differs: got " + JSON.stringify(text.slice(0, 200)) + " want " + JSON.stringify(c.expectedText.slice(0, 200)));
  for (const s of c.expectedContains || []) if (!text.includes(s)) problems.push("missing substring " + JSON.stringify(s));
  for (const s of c.expectedNotContains || []) if (text.includes(s)) problems.push("unexpected substring " + JSON.stringify(s));
  if (c.expectedLengthMin && text.length < c.expectedLengthMin) problems.push("length " + text.length + " below " + c.expectedLengthMin);
  if (c.expectedLengthMax && text.length > c.expectedLengthMax) problems.push("length " + text.length + " above " + c.expectedLengthMax);

  const types = [...new Set(analyzeContract(text).findings.map((f) => f.type))];
  row.types = types;
  for (const t of c.expectedTypes || []) if (!types.includes(t)) problems.push("detector " + t + " did not fire");
  for (const t of c.missingTypes || []) if (types.includes(t)) problems.push("detector " + t + " fired although the clause is unreachable");

  if (problems.length) {
    row.result = "FAIL";
    row.detail = problems.join("; ");
    failures.push({ ...row, expected: describeExpectation(c), actual: problems.join("; "), classification: "unclassified" });
  }
  rows.push(row);
}

function describeExpectation(c) {
  const parts = [];
  if (c.expectedText !== undefined) parts.push("text " + JSON.stringify(c.expectedText.slice(0, 120)));
  if (c.expectedContains) parts.push("contains " + JSON.stringify(c.expectedContains));
  if (c.expectedLengthMin) parts.push("length >= " + c.expectedLengthMin);
  if (c.expectedTypes) parts.push("detectors " + c.expectedTypes.join(", "));
  if (c.missingTypes) parts.push("no detector " + c.missingTypes.join(", "));
  return parts.join(" | ");
}

const passed = rows.filter((r) => r.result === "PASS").length;
const failed = rows.length - passed;
const defects = cases.filter((c) => c.defect);
const calibration = JSON.parse(readFileSync(join(here, "cases.json"), "utf8")).calibration || [];

const lines = [];
lines.push("# Ingest-Faelle");
lines.push("");
lines.push("Fixture generator: node \"Vertraege/ingest/build.mjs\"");
lines.push("Checker: node \"Vertraege/ingest/check.mjs\"");
lines.push("");
lines.push("Ergebnis: " + passed + " von " + rows.length + " Faellen bestanden, " + failed + " Fehler.");
lines.push("");
lines.push("## Faelle");
lines.push("");
lines.push("| Fall | Fixture | Ausgabe | Ergebnis | Detektoren |");
lines.push("| --- | --- | --- | --- | --- |");
for (const r of rows) lines.push("| " + r.id + " | " + r.file + " | " + r.format + " | " + r.result + " | " + (r.types.length ? r.types.join(", ") : (r.result === "FAIL" ? r.detail : "keine")) + " |");
lines.push("");
lines.push("## Fehlerdetails");
lines.push("");
if (!failures.length) {
  lines.push("Keine. Alle extrahierten Texte und alle erwarteten Detektoren stimmen.");
} else {
  for (const f of failures) {
    lines.push("- " + f.id + " (" + f.file + ")");
    lines.push("  - erwartet: " + f.expected);
    lines.push("  - tatsaechlich: " + f.actual);
    lines.push("  - Einstufung: " + f.classification);
  }
}
lines.push("");
lines.push("Einstufungsregel: eine Abweichung ist ein Extraktor-Defekt, wenn der extrahierte Text rechtlich falsch oder unvollstaendig ist, und eine falsche Erwartung, wenn Fixture oder Erwartungstext falsch waren.");
lines.push("");
lines.push("## Dokumentierte Defekte");
lines.push("");
for (const d of defects) {
  lines.push("- " + d.id + " (" + d.file + "): " + d.defect);
}
lines.push("");
lines.push("## Kalibrierung");
lines.push("");
for (const k of calibration) {
  lines.push("- " + k.id + ": erste Erwartung " + k.first + ", tatsaechlich " + k.actual + ". Einstufung: " + k.classification);
}
lines.push("");

writeFileSync(join(here, "report.md"), lines.join("\n") + "\n");

for (const r of rows) console.log((r.result === "PASS" ? "PASS " : "FAIL ") + r.id.padEnd(32) + (r.detail ? r.detail.slice(0, 110) : ""));
console.log("check: " + rows.length + " cases, " + passed + " passed, " + failed + " failed");
console.log("report: Vertraege/ingest/report.md");
process.exitCode = failed ? 1 : 0;
