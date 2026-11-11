import test from 'node:test';
import assert from 'node:assert/strict';
import { analyzeContract } from '../src/analyze.js';

test('nexusscale msa real-world audit: detects all comprehensive traps', () => {
  const contract = `RAHMENVERTRAG ÜBER DIE BEREITSTELLUNG DIGITALER INFRASTRUKTUR- UND DATENVERARBEITUNGSDIENSTE (SaaS-AGB / MSA)
Zwischen:

NexusScale Technologies S.à r.l., eingetragen im Handels- und Gesellschaftsregister Luxemburg unter B-984120, geschäftsansässig 14, Rue de l’Innovation, L-1240 Luxembourg (nachfolgend „Dienstleister“)

und

dem im digitalen Bestellschein ausgewiesenen gewerblichen Vertragspartner (nachfolgend „Kunde“; Dienstleister und Kunde nachfolgend einzeln eine „Partei“ und gemeinsam die „Parteien“).

§ 1 Gegenstand der Vereinbarung und Vorrangregelung
1.1 Der Dienstleister erbringt modulare, cloudbasierte Infrastruktur- und Analyseleistungen gemäß den zum Zeitpunkt der Bereitstellung gültigen Service-Spezifikationen. Soweit im jeweiligen Leistungsschein (Service Order Form) oder in etwaigen Anhängen abweichende Bestimmungen getroffen werden, gehen diese den Regelungen dieses Rahmenvertrags vor, es sei denn, es handelt sich um Haftungs-, Entgelt- oder Geistiges-Eigentum-Regelungen der §§ 6, 7 und 12, welche ungeachtet entgegenstehender Nebenabreden stets zwingend Vorrang genießen.

1.2 Allgemeine Geschäftsbedingungen des Kunden finden selbst dann keine Anwendung, wenn der Kunde in Angebots- oder Bestätigungsschreiben auf deren Geltung hinweist und der Dienstleister dem nicht ausdrücklich widerspricht.

§ 2 Bereitstellung, Service Levels und Wartung
2.1 Der Dienstleister bemüht sich nach besten wirtschaftlichen Kräften (commercially reasonable efforts), eine mittlere Verfügbarkeit der Kernsysteme von 99,5 % auf das Kalenderjahr gerechnet zu gewährleisten. Zeiten planmäßiger Wartung, Notfallwartungen, netzwerkbedingte Ausfälle außerhalb des autonomen Systems des Dienstleisters sowie Ausfallzeiten infolge höherer Gewalt (Force Majeure) oder behördlicher Anordnungen gelten nicht als Beeinträchtigung der Verfügbarkeit.

2.2 Etwaige Service-Level-Credits werden ausschließlich auf zukünftige Rechnungen angerechnet, verfallen jedoch ersatzlos, sofern der Kunde den Minderungsanspruch nicht binnen 72 Stunden nach Beginn des maßgeblichen Vorfalls über das Ticketsystem des Dienstleisters formell anmeldet.

§ 3 Vergütung, Anpassungsklauseln und Zahlungsverzug
3.1 Die Vergütung bemisst sich nach den im Bestellschein fixierten Gebühren zzgl. der jeweils geltenden gesetzlichen Umsatzsteuer.

3.2 Zur Abfederung von Infrastruktur-, Hosting- und Personalkostensteigerungen behält sich der Dienstleister das Recht vor, die vereinbarten wiederkehrenden Entgelte jeweils zum 1. Januar eines Kalenderjahres mit einer Vorankündigungsfrist von mindestens vierzehn (14) Kalendertagen in Textform um bis zu fünfzehn Prozent (15 %) bezogen auf das vorangegangene Abrechnungsjahr anzupassen. Widerspricht der Kunde dieser Erhöhung nicht innerhalb von zehn (10) Werktagen ab Zugang der Mitteilung, gilt die Entgeltanpassung als unwiderruflich genehmigt; hierauf wird der Dienstleister in der Mitteilung hinweisen.

3.3 Sollte der Kunde mit einer fälligen Zahlung um mehr als vierzehn (14) Tage in Verzug geraten, ist der Dienstleister berechtigt, ohne vorherige Abmahnung den Zugang zur Softwareplattform bis zum vollständigen Zahlungsausgleich zu sperren, wobei die Verpflichtung des Kunden zur Entrichtung der vertraglichen Grundgebühren hiervon unberührt bleibt.

§ 4 Datennutzung, Telemetrie und Modelltraining
4.1 Die Parteien sind sich einig, dass sämtliche durch den Kunden oder dessen Endnutzer im Zuge der Inanspruchnahme der vertragsgegenständlichen Dienste in die Infrastruktur eingebrachten oder daraus resultierenden Datenstrukturen und Transaktionslogs als Telemetrie- und Betriebsdaten gelten, soweit sie keine direkten personenbezogenen Identifikatoren mehr enthalten oder einer Pseudonymisierung nach Maßgabe der internen Richtlinien des Dienstleisters unterzogen wurden.

4.2 Der Kunde räumt dem Dienstleister hiermit ein weltweites, unwiderrufliches, übertragbares, unterlizenzierbares und gebührenfreies Nutzungsrecht an sämtlichen dergestalt aggregierten oder verarbeiteten Datensätzen ein, insbesondere zum Zwecke des Trainings, des Finetunings, der Validierung und der Weiterentwicklung interner Algorithmen, statistischer Vorhersagemodelle und Machine-Learning-Komponenten.

§ 5 Vertraulichkeit und Referenznennung
5.1 Die Parteien verpflichten sich, vertrauliche Informationen der jeweils anderen Partei vertraulich zu behandeln und nicht an unbefugte Dritte weiterzugeben.

5.2 Ungeachtet der Bestimmungen in Ziffer 5.1 ist der Dienstleister berechtigt, den Kundennamen sowie Unternehmenskennzeichen und Bildmarken (Logos) des Kunden in Marketingunterlagen, auf öffentlichen Webseiten, in Präsentationen und bei Pressemitteilungen im Rahmen von Referenzangaben unentgeltlich zu nutzen. Dieses Recht erstreckt sich über die gesamte Vertragslaufzeit sowie einen Zeitraum von vierundzwanzig (24) Monaten nach Beendigung des Vertragsverhältnisses.

§ 6 Haftungsmaßstab und Freistellungsverpflichtungen
6.1 Der Dienstleister haftet unbeschränkt für Schäden aus der Verletzung des Lebens, des Körpers oder der Gesundheit sowie für vorsätzlich oder grob fahrlässig herbeigeführte Schäden.

6.2 Bei einfacher Fahrlässigkeit haftet der Dienstleister – vorbehaltlich der Ziffer 6.1 – nur bei Verletzung einer wesentlichen Vertragspflicht (Kardinalpflicht). In diesem Fall ist die Gesamthaftung des Dienstleisters für sämtliche Schäden, die innerhalb eines Vertragsjahres entstehen, summenmäßig auf den Betrag begrenzt, den der Kunde in den vorangegangenen drei (3) Monaten an Vergütung tatsächlich an den Dienstleister entrichtet hat. Eine Haftung für entgangenen Gewinn, mittelbare Schäden, Folgeschäden, Datenverlust oder Betriebsunterbrechungen ist im gesetzlich zulässigen Rahmen vollständig ausgeschlossen.

6.3 Der Kunde verpflichtet sich, den Dienstleister von sämtlichen Ansprüchen, Bußgeldern, Schäden und Prozesskosten Dritter (einschließlich Aufsichtsbehörden) vollumfänglich freizustellen, die daraus resultieren, dass die vom Kunden über die Plattform verarbeiteten Daten oder Workflows Rechte Dritter verletzen oder gegen regulatorische Bestimmungen verstoßen.

§ 7 Audit-Rechte und Nachweisführung
7.1 Der Dienstleister oder ein von ihm beauftragter, zur Verschwiegenheit verpflichteter Wirtschaftsprüfer ist berechtigt, nach Vorankündigung von fünf (5) Werktagen während der üblichen Geschäftszeiten die Geschäftsräume, IT-Systeme und Nutzerkonten des Kunden zu inspizieren, um die Einhaltung der Lizenzbeschränkungen und Abrechnungsparameter zu verifizieren.

7.2 Ergibt eine solche Überprüfung eine Überschreitung des lizenzierten Nutzungsvolumens um mehr als drei Prozent (3 %) oder andere wesentliche Vertragsverstöße, hat der Kunde die Kosten des Audits in voller Höhe zu tragen und die Differenz für den gesamten bisherigen Bereitstellungszeitraum zuzüglich einer Vertragsstrafe in Höhe von zwanzig Prozent (20 %) des nachberechneten Betrages binnen zehn (10) Tagen auszugleichen.

§ 8 Laufzeit, Verlängerung und Kündigung
8.1 Dieser Vertrag tritt mit Freischaltung der Dienste in Kraft und wird für eine anfängliche feste Mindestvertragslaufzeit von vierundzwanzig (24) Monaten geschlossen.

8.2 Er verlängert sich jeweils automatisch um einen weiteren Festzeitraum von zwölf (12) Monaten, sofern er nicht von einer Partei mit einer Kündigungsfrist von mindestens einhundertachtzig (180) Kalendertagen vor Ablauf der jeweiligen Vertragslaufzeit schriftlich per eingeschriebenem Brief an den Sitz der anderen Partei gekündigt wird.

8.3 Das Recht zur außerordentlichen Kündigung aus wichtigem Grund bleibt unberührt. Ein wichtiger Grund liegt für den Dienstleister insbesondere dann vor, wenn der Kunde mit fälligen Zahlungen um mehr als dreißig (30) Tage in Verzug ist oder wesentliche Kooperationspflichten nach Ziffer 7.1 verweigert.

§ 9 Abtretung und Change of Control
9.1 Der Kunde ist ohne vorherige schriftliche Zustimmung des Dienstleisters nicht berechtigt, Rechte und Pflichten aus diesem Vertrag ganz oder teilweise an Dritte (einschließlich verbundener Unternehmen i.S.d. §§ 15 ff. AktG) abzutreten.

9.2 Der Dienstleister ist berechtigt, den Vertrag im Falle eines direkten oder indirekten Kontrollwechsels auf Seiten des Kunden (Change of Control) mit sofortiger Wirkung außerordentlich zu kündigen oder eine Sicherheitsleistung in Höhe des voraussichtlichen Jahresentgelts zu verlangen. Demgegenüber ist der Dienstleister jederzeit berechtigt, diesen Vertrag ohne Zustimmung des Kunden auf verbundene Unternehmen oder Rechtsnachfolger im Zuge von Umstrukturierungen oder Unternehmenstransaktionen zu übertragen.

§ 10 Schlussbestimmungen, Gerichtsstand und Salvatorische Klausel
10.1 Es gilt ausschließlich das Recht des Großherzogtums Luxemburg unter Ausschluss des UN-Kaufrechts (CISG) und der kollisionsrechtlichen Verweisungsnormen.

10.2 Ausschließlicher Gerichtsstand für sämtliche Streitigkeiten aus oder im Zusammenhang mit diesem Vertrag ist die Stadt Luxemburg. Der Dienstleister bleibt jedoch berechtigt, am allgemeinen Gerichtsstand des Kunden Klage zu erheben.

10.3 Sollten einzelne Bestimmungen dieses Vertrages ganz oder teilweise unwirksam oder undurchführbar sein oder werden, berührt dies die Wirksamkeit der übrigen Bestimmungen nicht. Anstelle der unwirksamen Bestimmung gilt eine solche Regelung als vereinbart, die dem wirtschaftlichen Zweck der unwirksamen Bestimmung am nächsten kommt.`;

  const res = analyzeContract(contract);
  assert.equal(res.riskLevel, 'hoch');
  assert.equal(res.riskScore, 100);
  assert.ok(res.findings.length >= 18, `Expected at least 18 findings, got ${res.findings.length}`);

  const types = new Set(res.findings.map(f => f.type));
  const expectedTypes = [
    'unverbindliche_sla_falle',
    'deemed_acceptance',
    'unilateral_change',
    'service_suspension',
    'liability',
    'third_party_indemnity',
    'penalty',
    'auto_renewal',
    'notice_period',
    'jurisdiction',
    'salvatorische_klausel_falle',
    'entire_agreement',
    'price_escalation',
    'data_grant',
    'marketing_reference',
    'unreasonable_audit',
    'written_form_trap',
    'asymmetric_termination',
  ];

  for (const expected of expectedTypes) {
    assert.ok(types.has(expected), `Missing expected detector in audit: ${expected}`);
  }
});
