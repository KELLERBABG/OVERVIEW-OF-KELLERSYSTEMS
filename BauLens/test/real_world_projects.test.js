import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { runChecks, VERDICT } from '../src/engine.js';
import { validateBuildingSpec } from '../src/schema.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const fixturesDir = path.join(__dirname, 'fixtures');

function loadJsonFixture(name) {
  const file = path.join(fixturesDir, name);
  return JSON.parse(fs.readFileSync(file, 'utf-8'));
}

test('Real-World Project: Munich Residential MFH (BayBO 2026)', () => {
  const spec = loadJsonFixture('real_mfh_muenchen_baybo.json');
  
  // 1. Schema check
  const schemaRes = validateBuildingSpec(spec);
  assert.equal(schemaRes.valid, true);

  // 2. Engine evaluation under Bavaria (BayBO 2026)
  const evalRes = runChecks(spec, { land: 'BY' });
  assert.equal(evalRes.counts.fail, 0);
  assert.equal(evalRes.verdict, VERDICT.AUFLAGEN); // Grenzbebauung reminder

  // Check GK 3 (height 8.4m > 7m)
  const gkResult = evalRes.results.find(r => r.id === 'gebaeudeklasse');
  assert.ok(gkResult);
  assert.equal(gkResult.findings[0].status, 'ok');
  assert.ok(gkResult.findings[0].message.includes('Gebäudeklasse 3'));
  assert.ok(gkResult.para.includes('BayBO'));

  // Check GRZ (0.31 <= 0.35)
  const grzResult = evalRes.results.find(r => r.id === 'grz');
  assert.ok(grzResult);
  assert.equal(grzResult.findings[0].status, 'ok');

  // Check GFZ (0.88 <= 1.00)
  const gfzResult = evalRes.results.find(r => r.id === 'gfz');
  assert.ok(gfzResult);
  assert.equal(gfzResult.findings[0].status, 'ok');

  // Check Parking (6 WE with 6 PKW spaces)
  const stellplatzResult = evalRes.results.find(r => r.id === 'stellplaetze');
  assert.ok(stellplatzResult);
  assert.equal(stellplatzResult.findings[0].status, 'ok');
});

test('Real-World Project: Stuttgart Hillside EFH (LBO BW) Setback Challenge', () => {
  const spec = loadJsonFixture('real_efh_stuttgart_lbo.json');

  const schemaRes = validateBuildingSpec(spec);
  assert.equal(schemaRes.valid, true);

  // In Baden-Württemberg, LBO BW requires minimum setback of 5.0m
  // North side is only 3.8m -> MUST FAIL
  const evalRes = runChecks(spec, { land: 'BW' });
  assert.equal(evalRes.verdict, VERDICT.NICHT_BESTANDEN);
  assert.ok(evalRes.counts.fail >= 1);

  const setbackResult = evalRes.results.find(r => r.id === 'abstandsflaeche');
  assert.ok(setbackResult);
  const failFinding = setbackResult.findings.find(f => f.status === 'fail');
  assert.ok(failFinding);
  assert.ok(setbackResult.para.includes('LBO BW'));
  assert.ok(failFinding.message.includes('Nordseite'));
  assert.ok(failFinding.message.includes('mindestens 5 m'));

  // If evaluated under MBO (which requires min 3.0m and 0.5 H = 3.6m), 3.8m PASSES!
  const mboEval = runChecks(spec, { land: 'MBO' });
  const mboSetback = mboEval.results.find(r => r.id === 'abstandsflaeche');
  assert.ok(mboSetback.findings.every(f => f.status === 'ok'));
});

test('Real-World Project: Berlin High-Rise GK 5 (BauO Bln)', () => {
  const spec = loadJsonFixture('real_hochhaus_berlin_bauobln.json');

  const schemaRes = validateBuildingSpec(spec);
  assert.equal(schemaRes.valid, true);

  const evalRes = runChecks(spec, { land: 'BE' });
  
  // Height 26.5m > 22m -> Gebäudeklasse 5
  const gkResult = evalRes.results.find(r => r.id === 'gebaeudeklasse');
  assert.ok(gkResult);
  assert.ok(gkResult.findings[0].message.includes('Gebäudeklasse 5'));

  // Escape routes & treppenraum must pass in GK 5
  const rettungResult = evalRes.results.find(r => r.id === 'rettungswege');
  assert.ok(rettungResult);
  assert.ok(rettungResult.findings.every(f => f.status === 'ok'));

  assert.ok(evalRes.results.length > 0);
});

test('Real-World Project: Cologne Commercial Logistics Hall (BauO NRW)', () => {
  const spec = loadJsonFixture('real_gewerbe_koeln_bauonrw.json');

  const schemaRes = validateBuildingSpec(spec);
  assert.equal(schemaRes.valid, true);

  const evalRes = runChecks(spec, { land: 'NW' });
  
  // GRZ 0.62 is within configured GE limit of 0.80
  const grzResult = evalRes.results.find(r => r.id === 'grz');
  assert.equal(grzResult.findings[0].status, 'ok');

  // GFZ 1.10 is within limit 1.60
  const gfzResult = evalRes.results.find(r => r.id === 'gfz');
  assert.equal(gfzResult.findings[0].status, 'ok');

  // 16 parking spaces for 1 WE (office + warehouse)
  const stellplatzResult = evalRes.results.find(r => r.id === 'stellplaetze');
  assert.equal(stellplatzResult.findings[0].status, 'ok');
  assert.equal(evalRes.counts.fail, 0);
});
