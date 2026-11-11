import test from 'node:test';
import assert from 'node:assert/strict';
import { analyzeContract } from '../src/analyze.js';

test('enterprise benchmark: detects all 54 standard commercial contract clauses & boilerplates', () => {
  const tests = [
    // 1. auto_renewal
    { name: 'auto_renewal DE', text: 'Der Vertrag verlängert sich automatisch um jeweils ein weiteres Jahr, sofern er nicht 3 Monate vor Ablauf gekündigt wird.', expected: 'auto_renewal' },
    { name: 'auto_renewal EN', text: 'This Agreement automatically renews for successive 12-month periods unless either party gives 60 days written notice.', expected: 'auto_renewal' },
    // 2. notice_period
    { name: 'notice_period DE', text: 'Die Kündigungsfrist beträgt 6 Monate zum Jahresende.', expected: 'notice_period' },
    // 3. liability
    { name: 'liability Vorsatz', text: 'Der Anbieter haftet nur bei Vorsatz.', expected: 'liability' },
    { name: 'liability Fahrlässigkeit', text: 'Die Haftung für leichte Fahrlässigkeit wird ausgeschlossen.', expected: 'liability' },
    { name: 'liability EN Consequential', text: 'In no event shall Provider be liable for any indirect, special, incidental, or consequential damages.', expected: 'liability' },
    // 4. penalty
    { name: 'penalty Vertragsstrafe', text: 'Für jeden Fall der Zuwiderhandlung wird eine Vertragsstrafe in Höhe von 10.000 Euro fällig.', expected: 'penalty' },
    { name: 'penalty Mahnpauschale', text: 'Für jede Mahnung wird ein pauschalierter Schadensersatz von 25,00 Euro berechnet, es sei denn der Kunde weist nach, dass überhaupt kein Schaden entstanden ist.', expected: 'penalty' },
    // 5. jurisdiction
    { name: 'jurisdiction Sitz', text: 'Ausschließlicher Gerichtsstand für alle Streitigkeiten ist der Sitz des Anbieters.', expected: 'jurisdiction' },
    { name: 'jurisdiction Delaware', text: 'Es gilt das Recht des Staates Delaware unter Ausschluss des UN-Kaufrechts.', expected: 'jurisdiction' },
    { name: 'jurisdiction EN venue', text: 'This Agreement is governed by the laws of England and the parties submit to the exclusive jurisdiction of the English courts.', expected: 'jurisdiction' },
    // 6. non_compete
    { name: 'non_compete DE', text: 'Dem Kunden ist es untersagt, nach Beendigung des Vertrages für 2 Jahre im gleichen Geschäftsfeld konkurrierend tätig zu werden.', expected: 'non_compete' },
    // 7. data_grant
    { name: 'data_grant DE', text: 'Der Kunde räumt dem Anbieter an allen Inhalten ein zeitlich und räumlich uneingeschränktes ausschließliches Nutzungsrecht ein.', expected: 'data_grant' },
    // 8. default_interest
    { name: 'default_interest DE', text: 'Bei Zahlungsverzug werden Verzugszinsen in Höhe von 12 % p.a. fällig.', expected: 'default_interest' },
    // 9. unilateral_change
    { name: 'unilateral_change DE', text: 'Änderungen der Bedingungen gelten als genehmigt, wenn der Kunde nicht binnen 4 Wochen schriftlich widerspricht.', expected: 'unilateral_change' },
    // 10. setoff_restriction
    { name: 'setoff_restriction Ausschluss', text: 'Eine Aufrechnung mit Gegenansprüchen ist ausgeschlossen.', expected: 'setoff_restriction' },
    { name: 'setoff_restriction Nicht berechtigt', text: 'Der Kunde ist zur Aufrechnung nicht berechtigt.', expected: 'setoff_restriction' },
    { name: 'setoff_restriction Gegenansprüche', text: 'Gegenansprüche berechtigen nicht zur Aufrechnung.', expected: 'setoff_restriction' },
    { name: 'setoff_restriction Zurückbehaltung', text: 'Ein Zurückbehaltungsrecht steht dem Kunden nicht zu.', expected: 'setoff_restriction' },
    // 11. deemed_acceptance
    { name: 'deemed_acceptance Rügefrist', text: 'Mängel sind innerhalb von 3 Tagen schriftlich zu rügen, andernfalls gilt das Werk als vorbehaltlos genehmigt.', expected: 'deemed_acceptance' },
    // 12. statute_of_limitations
    { name: 'statute_of_limitations Verkürzung', text: 'Alle Mängelansprüche verjähren innerhalb von 6 Monaten ab Ablieferung.', expected: 'statute_of_limitations' },
    // 13. written_form_trap
    { name: 'written_form_trap Doppelt', text: 'Änderungen bedürfen der Schriftform. Die Abbedingung dieser Schriftformklausel bedarf ihrerseits der Schriftform.', expected: 'written_form_trap' },
    { name: 'written_form_trap Kündigung', text: 'Kündigungen bedürfen zu ihrer Wirksamkeit der Schriftform per Einschreiben.', expected: 'written_form_trap' },
    // 14. burden_of_proof
    { name: 'burden_of_proof Beweislast', text: 'Die Beweislast für die Fehlerhaftigkeit der Software trägt stets der Kunde.', expected: 'burden_of_proof' },
    // 15. dynamic_url_referral
    { name: 'dynamic_url_referral URL', text: 'Es gelten die jeweils aktuellen Richtlinien abrufbar unter https://example.com/guidelines in ihrer jeweils gültigen Fassung.', expected: 'dynamic_url_referral' },
    // 16. vicarious_agents
    { name: 'vicarious_agents Gehilfen', text: 'Der Anbieter schließt jegliche Haftung für Handlungen seiner Erfüllungsgehilfen und Subunternehmer aus.', expected: 'vicarious_agents' },
    // 17. unreasonable_audit
    { name: 'unreasonable_audit Vor-Ort', text: 'Der Anbieter darf die Server und Büroräume des Kunden jederzeit ohne Vorankündigung auditieren.', expected: 'unreasonable_audit' },
    // 18. data_lockin
    { name: 'data_lockin Exportgebühr', text: 'Für den Export und die Migration der Kundendaten bei Vertragsbeendigung wird eine gesonderte Herausgabegebühr von 5.000 Euro erhoben.', expected: 'data_lockin' },
    // 19. third_party_indemnity
    { name: 'third_party_indemnity DE', text: 'Der Kunde stellt den Anbieter von sämtlichen Ansprüchen Dritter verschuldensunabhängig frei.', expected: 'third_party_indemnity' },
    { name: 'third_party_indemnity EN', text: 'Customer shall defend, indemnify, and hold harmless Provider against any third-party claims.', expected: ['third_party_indemnity', 'liability'] },
    // 20. asymmetric_termination
    { name: 'asymmetric_termination DE', text: 'Das Recht des Kunden zur außerordentlichen Kündigung aus wichtigem Grund wird ausgeschlossen.', expected: 'asymmetric_termination' },
    // 21. assignment_restriction
    { name: 'assignment_restriction DE', text: 'Die Abtretung von Geldforderungen gegen den Anbieter ist ohne vorherige schriftliche Zustimmung ausgeschlossen.', expected: 'assignment_restriction' },
    // 22. force_majeure_asymmetry
    { name: 'force_majeure_asymmetry DE', text: 'Höhere Gewalt befreit den Anbieter von der Leistungspflicht, während die Zahlungspflicht des Kunden in voller Höhe fortbesteht.', expected: 'force_majeure_asymmetry' },
    // 23. non_solicitation
    { name: 'non_solicitation DE', text: 'Dem Kunden ist es für 24 Monate nach Vertragsende untersagt, Mitarbeiter des Anbieters abzuwerben oder einzustellen.', expected: 'non_solicitation' },
    // 24. unilateral_convenience_termination
    { name: 'unilateral_convenience_termination DE', text: 'Eine ordentliche Kündigung steht nur dem Anbieter zu.', expected: 'unilateral_convenience_termination' },
    // 25. excessive_survival
    { name: 'excessive_survival DE', text: 'Die Geheimhaltungspflicht und Treuepflichten gelten unbegrenzt und fortwirkend auf unbestimmte Zeit über die Beendigung hinaus.', expected: 'excessive_survival' },
    // 26. price_escalation
    { name: 'price_escalation DE', text: 'Der Anbieter ist berechtigt, die Preise jährlich um bis zu 10 % nach billigem Ermessen einseitig zu erhöhen.', expected: 'price_escalation' },
    // 27. salvatorische_klausel_falle
    { name: 'salvatorische_klausel_falle DE', text: 'Sollte eine Bestimmung unwirksam sein, verpflichten sich die Parteien, die unwirksame Bestimmung durch eine solche zu ersetzen, die dem wirtschaftlichen Zweck am nächsten kommt.', expected: 'salvatorische_klausel_falle' },
    // 28. extended_retention_of_title
    { name: 'extended_retention_of_title DE', text: 'Die Ware bleibt Eigentum des Verkäufers bis zur vollständigen Begleichung aller Forderungen aus der gesamten Geschäftsbeziehung.', expected: 'extended_retention_of_title' },
    // 29. subcontractor_uncontrolled
    { name: 'subcontractor_uncontrolled DE', text: 'Der Auftragnehmer ist berechtigt, Unterauftragnehmer nach freiem Ermessen ohne Information des Auftraggebers einzusetzen.', expected: 'subcontractor_uncontrolled' },
    // 30. audit_log_manipulation
    { name: 'audit_log_manipulation DE', text: 'Die elektronischen Protokolldateien und Logfiles des Anbieters gelten als unwiderlegbarer Vollbeweis für erbrachte Leistungen.', expected: 'audit_log_manipulation' },
    // 31. service_suspension
    { name: 'service_suspension DE', text: 'Bei Zahlungsverzug von mehr als 7 Tagen ist der Anbieter berechtigt, den Zugang zur Cloud-Infrastruktur sofort zu sperren.', expected: 'service_suspension' },
    // 32. insolvency_termination
    { name: 'insolvency_termination DE', text: 'Der Vertrag endet automatisch bei Stellung eines Antrags auf Eröffnung des Insolvenzverfahrens über das Vermögen des Kunden.', expected: 'insolvency_termination' },
    // 33. tax_indemnity_shift
    { name: 'tax_indemnity_shift DE', text: 'Nachträgliche Umsatzsteuer- oder Quellensteuernachforderungen der Finanzbehörden sind vollumfänglich vom Kunden zu erstatten.', expected: 'tax_indemnity_shift' },
    // 34. asymmetric_confidentiality
    { name: 'asymmetric_confidentiality DE', text: 'Der Kunde verpflichtet sich zur strengsten Geheimhaltung aller Unterlagen; für den Anbieter gilt dies nicht.', expected: 'asymmetric_confidentiality' },
    // 35. as_is_software_disclaimer
    { name: 'as_is_software_disclaimer DE', text: 'Die Software wird im Zustand wie gesehen und ohne Mängelgewähr bereitgestellt.', expected: 'as_is_software_disclaimer' },
    { name: 'as_is_software_disclaimer EN', text: 'Software is provided as is without warranty of any kind.', expected: 'as_is_software_disclaimer' },
    // 36. entire_agreement
    { name: 'entire_agreement DE Nebenabreden', text: 'Mündliche Nebenabreden bestehen nicht.', expected: 'entire_agreement' },
    { name: 'entire_agreement DE Vorherige Absprachen', text: 'Vorherige Vereinbarungen und Angebote verlieren mit Unterzeichnung ihre Gültigkeit.', expected: 'entire_agreement' },
    { name: 'entire_agreement EN', text: 'This Agreement constitutes the entire agreement between the parties and supersedes all prior agreements and understandings.', expected: 'entire_agreement' },
    // 37. marketing_reference
    { name: 'marketing_reference DE', text: 'Der Anbieter ist berechtigt, den Kunden auf seiner Website als Referenz zu nennen.', expected: 'marketing_reference' },
    // 38. feedback_ip_grab
    { name: 'feedback_ip_grab DE', text: 'Alle Verbesserungsvorschläge und Feedback gehen unentgeltlich in das ausschließliche Eigentum des Anbieters über.', expected: 'feedback_ip_grab' },
    // 39. fictitious_receipt
    { name: 'fictitious_receipt DE', text: 'Rechnungen gelten 3 Tage nach Absendung als zugegangen.', expected: 'fictitious_receipt' },
    // 40. unverbindliche_sla_falle
    { name: 'unverbindliche_sla_falle DE', text: 'Die angegebene Verfügbarkeit von 99,5 % ist unverbindlich und begründet keinen Schadensersatzanspruch.', expected: 'unverbindliche_sla_falle' },
  ];

  for (const t of tests) {
    const res = analyzeContract(t.text);
    const types = res.findings.map(f => f.type);
    const expectedList = Array.isArray(t.expected) ? t.expected : [t.expected];
    const ok = expectedList.some(e => types.includes(e));
    assert.ok(ok, `Clause "${t.name}" failed detection! Expected: ${JSON.stringify(t.expected)}, Got: [${types.join(', ')}], Text: "${t.text}"`);
  }
});
