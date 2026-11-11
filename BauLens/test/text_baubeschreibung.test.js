import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseBaubeschreibung } from '../src/textparser.js';
import { runChecks, VERDICT } from '../src/engine.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const fixturesDir = path.join(__dirname, 'fixtures');

test('Text Ingestion: parses real German construction specification text fixture', () => {
  const textFile = path.join(fixturesDir, 'sample_baubeschreibung_expose.txt');
  const content = fs.readFileSync(textFile, 'utf-8');

  const { spec, confidence, extractedKeys } = parseBaubeschreibung(content);

  assert.ok(confidence >= 0.7);
  assert.equal(spec.bundesland, 'BY');
  assert.equal(spec.grundstueck.flaeche_m2, 920.0);
  assert.equal(spec.gebaeude.hoehe_m, 8.4);
  assert.equal(spec.gebaeude.geschosse_ueber_grund, 3);
  assert.equal(spec.gebaeude.wohnungen_anzahl, 6);
  assert.equal(spec.gebaeude.grz_ist, 0.31);
  assert.equal(spec.gebaeude.gfz_ist, 0.88);
  assert.equal(spec.grundstueck.grenzabstaende.norden, 5.4);
  assert.equal(spec.grundstueck.grenzabstaende.sueden, 4.8);
  assert.equal(spec.grundstueck.grenzabstaende.osten, 4.2);
  assert.equal(spec.grundstueck.grenzabstaende.westen, 4.0);
  assert.equal(spec.gebaeude.stellplaetze_ist, 6);

  // Run the parsed spec directly through compliance engine
  const evalRes = runChecks(spec);
  assert.equal(evalRes.counts.fail, 0);
  assert.ok(evalRes.verdict === VERDICT.BESTANDEN || evalRes.verdict === VERDICT.AUFLAGEN);
});

test('Text Ingestion: handles messy real-world OCR text with formatting quirks', () => {
  const messyText = `
    Bauvorhaben: Neubau EFH in 70173 Stuttgart, Baden-Württemberg.
    Das Grundstueck umfasst 650,50 qm Fläche.
    Wandhöhe beträgt ca. 6,80m bei 2 Vollgeschossen und 1 Wohneinheit.
    GRZ: 0,25. Abstände: Nord 4,50m, Süd 5,00m, Ost 3,80m, West 4,00m.
    2 Stellplätze nachgewiesen.
  `;

  const { spec } = parseBaubeschreibung(messyText);
  assert.equal(spec.bundesland, 'BW');
  assert.equal(spec.grundstueck.flaeche_m2, 650.5);
  assert.equal(spec.gebaeude.hoehe_m, 6.8);
  assert.equal(spec.gebaeude.geschosse_ueber_grund, 2);
  assert.equal(spec.gebaeude.wohnungen_anzahl, 1);
  assert.equal(spec.gebaeude.grz_ist, 0.25);
});
