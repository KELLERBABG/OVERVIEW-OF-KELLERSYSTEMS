import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { detectKind, ingestFile, KINDS } from "../src/ingest.js";
import { handleRequest, METHODS } from "../src/ipc.js";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const fixturesDir = join(root, "test", "fixtures");
const fixturePath = (name) => join(fixturesDir, name);
const fixtureText = (name) => readFileSync(fixturePath(name), "utf8");

/* The desktop workhorse checks whole folders, so ingestion is the part that
   decides what a file *is*. These tests pin that mapping down: a wrong guess
   here silently produces a plausible-looking verdict from the wrong data. */

test("ingest: file kinds are decided by extension and, where ambiguous, content", () => {
  assert.equal(detectKind("modell.ifc"), "ifc");
  assert.equal(detectKind("lageplan.dxf"), "dxf");
  assert.equal(detectKind("plan.gml"), "xplan");
  assert.equal(detectKind("plan.xml"), "xplan");
  assert.equal(detectKind("grenzen.geojson"), "bplan_geojson");
  assert.equal(detectKind("vorhaben.deploy"), "unsupported");

  /* A .json is a spec when it carries spec keys ... */
  assert.equal(detectKind("spec.json", '{"gebaeude":{"hoehe_m":8}}'), "spec");
  /* ... and a B-Plan when it is a GeoJSON FeatureCollection. */
  const geojson = JSON.stringify({ type: "FeatureCollection", features: [] });
  assert.equal(detectKind("baugrenze.json", geojson), "bplan_geojson");

  /* A .txt is a Textteil when it says so, an Exposé otherwise. */
  assert.equal(detectKind("teil-b.txt", "Festsetzungen des Bebauungsplans, GRZ 0,4"), "bplan_text");
  assert.equal(detectKind("expose.txt", "Neubau eines Einfamilienhauses, Wohnfläche 160 m²"), "text");
});

test("ingest: every returned kind is one of the declared kinds", () => {
  for (const name of ["sample_cad_lageplan.dxf", "sample_bim_lod200.ifc", "sample_baubeschreibung_expose.txt", "good_spec.json"]) {
    const ingested = ingestFile(fixturePath(name), fixtureText(name));
    assert.ok(KINDS.includes(ingested.kind), `${name} -> ${ingested.kind}`);
    assert.equal(typeof ingested.spec, "object");
    assert.ok(ingested.source, `${name} names its source`);
    assert.equal(ingested.project, name);
  }
});

test("ingest: a DXF becomes plot area, setbacks and GRZ - and admits what it guessed", () => {
  const ingested = ingestFile("lageplan.dxf", fixtureText("sample_cad_lageplan.dxf"));
  assert.equal(ingested.kind, "dxf");
  assert.ok(ingested.spec.grundstueck.flaeche_m2 > 0);
  assert.ok(Object.keys(ingested.spec.grundstueck.grenzabstaende).length >= 4);
  assert.ok(ingested.spec.gebaeude.grz_ist > 0 && ingested.spec.gebaeude.grz_ist <= 1);

  /* Height cannot come from a 2D plot plan, so it must be reported as missing
     rather than invented. */
  assert.equal(ingested.spec.gebaeude.hoehe_m, undefined);
  const notes = ingested.notes.join(" ");
  assert.match(notes, /Gebäudehöhe/);
});

test("ingest: an IFC becomes storeys, height, units and parking", () => {
  const ingested = ingestFile("modell.ifc", fixtureText("sample_bim_lod200.ifc"));
  assert.equal(ingested.kind, "ifc");
  assert.equal(ingested.source, "IFC-BIM-Modell");
  assert.ok(ingested.spec.gebaeude.hoehe_m > 0);
  assert.ok(ingested.spec.gebaeude.geschosse_ueber_grund >= 1);
  assert.ok(Number.isInteger(ingested.spec.gebaeude.wohnungen_anzahl));
  assert.match(ingested.notes.join(" "), /Bundesland/);
});

test("ingest: the unsupported case and a broken file fail with a code, not a crash", () => {
  assert.throws(() => ingestFile("plan.dwg", "whatever"), (error) => error.code === "unsupported_file");
  assert.throws(() => ingestFile("spec.json", "{not json"), (error) => error.code === "invalid_spec");
});

test("ipc: check_file is part of the protocol and reads a real fixture", () => {
  assert.ok(METHODS.includes("check_file"));

  const response = handleRequest({
    id: "f1",
    method: "check_file",
    params: { path: fixturePath("sample_bim_lod200.ifc"), land: "BY", repair: true },
  });

  assert.equal(response.ok, true, JSON.stringify(response.error ?? {}));
  const result = response.result;
  assert.equal(result.kind, "ifc");
  assert.equal(result.profile.code, "BY");
  assert.ok(["bestanden", "mit Auflagen", "nicht bestanden"].includes(result.verdict));
  assert.ok(Array.isArray(result.findings) && result.findings.length > 0);
  assert.ok(Array.isArray(result.repairs));
  assert.ok(Array.isArray(result.notes) && result.notes.length > 0);

  /* The ingestion trace travels with the result: the shell renders it, and a
     verdict that rests on guessed values has to say so. */
  assert.equal(typeof result.source, "string");
});

test("ipc: check_file refuses a missing path, an unreadable file and an unknown format", () => {
  const missing = handleRequest({ id: 1, method: "check_file", params: {} });
  assert.equal(missing.ok, false);
  assert.equal(missing.error.code, "missing_path");

  const gone = handleRequest({ id: 2, method: "check_file", params: { path: join(fixturesDir, "nope.json") } });
  assert.equal(gone.ok, false);
  assert.equal(gone.error.code, "file_unreadable");

  const unsupported = handleRequest({
    id: 3,
    method: "check_file",
    params: { path: fixturePath("sample_baubeschreibung_expose.txt").replace(/\.txt$/, ".dwg") },
  });
  assert.equal(unsupported.ok, false);
  assert.ok(["unsupported_file", "file_unreadable"].includes(unsupported.error.code));
});
