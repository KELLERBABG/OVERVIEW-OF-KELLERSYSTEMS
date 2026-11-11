import { test } from "node:test";
import assert from "node:assert/strict";
import { resolveCountry, getRegionsForCountry, COUNTRIES, DACH_REGIONS } from "../src/dach.js";
import { getProfile, resolveProfileCode } from "../src/profiles.js";
import { runChecks, VERDICT } from "../src/engine.js";
import { runAllLandsMatrix } from "../src/matrix.js";

test("resolveCountry accurately detects DE, AT, and CH from specs and address cues", () => {
  assert.equal(resolveCountry({ land: "AT" }), "AT");
  assert.equal(resolveCountry({ country: "Österreich" }), "AT");
  assert.equal(resolveCountry({ land: "CH" }), "CH");
  assert.equal(resolveCountry({ staat: "Schweiz" }), "CH");
  assert.equal(resolveCountry({ bundesland: "AT_W" }), "AT");
  assert.equal(resolveCountry({ kanton: "ZH" }), "CH");
  assert.equal(resolveCountry({ adresse: "Kärntner Straße 15, 1010 Wien" }), "AT");
  assert.equal(resolveCountry({ adresse: "Bahnhofstrasse 45, 8001 Zürich" }), "CH");
  assert.equal(resolveCountry({ adresse: "Alexanderplatz 1, 10178 Berlin" }), "DE");
  assert.equal(resolveCountry({}), "DE"); // Default fallback
});

test("getRegionsForCountry filters regions appropriately", () => {
  const atRegions = getRegionsForCountry("AT");
  assert.equal(atRegions.length, 10); // AT_OIB + 9 Bundesländer
  assert.ok(atRegions.some((r) => r.code === "AT_W"));
  assert.ok(atRegions.some((r) => r.code === "AT_T"));

  const chRegions = getRegionsForCountry("CH");
  assert.equal(chRegions.length, 7); // CH_IVHB + 6 Kantone
  assert.ok(chRegions.some((r) => r.code === "CH_ZH"));
  assert.ok(chRegions.some((r) => r.code === "CH_BE"));

  const deRegions = getRegionsForCountry("DE");
  assert.equal(deRegions.length, 0); // German regions in PROFILES
});

test("Austria (AT_W Wien) profile executes with Wiener Bauordnung citation", () => {
  const spec = {
    projekt: "Dachausbau Wien Josefstadt",
    land: "AT",
    bundesland: "AT_W",
    gebaeude: {
      hoehe_m: 11.5,
      geschosse_ueber_grund: 4,
      wohnungen_anzahl: 6,
      grz_ist: 0.35,
      gfz_ist: 1.10,
      stellplaetze_ist: 6,
      fahrradstellplaetze_ist: 12,
      zweiter_rettungsweg_vorhanden: true,
      treppenraum_vorhanden: true,
      aufenthaltsraeume: [
        { name: "Top 1 Wohnzimmer", flaeche_m2: 32, fensterflaeche_m2: 4.5, raumhoehe_m: 2.65, etage: 3 }
      ]
    },
    grundstueck: {
      flaeche_m2: 650,
      grenzabstaende: {
        norden: 6.0,
        sueden: 6.0,
        osten: 6.0,
        westen: 6.0
      }
    }
  };

  const res = runChecks(spec, { land: "AT_W" });
  assert.ok(res);
  assert.equal(res.profile.code, "AT_W");
  assert.equal(res.profile.citationBase, "BO für Wien");
  assert.equal(res.verdict, VERDICT.BESTANDEN);

  const setbackResult = res.results.find((r) => r.id === "abstandsflaeche");
  assert.ok(setbackResult);
  assert.ok(setbackResult.para.includes("BO"));
});

test("Switzerland (CH_ZH Zürich) profile executes with PBG ZH citation", () => {
  const spec = {
    projekt: "Mehrfamilienhaus Zürich Oerlikon",
    land: "CH",
    kanton: "ZH",
    gebaeude: {
      hoehe_m: 9.0,
      geschosse_ueber_grund: 3,
      wohnungen_anzahl: 4,
      grz_ist: 0.30,
      gfz_ist: 0.90,
      stellplaetze_ist: 4,
      fahrradstellplaetze_ist: 8,
      zweiter_rettungsweg_vorhanden: true,
      treppenraum_vorhanden: true,
      aufenthaltsraeume: [
        { name: "Wohnen 1.OG", flaeche_m2: 28, fensterflaeche_m2: 4.0, raumhoehe_m: 2.50, etage: 1 }
      ]
    },
    grundstueck: {
      flaeche_m2: 500,
      grenzabstaende: {
        norden: 4.8,
        sueden: 4.8,
        osten: 4.8,
        westen: 4.8
      }
    }
  };

  const res = runChecks(spec, { land: "CH_ZH" });
  assert.ok(res);
  assert.equal(res.profile.code, "CH_ZH");
  assert.equal(res.profile.citationBase, "PBG ZH");
  assert.equal(res.verdict, VERDICT.BESTANDEN);
});

test("runAllLandsMatrix filters by country (DE, AT, CH)", () => {
  const spec = {
    projekt: "Typenhaus DACH",
    gebaeude: {
      hoehe_m: 8.0,
      geschosse_ueber_grund: 2,
      wohnungen_anzahl: 2,
      grz_ist: 0.25,
      gfz_ist: 0.50,
      stellplaetze_ist: 2,
      fahrradstellplaetze_ist: 4,
      zweiter_rettungsweg_vorhanden: true,
      treppenraum_vorhanden: true
    },
    grundstueck: {
      flaeche_m2: 600,
      grenzabstaende: { norden: 5.5, sueden: 5.5, osten: 5.5, westen: 5.5 }
    }
  };

  const deMatrix = runAllLandsMatrix(spec, { country: "DE" });
  assert.equal(deMatrix.totalStates, 17);

  const atMatrix = runAllLandsMatrix(spec, { country: "AT" });
  assert.equal(atMatrix.totalStates, 10);

  const chMatrix = runAllLandsMatrix(spec, { country: "CH" });
  assert.equal(chMatrix.totalStates, 7); // CH_IVHB + 6 Leitkantone (ZH, BE, BS, LU, SG, GE)

  const allMatrix = runAllLandsMatrix(spec, { country: "DACH" });
  assert.equal(allMatrix.totalStates, 34);
});

