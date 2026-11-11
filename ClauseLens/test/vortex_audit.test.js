import test from 'node:test';
import assert from 'node:assert/strict';
import { analyzeContract } from '../src/analyze.js';

test('vortex engineering msa-dev real-world audit: detects all comprehensive traps with zero false positives', () => {
  const contract = `RAHMENVERTRAG ÜBER INDIVIDUALENTWICKLUNG, IMPLEMENTIERUNG UND SYSTEMINTEGRATION (MSA-DEV)
Zwischen:

Vortex Engineering Solutions GmbH, vertreten durch die Geschäftsführung, Industriestraße 88, 60311 Frankfurt am Main (nachfolgend „Auftragnehmer“)

und

dem im Einzelauftrag ausgewiesenen Auftraggeber (nachfolgend „Auftraggeber“; gemeinsam die „Parteien“).

§ 1 Vertragsgegenstand und Leistungscharakter
1.1 Der Auftragnehmer erbringt für den Auftraggeber Entwicklungs-, Integrations- und Beratungsleistungen auf Basis agiler Methodiken (z. B. Scrum oder Kanban) gemäß den in den jeweiligen Leistungsbeschreibungen (Statements of Work – „SOW“) definierten Meilensteinen und Epics.

1.2 Unbeschadet der Verwendung von Begriffen wie „Werk“, „Abnahme“, „Liefergegenstand“ oder „Projektziel“ in den SOWs vereinbaren die Parteien ausdrücklich, dass der Auftragnehmer die Leistungen ausschließlich als Dienstvertrag im Sinne der §§ 611 ff. BGB schuldet. Die Erzielung eines konkreten wirtschaftlichen, funktionalen oder technischen Erfolgs ist nicht geschuldet, es sei denn, ein solcher Erfolg wurde im jeweiligen SOW unter ausdrücklicher Zitierung dieser Ziffer 1.2 als „Garantierter Werkerfolg nach § 631 BGB“ tituliert und mit gesonderten Qualitätsmetriken versehen.

§ 2 Vergütung, Zeiterfassung und Budgetgrenzen
2.1 Die Abrechnung erfolgt nach tatsächlichem Zeit- und Materialaufwand (Time & Material) auf Grundlage der im jeweiligen SOW vereinbarten Tagessätze. Ein Personentag umfasst acht (8) Zeitstunden. Soweit im SOW ein „Budgetrahmen“, „Kostendeckel“ oder „Cap“ genannt wird, versteht sich dieser lediglich als unverbindliche Kostenschätzung (Soft Cap).

2.2 Der Auftragnehmer ist berechtigt, den Schätzrahmen um bis zu fünfundzwanzig Prozent (25 %) zu überschreiten, ohne dass es hierfür einer gesonderten Zustimmung des Auftraggebers bedarf. Zeigt sich im Projektverlauf, dass der Schätzrahmen um mehr als 25 % überschritten wird, informiert der Auftragnehmer den Auftraggeber hierüber; widerspricht der Auftraggeber dem weiteren Fortgang der Arbeiten nicht binnen fünf (5) Kalendertagen in Textform, gilt die Budgeterweiterung als genehmigt.

2.3 Detaillierte Leistungsnachweise (Timesheets) werden dem Auftraggeber zweiwöchentlich digital übermittelt. Sie gelten als vorbehaltlos genehmigt und freigegeben, sofern der Auftraggeber nicht innerhalb von sieben (7) Kalendertagen nach Übermittlung begründeten Widerspruch erhebt.

§ 3 Mitwirkungspflichten und Abnahmefiktion
3.1 Der Auftraggeber stellt die für die Durchführung der Entwicklungsarbeiten erforderlichen Zugänge, Staging-Umgebungen, API-Schlüssel sowie Testdaten zeitgerecht zur Verfügung. Verzögerungen, die auf einer nicht rechtzeitigen oder unvollständigen Mitwirkung beruhen, führen zu einer automatischen Verlängerung etwaiger Lieferfristen um den doppelten Zeitraum des Verzugs zzgl. einer Wiederanlaufzeit von zehn (10) Werktagen; etwaige Vorhalte- und Leerlaufkosten des Auftragnehmers sind vom Auftraggeber nach den regulären Tagessätzen vollumfänglich zu erstatten.

3.2 Soweit die Parteien für Teilleistungen formelle Review-Phasen vereinbaren, gilt eine Teilleistung oder ein Release als vollständig und mangelfrei abgenommen, wenn (a) der Auftraggeber die Abnahme nicht binnen zehn (10) Werktagen unter Benennung von Fehlern der Schweregrade 1 (Showstopper) verweigert oder (b) der Auftraggeber die Teilleistung produktiv nutzt oder in eine für Dritte zugängliche Produktionsumgebung überführt.

§ 4 Rechte an Arbeitsergebnissen und Vorbestehendem IP
4.1 Der Auftragnehmer räumt dem Auftraggeber an den individuell und ausschließlich für den Auftraggeber im Rahmen eines SOW erstellten Softwaremodulen mit vollständiger Bezahlung der geschuldeten Vergütung ein einfaches, nicht übertragbares, zeitlich und räumlich unbeschränktes Recht ein, diese für interne Geschäftszwecke zu nutzen.

4.2 Vom Rechtserwerb nach Ziffer 4.1 ausdrücklich ausgenommen sind Basiskomponenten, Frameworks, Bibliotheken, wiederverwendbare Software-Artefakte, mathematische Algorithmen sowie generische Code-Fragmente, die der Auftragnehmer bereits vor Vertragsschluss besessen hat oder die während der Durchführung des Vertrages unabhängig von spezifischen Kundendaten generiert wurden („Auftragnehmer-IP“). An diesen Komponenten erhält der Auftraggeber lediglich eine widerrufliche, nicht-exklusive Nutzungslizenz, welche mit Beendigung dieses Rahmenvertrags – gleich aus welchem Rechtsgrund – automatisch erlischt.

4.3 Der Auftraggeber räumt dem Auftragnehmer ein uneingeschränktes, unentgeltliches, unwiderrufliches und weltweites Nutzungs- und Verwertungsrecht an allen im Projekt gewonnenen Erkenntnissen, Architekturentwürfen und generischen Optimierungsmethoden ein.

§ 5 Abwerbe- und Beschäftigungsverbot
5.1 Der Auftraggeber verpflichtet sich, während der Laufzeit dieses Rahmenvertrags sowie für einen Zeitraum von vierundzwanzig (24) Monaten nach dessen Beendigung weder unmittelbar noch mittelbar Mitarbeiter, freie Mitarbeiter (Freelancer) oder Subunternehmer des Auftragnehmers abzuwerben, anzustellen oder auf freiberuflicher Basis für sich oder verbundene Unternehmen tätig werden zu lassen.

5.2 Für jeden Fall der schuldhaften Zuwiderhandlung gegen Ziffer 5.1 verpflichtet sich der Auftraggeber zur Zahlung einer verschuldensunabhängigen Vertragsstrafe in Höhe des doppelten Bruttojahresgehalts des betreffenden Mitarbeiters, mindestens jedoch in Höhe von einhundertfünfzigtausend Euro (150.000,00 EUR), unter Verzicht auf die Einrede des Fortsetzungszusammenhangs.

§ 6 Sachmängel, Verjährung und Rügepflichten
6.1 Der Auftragnehmer gewährleistet, dass die Software den im maßgeblichen SOW explizit vereinbarten Spezifikationen im Wesentlichen entspricht. Die Verjährungsfrist für Mängelansprüche beträgt sechs (6) Monate ab dem Zeitpunkt des Zugangs der Bereitstellungsanzeige oder der fiktiven Abnahme gemäß Ziffer 3.2.

6.2 Die Mängelansprüche des Auftraggebers setzen voraus, dass dieser den Mangel unverzüglich, spätestens jedoch innerhalb von drei (3) Werktagen nach Entdeckung, in reproduzierbarer Form rügt. Unterlässt der Auftraggeber die rechtzeitige Anzeige, gilt die Software hinsichtlich dieses Mangels als genehmigt.

6.3 Im Falle berechtigter Mängel erfolgt die Nachbesserung nach Wahl des Auftragnehmers durch Nachbesserung oder Ersatzlieferung. Schlägt die Nacherfüllung zweimalig fehl, ist der Auftraggeber lediglich zur Minderung berechtigt; ein Rücktritt vom Vertrag oder die Geltendmachung von Schadensersatz statt der Leistung ist ausgeschlossen.

§ 7 Haftung und Risikoverteilung
7.1 Der Auftragnehmer haftet nach den gesetzlichen Bestimmungen für Vorsatz und grobe Fahrlässigkeit.

7.2 Für einfache Fahrlässigkeit haftet der Auftragnehmer ausschließlich bei der Verletzung wesentlicher Vertragspflichten. Die Haftungshöhe ist hierbei für das gesamte Kalenderjahr auf maximal fünfzig Prozent (50 %) der im jeweiligen Einzelauftrag (SOW) vereinbarten Netto-Vergütung gedeckelt. Die Haftung für indirekte Schäden, entgangenen Gewinn, Produktionsausfall, Wiederherstellungskosten von Daten sowie Drittansprüche ist ausgeschlossen.

7.3 Der Auftraggeber trägt die alleinige Verantwortung dafür, dass die im Rahmen des Projekts eingesetzten Drittanbieter-Komponenten (einschließlich Open-Source-Software, Lizenzen und APIs) mit den geltenden Lizenzbedingungen vereinbar sind. Der Auftraggeber stellt den Auftragnehmer insoweit von sämtlichen Lizenzansprüchen Dritter auf erstes Anfordern frei.

§ 8 Zurückbehaltungsrecht und Kündigung
8.1 Das Recht zur Kündigung eines SOW aus wichtigem Grund bleibt vorbehalten. Eine ordentliche Kündigung eines laufenden SOW durch den Auftraggeber ist ausgeschlossen, es sei denn, die Parteien haben im SOW ausdrücklich etwas Abweichendes vereinbart.

8.2 Gerät der Auftraggeber mit einer Teilzahlung um mehr als zehn (10) Bankarbeitstage in Verzug, steht dem Auftragnehmer neben der Geltendmachung von Verzugszinsen ein umfassendes Zurückbehaltungsrecht an sämtlichen Quellcodes, Dokumentationen und Entwicklungsartefakten zu; dies gilt auch für solche Liefergegenstände, die Gegenstand bereits vollständig bezahlter vorangegangener Meilensteine waren.`;

  const res = analyzeContract(contract);
  assert.equal(res.riskLevel, 'hoch');
  assert.equal(res.riskScore, 100);
  assert.ok(res.findings.length >= 15, `Expected at least 15 findings, got ${res.findings.length}`);

  const types = new Set(res.findings.map(f => f.type));
  // Must NOT contain auto_renewal (delivery deadline extension is NOT an auto-renewal)
  assert.equal(types.has('auto_renewal'), false, 'auto_renewal triggered falsely on delivery deadline extension!');

  const expectedTypes = [
    'as_is_software_disclaimer',
    'penalty',
    'deemed_acceptance',
    'data_grant',
    'data_lockin',
    'statute_of_limitations',
    'liability',
    'asymmetric_termination',
    'extended_retention_of_title',
    'unilateral_change',
    'fictitious_receipt',
    'non_solicitation',
  ];

  for (const expected of expectedTypes) {
    assert.ok(types.has(expected), `Missing expected detector in audit: ${expected}`);
  }
});
