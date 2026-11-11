import test from "node:test";
import assert from "node:assert/strict";
import { RULES } from "../src/analyze.js";

const NEW_RULES = RULES.slice(20);
const LATEST_RULES = RULES.slice(40);

const TEST_CASES = [
  {
    type: "assignment_restriction",
    text: "Die Abtretung von Forderungen gegen den Anbieter ist ohne vorherige schriftliche Zustimmung des Anbieters ausgeschlossen.",
    expectedSeverity: "hoch"
  },
  {
    type: "force_majeure_asymmetry",
    text: "Ereignisse höherer Gewalt befreien den Anbieter von seinen Leistungspflichten, während die Zahlungspflicht des Kunden uneingeschränkt fortbesteht.",
    expectedSeverity: "hoch"
  },
  {
    type: "non_solicitation",
    text: "Dem Kunden ist es für die Dauer von zwei Jahren nach Vertragsende untersagt, Mitarbeiter des Anbieters abzuwerben oder einzustellen; bei Zuwiderhandlung fällt eine Vertragsstrafe an.",
    expectedSeverity: "hoch"
  },
  {
    type: "unilateral_convenience_termination",
    text: "Eine ordentliche Kündigung des Vertrages steht nur dem Anbieter zu; für den Kunden ist das Recht zur ordentlichen Kündigung ausgeschlossen.",
    expectedSeverity: "hoch"
  },
  {
    type: "excessive_survival",
    text: "Sämtliche Geheimhaltungspflichten und nachvertraglichen Bindungen dieses Vertrages überdauern das Vertragsende auf unbestimmte Zeit und gelten zeitlich unbegrenzt fort.",
    expectedSeverity: "hoch"
  },
  {
    type: "price_escalation",
    text: "Die Entgelte erhöhen sich automatisch um 8 % jährlich nach billigem Ermessen des Anbieters ohne gesondertes Sonderkündigungsrecht.",
    expectedSeverity: "hoch"
  },
  {
    type: "salvatorische_klausel_falle",
    text: "Sollten einzelne Bestimmungen unwirksam sein, so gilt diejenige Regelung als vereinbart, die nach billigem Ermessen des Anbieters dem gewollten Zweck am nächsten kommt.",
    expectedSeverity: "hoch"
  },
  {
    type: "extended_retention_of_title",
    text: "Die gelieferten Systeme bleiben im Eigentum des Anbieters bis zum vollständigen Ausgleich sämtlicher Verbindlichkeiten aus der gesamten bestehenden Geschäftsbeziehung.",
    expectedSeverity: "hoch"
  },
  {
    type: "subcontractor_uncontrolled",
    text: "Der Anbieter ist berechtigt, jederzeit ohne vorherige Ankündigung oder Zustimmung beliebige Subunternehmer für personenbezogene Daten einzusetzen.",
    expectedSeverity: "hoch"
  },
  {
    type: "audit_log_manipulation",
    text: "Die Systemprotokolle und Logfiles des Anbieters liefern den vollständigen Beweis aller Zugriffe und gelten als unwiderlegbar.",
    expectedSeverity: "hoch"
  },
  {
    type: "service_suspension",
    text: "Der Anbieter ist berechtigt, bei Zahlungsrückstand ohne Vorankündigung den Zugang sofort zu sperren; die Sperrung lässt die Zahlungspflicht unberührt.",
    expectedSeverity: "hoch"
  },
  {
    type: "insolvency_termination",
    text: "Der Vertrag endet automatisch bei Antrag auf Eröffnung eines Insolvenzverfahrens über das Vermögen des Kunden.",
    expectedSeverity: "hoch"
  },
  {
    type: "tax_indemnity_shift",
    text: "Sämtliche Steuern und Steuernachforderungen gehen zu Lasten des Kunden und sind vom Kunden nachzuentrichten.",
    expectedSeverity: "mittel"
  },
  {
    type: "asymmetric_confidentiality",
    text: "Der Kunde verpflichtet sich, alle ihm bekanntwerdenden geschäftlichen Informationen streng vertraulich zu behandeln und Stillschweigen zu bewahren.",
    expectedSeverity: "mittel"
  },
  {
    type: "as_is_software_disclaimer",
    text: "Die Software wird wie gesehen bereitgestellt; jede Sachmängelhaftung für Standardfunktionen ist vollständig ausgeschlossen.",
    expectedSeverity: "hoch"
  },
  {
    type: "entire_agreement",
    text: "Mündliche Nebenabreden bestehen nicht; frühere Absprachen und Verhandlungen verlieren ihre Gültigkeit.",
    expectedSeverity: "mittel"
  },
  {
    type: "marketing_reference",
    text: "Der Anbieter ist berechtigt, den Kunden als Referenz zu benennen und das Logo zu Werbezwecken zu nutzen.",
    expectedSeverity: "mittel"
  },
  {
    type: "feedback_ip_grab",
    text: "Sämtliche Vorschläge, Feedback und Ideen des Kunden gehen unentgeltlich in das Eigentum des Anbieters über.",
    expectedSeverity: "mittel"
  },
  {
    type: "fictitious_receipt",
    text: "Rechnungen und Mitteilungen gelten 3 Tage nach Absendung als zugegangen.",
    expectedSeverity: "mittel"
  },
  {
    type: "unverbindliche_sla_falle",
    text: "Die Verfügbarkeit von 99,9% ist unverbindlich und begründet keine vertraglichen Minderungsansprüche.",
    expectedSeverity: "hoch"
  }
];

test("new detectors suite: all newly defined detectors trigger accurately", () => {
  assert.equal(NEW_RULES.length, 27);
  assert.equal(LATEST_RULES.length, 7);
  for (const tc of TEST_CASES) {
    const rule = NEW_RULES.find(r => r.type === tc.type);
    assert.ok(rule, `Rule ${tc.type} must exist in NEW_RULES`);
    
    let hit = false;
    let sev = null;
    for (const pat of rule.patterns) {
      const m = pat.exec(tc.text);
      if (m) {
        hit = true;
        sev = typeof rule.severity === "function" ? rule.severity(tc.text, m) : rule.severity;
        break;
      }
    }
    assert.ok(hit, `Rule ${tc.type} should match text: "${tc.text}"`);
    assert.equal(sev, tc.expectedSeverity, `Severity for ${tc.type} should be ${tc.expectedSeverity}`);
  }

  const latestCases = [
    {
      type: "agency_commission_exclusion",
      text: "Ein Direktauftrag eines Kunden aus dem zugewiesenen Gebiet begründet keine Provision.",
      expectedSeverity: "hoch",
    },
    {
      type: "agency_goodwill_waiver",
      text: "Nach dem Vertragsende besteht kein Anspruch auf Ausgleich für den aufgebauten Kundenstamm.",
      expectedSeverity: "hoch",
    },
    {
      type: "international_data_processing",
      text: "Personenbezogene Daten werden auf Servern außerhalb der Europäischen Union verarbeitet.",
      expectedSeverity: "mittel",
    },
    {
      type: "mandatory_expert_determination",
      text: "Bei Streitigkeiten wird ein Sachverständigenverfahren durchgeführt. Die Kosten des Verfahrens sind vom Kunden vorzustrecken; der Rechtsweg ist erst nach Abschluss des Verfahrens eröffnet.",
      expectedSeverity: "mittel",
    },
    {
      type: "service_credit_forfeiture",
      text: "Werden kritisch eingestufte Aktualisierungen nicht binnen 24 Stunden eingespielt, entfällt der Anspruch auf vereinbarte Gutschriften.",
      expectedSeverity: "mittel",
    },
    {
      type: "early_termination_charge",
      text: "Bei vorzeitiger Beendigung schuldet der Besteller eine pauschale Abstandszahlung von 25 Prozent der offenen Vergütung.",
      expectedSeverity: "mittel",
    },
    {
      type: "maintenance_conditional_shutdown",
      text: "Der Auftragnehmer darf die Anlage nach zwei Jahren abschalten, wenn kein Wartungsvertrag abgeschlossen wurde.",
      expectedSeverity: "mittel",
    },
  ];
  for (const tc of latestCases) {
    const rule = LATEST_RULES.find((candidate) => candidate.type === tc.type);
    assert.ok(rule, `Rule ${tc.type} must exist in LATEST_RULES`);
    const matched = rule.patterns.find((pattern) => pattern.test(tc.text));
    assert.ok(matched, `${tc.type} should match its test clause`);
    const match = matched.exec(tc.text);
    const severity = typeof rule.severity === "function" ? rule.severity(tc.text, match) : rule.severity;
    assert.equal(severity, tc.expectedSeverity);
  }
});
