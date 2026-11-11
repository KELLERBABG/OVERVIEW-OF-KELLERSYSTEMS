"use strict";

import test from "node:test";
import assert from "node:assert/strict";
import { analyzeContract } from "../src/analyze.js";

/**
 * Red Team Stress-Test Suite: B2B SaaS & Cloud Trap Clauses
 * 
 * Tests 6 adversarial deceptive formulations across:
 * 1. Performance Reductions & Feature Stripping (§ 307 / § 308 Nr. 4 BGB)
 * 2. Dynamic Price Increases & Opaque Indices (§ 307 Abs. 1 BGB)
 * 3. Deemed Acceptance & Silence Fiction (§ 308 Nr. 5 BGB, BGH XI ZR 26/20)
 * 
 * Along with 3 compliant baseline variations that must NOT trigger false alarms.
 */

export const RED_TEAM_CASES = [
  // CATEGORY 1: Feature Stripping & Performance Reduction
  {
    id: "ADV-PERF-01",
    category: "Performance Reduction / Feature Stripping",
    title: "Agile Lifecycle & Workflow Modifizierung ohne Herabsetzungsrecht",
    statute: "§ 307 Abs. 1, 2 BGB i.V.m. § 308 Nr. 4 BGB",
    type: "adversarial",
    text: "Im Rahmen des kontinuierlichen Produktlebenszyklus und agiler Software-Iterationen modifiziert der Provider funktionale Module sowie Drittanbieter-Integrationen nach Maßgabe des Stands der Technik; ein Anspruch auf Beibehaltung spezifischer Workflows oder Herabsetzung der periodischen Vergütung besteht in diesen Fällen nicht.",
  },
  {
    id: "ADV-PERF-02",
    category: "Performance Reduction / Feature Stripping",
    title: "Upstream Cloud-Partner Wegfall mit ersatzlosem Erlöschen der Leistungspflicht",
    statute: "§ 307 Abs. 1, 2 BGB, § 308 Nr. 4 BGB",
    type: "adversarial",
    text: "Sollten vorgelagerte Cloud-Hosting-Dienstleister oder Schnittstellenpartner die Bereitstellung einzelner Programmkomponenten einstellen, erlischt die Leistungspflicht des Providers diesbezüglich ersatzlos, ohne dass dem Kunden hieraus Minderungsansprüche oder Rückabwicklungsrechte erwachsen.",
  },

  // CATEGORY 2: Dynamic Price Increases & Opaque Indices
  {
    id: "ADV-PRICE-01",
    category: "Dynamic Price Increase",
    title: "Interner Cloud-Hosting-Kostenindex mit absolutem Kündigungsausschluss",
    statute: "§ 307 Abs. 1 BGB (Transparenzgebot, Äquivalenzstörung)",
    type: "adversarial",
    text: "Die monatlichen Plattformgebühren werden zu Beginn eines jeden Vertragsjahres an die Veränderungsrate des intern ermittelten Cloud-Hosting-Kostenindexes angepasst, wobei jährliche Teuerungszuschläge von bis zu 9,5 % als vereinbart gelten und eine ordentliche oder außerordentliche Lösung vom Vertrag ausschließen.",
  },
  {
    id: "ADV-PRICE-02",
    category: "Dynamic Price Increase",
    title: "IT-Sektor Indexierung mit 15 % Schwellenwert für Sonderkündigung",
    statute: "§ 307 Abs. 1 BGB (Unangemessene Hürde für Lösungsrecht)",
    type: "adversarial",
    text: "Der Betreiber behält sich vor, die vereinbarte Vergütung jährlich neu anhand der Kostenentwicklung im IT-Sektor anzupassen; ein Sonderkündigungsrecht steht dem Kunden erst dann zu, wenn die Preiserhöhung im laufenden Kalenderjahr 15 % übersteigt.",
  },

  // CATEGORY 3: Deemed Acceptance / Silence as Consent
  {
    id: "ADV-CONSENT-01",
    category: "Deemed Acceptance / Silence Fiction",
    title: "Portal-Login nach 14 Tagen als rechtsgeschäftliches Einverständnis",
    statute: "§ 308 Nr. 5 BGB, BGH XI ZR 26/20",
    type: "adversarial",
    text: "Überarbeitete Lizenzbedingungen werden dem Kunden durch Bereitstellung im Administrationsportal angezeigt; loggt sich ein autorisierter Benutzer nach Ablauf einer Frist von 14 Tagen weiterhin in die Plattform ein, begründet dieses Verhalten das rechtliche Einverständnis mit der Neufassung der Bedingungen.",
  },
  {
    id: "ADV-CONSENT-02",
    category: "Deemed Acceptance / Silence Fiction",
    title: "Widerspruchsversäumnis nach Rechnungsstellung mit Weiternutzungsfiktion",
    statute: "§ 308 Nr. 5 BGB, BGH XI ZR 26/20",
    type: "adversarial",
    text: "Widerspricht der Kunde modifizierten Service-Level-Agreements nicht binnen vier Wochen nach Rechnungsstellung in Textform, gilt die fortgesetzte Nutzung der Software als unwiderrufliche Genehmigung der geänderten Bedingungen.",
  },

  // COMPLIANT BASELINES
  {
    id: "COMP-PERF-01",
    category: "Compliant Baseline - Performance Maintenance",
    title: "Wartung unter Erhalt der Kernfunktionen & Sonderkündigungsrecht",
    statute: "§ 305c, § 307 BGB konform",
    type: "compliant",
    text: "Der Anbieter führt regelmäßige Sicherheits- und Wartungsupdates durch, wobei die wesentlichen Kernfunktionen der Software erhalten bleiben. Führt ein technisches Update ausnahmsweise zu einer wesentlichen Beeinträchtigung vertraglicher Hauptleistungspflichten, kann der Kunde den Vertrag außerordentlich kündigen.",
  },
  {
    id: "COMP-PRICE-01",
    category: "Compliant Baseline - Price Indexation",
    title: "Symmetrische VPI-Kopplung mit 2 Monaten Ankündigung und Kündigungsrecht",
    statute: "§ 307 Abs. 1 BGB konform (Destatis VPI)",
    type: "compliant",
    text: "Eine Anpassung der Nutzungsgebühren erfolgt gekoppelt an die Entwicklung des vom Statistischen Bundesamt amtlich festgestellten Verbraucherpreisindex (VPI). Anpassungen sind sowohl bei Kostensteigerungen als auch bei Kostensenkungen möglich und berechtigen den Kunden im Erhöhungsfall zur Kündigung mit Frist von einem Monat.",
  },
  {
    id: "COMP-CONSENT-01",
    category: "Compliant Baseline - Terms Modification",
    title: "Ausdrückliche Zustimmungspflicht mit Fiktionsausschluss",
    statute: "§ 308 Nr. 5 BGB, BGH XI ZR 26/20 konform",
    type: "compliant",
    text: "Änderungen dieser Vertragsbedingungen bedürfen zu ihrer Wirksamkeit der ausdrücklichen schriftlichen Bestätigung beider Parteien. Aus einem Schweigen oder dem fortgesetzten Zugriff auf die Software können keine rechtsgeschäftlichen Willenserklärungen abgeleitet werden.",
  },
];

test("compliant baselines: must NOT trigger any 'hoch' findings and remain low risk", () => {
  const compliantCases = RED_TEAM_CASES.filter((c) => c.type === "compliant");
  for (const c of compliantCases) {
    const res = analyzeContract(c.text);
    assert.equal(res.counts.hoch, 0, `Compliant clause ${c.id} triggered hoch finding`);
    assert.equal(res.findings.length, 0, `Compliant clause ${c.id} should have 0 findings`);
    assert.equal(res.riskLevel, "niedrig", `Compliant clause ${c.id} should have niedrig risk`);
  }
});

test("adversarial stress-test: execute against analyzeContract and record detection telemetry", () => {
  const advCases = RED_TEAM_CASES.filter((c) => c.type === "adversarial");
  const results = advCases.map((c) => {
    const res = analyzeContract(c.text);
    return {
      id: c.id,
      detected: res.findings.length > 0,
      findings: res.findings,
      riskScore: res.riskScore,
    };
  });

  // Verify that known detected clauses are properly flagged
  const detected02 = results.find((r) => r.id === "ADV-PRICE-02");
  assert.ok(detected02.detected, "ADV-PRICE-02 must be detected");
  assert.equal(detected02.findings[0].type, "unilateral_change");

  const detectedConsent02 = results.find((r) => r.id === "ADV-CONSENT-02");
  assert.ok(detectedConsent02.detected, "ADV-CONSENT-02 must be detected");
  assert.equal(detectedConsent02.findings[0].type, "unilateral_change");
});
