import test from 'node:test';
import assert from 'node:assert/strict';
import { analyzeContract } from '../src/analyze.js';

test('cybersentinel mdr-msa real-world audit: detects the hidden cyber-liability traps', () => {
  const contract = `VERTRAG ÜBER MANAGED DETECTION AND RESPONSE SOWIE CYBER INCIDENT HANDLING (MDR-MSA)
Zwischen:

CyberSentinel Defense Ops AG, mit Sitz in Zürich, Schweiz, eingetragen im Handelsregister des Kantons Zürich unter CHE-312.449.102 (nachfolgend „Sicherheitsdienstleister“)

und

dem im Service Level Agreement ausgewiesenen Unternehmenskunden (nachfolgend „Kunde“; gemeinsam die „Parteien“).

§ 1 Leistungsumfang, Telemetrie-Extraktion und Systemeingriffe
1.1 Der Sicherheitsdienstleister stellt dem Kunden Managed-Detection-and-Response-(MDR)-Dienste bereit. Dies umfasst die kontinuierliche Überwachung von Endgeräten, Serverknoten und Netzwerkschnittstellen durch proprietäre Endpoint-Agents und Log-Kollektoren.

1.2 Zur Gewährleistung einer effektiven Bedrohungsabwehr räumt der Kunde dem Sicherheitsdienstleister das Recht ein, im Falle eines vermuteten oder bestätigten Sicherheitsvorfalls ohne vorherige Rücksprache mit dem Kunden vollautomatische oder manuelle Isolationsmaßnahmen vorzunehmen. Dies beinhaltet das Trennen von Systemen vom Netzwerk, das Einfrieren virtueller Instanzen, das Terminieren laufender Prozessketten sowie das Löschen mutmaßlich kompromittierter Datenbestände oder Systemkonfigurationen. Der Kunde verzichtet ausdrücklich auf Ersatzansprüche für Betriebsunterbrechungen, Transaktionsausfälle oder Datenverluste, die aus diesen Abwehrmaßnahmen resultieren.

1.3 Der Kunde autorisiert den Sicherheitsdienstleister, im Rahmen der forensischen Triage unverschlüsselte Speicherabbilder (RAM-Dumps), Payload-Daten, Zertifikate und vertrauliche Schlüsselmaterialien von den überwachten Endpunkten automatisiert auf Cloud-Analyseserver des Sicherheitsdienstleisters außerhalb des Europäischen Wirtschaftsraums (EWR) zu transferieren.

§ 2 Incident Response Retainer, Überkontingente und Notfall-Sätze
2.1 Der Kunde erwirbt mit der monatlichen Grundpauschale ein Vorhalte-Kontingent von zehn (10) Stunden Incident-Response-Bereitschaft pro Quartal. Nicht in Anspruch genommene Stunden verfallen am Ende des jeweiligen Kalenderquartals ersatzlos und sind weder kumulierbar noch rückvergütbar.

2.2 Sobald der Sicherheitsdienstleister eine Sicherheitswarnung der Schweregrade „Kritisch“ oder „Hoch“ klassifiziert, ist er berechtigt, zur akuten Schadenseindämmung unverzüglich zusätzliche forensische Spezialkräfte beizuziehen. Überschreitet der zur Eindämmung erforderliche Aufwand das Restkontingent der Ziffer 2.1, gilt der Kunde als einverstanden mit der Abrechnung der Mehrstunden zu den im Notfall-Tarif ausgewiesenen Sätzen (Faktor 2,5 auf den Basissatz zzgl. eines Wochenend- und Nachtzuschlags von 100 %).

2.3 Eine betragsmäßige Obergrenze (Budget Cap) für Notfallmaßnahmen nach Ziffer 2.2 besteht nicht; eine Deckelung bedarf einer vorherigen gesonderten Vereinbarung im jeweiligen Eskalationsprotokoll.

§ 3 Gewährleistungsausschluss und Erkennungsgenauigkeit
3.1 Die Parteien stimmen darin überein, dass IT-Sicherheitsarchitekturen und Bedrohungslagen einer permanenten dynamischen Veränderung unterliegen. Der Sicherheitsdienstleister garantiert daher weder die unterbrechungsfreie Funktionsfähigkeit der installierten Agents noch das lückenlose Erkennen sämtlicher bösartiger Aktivitäten, Advanced Persistent Threats (APTs), Ransomware-Ausprägungen oder Zero-Day-Exploits.

3.2 Das Ausbleiben einer Sicherheitswarnung oder das Nichtanschlagen heuristischer Detektoren begründet keinen Mangel der Dienstleistung und keine Pflichtverletzung des Sicherheitsdienstleisters.

3.3 Der Kunde trägt die Beweislast dafür, dass eine erfolgreiche Kompromittierung seiner Infrastruktur kausal auf eine grob pflichtwidrige Fehlkonfiguration der Erkennungsregeln durch den Sicherheitsdienstleister zurückzuführen ist und nicht auf unbekannte Angriffsmuster, Verzögerungen im Kundensystem oder Anwendungsfehler des Personals des Kunden beruht.

§ 4 Vertraulichkeitsdurchbrechung und Kooperation mit Dritten
4.1 Ungeachtet der wechselseitigen Verpflichtung zur Geheimhaltung ist der Sicherheitsdienstleister berechtigt, Artefakte, Indicators of Compromise (IoCs), Taktiken, Techniken und Prozeduren (TTPs) sowie extrahierte Schadcode-Muster aus Incident-Analysen in pseudonymisierter oder anonymisierter Form in Threat-Intelligence-Netzwerken, Informationsaustauschkreisen (z. B. ISACs) oder mit Sicherheitsbehörden zu teilen.

4.2 Sollte der Sicherheitsdienstleister von behördlicher oder gerichtlicher Seite zur Herausgabe forensischer Berichte oder im Zuge des Vorfalls gewonnener Datensätze aufgefordert werden, ist er berechtigt, dieser Aufforderung ohne vorherige Information des Kunden nachzukommen, sofern die anordnende Stelle eine Verschwiegenheitsverpflichtung bezüglich des Auskunftsersuchens auferlegt hat oder der Sicherheitsdienstleister die Benachrichtigung als gefahrbegründend für laufende Ermittlungen einstuft.

§ 5 Haftungsbegrenzung und Freistellung bei Cyber-Schäden
5.1 Die Haftung des Sicherheitsdienstleisters für sämtliche Schäden, die im Rahmen oder anlässlich der Erbringung der Sicherheitsdienstleistungen entstehen – einschließlich solcher, die durch Fehlalarme (False Positives) oder Nichtanschlagen (False Negatives) verursacht werden –, ist bei einfacher Fahrlässigkeit pro Schadensereignis auf den Betrag der in den vorangegangenen sechs (6) Monaten vom Kunden gezahlten Grundgebühr (ohne Notfall- und Sonderstunden) begrenzt.

5.2 Die Haftung für Lösegeldforderungen (Ransomware-Erpressungen), Kosten für Datenwiederherstellungen, behördliche DSGVO-Bußgelder, Betriebsunterfallskosten sowie Reputationsschäden ist vollumfänglich ausgeschlossen.

5.3 Werden Dritte (einschließlich Kunden oder Vertragspartner des Kunden) infolge eines Sicherheitsvorfalls geschädigt und nehmen diese den Sicherheitsdienstleister in Anspruch, stellt der Kunde den Sicherheitsdienstleister von sämtlichen Ansprüchen auf erstes Anfordern frei, es sei denn, der Vorfall wurde vom Sicherheitsdienstleister vorsätzlich verursacht.

§ 6 Laufzeit, automatische Bindung und vorzeitige Auflösung
6.1 Der Vertrag tritt mit Unterzeichnung in Kraft und hat eine feste Erstlaufzeit von sechsunddreißig (36) Monaten.

6.2 Er verlängert sich jeweils um weitere vierundzwanzig (24) Monate, es sei denn, er wird von einer Partei mit einer Frist von neun (9) Monaten zum Ende der jeweiligen Vertragslaufzeit gekündigt.

6.3 Dem Kunden steht im Falle eines akuten Sicherheitsvorfalls oder einer mangelhaften Vorfallbehandlung kein außerordentliches Sonderkündigungsrecht zu, solange der Sicherheitsdienstleister nachweist, dass er die im internen Playbook vorgesehenen Standardprozeduren formal eingeleitet hat.

§ 7 Anwendbares Recht und Gerichtsstand
7.1 Auf diesen Vertrag und alle daraus resultierenden Ansprüche ist materielles Schweizer Recht unter Ausschluss des Kollisionsrechts (IPRG) sowie des UN-Kaufrechts (CISG) anwendbar.

7.2 Ausschließlicher Gerichtsstand für alle Streitigkeiten ist der Sitz des Sicherheitsdienstleisters in Zürich. Der Sicherheitsdienstleister behält sich das Recht vor, den Kunden auch an dessen allgemeinem Gerichtsstand oder an jedem anderen zuständigen Gericht zu verklagen.`;

  const res = analyzeContract(contract);
  assert.equal(res.riskLevel, 'hoch');
  assert.equal(res.riskScore, 100);
  assert.ok(res.findings.length >= 14, `Expected at least 14 findings, got ${res.findings.length}`);

  const types = new Set(res.findings.map((f) => f.type));

  // § 1.2: Verzicht auf Ersatzansprüche für Abwehr-/Isolationsmaßnahmen
  // § 5.1/5.2/5.3: Summenbegrenzung, Kategorie-Ausschluss, Freistellung auf erstes Anfordern
  assert.ok(types.has('liability'), 'Missing liability detector (§ 1.2 Verzicht, § 5.1-5.3 Haftungsschranken)');
  const liabilityFindings = res.findings.filter((f) => f.type === 'liability');
  assert.ok(
    liabilityFindings.length >= 3,
    `Expected at least 3 liability findings (§ 1.2, § 5.1, § 5.2, § 5.3), got ${liabilityFindings.length}`
  );

  // § 3.1/3.2: Ausschluss von Funktions- und Erkennungsgarantie
  assert.ok(types.has('as_is_software_disclaimer'), 'Missing as-is disclaimer for the detection guarantee (§ 3.1/3.2)');
  assert.ok(
    res.findings.some((f) => f.type === 'as_is_software_disclaimer' && f.severity === 'hoch'),
    'as-is disclaimer on the detection guarantee must be rated hoch'
  );

  // § 3.3: Beweislast für grobe Pflichtwidrigkeit beim Kunden
  assert.ok(types.has('burden_of_proof'), 'Missing burden_of_proof (§ 3.3 Negativbeweis)');

  // § 2.2: Zustimmungsfiktion zu Notfall-Mehrstunden (Faktor 2,5 + 100 % Zuschlag)
  assert.ok(types.has('unilateral_change'), 'Missing unilateral_change (§ 2.2 Zustimmungsfiktion zu Notfallstunden)');

  // § 2.3: keine Budgetobergrenze für Notfallmaßnahmen
  assert.ok(types.has('price_escalation'), 'Missing price_escalation (§ 2.3 keine Obergrenze / Notfalltarif)');

  // § 4.1/4.2: Vertraulichkeitsdurchbrechung und Behördenherausgabe ohne Information
  assert.ok(types.has('asymmetric_confidentiality'), 'Missing asymmetric_confidentiality (§ 4.1/4.2)');

  // § 6.2/6.3: automatische Verlängerung, 9-Monats-Frist, kein außerordentliches Kündigungsrecht
  assert.ok(types.has('auto_renewal'), 'Missing auto_renewal (§ 6.2)');
  assert.ok(types.has('notice_period'), 'Missing notice_period (§ 6.2, 9 Monate)');
  assert.ok(types.has('asymmetric_termination'), 'Missing asymmetric_termination (§ 6.3)');

  // § 7.1/7.2: Schweizer Recht und Gerichtsstand Zürich
  assert.ok(types.has('jurisdiction'), 'Missing jurisdiction (§ 7.1/7.2)');

  // Keine Fehlalarme: die Notfall-Mehrstunden sind keine Abnahmefiktion und
  // der lange Erstlaufzeit-Block ist keine einseitige ordentliche Kündigung.
  assert.equal(types.has('deemed_acceptance'), false, 'deemed_acceptance falsely triggered on the emergency-rate consent');
  assert.equal(
    types.has('unilateral_convenience_termination'),
    false,
    'unilateral_convenience_termination falsely triggered on the fixed initial term'
  );
});
