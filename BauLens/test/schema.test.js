import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { validateSpec } from "../src/schema.js";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const fixture = (name) => JSON.parse(readFileSync(join(root, "test", "fixtures", name), "utf8"));

test("Schema akzeptiert das vollstaendige Good-Spec ohne Fehler", () => {
  const res = validateSpec(fixture("good_spec.json"));
  assert.equal(res.ok, true);
  assert.deepEqual(res.errors, []);
});

test("Bad-Spec ist schema-gueltig (Verstoesse sind Regel-, keine Schemafaelle)", () => {
  const res = validateSpec(fixture("bad_spec.json"));
  assert.equal(res.ok, true);
  assert.ok(res.warnings.length > 0);
});

test("Nicht-Objekt-Eingaben werden abgelehnt", () => {
  for (const bad of [null, [], 42, "spec"]) {
    const res = validateSpec(bad);
    assert.equal(res.ok, false);
    assert.ok(res.errors.length > 0);
  }
});

test("Falscher Datentyp bei hoehe_m ist ein Fehler", () => {
  const res = validateSpec({ gebaeude: { hoehe_m: "sieben Meter" } });
  assert.equal(res.ok, false);
  assert.ok(res.errors.some((e) => e.includes("hoehe_m")));
});

test("Negative Flaechen sind ein Fehler", () => {
  const res = validateSpec({ grundstueck: { flaeche_m2: -100 } });
  assert.equal(res.ok, false);
  assert.ok(res.errors.some((e) => e.includes("flaeche_m2")));
});

test("Negativer Grenzabstand ist ein Fehler", () => {
  const res = validateSpec({ grundstueck: { grenzabstaende: { norden: -1 } } });
  assert.equal(res.ok, false);
  assert.ok(res.errors.some((e) => e.includes("norden")));
});

test("grz_ist ausserhalb [0,1] ist ein Fehler", () => {
  const res = validateSpec({ gebaeude: { grz_ist: 1.5 } });
  assert.equal(res.ok, false);
  assert.ok(res.errors.some((e) => e.includes("grz_ist")));
});

test("aufenthaltsraeume als Nicht-Array ist ein Fehler", () => {
  const res = validateSpec({ gebaeude: { aufenthaltsraeume: "drei Raeume" } });
  assert.equal(res.ok, false);
  assert.ok(res.errors.some((e) => e.includes("aufenthaltsraeume")));
});

test("Boolean-Felder mit falschem Typ sind Fehler", () => {
  const res = validateSpec({ gebaeude: { treppenraum_vorhanden: "ja" } });
  assert.equal(res.ok, false);
  assert.ok(res.errors.some((e) => e.includes("treppenraum_vorhanden")));
});

test("Fehlende optionale Felder erzeugen Warnungen, keine Fehler", () => {
  const res = validateSpec({});
  assert.equal(res.ok, true);
  assert.ok(res.warnings.some((w) => w.includes("grundstueck")));
  assert.ok(res.warnings.some((w) => w.includes("gebaeude")));
});

test("Unbekannte Felder (Tippfehler) erzeugen Warnungen", () => {
  const res = validateSpec({ gebaeude: { hoee_m: 8 } });
  assert.equal(res.ok, true);
  assert.ok(res.warnings.some((w) => w.includes("hoee_m")));
});

test("Ungueltiges grz_limit in einstellungen ist ein Fehler", () => {
  const res = validateSpec({ einstellungen: { grz_limit: 2 } });
  assert.equal(res.ok, false);
});

test("Gueltiges gfz_ist und stellplaetze_ist werden akzeptiert", () => {
  const res = validateSpec({
    gebaeude: { gfz_ist: 1.2, stellplaetze_ist: 5, fahrradstellplaetze_ist: 10 },
    einstellungen: { gfz_limit: 1.6, pkw_stellplatz_faktor: 1.0 },
  });
  assert.equal(res.ok, true);
});

test("Negatives gfz_ist oder unzulaessiges gfz_limit erzeugen Fehler", () => {
  const res1 = validateSpec({ gebaeude: { gfz_ist: -0.5 } });
  assert.equal(res1.ok, false);
  const res2 = validateSpec({ einstellungen: { gfz_limit: -1 } });
  assert.equal(res2.ok, false);
});

