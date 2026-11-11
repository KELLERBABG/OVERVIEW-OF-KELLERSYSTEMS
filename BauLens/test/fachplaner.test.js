import { test } from "node:test";
import assert from "node:assert/strict";
import brandwand from "../src/rules/brandwand.js";
import barrierefreiheit from "../src/rules/barrierefreiheit.js";
import schallschutz from "../src/rules/schallschutz.js";

test("Brandwand: setback < 2.50m triggers warning and REI 90-M requirement", () => {
  const specClose = {
    grundstueck: {
      grenzabstaende: { osten: 1.8, westen: 4.5, norden: 5.0, sueden: 5.0 },
    },
  };
  const resClose = brandwand.evaluate(specClose);
  assert.equal(resClose.length, 1);
  assert.equal(resClose[0].status, "warn");
  assert.ok(resClose[0].message.includes("Brandwand"));
  assert.ok(resClose[0].message.includes("osten (1,8 m)"));

  const specFar = {
    grundstueck: {
      grenzabstaende: { osten: 3.0, westen: 4.5, norden: 5.0, sueden: 5.0 },
    },
  };
  const resFar = brandwand.evaluate(specFar);
  assert.equal(resFar.length, 1);
  assert.equal(resFar[0].status, "ok");
  assert.ok(resFar[0].message.includes("keine gebäudeabschlussseitige Brandwand"));
});

test("Barrierefreiheit: elevator required for 4+ storeys or height > 13m (§50 MBO)", () => {
  // 4 storeys without elevator -> fail
  const specNoElevator = {
    gebaeude: { geschosse_ueber_grund: 4, hoehe_m: 12.0, aufzug_vorhanden: false, wohnungen_anzahl: 6 },
  };
  const resFail = barrierefreiheit.evaluate(specNoElevator);
  assert.ok(resFail.some((f) => f.status === "fail" && f.message.toLowerCase().includes("aufzug")));

  // 4 storeys with elevator -> ok
  const specWithElevator = {
    gebaeude: { geschosse_ueber_grund: 4, hoehe_m: 12.0, aufzug_vorhanden: true, wohnungen_anzahl: 6, eingang_stufenlos: true },
  };
  const resOk = barrierefreiheit.evaluate(specWithElevator);
  assert.ok(resOk.some((f) => f.status === "ok" && f.message.toLowerCase().includes("aufzug")));
  assert.ok(resOk.some((f) => f.status === "ok" && f.message.includes("Stufenloser Hauseingang")));

  // Ramp steeper than 6% -> warn
  const specSteepRamp = {
    gebaeude: { geschosse_ueber_grund: 2, hoehe_m: 6.5, wohnungen_anzahl: 4, eingang_stufenlos: true, rampen_neigung_prozent: 8.5 },
  };
  const resRamp = barrierefreiheit.evaluate(specSteepRamp);
  assert.ok(resRamp.some((f) => f.status === "warn" && f.message.includes("Rampenneigung 8,5 % übersteigt maximal zulässige 6.0 %")));
});

test("Schallschutz: outdoor heat pump boundary buffer check (TA Lärm)", () => {
  // Outdoor heat pump with 1.8m distance -> warn
  const specCloseWP = {
    gebaeude: { waermepumpe_aussen_vorhanden: true, abstand_waermepumpe_grenze_m: 1.8 },
  };
  const resWP = schallschutz.evaluate(specCloseWP);
  assert.ok(resWP.some((f) => f.status === "warn" && f.message.includes("TA-Lärm-Empfehlungsabstand")));

  // Outdoor heat pump with 4.0m distance -> ok
  const specFarWP = {
    gebaeude: { waermepumpe_aussen_vorhanden: true, abstand_waermepumpe_grenze_m: 4.0 },
  };
  const resWPOk = schallschutz.evaluate(specFarWP);
  assert.ok(resWPOk.some((f) => f.status === "ok" && f.message.includes("TA Lärm Nachbarschutz plausibel")));

  // High noise level (LPB V) -> warn
  const specLpb = {
    gebaeude: { schallschutz_laermpegelbereich: 5 },
  };
  const resLpb = schallschutz.evaluate(specLpb);
  assert.ok(resLpb.some((f) => f.status === "warn" && f.message.includes("Lärmpegelbereich 5")));
});

