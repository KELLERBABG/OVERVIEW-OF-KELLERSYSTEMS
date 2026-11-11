import { test } from "node:test";
import assert from "node:assert/strict";
import {
  parseXPlanGml,
  parseBPlanGeoJson,
  parseBPlanText,
  checkBuildingWithinBaugrenze,
  isPointInPolygon,
} from "../src/bplanparser.js";
import bplanRule from "../src/rules/bplan_festsetzungen.js";

test("B-Plan: XPlanung GML parsing extracts standard provisions", () => {
  const gmlSample = `<?xml version="1.0" encoding="UTF-8"?>
<xplan:XPlanAuszug xmlns:xplan="http://www.xplan-raumordnung.de/xplan/5.4" xmlns:gml="http://www.opengis.net/gml/3.2">
  <xplan:BP_BaugebietsTeilFlaeche>
    <xplan:besondereArtDerBaulNutzung>WA</xplan:besondereArtDerBaulNutzung>
    <xplan:GRZ>0.35</xplan:GRZ>
    <xplan:GFZ>0.8</xplan:GFZ>
    <xplan:Z>2</xplan:Z>
    <xplan:firsthoehe>9.50</xplan:firsthoehe>
    <xplan:traufhoehe>6.00</xplan:traufhoehe>
    <xplan:bauweise>1000 (offene Bauweise)</xplan:bauweise>
  </xplan:BP_BaugebietsTeilFlaeche>
  <xplan:BP_Baugrenze>
    <gml:posList>0.0 0.0 20.0 0.0 20.0 30.0 0.0 30.0 0.0 0.0</gml:posList>
  </xplan:BP_Baugrenze>
</xplan:XPlanAuszug>`;

  const parsed = parseXPlanGml(gmlSample);
  assert.equal(parsed.xplan_version, "5.4");
  assert.equal(parsed.baugebiet, "WA");
  assert.equal(parsed.grz_max, 0.35);
  assert.equal(parsed.gfz_max, 0.8);
  assert.equal(parsed.vollgeschosse_max, 2);
  assert.equal(parsed.firsthoehe_max_m, 9.5);
  assert.equal(parsed.traufhoehe_max_m, 6.0);
  assert.equal(parsed.bauweise, "offen");
  assert.equal(parsed.baugrenze_polygon.length, 4);
});

test("B-Plan: GeoJSON parsing extracts zoning attributes and geometry", () => {
  const geojsonSample = {
    type: "FeatureCollection",
    features: [
      {
        type: "Feature",
        properties: {
          baugebiet: "WR",
          GRZ: 0.4,
          GFZ: 1.2,
          vollgeschosse: 3,
          firsthoehe_max: 11.5,
          traufhoehe_max: 7.2,
          bauweise: "offen",
          dachform: "Satteldach",
        },
        geometry: {
          type: "Polygon",
          coordinates: [
            [
              [0, 0],
              [25, 0],
              [25, 25],
              [0, 25],
              [0, 0],
            ],
          ],
        },
      },
    ],
  };

  const parsed = parseBPlanGeoJson(geojsonSample);
  assert.equal(parsed.baugebiet, "WR");
  assert.equal(parsed.grz_max, 0.4);
  assert.equal(parsed.gfz_max, 1.2);
  assert.equal(parsed.vollgeschosse_max, 3);
  assert.equal(parsed.firsthoehe_max_m, 11.5);
  assert.equal(parsed.traufhoehe_max_m, 7.2);
  assert.equal(parsed.baugrenze_polygon.length, 4);
});

test("B-Plan: Textual provisions extraction parses German B-Plan Textteil", () => {
  const text = `
    Bebauungsplan Nr. 42 "Am Sonnenhang"
    Art der baulichen Nutzung: Allgemeines Wohngebiet (WA)
    Maß der baulichen Nutzung: II Vollgeschosse, GRZ 0,35, GFZ 0,7
    Höhenfestsetzung: Firsthöhe max. 10,20 m, Traufhöhe max. 6,50 m
    Gestaltung: Nur Satteldach (SD) zulässig, Dachneigung 38° bis 45°
    Bauweise: Offene Bauweise
  `;

  const parsed = parseBPlanText(text);
  assert.equal(parsed.baugebiet, "WA");
  assert.equal(parsed.vollgeschosse_max, 2);
  assert.equal(parsed.grz_max, 0.35);
  assert.equal(parsed.gfz_max, 0.7);
  assert.equal(parsed.firsthoehe_max_m, 10.2);
  assert.equal(parsed.traufhoehe_max_m, 6.5);
  assert.ok(parsed.zulaessige_dachformen.includes("Satteldach"));
  assert.equal(parsed.dachneigung_min_grad, 38);
  assert.equal(parsed.dachneigung_max_grad, 45);
  assert.equal(parsed.bauweise, "offen");
});

test("B-Plan Geometry: 2D Baugrenze polygon check detects violations and valid footprints", () => {
  const baugrenze = [
    { x: 0, y: 0 },
    { x: 30, y: 0 },
    { x: 30, y: 30 },
    { x: 0, y: 30 },
  ];

  // Inside
  const insideBuilding = [
    { x: 5, y: 5 },
    { x: 20, y: 5 },
    { x: 20, y: 20 },
    { x: 5, y: 20 },
  ];
  const resInside = checkBuildingWithinBaugrenze(insideBuilding, baugrenze);
  assert.equal(resInside.valid, true);
  assert.equal(resInside.violations.length, 0);

  // Outside (one point at x=35)
  const outsideBuilding = [
    { x: 5, y: 5 },
    { x: 35, y: 5 },
    { x: 35, y: 20 },
    { x: 5, y: 20 },
  ];
  const resOutside = checkBuildingWithinBaugrenze(outsideBuilding, baugrenze);
  assert.equal(resOutside.valid, false);
  assert.ok(resOutside.violations.length > 0);
});

test("B-Plan Rule: evaluates height limits, roof angles and Baugrenze", () => {
  // Compliant spec
  const goodSpec = {
    gebaeude: {
      firsthoehe_m: 9.0,
      traufhoehe_m: 5.5,
      geschosse_ueber_grund: 2,
      dachform: "Satteldach",
      dachneigung_grad: 40,
    },
    bplan: {
      firsthoehe_max_m: 10.0,
      traufhoehe_max_m: 6.0,
      vollgeschosse_max: 2,
      zulaessige_dachformen: ["Satteldach", "Walmdach"],
      dachneigung_min_grad: 35,
      dachneigung_max_grad: 45,
      baugrenze_eingehalten: true,
    },
  };
  const resGood = bplanRule.evaluate(goodSpec);
  assert.ok(resGood.length >= 5);
  assert.ok(resGood.every((f) => f.status === "ok"));

  // Non-compliant spec (height + roof violations)
  const badSpec = {
    gebaeude: {
      firsthoehe_m: 11.2,
      traufhoehe_m: 6.8,
      geschosse_ueber_grund: 3,
      dachform: "Flachdach",
      dachneigung_grad: 0,
    },
    bplan: {
      firsthoehe_max_m: 9.5,
      traufhoehe_max_m: 6.0,
      vollgeschosse_max: 2,
      zulaessige_dachformen: ["Satteldach"],
      dachneigung_min_grad: 35,
      dachneigung_max_grad: 48,
      baugrenze_eingehalten: false,
    },
  };
  const resBad = bplanRule.evaluate(badSpec);
  assert.ok(resBad.some((f) => f.status === "fail" && f.message.includes("Firsthöhe")));
  assert.ok(resBad.some((f) => f.status === "fail" && f.message.includes("Traufhöhe")));
  assert.ok(resBad.some((f) => f.status === "fail" && f.message.includes("Vollgeschossen")));
  assert.ok(resBad.some((f) => f.status === "fail" && f.message.includes("Dachform")));
  assert.ok(resBad.some((f) => f.status === "fail" && f.message.includes("Baugrenze")));
});

