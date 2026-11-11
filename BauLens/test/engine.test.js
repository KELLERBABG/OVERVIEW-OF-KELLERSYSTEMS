import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { runChecks, VERDICT } from "../src/engine.js";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const fixture = (name) => JSON.parse(readFileSync(join(root, "test", "fixtures", name), "utf8"));

const countStatus = (runResult, status) =>
  runResult.results.reduce(
    (acc, r) => acc + r.findings.filter((f) => f.status === status).length,
    0
  );

test("Good-Spec laeuft komplett durch mit Urteil 'bestanden'", () => {
  const res = runChecks(fixture("good_spec.json"));
  assert.equal(res.verdict, VERDICT.BESTANDEN);
  assert.equal(res.counts.fail, 0);
  assert.equal(res.counts.warn, 0);
  assert.equal(res.results.length, 9);
  for (const r of res.results) {
    assert.equal(typeof r.id, "string");
    assert.ok(r.para.includes("§"));
    assert.ok(r.findings.length > 0);
  }
});

test("Bad-Spec erzeugt mindestens 4 Fehler und Urteil 'nicht bestanden'", () => {
  const res = runChecks(fixture("bad_spec.json"));
  assert.equal(res.verdict, VERDICT.NICHT_BESTANDEN);
  const fails = countStatus(res, "fail");
  assert.ok(fails >= 4, `erwartet >= 4 Fehler, erhalten ${fails}`);
});

test("Bad-Spec enthaelt die geforderten Regelverstoesse", () => {
  const res = runChecks(fixture("bad_spec.json"));
  const idsMitFehler = new Set(
    res.results.flatMap((r) =>
      r.findings.filter((f) => f.status === "fail").map(() => r.id)
    )
  );
  for (const id of ["abstandsflaeche", "tageslicht", "raumhoehen", "rettungswege"]) {
    assert.ok(idsMitFehler.has(id), `Regel ${id} sollte einen Fehler liefern`);
  }
});

test("Warn-only-Spec ergibt 'mit Auflagen' und Exit-Logik 0", () => {
  const spec = { gebaeude: { hoehe_m: 8, geschosse_ueber_grund: 2, grz_ist: 0.5 } };
  const res = runChecks(spec);
  assert.equal(res.verdict, VERDICT.AUFLAGEN);
  assert.equal(res.counts.fail, 0);
  assert.ok(res.counts.warn > 0);
});

test("--rules-Filter beschraenkt die ausgefuehrten Regeln", () => {
  const res = runChecks(fixture("bad_spec.json"), { rules: ["tageslicht"] });
  assert.equal(res.results.length, 1);
  assert.equal(res.results[0].id, "tageslicht");
  assert.equal(res.verdict, VERDICT.NICHT_BESTANDEN);
});

test("Unbekannter Regelfilter liefert keine Ergebnisse (bestanden per Leerlauf)", () => {
  const res = runChecks(fixture("good_spec.json"), { rules: ["gibtsnicht"] });
  assert.equal(res.results.length, 0);
  assert.equal(res.verdict, VERDICT.BESTANDEN);
});
