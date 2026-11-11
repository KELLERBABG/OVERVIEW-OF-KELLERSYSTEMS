import test from 'node:test';
import assert from 'node:assert/strict';
import { runChecks } from '../src/engine.js';
import { berechneGebaeudeklasse } from '../src/rules/gebaeudeklasse.js';

test('Regulatory Edge Case: §19 BauNVO GRZ limit evaluation and warnings', () => {
  const baseSpec = {
    bundesland: 'BY',
    grundstueck: { flaeche_m2: 1000 },
    gebaeude: { hoehe_m: 6.0, geschosse_ueber_grund: 2, wohnungen_anzahl: 2 },
    einstellungen: { grz_limit: 0.40 }
  };

  // Case 1: Exact limit (0.40) -> ok
  const evalExact = runChecks({ ...baseSpec, gebaeude: { ...baseSpec.gebaeude, grz_ist: 0.40 } });
  const fExact = evalExact.results.find(r => r.id === 'grz');
  assert.equal(fExact.findings[0].status, 'ok');

  // Case 2: Slight exceedance (0.42) -> warn (requires exception §31 BauGB)
  const evalExceed = runChecks({ ...baseSpec, gebaeude: { ...baseSpec.gebaeude, grz_ist: 0.42 } });
  const fExceed = evalExceed.results.find(r => r.id === 'grz');
  assert.equal(fExceed.findings[0].status, 'warn');
  assert.ok(fExceed.findings[0].message.includes('0,42') || fExceed.findings[0].message.includes('0.42'));
});

test('Regulatory Edge Case: §20 BauNVO GFZ limit evaluation and failure', () => {
  const baseSpec = {
    bundesland: 'NW',
    grundstueck: { flaeche_m2: 800 },
    gebaeude: { hoehe_m: 9.0, geschosse_ueber_grund: 3, wohnungen_anzahl: 4 },
    einstellungen: { gfz_limit: 1.20 }
  };

  // Case 1: Within limit (1.10) -> ok
  const evalOk = runChecks({ ...baseSpec, gebaeude: { ...baseSpec.gebaeude, gfz_ist: 1.10 } });
  const fOk = evalOk.results.find(r => r.id === 'gfz');
  assert.equal(fOk.findings[0].status, 'ok');

  // Case 2: Exceeding limit (1.45) -> fail (blocking deficiency)
  const evalFail = runChecks({ ...baseSpec, gebaeude: { ...baseSpec.gebaeude, gfz_ist: 1.45 } });
  const fFail = evalFail.results.find(r => r.id === 'gfz');
  assert.equal(fFail.findings[0].status, 'fail');
  assert.ok(fFail.findings[0].message.includes('1,45') || fFail.findings[0].message.includes('1.45'));
});

test('Regulatory Edge Case: State Setback Rules Comparison (BW vs BY vs MBO)', () => {
  const spec = {
    gebaeude: { hoehe_m: 8.0, geschosse_ueber_grund: 2, wohnungen_anzahl: 1 },
    grundstueck: {
      flaeche_m2: 600,
      grenzabstaende: { norden: 4.0, sueden: 4.0, osten: 4.0, westen: 4.0 }
    }
  };

  // In Bavaria (BayBO: no §6 MBO setback; handled via Grenzbebauung) -> 0 fails
  const evalBY = runChecks(spec, { land: 'BY' });
  assert.equal(evalBY.counts.fail, 0);

  // In MBO (0.5 H = 4.0m, min 3.0m) -> 4.0m PASSES
  const evalMBO = runChecks(spec, { land: 'MBO' });
  const fMBO = evalMBO.results.find(r => r.id === 'abstandsflaeche');
  assert.ok(fMBO.findings.every(f => f.status === 'ok'));

  // In Baden-Württemberg (0.5 H = 4.0m, BUT min is 5.0m!) -> 4.0m FAILS!
  const evalBW = runChecks(spec, { land: 'BW' });
  const fBW = evalBW.results.find(r => r.id === 'abstandsflaeche');
  assert.ok(fBW.findings.some(f => f.status === 'fail'));
  assert.ok(fBW.findings.find(f => f.status === 'fail').message.includes('5 m'));
});

test('Regulatory Edge Case: Gebäudeklasse Transition Thresholds', () => {
  // Height 6.5m, 1 unit -> GK 1
  assert.equal(berechneGebaeudeklasse({ gebaeude: { hoehe_m: 6.5, wohnungen_anzahl: 1 } }), 1);

  // Height 6.5m, 2 units -> GK 2
  assert.equal(berechneGebaeudeklasse({ gebaeude: { hoehe_m: 6.5, wohnungen_anzahl: 2 } }), 2);

  // Height 7.5m, 3 units -> GK 3 (units >= 3)
  assert.equal(berechneGebaeudeklasse({ gebaeude: { hoehe_m: 7.5, wohnungen_anzahl: 3 } }), 3);

  // Height 13.5m -> GK 4 (height > 13m)
  assert.equal(berechneGebaeudeklasse({ gebaeude: { hoehe_m: 13.5, wohnungen_anzahl: 8 } }), 4);

  // Height 22.5m -> GK 5 (height > 22m)
  assert.equal(berechneGebaeudeklasse({ gebaeude: { hoehe_m: 22.5, wohnungen_anzahl: 20 } }), 5);
});

test('Regulatory Edge Case: Daylighting 1/8 Boundary Check (DIN 5034)', () => {
  // Room area 40 m², 5.0 m² window (5/40 = 12.5% = exact 1/8) -> ok
  const evalExact = runChecks({
    gebaeude: {
      aufenthaltsraeume: [{ name: "Zimmer A", flaeche_m2: 40.0, fenster_flaeche_m2: 5.0, lichte_hoehe_m: 2.5 }]
    }
  });
  const fExact = evalExact.results.find(r => r.id === 'tageslicht');
  assert.equal(fExact.findings[0].status, 'ok');

  // Room area 40 m², 4.9 m² window (4.9/40 = 12.25% < 12.5%) -> fail
  const evalDeficit = runChecks({
    gebaeude: {
      aufenthaltsraeume: [{ name: "Zimmer B", flaeche_m2: 40.0, fenster_flaeche_m2: 4.9, lichte_hoehe_m: 2.5 }]
    }
  });
  const fDeficit = evalDeficit.results.find(r => r.id === 'tageslicht');
  assert.equal(fDeficit.findings.some(f => f.status === 'fail'), true);
});
