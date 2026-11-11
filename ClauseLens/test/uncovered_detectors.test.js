import test from "node:test";
import assert from "node:assert/strict";
import { analyzeContract } from "../src/analyze.js";

const cases = [
  ["agency_commission_exclusion", "Ein Direktauftrag eines Kunden aus dem zugewiesenen Gebiet begründet keine Provision.", "Der Handelsvertreter erhält keine Provision für die Vermittlung von Versicherungen, die nicht zum Vertragsgegenstand gehören."],
  ["agency_goodwill_waiver", "Nach dem Vertragsende besteht kein Anspruch auf Ausgleich für den aufgebauten Kundenstamm.", "Ein gesetzlicher Ausgleichsanspruch nach § 89b HGB bleibt unberührt."],
  ["international_data_processing", "Personenbezogene Daten werden auf Servern außerhalb der Europäischen Union verarbeitet.", "Personenbezogene Daten werden ausschließlich innerhalb des Europäischen Wirtschaftsraums verarbeitet."],
  ["mandatory_expert_determination", "Bei Rechnungsstreitigkeiten wird zunächst ein Sachverständigenverfahren durchgeführt. Die Kosten des Verfahrens sind vom Kunden vorzustrecken; der Rechtsweg ist erst nach Abschluss eröffnet.", "Die Parteien können vor einem Gerichtsverfahren freiwillig einen unabhängigen Sachverständigen konsultieren; jede Partei trägt ihre Kosten selbst."],
  ["service_credit_forfeiture", "Werden kritisch eingestufte Aktualisierungen nicht binnen 24 Stunden eingespielt, entfällt der Anspruch auf vereinbarte Gutschriften.", "Sicherheitsupdates werden zeitnah installiert. Bereits erworbene Service-Gutschriften bleiben davon unberührt."],
  ["early_termination_charge", "Bei vorzeitiger Beendigung schuldet der Besteller eine pauschale Abstandszahlung von 25 Prozent der offenen Vergütung.", "Bei ordnungsgemäßer Beendigung fällt keine zusätzliche Zahlung an; nachgewiesene Schäden bleiben nach Gesetz ersatzfähig."],
  ["maintenance_conditional_shutdown", "Der Auftragnehmer darf die Anlage nach zwei Jahren abschalten, wenn kein Wartungsvertrag abgeschlossen wurde.", "Ein Wartungsvertrag kann zusätzlich abgeschlossen werden; sein Fehlen berechtigt nicht zur Abschaltung der Anlage."],
];

test("new uncovered-clause detectors flag the marked risk and avoid nearby benign wording", () => {
  for (const [type, deceptive, benign] of cases) {
    const positive = analyzeContract(deceptive).findings;
    assert.ok(positive.some((finding) => finding.type === type), `${type} should detect its deceptive wording`);

    const negative = analyzeContract(benign).findings;
    assert.ok(!negative.some((finding) => finding.type === type), `${type} should not flag its benign control`);
  }
});

test("international-data detector handles both countries-and-EU wording and arbitrary third countries", () => {
  for (const text of [
    "Zur Auswertung der Messdaten darf der Dienstleister Gesellschaften mit Sitz außerhalb der Schweiz und der Europäischen Union einsetzen.",
    "Kundendaten werden in einem Drittland verarbeitet.",
  ]) {
    assert.ok(analyzeContract(text).findings.some((finding) => finding.type === "international_data_processing"));
  }
});
