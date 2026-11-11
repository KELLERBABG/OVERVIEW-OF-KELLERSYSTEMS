"use strict";

import test from "node:test";
import assert from "node:assert/strict";
import { analyzeContract } from "../src/analyze.js";

/**
 * Brain Knowledge Regression Suite:
 * Validates new BGB & BGH high-risk clauses extracted from German court rulings:
 * 1. written_form_trap (§ 309 Nr. 13 BGB, § 305b BGB)
 * 2. burden_of_proof (§ 309 Nr. 12 BGB)
 * 3. dynamic_url_referral (BGH III ZR 59/24)
 */

test("brain corpus: detects strict written form trap (§ 309 Nr. 13 BGB)", () => {
  const strict = "Änderungen und Ergänzungen dieses Vertrages bedürfen der Schriftform. Mündliche Nebenabreden sind unwirksam.";
  const res = analyzeContract(strict);
  const hit = res.findings.find(f => f.type === "written_form_trap");
  assert.ok(hit, "Must detect strict written form clause");
  assert.equal(hit.severity, "hoch");
});

test("brain corpus: compliant text form clause is clean (§ 126b BGB)", () => {
  const compliant = "Änderungen und Ergänzungen dieses Vertrages bedürfen der Textform (z. B. E-Mail). Mündliche Individualabreden haben stets Vorrang.";
  const res = analyzeContract(compliant);
  const hit = res.findings.find(f => f.type === "written_form_trap");
  assert.equal(hit, undefined, "Compliant text form must not be flagged");
});

test("brain corpus: detects deemed completeness & burden of proof shift (§ 309 Nr. 12 BGB)", () => {
  const trap = "Der Kunde bestätigt mit Freischaltung des Accounts, dass die Software mangelfrei und fehlerfrei übergeben wurde. Die Beweislast für Mängel obliegt dem Kunden.";
  const res = analyzeContract(trap);
  const hit = res.findings.find(f => f.type === "burden_of_proof");
  assert.ok(hit, "Must detect burden of proof trap");
  assert.equal(hit.severity, "hoch");
});

test("brain corpus: detects dynamic online AGB URL referral (BGH III ZR 59/24)", () => {
  const trap = "Für den Leistungsumfang und die Vergütung gelten die jeweils aktuellen AGB des Anbieters, abrufbar unter www.anbieter-cloud.de/agb.";
  const res = analyzeContract(trap);
  const hit = res.findings.find(f => f.type === "dynamic_url_referral");
  assert.ok(hit, "Must detect dynamic URL referral as invalid under BGH III ZR 59/24");
  assert.equal(hit.severity, "hoch");
});

test("brain corpus: detects all 6 traps in enterprise cloud agreement with high risk score", () => {
  const contract = `Cloud-Enterprise- und Rahmenentwicklungsvereinbarung
§ 3 Verfügbarkeit, Service Levels und Betriebsstörungen
(2) Unterschreitet der Anbieter die vereinbarte Verfügbarkeit, erhält der Kunde auf gesonderten Antrag eine Service-Gutschrift (Service Credit) in Höhe von 5 % der monatlichen Grundgebühr pro angefangene 24 Stunden Ausfallzeit, maximal jedoch 15 % der monatlichen Gebühr. Diese Gutschrift stellt die abschließende und ausschließliche Kompensation für Ausfälle und Störungen dar; weitergehende Ansprüche des Kunden – einschließlich gesetzlicher Minderungsrechte, Rücktritt oder Schadensersatz wegen Nichterfüllung – sind insoweit vollumfänglich abgegolten.

§ 5 Abrechnung, Zahlungsmodalitäten und Leistungsverweigerung
(2) Ein Zurückbehaltungsrecht oder die Einrede des nicht erfüllten Vertrages steht dem Kunden nur zu, sofern der zugrunde liegende Gegenanspruch aus demselben konkreten Monatsrechnungsabschnitt herrührt und unbestritten ist. Die Geltendmachung eines Zurückbehaltungsrechts wegen Mängeln an vorgelagerten Softwaremodulen ist für die laufende Betriebspauschale ausgeschlossen.

§ 8 Schutzrechte, Datenintegrität und Fehleranalyse
(2) Zur Gewährleistung automatisierter Lastverteilung, proaktiver Fehlererkennung und kontinuierlicher Qualitätsverbesserung gestattet der Kunde dem Anbieter die automatisierte semantische Analyse der verarbeiteten Datenströme. Der Anbieter ist berechtigt, hieraus gewonnene mathematische Gewichtungen, strukturelle Muster und Systemparameter dauerhaft, unentgeltlich und über das Vertragsende hinaus für eigene Entwicklungs- und Modellierungszwecke weiterzuverwenden, sofern keine direkte Re-Identifikation einzelner Klienten des Kunden erfolgt.

§ 11 Schadensabwendung und Freistellungsverpflichtung
(2) Verursachen über die Instanz des Kunden initiierte Datenübertragungen Störungen im Gesamtsystem des Anbieters oder machen Dritte hieraus Ansprüche wegen Rechtsverletzungen geltend, stellt der Kunde den Anbieter im Innenverhältnis von allen resultierenden Schäden und Kosten der Rechtsverteidigung frei. Diese Freistellung gilt verschuldensunabhängig, es sei denn, der Kunde weist lückenlos nach, dass die Ursache ausschließlich auf einer fehlerhaften Basiskomponente des Anbieters beruhte.

§ 14 Kündigung und Formvorschriften
(2) Jede Kündigung bedarf zu ihrer Wirksamkeit der eigenhändig unterzeichneten Schriftform und muss dem Anbieter per eingeschriebenem Brief mit Rückschein an die im Handelsregister eingetragene Geschäftsanschrift zugehen; die elektronische Form (§ 126a BGB) sowie Textform (§ 126b BGB) sind ausdrücklich ausgeschlossen.

§ 17 Schlussbestimmungen und Vertragsänderungen
(2) Das Erfordernis der Schriftform kann seinerseits nur schriftlich abbedungen werden; eine konkludente oder mündliche Abbedingung dieses Formerfordernisses ist ausgeschlossen.`;

  const res = analyzeContract(contract);
  assert.equal(res.riskScore, 100, "Enterprise adversarial contract must trigger maximum risk score (100)");
  assert.equal(res.riskLevel, "hoch");
  
  const types = new Set(res.findings.map(f => f.type));
  assert.ok(types.has("liability"), "Must detect Sole Remedy & Strict Indemnity under liability");
  assert.ok(types.has("setoff_restriction"), "Must detect retention restriction under setoff_restriction");
  assert.ok(types.has("data_grant"), "Must detect model weight scraping under data_grant");
  assert.ok(types.has("burden_of_proof"), "Must detect negative burden of proof under burden_of_proof");
  assert.ok(types.has("written_form_trap"), "Must detect strict termination & double written form trap");
});
