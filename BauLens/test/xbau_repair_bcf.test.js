import { test } from "node:test";
import assert from "node:assert/strict";
import { generateXBauXml, generateBauantragFormular, generateAbweichungsAntrag } from "../src/xbau.js";
import { generateComplianceRepairs } from "../src/repair.js";
import { exportBcfCollection, createBcfTopic } from "../src/bcf.js";

test("XBau: generates valid OZG-compliant XML message with header and parcel details", () => {
  const spec = {
    projekt: "Neubau Wohnquartier Isarauen",
    bundesland: "BY",
    stadt: "München",
    grundstueck: { flurstueck: "742/1", gemarkung: "München-Sendling", flaeche_m2: 1200 },
    gebaeude: { hoehe_m: 11.5, geschosse_ueber_grund: 4, wohnungen_anzahl: 8, grz_ist: 0.35, gfz_ist: 1.10 },
  };
  const evalResult = { verdict: "pass", sealSha256: "abc123sha256" };

  const xml = generateXBauXml(spec, evalResult);
  assert.ok(xml.includes("<xbau:baugenehmigung.antrag.0200"));
  assert.ok(xml.includes("<xbau:bezeichnung>Neubau Wohnquartier Isarauen</xbau:bezeichnung>"));
  assert.ok(xml.includes("<xbau:flurstuecksnummer>742/1</xbau:flurstuecksnummer>"));
  assert.ok(xml.includes("<xbau:hoeheGebaeudeOberkante>11.5</xbau:hoeheGebaeudeOberkante>"));
  assert.ok(xml.includes("<xbau:status>Bestanden</xbau:status>"));
  assert.ok(xml.includes("abc123sha256"));
});

test("XBau: generates readable Bauantrag and Abweichungsantrag forms", () => {
  const spec = {
    projekt: "MFH Schillerstraße",
    bundesland: "BW",
    stadt: "Stuttgart",
    grundstueck: { flurstueck: "99/2", flaeche_m2: 650 },
    gebaeude: { hoehe_m: 9.0, wohnungen_anzahl: 3, geschosse_ueber_grund: 3 },
  };

  const form = generateBauantragFormular(spec);
  assert.ok(form.includes("ANTRAG AUF BAugenehmigung"));
  assert.ok(form.includes("Stuttgart (BW)"));
  assert.ok(form.includes("Flurstück 99/2"));

  const abweichung = generateAbweichungsAntrag({
    para: "§ 6 LBO BW",
    message: "Grenzabstand Ost 3.8 m unterschreitet Mindestmaß von 5.0 m",
  });
  assert.ok(abweichung.includes("ANTRAG AUF ABWEICHUNG / BEFREIUNG"));
  assert.ok(abweichung.includes("§ 6 LBO BW"));
  assert.ok(abweichung.includes("Städtebauliche Vertretbarkeit"));
});

test("Compliance Repair Engine: calculates spatial shift when opposite buffer exists", () => {
  const spec = {
    gebaeude: { hoehe_m: 8.0 },
    grundstueck: {
      grenzabstaende: { osten: 2.1, westen: 6.5, norden: 5.0, sueden: 5.0 },
    },
  };
  // Req = 4.0 m (8 * 0.5)
  // East has 2.1 m -> deficit 1.9 m
  // West has 6.5 m -> buffer = 6.5 - 4.0 = 2.5 m (>= 1.9 m)
  const findings = [
    { ruleId: "abstandsflaeche", status: "fail", message: "Grenzabstand Ost (2,1 m) unterschreitet erforderliche Tiefe von 4,0 m." },
  ];

  const repairs = generateComplianceRepairs(spec, findings, { factor: 0.5, minM: 3.0 });
  assert.ok(repairs.length >= 1);
  const shift = repairs.find((r) => r.type === "spatial_shift");
  assert.ok(shift);
  assert.ok(shift.title.includes("Verschiebung des Baukörpers um 1,9 m nach Westen"));
  assert.equal(shift.patch["grundstueck.grenzabstaende.osten"], 4.0);
  assert.equal(shift.patch["grundstueck.grenzabstaende.westen"], 4.6);
});

test("BCF Engine: converts violations into BIM Collaboration Format 2.1 topics", () => {
  const findings = [
    { ruleId: "abstandsflaeche", status: "fail", message: "Grenzabstand Ost 2.1 m unterschreitet 4.0 m." },
    { ruleId: "gfz", status: "warn", message: "GFZ 1.35 überschreitet Richtwert 1.20." },
    { ruleId: "tageslicht", status: "ok", message: "Tageslicht ausreichend." },
  ];

  const bcf = exportBcfCollection(findings);
  assert.equal(bcf.version.version_id, "2.1");
  assert.equal(bcf.topics.length, 2);
  assert.equal(bcf.topics[0].priority, "Critical");
  assert.equal(bcf.topics[1].priority, "Major");
  assert.ok(bcf.topics[0].viewpoint.perspective_camera);
});

