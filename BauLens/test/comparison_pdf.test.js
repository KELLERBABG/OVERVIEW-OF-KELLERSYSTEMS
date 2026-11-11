import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { runChecks } from "../src/engine.js";
import { buildDossierPdf } from "../src/dossierpdf.js";
import { handleRequest } from "../src/ipc.js";

/* The Vorher-/Nachher-Blatt is what a reviewer reads next to the Befund, so the
   tests check the facts it prints (the variant's Prüfziffer, the label, the
   section itself) against the same engine run the sidecar uses, not against a
   copy of the layout. */

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const fixturePath = (name) => join(root, "test", "fixtures", name);
const fixture = (name) => JSON.parse(readFileSync(fixturePath(name), "utf8"));

const DATUM = "02.10.2026";
const SECTION = Buffer.from("Vorher-/Nachher-Vergleich", "latin1");

/* A Heilungsvariante: the Grenzabstände of the failing fixture are widened.
   The spec differs from the base, so its Prüfziffer must differ as well. */
function variantSpec() {
  const spec = fixture("bad_spec.json");
  spec.grundstueck.grenzabstaende = { norden: 6.0, sueden: 5.0 };
  return spec;
}

test("comparison: the Befund has no Vergleichsblatt without a variant", () => {
  const spec = fixture("bad_spec.json");
  const run = runChecks(spec);
  const projectData = { projekt: spec.projekt, ...spec };
  const built = buildDossierPdf(projectData, run, { datum: DATUM });

  assert.equal(built.buffer.includes(SECTION), false, "no comparison section without options.comparison");
});

test("comparison: a variant adds the section with its label and Prüfziffer", () => {
  const base = fixture("bad_spec.json");
  const baseRun = runChecks(base);
  const baseData = { projekt: base.projekt, ...base };

  const variant = variantSpec();
  const variantRun = runChecks(variant);
  const variantData = { projekt: variant.projekt, ...variant };

  const built = buildDossierPdf(baseData, baseRun, {
    datum: DATUM,
    comparison: { projectData: variantData, runResult: variantRun, label: "Grenzabstand 6 m" },
  });

  assert.ok(built.buffer.includes(SECTION), "the section is drawn");
  assert.ok(built.buffer.includes(Buffer.from("Grenzabstand 6 m", "latin1")), "the label is printed");
  assert.ok(built.buffer.includes(Buffer.from("Prüfziffer der Variante", "latin1")), "the variant's Prüfziffer line is printed");
  assert.equal(built.pages >= 1, true);
});

test("comparison: dossier_pdf with params.variant builds a different dossier than without", () => {
  const plain = handleRequest({ id: 1, method: "dossier_pdf", params: { file: fixturePath("bad_spec.json"), datum: DATUM } });
  assert.equal(plain.ok, true, plain.error && plain.error.message);

  const withVariant = handleRequest({
    id: 2,
    method: "dossier_pdf",
    params: {
      file: fixturePath("bad_spec.json"),
      datum: DATUM,
      variant: { spec: variantSpec(), label: "Grenzabstand 6 m" },
    },
  });
  assert.equal(withVariant.ok, true, withVariant.error && withVariant.error.message);

  assert.equal(withVariant.result.hash, plain.result.hash, "the Befund's Prüfziffer is the base's, with or without a variant");
  assert.notEqual(withVariant.result.base64, plain.result.base64, "the bytes differ by the comparison section");
  const bytes = Buffer.from(withVariant.result.base64, "base64");
  assert.ok(bytes.includes(SECTION), "the variant request prints the comparison");
  assert.equal(Buffer.from(plain.result.base64, "base64").includes(SECTION), false, "the plain request does not");
});

test("comparison: a variant without a spec is rejected, not silently ignored", () => {
  const response = handleRequest({
    id: 3,
    method: "dossier_pdf",
    params: { file: fixturePath("bad_spec.json"), datum: DATUM, variant: { label: "ohne Spec" } },
  });
  assert.equal(response.ok, false);
});
