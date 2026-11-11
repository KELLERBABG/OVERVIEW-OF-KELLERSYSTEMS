import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

import gebaeudeklasse, { berechneGebaeudeklasse } from "../src/rules/gebaeudeklasse.js";
import abstandsflaeche from "../src/rules/abstandsflaeche.js";
import grz from "../src/rules/grz.js";
import gfz from "../src/rules/gfz.js";
import stellplaetze from "../src/rules/stellplaetze.js";
import tageslicht from "../src/rules/tageslicht.js";
import raumhoehen from "../src/rules/raumhoehen.js";
import rettungswege from "../src/rules/rettungswege.js";
import vollgeschosse from "../src/rules/vollgeschosse.js";
import { ALL_RULES, selectRules } from "../src/rules/index.js";
import { parseBaubeschreibung } from "../src/textparser.js";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const fixture = (name) => JSON.parse(readFileSync(join(root, "test", "fixtures", name), "utf8"));

const statuses = (findings) => findings.map((f) => f.status);
const messages = (findings) => findings.map((f) => f.message).join(" | ");

test("Alle 9 Regeln sind registriert und haben id/para/title/evaluate", () => {
  assert.equal(ALL_RULES.length, 9);
  for (const rule of ALL_RULES) {
    assert.equal(typeof rule.id, "string");
    assert.ok(rule.para.includes("§"));
    assert.equal(typeof rule.title, "string");
    assert.equal(typeof rule.evaluate, "function");
  }
});

test("selectRules filtert nach IDs; ohne Filter alle", () => {
  assert.equal(selectRules(["tageslicht"]).length, 1);
  assert.equal(selectRules().length, 9);
  assert.equal(selectRules([]).length, 9);
  assert.equal(selectRules(["gibtsnicht"]).length, 0);
});

// --- gebaeudeklasse ---
test("Gebaeudeklasse: Hoehe 25 m -> GK5", () => {
  const spec = { gebaeude: { hoehe_m: 25, wohnungen_anzahl: 10, geschosse_ueber_grund: 8 } };
  assert.equal(berechneGebaeudeklasse(spec), 5);
});

test("Gebaeudeklasse: Hoehe 15 m -> GK4", () => {
  assert.equal(berechneGebaeudeklasse({ gebaeude: { hoehe_m: 15 } }), 4);
});

test("Gebaeudeklasse: 5+ Vollgeschosse zwingt in GK4 trotz geringer Hoehe", () => {
  assert.equal(berechneGebaeudeklasse({ gebaeude: { hoehe_m: 12, geschosse_ueber_grund: 6 } }), 4);
});

test("Gebaeudeklasse: kleines Einfamilienhaus -> GK1", () => {
  const spec = { gebaeude: { hoehe_m: 6.5, wohnungen_anzahl: 1, geschosse_ueber_grund: 1 } };
  const res = gebaeudeklasse.evaluate(spec);
  assert.deepEqual(statuses(res), ["ok"]);
  assert.ok(messages(res).includes("Gebäudeklasse 1"));
});

test("Gebaeudeklasse ohne Hoehe -> warn", () => {
  assert.deepEqual(statuses(gebaeudeklasse.evaluate({ gebaeude: {} })), ["warn"]);
});

// --- abstandsflaeche ---
test("Abstandsflaeche: H=10 -> 5 m erforderlich, Verstoß auf Ost-Seite", () => {
  const spec = {
    gebaeude: { hoehe_m: 10 },
    grundstueck: { grenzabstaende: { osten: 4, westen: 6 } },
  };
  const res = abstandsflaeche.evaluate(spec);
  assert.ok(statuses(res).includes("fail"));
  assert.ok(messages(res).includes("Ostseite"));
  assert.ok(!messages(res).includes("Westseite"));
});

test("Abstandsflaeche: Minimum 3 m greift bei niedrigem Gebaeude", () => {
  const spec = {
    gebaeude: { hoehe_m: 4 },
    grundstueck: { grenzabstaende: { sueden: 2.9, norden: 3 } },
  };
  const res = abstandsflaeche.evaluate(spec);
  assert.ok(statuses(res).includes("fail"));
  assert.ok(messages(res).includes("Südseite"));
});

test("Abstandsflaeche: alle Seiten ausreichend -> ok", () => {
  const spec = {
    gebaeude: { hoehe_m: 8 },
    grundstueck: { grenzabstaende: { norden: 5, osten: 5, sueden: 5, westen: 5 } },
  };
  assert.ok(statuses(abstandsflaeche.evaluate(spec)).includes("ok"));
});

test("Abstandsflaeche: Grenzgarage an Nordseite ist freigestellt -> ok", () => {
  const spec = {
    gebaeude: {
      hoehe_m: 8,
      grenzgarage: {
        vorhanden: true,
        wandhoehe_m: 2.8,
        laenge_einzelgrenze_m: 6.0,
        laenge_gesamt_m: 6.0,
        grenzseite: "norden",
      },
    },
    grundstueck: { grenzabstaende: { norden: 0.0, osten: 5, sueden: 5, westen: 5 } },
  };
  const res = abstandsflaeche.evaluate(spec);
  assert.ok(statuses(res).includes("ok"));
  assert.ok(messages(res).includes("privilegierte Grenzgarage"));
});

test("Abstandsflaeche: Grenzgarage überschreitet 9 m Grenze -> fail", () => {
  const spec = {
    gebaeude: {
      hoehe_m: 8,
      grenzgarage: {
        vorhanden: true,
        wandhoehe_m: 2.8,
        laenge_einzelgrenze_m: 11.0,
        grenzseite: "norden",
      },
    },
    grundstueck: { grenzabstaende: { norden: 0.0, osten: 5, sueden: 5, westen: 5 } },
  };
  const res = abstandsflaeche.evaluate(spec);
  assert.ok(statuses(res).includes("fail"));
  assert.ok(messages(res).includes("überschreitet Privilegierungsgrenzen"));
});

// --- grz ---
test("GRZ ueber Standard-Limit 0,4 -> warn", () => {
  const res = grz.evaluate({ gebaeude: { grz_ist: 0.52 } });
  assert.deepEqual(statuses(res), ["warn"]);
});

test("GRZ unter Limit -> ok", () => {
  assert.deepEqual(statuses(grz.evaluate({ gebaeude: { grz_ist: 0.3 } })), ["ok"]);
});

test("GRZ-Limit ist konfigurierbar", () => {
  const spec = { gebaeude: { grz_ist: 0.52 }, einstellungen: { grz_limit: 0.6 } };
  assert.deepEqual(statuses(grz.evaluate(spec)), ["ok"]);
});

test("GRZ ohne Angabe -> warn", () => {
  assert.deepEqual(statuses(grz.evaluate({ gebaeude: {} })), ["warn"]);
});

// --- tageslicht ---
test("Tageslicht: Fensterflaeche < 1/8 der Grundflaeche -> fail", () => {
  const spec = {
    gebaeude: { aufenthaltsraeume: [{ name: "Büro", flaeche_m2: 20, fensterflaeche_m2: 1.5 }] },
  };
  const res = tageslicht.evaluate(spec);
  assert.deepEqual(statuses(res), ["fail"]);
  assert.ok(messages(res).includes("Büro"));
});

test("Tageslicht: exakt 1/8 erfuellt -> ok", () => {
  const spec = {
    gebaeude: { aufenthaltsraeume: [{ name: "Wohnen", flaeche_m2: 24, fensterflaeche_m2: 3 }] },
  };
  assert.ok(statuses(tageslicht.evaluate(spec)).includes("ok"));
});

test("Tageslicht: unvollstaendiger Raum -> warn", () => {
  const spec = {
    gebaeude: { aufenthaltsraeume: [{ name: "Kammer", flaeche_m2: 9 }] },
  };
  assert.ok(statuses(tageslicht.evaluate(spec)).includes("warn"));
});

// --- raumhoehen ---
test("Raumhoehe 2,30 m -> warn (reduzierter Bereich)", () => {
  const spec = { gebaeude: { aufenthaltsraeume: [{ name: "Zimmer", raumhoehe_m: 2.3 }] } };
  assert.deepEqual(statuses(raumhoehen.evaluate(spec)), ["warn"]);
});

test("Raumhoehe 2,05 m -> fail", () => {
  const spec = { gebaeude: { aufenthaltsraeume: [{ name: "Kellerzimmer", raumhoehe_m: 2.05 }] } };
  assert.deepEqual(statuses(raumhoehen.evaluate(spec)), ["fail"]);
});

test("Raumhoehe 2,50 m -> ok", () => {
  const spec = { gebaeude: { aufenthaltsraeume: [{ name: "Wohnen", raumhoehe_m: 2.5 }] } };
  assert.ok(statuses(raumhoehen.evaluate(spec)).includes("ok"));
});

// --- rettungswege ---
test("Rettungswege: GK4 ohne Treppenraum/zweiten Rettungsweg -> fail", () => {
  const spec = {
    gebaeude: { hoehe_m: 15, geschosse_ueber_grund: 4, treppenraum_vorhanden: false, zweiter_rettungsweg_vorhanden: false },
  };
  assert.ok(statuses(rettungswege.evaluate(spec)).includes("fail"));
});

test("Rettungswege: GK2 -> ok", () => {
  const spec = { gebaeude: { hoehe_m: 7, geschosse_ueber_grund: 2 } };
  assert.ok(statuses(rettungswege.evaluate(spec)).includes("ok"));
});

test("Rettungswege: Treppenraum ja, zweiter Rettungsweg nein -> warn", () => {
  const spec = {
    gebaeude: { hoehe_m: 15, geschosse_ueber_grund: 4, treppenraum_vorhanden: true, zweiter_rettungsweg_vorhanden: false },
  };
  assert.ok(statuses(rettungswege.evaluate(spec)).includes("warn"));
});

test("Rettungswege: beides vorhanden -> ok", () => {
  const spec = {
    gebaeude: { hoehe_m: 15, geschosse_ueber_grund: 4, treppenraum_vorhanden: true, zweiter_rettungsweg_vorhanden: true },
  };
  assert.ok(statuses(rettungswege.evaluate(spec)).includes("ok"));
});

test("Rettungswege: >= 5 Vollgeschosse kritisch auch ohne klare Klasse", () => {
  const spec = { gebaeude: { geschosse_ueber_grund: 6 } };
  assert.notEqual(statuses(rettungswege.evaluate(spec))[0], "ok");
});

// --- vollgeschosse ---
test("Vollgeschosse: 15 m / 4 Geschosse = 3,75 m -> warn", () => {
  const spec = { gebaeude: { hoehe_m: 15, geschosse_ueber_grund: 4 } };
  assert.deepEqual(statuses(vollgeschosse.evaluate(spec)), ["warn"]);
});

test("Vollgeschosse: 7 m / 2 Geschosse = 3,50 m -> ok", () => {
  const spec = { gebaeude: { hoehe_m: 7, geschosse_ueber_grund: 2 } };
  assert.deepEqual(statuses(vollgeschosse.evaluate(spec)), ["ok"]);
});

test("Vollgeschosse: Dachgeschoss >= 67% Fläche über 2,30 m gilt als Vollgeschoss -> warn", () => {
  const spec = {
    gebaeude: {
      hoehe_m: 9,
      geschosse_ueber_grund: 2,
      dachgeschoss: {
        vorhanden: true,
        flaeche_ueber_2_30m_m2: 80,
        flaeche_unterliegend_m2: 100, // 80% >= 67%
      },
    },
  };
  const res = vollgeschosse.evaluate(spec);
  assert.ok(statuses(res).includes("warn"));
  assert.ok(messages(res).includes("Dachgeschoss gilt als Vollgeschoss"));
});

test("Vollgeschosse: Dachgeschoss < 67% Fläche über 2,30 m ist kein Vollgeschoss -> ok", () => {
  const spec = {
    gebaeude: {
      hoehe_m: 9,
      geschosse_ueber_grund: 2,
      dachgeschoss: {
        vorhanden: true,
        flaeche_ueber_2_30m_m2: 50,
        flaeche_unterliegend_m2: 100, // 50% < 67%
      },
    },
  };
  const res = vollgeschosse.evaluate(spec);
  assert.ok(statuses(res).includes("ok"));
  assert.ok(messages(res).includes("Dachgeschoss ist KEIN Vollgeschoss"));
});

test("Vollgeschosse ohne Daten -> keine Befunde", () => {
  assert.equal(vollgeschosse.evaluate({}).length, 0);
});

// --- gfz ---
test("GFZ: innerhalb Richtwert -> ok", () => {
  const spec = { gebaeude: { gfz_ist: 0.8 }, einstellungen: { gfz_limit: 1.2 } };
  assert.deepEqual(statuses(gfz.evaluate(spec)), ["ok"]);
});

test("GFZ: ueberschreitet Limit -> fail", () => {
  const spec = { gebaeude: { gfz_ist: 1.5 }, einstellungen: { gfz_limit: 1.2 } };
  assert.deepEqual(statuses(gfz.evaluate(spec)), ["fail"]);
  assert.ok(messages(gfz.evaluate(spec)).includes("Befreiung"));
});

test("GFZ ohne Angabe bei konfiguriertem Limit -> warn", () => {
  assert.deepEqual(statuses(gfz.evaluate({ einstellungen: { gfz_limit: 1.2 } })), ["warn"]);
});

test("GFZ ohne Angabe und ohne Limit -> keine Befunde", () => {
  assert.equal(gfz.evaluate({}).length, 0);
});

// --- stellplaetze ---
test("Stellplaetze: 4 WE mit 4 PKW und 8 Fahrradplaetzen -> ok", () => {
  const spec = {
    gebaeude: { wohnungen_anzahl: 4, stellplaetze_ist: 4, fahrradstellplaetze_ist: 8 },
  };
  const res = stellplaetze.evaluate(spec);
  assert.deepEqual(statuses(res), ["ok", "ok"]);
});

test("Stellplaetze: PKW Defizit -> fail", () => {
  const spec = {
    gebaeude: { wohnungen_anzahl: 4, stellplaetze_ist: 2 },
  };
  const res = stellplaetze.evaluate(spec);
  assert.deepEqual(statuses(res), ["fail"]);
  assert.ok(messages(res).includes("Pkw-Stellplatzdefizit"));
});

test("Stellplaetze: deklariert aber ohne Wohnungsanzahl -> warn", () => {
  assert.deepEqual(statuses(stellplaetze.evaluate({ gebaeude: { stellplaetze_ist: 2 } })), ["warn"]);
});

// --- textparser ---
test("Textparser: erkennt Bundesland, Flurstück und Hoehe aus Text", () => {
  const sample = `
    Bauvorhaben Neubau MFH in München, Bayern.
    Grundstücksfläche: 850 qm, BGF ca. 600 m2.
    Gebäudehöhe: 11,2 m, 3 Vollgeschosse über Grund.
    Insgesamt 6 Wohneinheiten, 6 PKW-Stellplätze.
    Abstand Nord: 5.5m, Süd: 4.2m.
    GRZ: 0.35, GFZ: 0.9.
  `;
  const { spec, confidence, extractedKeys } = parseBaubeschreibung(sample);
  assert.equal(spec.bundesland, "BY");
  assert.equal(spec.grundstueck.flaeche_m2, 850);
  assert.equal(spec.gebaeude.hoehe_m, 11.2);
  assert.equal(spec.gebaeude.geschosse_ueber_grund, 3);
  assert.equal(spec.gebaeude.wohnungen_anzahl, 6);
  assert.equal(spec.gebaeude.stellplaetze_ist, 6);
  assert.equal(spec.gebaeude.grz_ist, 0.35);
  assert.equal(spec.gebaeude.gfz_ist, 0.9);
  assert.equal(spec.grundstueck.grenzabstaende.norden, 5.5);
  assert.equal(spec.grundstueck.grenzabstaende.sueden, 4.2);
  assert.ok(confidence >= 80);
});

