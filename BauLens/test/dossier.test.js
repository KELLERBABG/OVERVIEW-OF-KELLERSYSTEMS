import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { runChecks } from "../src/engine.js";
import { generateDossierHtml, generateCadSvgPlan, computeReportHash } from "../src/dossier.js";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const fixture = (name) => JSON.parse(readFileSync(join(root, "test", "fixtures", name), "utf8"));

test("generateCadSvgPlan outputs valid SVG with boundary and dimension text", () => {
  const svg = generateCadSvgPlan({
    nord: 4.5,
    sued: 4.0,
    ost: 5.0,
    west: 5.0,
    reqSetback: 3.5,
    isPass: true,
  });

  assert.ok(svg.includes("<svg"));
  assert.ok(svg.includes("FLURSTÜCKSGRENZE"));
  assert.ok(svg.includes("HAUSKÖRPER"));
  assert.ok(svg.includes("N: 4.50m"));
});

test("computeReportHash produces deterministic 64-char hex string", () => {
  const spec = fixture("good_spec.json");
  const hash1 = computeReportHash(spec, "bestanden");
  const hash2 = computeReportHash(spec, "bestanden");
  assert.equal(hash1, hash2);
  assert.equal(hash1.length, 64);
});

test("computeReportHash seals one document, not one calling convention", () => {
  /* The seal has to be reproducible by whoever checks the printed dossier, so it
     may only depend on the facts. A missing field used to be null at one call
     site and undefined at another -- JSON.stringify keeps the first and drops
     the second -- so the same plan got two Prüfziffern depending on whether it
     was checked on the command line or in the desktop app. */
  const base = { projekt: "Am Hang 3", grundstueck: { flaeche_m2: 600 } };
  const withNulls = { projekt: "Am Hang 3", grundstueck: { flaeche_m2: 600 }, aktenzeichen: null, bauherr: undefined };
  const reordered = { grundstueck: { flaeche_m2: 600 }, projekt: "Am Hang 3" };

  assert.equal(computeReportHash(withNulls, "bestanden"), computeReportHash(base, "bestanden"));
  assert.equal(computeReportHash(reordered, "bestanden"), computeReportHash(base, "bestanden"));

  /* A field that carries a value still changes the seal. */
  const stamped = { projekt: "Am Hang 3", grundstueck: { flaeche_m2: 600 }, aktenzeichen: "BV-2026-42" };
  assert.notEqual(computeReportHash(stamped, "bestanden"), computeReportHash(base, "bestanden"));
  assert.notEqual(computeReportHash(base, "nicht bestanden"), computeReportHash(base, "bestanden"));
});

test("computeReportHash seals the building data the site plan draws from", () => {
  /* The Lageplan derives the drawn Hauskörper from the building data, so two
     plans that differ only there must not be able to share a Prüfziffer: the
     seal is what tells two printed sheets apart. */
  const base = { projekt: "Am Hang 3", grundstueck: { flaeche_m2: 600 }, gebaeude: { grz_ist: 0.5 } };
  const otherCoverage = { projekt: "Am Hang 3", grundstueck: { flaeche_m2: 600 }, gebaeude: { grz_ist: 0.45 } };

  assert.notEqual(
    computeReportHash(otherCoverage, "mit Auflagen"),
    computeReportHash(base, "mit Auflagen"),
    "a different GRZ draws a different Hauskörper and must change the seal"
  );
  assert.equal(
    computeReportHash({ ...base, gebaeude: { grz_ist: 0.5 } }, "mit Auflagen"),
    computeReportHash(base, "mit Auflagen"),
    "the same building data still seals the same"
  );
});

test("generateDossierHtml produces complete DIN A4 official dossier", () => {
  const spec = fixture("good_spec.json");
  const runResult = runChecks(spec);
  const html = generateDossierHtml(spec, runResult, { datum: "02.10.2026" });

  assert.ok(html.includes("Bauaufsichtlicher Vorprüfungsbefund"));
  assert.ok(html.includes(spec.projekt));
  assert.ok(html.includes("DIGITALES PRÜFSIEGEL (SHA-256)"));
  assert.ok(html.includes("Unterschrift bauvorlageberechtigte(r) Entwurfsverfasser(in)"));
  assert.ok(html.includes("<svg"));
});
