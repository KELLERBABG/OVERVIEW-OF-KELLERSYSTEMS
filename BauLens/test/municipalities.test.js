import { test } from "node:test";
import assert from "node:assert/strict";
import {
  MUNICIPALITIES,
  MUNICIPALITY_CODES,
  getMunicipality,
  resolveMunicipalityCode,
  getMunicipalitiesForLand,
} from "../src/municipalities.js";
import { runChecks } from "../src/engine.js";

test("Municipalities Registry contains key cities with statutes", () => {
  assert.ok(MUNICIPALITY_CODES.length >= 20);
  const muc = getMunicipality("MUENCHEN");
  assert.equal(muc.bundesland, "BY");
  assert.ok(muc.satzungen.stellplatz);
  assert.ok(muc.satzungen.gruendach);

  const ber = getMunicipality("BERLIN");
  assert.equal(ber.bundesland, "BE");
  assert.equal(ber.satzungen.stellplatz.pkwFaktor, 0.0); // abolished

  const ms = getMunicipality("MUENSTER");
  assert.equal(ms.bundesland, "NW");
  assert.equal(ms.satzungen.stellplatz.radFaktor, 3.0); // Fahrradhauptstadt
});

test("resolveMunicipalityCode resolves from flag, spec key, and text heuristic", () => {
  assert.equal(resolveMunicipalityCode({}, "MUENCHEN"), "MUENCHEN");
  assert.equal(resolveMunicipalityCode({ kommune: "KOELN" }), "KOELN");
  assert.equal(resolveMunicipalityCode({ projekt: "Wohnanlage Frankfurt Innenstadt" }), "FRANKFURT");
  assert.equal(resolveMunicipalityCode({ adresse: "Königsallee, Düsseldorf" }), "DUESSELDORF");
  assert.equal(resolveMunicipalityCode({ projekt: "Quartier am Maschsee, Hannover" }), "HANNOVER");
  assert.equal(resolveMunicipalityCode({ adresse: "Prinzipalmarkt, Münster" }), "MUENSTER");
  assert.equal(resolveMunicipalityCode({}), null);
});

test("getMunicipalitiesForLand filters by state", () => {
  const byCities = getMunicipalitiesForLand("BY");
  assert.ok(byCities.some((c) => c.code === "MUENCHEN"));
  assert.ok(byCities.some((c) => c.code === "NUERNBERG"));
  assert.ok(byCities.some((c) => c.code === "AUGSBURG"));
  assert.ok(!byCities.some((c) => c.code === "BERLIN"));

  const nwCities = getMunicipalitiesForLand("NW");
  assert.ok(nwCities.some((c) => c.code === "KOELN"));
  assert.ok(nwCities.some((c) => c.code === "ESSEN"));
  assert.ok(nwCities.some((c) => c.code === "DORTMUND"));
  assert.ok(nwCities.some((c) => c.code === "BONN"));
  assert.ok(nwCities.some((c) => c.code === "MUENSTER"));
});

test("Berlin profile abolishes Kfz Stellplatzpflicht for residential units", () => {
  const spec = {
    bundesland: "BE",
    kommune: "BERLIN",
    gebaeude: {
      wohnungen_anzahl: 6,
      stellplaetze_ist: 0,
      fahrradstellplaetze_ist: 12,
    },
  };
  const res = runChecks(spec, { rules: ["stellplaetze"] });
  const stellResult = res.results.find((r) => r.id === "stellplaetze");
  assert.ok(stellResult);
  assert.ok(stellResult.findings.some((f) => f.status === "ok" && f.message.includes("Keine Kfz-Stellplatzpflicht")));
});

test("Munich profile enforces Gründachpflicht for flat roofs", () => {
  const specFail = {
    bundesland: "BY",
    kommune: "MUENCHEN",
    gebaeude: {
      dachneigung_grad: 0,
      gruendach_vorhanden: false,
    },
  };
  const resFail = runChecks(specFail, { rules: ["gruendach"] });
  const gruenResult = resFail.results.find((r) => r.id === "gruendach");
  assert.ok(gruenResult.findings.some((f) => f.status === "fail" && f.message.includes("Flachdächer bis 15° müssen")));

  const specOk = {
    bundesland: "BY",
    kommune: "MUENCHEN",
    gebaeude: {
      dachneigung_grad: 0,
      gruendach_vorhanden: true,
      gruendach_substrat_cm: 12,
    },
  };
  const resOk = runChecks(specOk, { rules: ["gruendach"] });
  const gruenOk = resOk.results.find((r) => r.id === "gruendach");
  assert.ok(gruenOk.findings.some((f) => f.status === "ok" && f.message.includes("Dachbegrünungspflicht erfüllt")));
});

test("Steep roof (Satteldach 45 deg) is exempt from Gründachpflicht", () => {
  const spec = {
    bundesland: "BY",
    kommune: "MUENCHEN",
    gebaeude: {
      dachneigung_grad: 45,
    },
  };
  const res = runChecks(spec, { rules: ["gruendach"] });
  const gruen = res.results.find((r) => r.id === "gruendach");
  assert.ok(gruen.findings.some((f) => f.status === "ok" && f.message.includes("Befreit von der kommunalen Gründachpflicht")));
});
