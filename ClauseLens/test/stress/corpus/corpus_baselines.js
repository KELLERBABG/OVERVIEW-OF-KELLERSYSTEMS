"use strict";

/**
 * False-Positive-Baseline-Korpus
 * ==============================
 *
 * Alle Fälle hier sind rechtlich zulässig und ausgewogen formuliert.
 * Sie dienen als Gegenprobe: meldet die Engine einen dieser Fälle mit
 * Schwere "mittel" oder "hoch", ist das ein echter False Positive, der im
 * Stress-Report als solcher ausgewiesen wird.
 *
 * Bewusster Aufbau: mehrere Klauseln liegen absichtlich dicht an der
 * Trigger-Wortwahl eines Detektors (z. B. "verlängert sich",
 * "nachvertragliches Wettbewerbsverbot", "Verjährung", "Audit mit
 * Vorankündigung"), sind aber inhaltlich gesetzeskonform. Genau dort
 * entstehen typische Regex-Fehlalarme.
 */

const CONTRACT_SAAS_DE = `B2B SaaS-Rahmenvertrag – Cloud-Service
§ 1 Vertragsgegenstand
Der Anbieter überlässt dem Kunden die Software "AcmeCloud Suite" als Software-as-a-Service zur geschäftlichen Nutzung. Der Funktionsumfang ergibt sich aus Anlage 1 (Leistungsbeschreibung, Version 2.4).

§ 2 Laufzeit und Kündigung
Der Vertrag hat eine Festlaufzeit von zwölf Monaten ab Bereitstellung. Er wird nur dann fortgesetzt, wenn beide Parteien spätestens drei Monate vor Laufzeitende eine Verlängerung schriftlich vereinbaren. Jede Partei kann den Vertrag mit einer Frist von drei Monaten zum Laufzeitende kündigen. Das Recht beider Parteien zur Kündigung aus wichtigem Grund nach § 314 BGB bleibt unberührt.

§ 3 Vergütung und Zahlung
Die monatliche Grundgebühr beträgt 2.400 EUR netto und ist 14 Tage nach Rechnungszugang zur Zahlung fällig. Der Kunde darf mit unbestrittenen oder rechtskräftig festgestellten Gegenforderungen aufrechnen.

§ 4 Haftung
Der Anbieter haftet unbeschränkt für Schäden aus der Verletzung des Lebens, des Körpers oder der Gesundheit sowie für vorsätzlich oder grob fahrlässig verursachte Schäden. Bei einfacher Fahrlässigkeit haftet der Anbieter nur bei Verletzung wesentlicher Vertragspflichten (Kardinalpflichten), begrenzt auf den vertragstypischen, vorhersehbaren Schaden.

§ 5 Service-Level
Der Anbieter gewährleistet eine monatliche Verfügbarkeit von 99,5 %. Unterschreitet er diesen Wert, erhält der Kunde ohne gesonderten Antrag eine Gutschrift von 5 % der Monatsgebühr je angefangene 24 Stunden Ausfallzeit. Bei Unterschreitung von 98 % in zwei aufeinanderfolgenden Monaten kann der Kunde den Vertrag mit sofortiger Wirkung kündigen.

§ 6 Daten und Vertragsende
Der Kunde kann seine Daten jederzeit selbst exportieren; nach Vertragsende stellt der Anbieter sie kostenfrei in den Formaten JSON und CSV für 30 Tage bereit und löscht sie danach.

§ 7 Datenschutz
Soweit der Anbieter personenbezogene Daten im Auftrag verarbeitet, gilt die Anlage 2 (Auftragsverarbeitungsvertrag nach Art. 28 DSGVO).

§ 8 Mitwirkung und Änderungen
Änderungen dieses Vertrages einschließlich dieser Klausel bedürfen der Textform (z. B. per E-Mail) und der Zustimmung beider Parteien. Sollte eine Bestimmung unwirksam sein, tritt an ihre Stelle die gesetzliche Regelung.`;

const CONTRACT_NDA_MUTUAL = `Gegenseitige Geheimhaltungsvereinbarung (NDA)
§ 1 Vertrauliche Informationen
Vertrauliche Informationen sind alle Informationen, die eine Partei der anderen als vertraulich kennzeichnet oder die nach Art und Umständen der Übermittlung als vertraulich erkennbar sind.

§ 2 Verpflichtungen
Beide Parteien verpflichten sich wechselseitig, vertrauliche Informationen der jeweils anderen Partei streng vertraulich zu behandeln, ausschließlich für Zwecke der Prüfung einer Zusammenarbeit zu verwenden und nur solchen Mitarbeitern zugänglich zu machen, die sie zur Aufgabenerfüllung benötigen und entsprechend verpflichtet sind.

§ 3 Ausnahmen
Keine Vertraulichkeitspflicht besteht für Informationen, die offenkundig sind, ohne Verstoß gegen diese Vereinbarung öffentlich bekannt wurden, der empfangenden Partei bereits vorher rechtmäßig bekannt waren oder aufgrund gesetzlicher oder behördlicher Verpflichtung offenzulegen sind. Im letzteren Fall informiert die verpflichtete Partei die andere Partei vorab, soweit dies rechtlich zulässig ist.

§ 4 Dauer
Die Geheimhaltungspflicht gilt für die Dauer der Zusammenarbeit und drei Jahre nach deren Beendigung.

§ 5 Rückgabe und Vernichtung
Auf Verlangen gibt die empfangende Partei alle überlassenen Unterlagen zurück oder vernichtet sie und bestätigt dies auf Wunsch; gesetzliche Aufbewahrungspflichten bleiben unberührt.

§ 6 Haftung und Schlussbestimmungen
Die Haftung richtet sich nach den gesetzlichen Vorschriften. Änderungen bedürfen der Textform. Es gilt deutsches Recht; Gerichtsstand ist der gesetzliche Gerichtsstand.`;

const CONTRACT_WERKVERTRAG = `Softwareentwicklungsvertrag (Werkvertrag)
§ 1 Vertragsgegenstand
Der Auftragnehmer entwickelt für den Auftraggeber die in Anlage 1 beschriebene Softwarekomponente und übergibt sie einschließlich Quellcode, Build-Skripten und technischer Dokumentation.

§ 2 Abnahme
Nach Übergabe prüft der Auftraggeber die Leistung innerhalb von 30 Tagen auf vertragsgemäße Beschaffenheit und erklärt die Abnahme oder teilt konkrete Mängel mit. Eine Abnahme gilt erst dann als erfolgt, wenn der Auftragnehmer den Auftraggeber bei Beginn der Prüffrist ausdrücklich auf die Rechtsfolgen des § 640 Abs. 2 Satz 2 BGB hingewiesen hat.

§ 3 Vergütung
Die Vergütung beträgt 84.000 EUR netto, zahlbar in vier Meilensteinen nach jeweiliger Abnahme des Meilensteins innerhalb von 14 Tagen nach Rechnungszugang.

§ 4 Mängelrechte
Der Auftragnehmer leistet Nacherfüllung durch Beseitigung des Mangels. Schlägt die Nacherfüllung fehl, stehen dem Auftraggeber die gesetzlichen Rechte aus §§ 634, 636 BGB zu. Die Verjährung von Mängelansprüchen beträgt 24 Monate ab Abnahme; für Schäden aus Vorsatz, grober Fahrlässigkeit sowie aus der Verletzung des Lebens, des Körpers oder der Gesundheit gelten die gesetzlichen Verjährungsfristen.

§ 5 Rechte
Der Auftraggeber erhält an den Arbeitsergebnissen die ausschließlichen, übertragbaren Nutzungsrechte für alle bekannten Nutzungsarten. Der Auftragnehmer darf Referenzen nur nach vorheriger schriftlicher Zustimmung des Auftraggebers nennen.

§ 6 Haftung
Es gilt die gesetzliche Haftung. Für einfache Fahrlässigkeit ist die Haftung auf den vertragstypischen, vorhersehbaren Schaden begrenzt, ausgenommen die Verletzung von Kardinalpflichten.

§ 7 Kündigung
Beide Parteien können den Vertrag mit einer Frist von drei Monaten zum Quartalsende kündigen. Das Recht zur außerordentlichen Kündigung aus wichtigem Grund bleibt unberührt.

§ 8 Schlussbestimmungen
Änderungen bedürfen der Textform. Mündliche Nebenabreden und vorvertragliche Zusagen dieser Parteien bleiben verbindlich. Es gilt das Recht der Bundesrepublik Deutschland.`;

const CONTRACT_MANAGED_SERVICES = `Managed-Services-Vertrag
§ 1 Leistungsumfang
Der Anbieter betreibt für den Kunden die in Anlage 1 beschriebene Infrastruktur, überwacht sie 24/7 und stellt einen Service Desk mit Reaktionszeiten gemäß Anlage 2 bereit.

§ 2 Service-Level und Rechtsfolgen
Der Anbieter gewährleistet eine monatliche Verfügbarkeit von 99,9 % und eine Reaktionszeit von 30 Minuten für Störungen der Priorität 1. Bei Unterschreitung erhält der Kunde auf Antrag eine Gutschrift von 10 % der Monatspauschale je angefangene Stunde Ausfallzeit (maximal 50 % der Monatspauschale). Unabhängig von Gutschriften bleiben die gesetzlichen Minderungs- und Schadensersatzansprüche des Kunden bei schuldhafter Pflichtverletzung unberührt. Bei Unterschreitung der Verfügbarkeit von 98 % in zwei aufeinanderfolgenden Monaten kann der Kunde fristlos kündigen.

§ 3 Vergütung
Die Monatspauschale beträgt 6.800 EUR netto, fällig monatlich im Voraus. Bei vereinbarten zusätzlichen Leistungen gilt der in Anlage 3 genannte Stundensatz.

§ 4 Preisanpassung
Der Anbieter kann die Monatspauschale einmal jährlich entsprechend der Veränderung des vom Statistischen Bundesamt veröffentlichten Verbraucherpreisindexes anpassen; die Anpassung ist auf 3 % pro Jahr begrenzt und gilt nicht für laufende Leistungsperioden. Übersteigt die Anpassung 3 %, kann der Kunde den Vertrag mit sofortiger Wirkung kündigen.

§ 5 Mitwirkung des Kunden
Der Kunde benennt einen Ansprechpartner, stellt die notwendigen Zugänge bereit und meldet Störungen über den Service Desk.

§ 6 Haftung
Der Anbieter haftet unbeschränkt für Vorsatz, grobe Fahrlässigkeit sowie für Schäden aus der Verletzung des Lebens, des Körpers oder der Gesundheit. Bei einfacher Fahrlässigkeit haftet er nur bei Verletzung wesentlicher Vertragspflichten, begrenzt auf den vertragstypischen, vorhersehbaren Schaden. Die gesetzliche Beweislastverteilung bleibt unberührt.

§ 7 Geheimhaltung
Beide Parteien behandeln die von der jeweils anderen Partei als vertraulich bezeichneten Informationen vertraulich und verwenden sie ausschließlich zur Vertragsdurchführung.

§ 8 Kündigung
Der Vertrag läuft 24 Monate und kann von jeder Partei mit sechs Monaten Frist zum Laufzeitende gekündigt werden. Das Recht zur außerordentlichen Kündigung aus wichtigem Grund bleibt unberührt.

§ 9 Schlussbestimmungen
Änderungen bedürfen der Textform. Gerichtsstand ist der gesetzliche Gerichtsstand.`;

const CONTRACT_AVV = `Auftragsverarbeitungsvertrag (Art. 28 DSGVO)
§ 1 Gegenstand und Dauer
Der Auftragnehmer verarbeitet personenbezogene Daten des Verantwortlichen ausschließlich zur Erbringung der in Anlage 1 beschriebenen Leistungen (Kategorien betroffener Personen, Datenarten, Zwecke) für die Vertragsdauer.

§ 2 Weisungsbindung
Der Auftragnehmer verarbeitet die Daten nur auf dokumentierte Weisung des Verantwortlichen. Ist er der Ansicht, dass eine Weisung gegen die DSGVO verstößt, teilt er dies unverzüglich mit und darf die Weisung bis zur Klärung aussetzen.

§ 3 Subunternehmer
Der Einsatz weiterer Auftragsverarbeiter ist nur nach vorheriger Information des Verantwortlichen in Textform zulässig. Der Verantwortliche kann dem Einsatz aus wichtigem datenschutzrechtlichen Grund widersprechen; erfolgt kein Widerspruch innerhalb von 14 Tagen, gilt der Einsatz als genehmigt. Der Auftragnehmer verpflichtet Subunternehmer inhaltlich in gleicher Weise.

§ 4 Technische und organisatorische Maßnahmen
Der Auftragnehmer trifft die in Anlage 2 konkretisierten Maßnahmen zur Sicherheit der Verarbeitung (§ 32 DSGVO) und passt sie dem Stand der Technik an, ohne das Schutzniveau zu unterschreiten.

§ 5 Löschung und Rückgabe
Nach Abschluss der Leistungserbringung löscht der Auftragnehmer die Daten nach Wahl des Verantwortlichen oder gibt sie in einem strukturierten, gängigen und maschinenlesbaren Format zurück; gesetzliche Aufbewahrungspflichten bleiben unberührt.

§ 6 Meldepflichten
Der Auftragnehmer unterstützt den Verantwortlichen bei der Erfüllung der Meldepflichten (Art. 33, 34 DSGVO) sowie bei Datenschutz-Folgenabschätzungen und benennt einen Datenschutzkontakt.`;

const CONTRACT_LIZENZ = `Softwarelizenzvertrag (On-Premises)
§ 1 Lizenzgegenstand
Der Lizenzgeber überlässt dem Lizenznehmer das Standardprodukt "DataForge 7" zur Nutzung im eigenen Geschäftsbetrieb.

§ 2 Nutzungsrechte
Der Lizenznehmer erhält ein einfaches, nicht ausschließliches, nicht übertragbares und auf die Laufzeit dieses Vertrages beschränktes Nutzungsrecht zur Installation und Nutzung der Software auf der in Anlage 1 genannten Anzahl von Servern. Der Lizenznehmer darf Kopien für Sicherungszwecke anfertigen und die Software für seine eigenen Zwecke anpassen; die Nutzung zur Erbringung von Dienstleistungen für Dritte ist nicht gestattet.

§ 3 Gewährleistung
Der Lizenzgeber gewährleistet, dass die Software bei Übergabe im Wesentlichen der Produktbeschreibung entspricht. Mängel beseitigt der Lizenzgeber innerhalb angemessener Frist; schlägt die Beseitigung fehl, gelten die gesetzlichen Rechte aus § 437 BGB.

§ 4 Vergütung
Die Einmallizenzgebühr beträgt 19.500 EUR netto, fällig 30 Tage nach Lizenzübergabe. Der erste Vertragsjahr-Support beträgt 3.900 EUR netto jährlich.

§ 5 Haftung
Es gilt die gesetzliche Haftung. Eine Haftung für einfache Fahrlässigkeit ist auf den vertragstypischen, vorhersehbaren Schaden begrenzt, soweit keine Kardinalpflicht, keine Körperverletzung und kein Vorsatz oder grobe Fahrlässigkeit vorliegt.

§ 6 Laufzeit und Beendigung
Der Supportvertrag läuft 12 Monate und verlängert sich nicht automatisch; er muss von beiden Parteien ausdrücklich und schriftlich fortgesetzt werden. Nach Vertragsende darf der Lizenznehmer die Software weiterhin in der zuletzt gelieferten Version nutzen.`;

const CONTRACT_MSA_EN = `MASTER SERVICES AGREEMENT (Clean Baseline)
1. Services
Provider will perform the services described in each SOW. Each SOW is governed by this MSA. In case of conflict, the SOW prevails for that SOW only.

2. Fees and Payment
Customer pays the fees in each SOW within 30 days of a valid invoice. Amounts are exclusive of VAT. Customer may withhold or delay payment only with respect to amounts it disputes in good faith and must pay undisputed amounts when due.

3. Term and Termination
Each SOW runs for the term stated in the SOW. Either party may terminate for convenience with 60 days' written notice. Either party may terminate immediately for material breach not cured within 30 days of written notice, and for insolvency-related events to the extent permitted by applicable law.

4. Intellectual Property
Customer retains all rights in Customer Data and in materials it provides. Provider retains all rights in its pre-existing tools and in general know-how. Deliverables are licensed to Customer on a perpetual, worldwide, non-exclusive basis for its internal business purposes.

5. Data Protection
The parties will execute a data processing agreement where required. Provider will process personal data only on documented instructions and will inform Customer before engaging any new subprocessor, giving Customer an opportunity to object on reasonable data protection grounds.

6. Confidentiality
Each party will protect the other's confidential information with reasonable care, use it only to perform this MSA, and restrict access to personnel who need it. Confidentiality obligations apply for three years after termination, and indefinitely for trade secrets.

7. Warranties and Remedies
Provider warrants that services will be performed in a professional and workmanlike manner and that deliverables will materially conform to the applicable SOW. Customer's exclusive remedy for breach of this warranty is re-performance at no charge; if re-performance fails, Customer is entitled to a fee refund for the affected services, without prejudice to statutory rights that cannot be waived.

8. Limitation of Liability
Neither party is liable for indirect or consequential damages, except for breaches of confidentiality, infringement of IP rights, or wilful misconduct. Neither party's aggregate liability is limited for death, personal injury, fraud, or intentional misconduct. Subject to the preceding sentence, each party's aggregate liability is limited to the fees paid or payable under the affected SOW in the twelve months preceding the claim.

9. Independent Contractors; Assignment
The parties are independent contractors. Neither party may assign this MSA without the other's written consent, except to an affiliate or in connection with a merger or sale of substantially all assets.

10. Entire Agreement
This MSA and its SOWs set out the entire agreement between the parties and supersede prior proposals and discussions, except that individually negotiated written commitments signed by both parties remain binding. Amendments must be in writing and signed by both parties.

11. Governing Law and Venue
This MSA is governed by the laws of the Federal Republic of Germany, excluding the CISG. The statutory venue rules apply. The parties will first attempt to resolve disputes through good-faith negotiation.`;

const CONTRACT_DISTRIBUTION = `Rahmenvertrag über Vertriebspartnerschaft
§ 1 Gegenstand
Der Lieferant räumt dem Vertriebspartner das nicht ausschließliche Recht ein, die in Anlage 1 gelisteten Produkte im Gebiet Deutschland, Österreich und Schweiz zu vertreiben und zu bewerben.

§ 2 Rechte und Pflichten
Der Vertriebspartner bewirbt die Produkte mit eigenem Aufwand, hält die vom Lieferanten bereitgestellten Markenunterlagen ein und verwendet Marken und Logos des Lieferanten nur im vereinbarten Umfang und im Rahmen der gelieferten Materialien.

§ 3 Preise und Vergütung
Die Einkaufspreise ergeben sich aus Anlage 2. Der Lieferant ändert Preise mit einer Ankündigungsfrist von drei Monaten; bei einer Erhöhung um mehr als 5 % kann der Vertriebspartner den Vertrag zum Zeitpunkt des Inkrafttretens der Änderung kündigen. Die Provisionen werden monatlich abgerechnet und 30 Tage nach Rechnungsstellung ausgezahlt.

§ 4 Verzug und Zinsen
Gerät der Vertriebspartner mit einer fälligen Zahlung in Verzug, schuldet er Verzugszinsen in Höhe von fünf Prozentpunkten über dem Basiszinssatz (§ 288 BGB). Die Geltendmachung eines höheren Verzugsschadens bleibt unberührt.

§ 5 Haftung
Es gilt die gesetzliche Haftung. Die Haftung für einfache Fahrlässigkeit wird auf den vertragstypischen, vorhersehbaren Schaden begrenzt, ausgenommen bei Verletzung von Kardinalpflichten sowie bei Schäden aus der Verletzung von Leben, Körper oder Gesundheit.

§ 6 Laufzeit
Der Vertrag läuft 24 Monate und kann von beiden Parteien mit einer Frist von sechs Monaten zum Laufzeitende gekündigt werden. Beide Parteien behalten das Recht zur außerordentlichen Kündigung aus wichtigem Grund.

§ 7 Geheimhaltung
Beide Parteien verpflichten sich gegenseitig zur Geheimhaltung der ausgetauschten vertraulichen Informationen für die Dauer von drei Jahren nach Vertragsende.`;

const CONTRACT_ARBEIT_KARENZ = `Anstellungsvertrag – Auszug mit zulässigem Wettbewerbsverbot
§ 4 Probezeit, Vertragsdauer und Kündigung
Die Probezeit beträgt sechs Monate. Nach Ablauf der Probezeit gilt eine Kündigungsfrist von drei Monaten zum Monatsende. Das Recht beider Parteien zur Kündigung aus wichtigem Grund bleibt unberührt.

§ 8 Nachvertragliches Wettbewerbsverbot
Der Arbeitnehmer unterlässt für die Dauer von zwölf Monaten nach Beendigung des Arbeitsverhältnisses jede selbstständige oder unselbstständige Tätigkeit für Unternehmen, die mit dem Geschäftsfeld des Arbeitgebers in unmittelbarem Wettbewerb stehen. Das Verbot ist auf das Gebiet der Europäischen Union beschränkt.

§ 9 Karenzentschädigung
Für die Dauer des Wettbewerbsverbots zahlt der Arbeitgeber eine Karenzentschädigung in Höhe von 60 % der zuletzt bezogenen vertragsmäßigen Leistungen (§ 74 Abs. 2 HGB), monatlich zum Monatsende. Auf die Karenzentschädigung anzurechnende Einkünfte sind nach § 74c HGB zu berücksichtigen.

§ 10 Diensterfindungen
Der Arbeitnehmer meldet Diensterfindungen unverzüglich dem Arbeitgeber. Rechte und Pflichten richten sich nach dem Gesetz über Arbeitnehmererfindungen.

§ 11 Arbeitsbedingungen
Arbeitszeit, Urlaub und Entgelt richten sich nach diesem Vertrag und den anwendbaren tariflichen und gesetzlichen Bestimmungen.`;

const CONTRACT_PROCUREMENT = `Liefer- und Rahmenvertrag (Einkauf)
§ 1 Vertragsgegenstand
Der Lieferant liefert die in Anlage 1 aufgeführten Waren an den Besteller zur Abrufung nach Einzelabrufen. Anlage 1 kann einvernehmlich in Textform geändert werden.

§ 2 Preise und Lieferung
Es gelten die Preise der Anlage 2. Die Lieferung erfolgt DAP an den jeweils im Abruf genannten Standort. Teillieferungen bedürfen der Zustimmung des Bestellers.

§ 3 Abnahme und Mängel
Der Besteller untersucht die Ware unverzüglich nach Ablieferung. Offene Mängel zeigt er innerhalb von 14 Tagen nach Entdeckung an. Die gesetzlichen Ansprüche wegen Mängeln, einschließlich der Fristen nach §§ 438, 445b BGB, bleiben unberührt.

§ 4 Zahlung
Rechnungen sind 30 Tage netto ohne Abzug fällig. Der Besteller ist berechtigt, mit unbestrittenen oder rechtskräftig festgestellten Gegenforderungen aufzurechnen. Zurückbehaltungsrechte nach §§ 273, 320 BGB bleiben unberührt.

§ 5 Eigentumsvorbehalt
Die gelieferte Ware bleibt bis zur vollständigen Bezahlung der jeweiligen Rechnung Eigentum des Lieferanten. Nach vollständiger Zahlung geht das Eigentum ohne weitere Erklärung auf den Besteller über.

§ 6 Haftung und Versicherung
Der Lieferant haftet nach den gesetzlichen Vorschriften, insbesondere für Vorsatz, grobe Fahrlässigkeit sowie Schäden aus der Verletzung von Leben, Körper oder Gesundheit, und unterhält eine Betriebshaftpflichtversicherung mit einer Deckungssumme von 3 Mio. EUR.

§ 7 Laufzeit und Kündigung
Der Rahmenvertrag läuft 24 Monate und kann von jeder Partei mit einer Frist von sechs Monaten zum Laufzeitende gekündigt werden. Das Recht zur außerordentlichen Kündigung bleibt unberührt.`;

export const CASES = [
  {
    id: "base-contract-01",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "mixed",
    text: CONTRACT_SAAS_DE,
    note: "Ausgewogener SaaS-Rahmenvertrag: beidseitige Kündigung, transparente SLA-Gutschriften ohne Rechteausschluss, kostenfreier Datenexport im offenen Format, Haftung mit Kardinalpflichten- und Körperschaden-Ausnahme, Textform statt Schriftformzwang.",
  },
  {
    id: "base-contract-02",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "mixed",
    text: CONTRACT_NDA_MUTUAL,
    note: "Gegenseitiges NDA: wechselseitige Pflichten, übliche Ausnahmen (offenkundig, Vorwissen, gesetzliche Offenlegungspflicht), Befristung auf drei Jahre nach Beendigung, gesetzliche Haftung.",
  },
  {
    id: "base-contract-03",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "mixed",
    text: CONTRACT_WERKVERTRAG,
    note: "Werkvertrag nach BGB: Abnahme erst nach ausdrücklichem Hinweis auf § 640 Abs. 2 S. 2 BGB, 24 Monate Mängelverjährung mit Körperschaden- und Vorsatz-Ausnahme, Vorrang mündlicher Abreden ausdrücklich erhalten, Referenznennung nur mit Zustimmung.",
  },
  {
    id: "base-contract-04",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "mixed",
    text: CONTRACT_MANAGED_SERVICES,
    note: "Managed Services mit wirklich verbindlichem SLA (Gutschrift plus ausdrücklicher Erhalt gesetzlicher Minderungs- und Schadensersatzrechte, Sonderkündigungsrecht unter 98 %), Indexierung mit 3 %-Cap und Kündigungsrecht, beidseitige Kündigungsfristen.",
  },
  {
    id: "base-contract-05",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "mixed",
    text: CONTRACT_AVV,
    note: "DSGVO-Auftragsverarbeitungsvertrag: Weisungsbindung nach Art. 28 DSGVO, Subunternehmer nur mit Vorabinformation und Widerspruchsrecht, Löschung oder Rückgabe in maschinenlesbarem Format, Unterstützungspflichten.",
  },
  {
    id: "base-contract-06",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "mixed",
    text: CONTRACT_LIZENZ,
    note: "Softwarelizenz: ausdrücklich einfaches, nicht ausschließliches, laufzeitgebundenes Nutzungsrecht für den eigenen Geschäftsbetrieb, gesetzliche Mängelrechte nach § 437 BGB, keine automatische Verlängerung (Fortsetzung nur durch ausdrückliche schriftliche Vereinbarung), Weiternutzung der zuletzt gelieferten Version.",
  },
  {
    id: "base-contract-07",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "mixed",
    text: CONTRACT_MSA_EN,
    note: "Clean English MSA: mutual convenience termination with 60 days notice, subprocessor notice plus objection right, no automatic renewal, standard carve-outs from the liability cap (death, personal injury, fraud, confidentiality, IP), individual signed commitments expressly preserved, statutory venue.",
  },
  {
    id: "base-contract-08",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "mixed",
    text: CONTRACT_DISTRIBUTION,
    note: "Vertriebsvertrag: nicht ausschließliches Recht, Preisänderung mit dreimonatiger Ankündigung und Kündigungsrecht bei mehr als 5 %, gesetzliche Verzugszinsen nach § 288 BGB, beidseitige Kündigungsfristen, gegenseitige Geheimhaltung.",
  },
  {
    id: "base-contract-09",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "mixed",
    text: CONTRACT_ARBEIT_KARENZ,
    note: "Arbeitsvertrag mit zulässigem nachvertraglichem Wettbewerbsverbot nach §§ 74 ff. HGB: zwölf Monate, räumlich auf die EU begrenzt, Karenzentschädigung 60 % (§ 74 Abs. 2 HGB), Anrechnung nach § 74c HGB, beidseitige Kündigungsrechte.",
  },
  {
    id: "base-contract-10",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "mixed",
    text: CONTRACT_PROCUREMENT,
    note: "Einkaufsrahmenvertrag: angemessene Rügefrist von 14 Tagen nach Entdeckung mit ausdrücklichem Erhalt der gesetzlichen Mängelrechte, Aufrechnung und Zurückbehaltung nach §§ 273, 320 BGB ausdrücklich zugelassen, einfacher Eigentumsvorbehalt nur auf die jeweilige Rechnung bezogen.",
  },

  {
    id: "base-clause-01",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "liability",
    text: "Der Anbieter haftet unbeschränkt für Schäden aus der Verletzung des Lebens, des Körpers oder der Gesundheit sowie für vorsätzlich oder grob fahrlässig verursachte Schäden. Bei einfacher Fahrlässigkeit haftet der Anbieter nur bei Verletzung wesentlicher Vertragspflichten (Kardinalpflichten), begrenzt auf den vertragstypischen, vorhersehbaren Schaden.",
    note: "Gesetzeskonforme Haftungsklausel nach § 309 Nr. 7 BGB mit vollständiger Ausnahme für Körperschäden, Vorsatz und grobe Fahrlässigkeit sowie Kardinalpflichten-Vorbehalt.",
  },
  {
    id: "base-clause-02",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "penalty",
    text: "Für jeden Fall der nicht rechtzeitigen Bereitstellung schuldet der Anbieter eine pauschalierte Entschädigung in Höhe von 2 % des Auftragswertes, insgesamt höchstens 10 %. Dem Anbieter bleibt der Nachweis vorbehalten, dass ein wesentlich geringerer Schaden entstanden ist; dem Kunden der Nachweis eines höheren Schadens.",
    note: "Pauschlierter Schadensersatz mit ausdrücklichem Gegenbeweisvorbehalt (beide Richtungen) - entspricht § 309 Nr. 5 lit. b BGB.",
  },
  {
    id: "base-clause-03",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "statute_of_limitations",
    text: "Ansprüche wegen Sachmängeln verjähren in 24 Monaten ab Ablieferung. Für Schadensersatzansprüche wegen Vorsatz, grober Fahrlässigkeit sowie wegen der Verletzung des Lebens, des Körpers oder der Gesundheit gelten die gesetzlichen Verjährungsfristen.",
    note: "Zulässige Verkürzung auf 24 Monate mit ausdrücklicher Ausnahme für Körperschäden, Vorsatz und grobe Fahrlässigkeit (§ 309 Nr. 7 lit. a, b BGB).",
  },
  {
    id: "base-clause-04",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "notice_period",
    text: "Beide Parteien können diesen Vertrag mit einer Frist von drei Monaten zum Laufzeitende kündigen. Das Recht beider Parteien zur außerordentlichen Kündigung aus wichtigem Grund bleibt unberührt.",
    note: "Symmetrische Kündigungsfrist von drei Monaten für beide Parteien, gesetzliches Kündigungsrecht aus wichtigem Grund ausdrücklich erhalten.",
  },
  {
    id: "base-clause-05",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "auto_renewal",
    text: "Der Vertrag endet mit Ablauf der Festlaufzeit. Er wird nur dann fortgesetzt, wenn beide Parteien spätestens drei Monate vor Laufzeitende eine Verlängerung ausdrücklich schriftlich vereinbaren; ohne diese Vereinbarung endet der Vertrag ohne weitere Erklärung.",
    note: "Keine automatische Verlängerung, keine stillschweigende Bindung: Fortsetzung nur durch ausdrückliche beidseitige Erklärung.",
  },
  {
    id: "base-clause-06",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "setoff_restriction",
    text: "Der Kunde ist berechtigt, mit unbestrittenen oder rechtskräftig festgestellten Gegenforderungen aufzurechnen. Die gesetzlichen Zurückbehaltungs- und Leistungsverweigerungsrechte nach §§ 273, 320 BGB bleiben unberührt.",
    note: "Aufrechnung mit unbestrittenen und rechtskräftigen Forderungen ausdrücklich erlaubt, gesetzliche Zurückbehaltungsrechte nicht angetastet (§ 309 Nr. 2, 3 BGB).",
  },
  {
    id: "base-clause-07",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "unilateral_change",
    text: "Änderungen des Leistungsumfangs und der Vergütung werden nur mit vorheriger ausdrücklicher Zustimmung beider Parteien in Textform vereinbart. Ohne diese Zustimmung bleibt der vereinbarte Leistungsumfang unverändert; ein Widerspruch des Kunden ist nicht an Fristen gebunden.",
    note: "Kein einseitiges Änderungsrecht und keine Zustimmungsfiktion - jede Änderung erfordert die aktive Zustimmung.",
  },
  {
    id: "base-clause-08",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "written_form_trap",
    text: "Kündigungen und sonstige Erklärungen im Zusammenhang mit diesem Vertrag bedürfen der Textform (z. B. per E-Mail) und können formfrei per E-Mail erklärt werden; ein bestimmter Übermittlungsweg ist nicht vorgeschrieben. Änderungen dieses Vertrages, einschließlich dieser Klausel, können in Textform vereinbart werden.",
    note: "Textform ausdrücklich ausreichend statt Schriftformzwang oder Einschreibenrückschein (§ 309 Nr. 13 BGB).",
  },
  {
    id: "base-clause-09",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "deemed_acceptance",
    text: "Der Kunde prüft die Leistung innerhalb einer angemessenen Frist von 30 Tagen nach Übergabe und zeigt festgestellte Mängel an. Ansprüche wegen Mängeln können auch nach Ablauf dieser Frist uneingeschränkt geltend gemacht werden; die Prüffrist ist keine Ausschlussfrist und begründet keine Genehmigung.",
    note: "Keine Abnahme- oder Genehmigungsfiktion, ausdrücklich keine Ausschlussfrist, Rechte bleiben zeitlich unbeschränkt.",
  },
  {
    id: "base-clause-10",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "data_grant",
    text: "Der Kunde räumt dem Anbieter für die Dauer der Vertragslaufzeit ein einfaches, nicht ausschließliches, widerrufliches und auf die Zwecke der vertragsgemäßen Leistungserbringung beschränktes Recht ein, die übermittelten Inhalte zu hosten, zu sichern und technisch zu verarbeiten.",
    note: "Zweckgebundenes, einfaches, nicht ausschließliches und widerrufliches Nutzungsrecht - saubere Standardklausel (§ 31 Abs. 2 UrhG).",
  },
  {
    id: "base-clause-11",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "data_lockin",
    text: "Nach Beendigung des Vertrages stellt der Anbieter dem Kunden sämtliche Vertragsdaten innerhalb von 14 Tagen kostenfrei in einem strukturierten, gängigen und maschinenlesbaren Format (JSON und CSV) zur Verfügung. Die Datenüberlassung ist nicht von einer zusätzlichen Vergütung abhängig.",
    note: "Kostenfreie Datenherausgabe im offenen Standardformat ohne Zusatzentgelt (Art. 20 DSGVO, § 307 BGB).",
  },
  {
    id: "base-clause-12",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "jurisdiction",
    text: "Für alle Streitigkeiten aus und im Zusammenhang mit diesem Vertrag gilt der gesetzliche Gerichtsstand nach den Vorschriften der Zivilprozessordnung. Eine ausschließliche Zuständigkeit eines ausländischen Gerichts wird nicht vereinbart; die Parteien können alternativ ein Mediationsverfahren vereinbaren.",
    note: "Verweis auf den gesetzlichen Gerichtsstand nach §§ 12 ff. ZPO ohne Beschränkung auf einen Sitz oder ein Auslandsforum.",
  },
  {
    id: "base-clause-13",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "non_compete",
    text: "Für die Dauer von zwölf Monaten nach Beendigung dieses Vertrages ist dem Dienstleister eine Tätigkeit für unmittelbare Wettbewerber im Kernbereich des Kunden untersagt. Der Kunde zahlt hierfür eine Karenzentschädigung in Höhe von 60 % der zuletzt bezogenen monatlichen Vergütung, monatlich im Voraus, und verzichtet nicht auf gleichwertige Gegenleistungen.",
    note: "Nachvertragliches Wettbewerbsverbot mit angemessener Befristung und voller Karenzentschädigung, sachlich eng begrenzt (§ 74 Abs. 2 HGB analog, § 307 BGB).",
  },
  {
    id: "base-clause-14",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "subcontractor_uncontrolled",
    text: "Beabsichtigt der Anbieter, Leistungen durch weitere Auftragnehmer erbringen zu lassen, informiert er den Kunden spätestens 14 Tage vor Aufnahme der Tätigkeit in Textform. Der Kunde kann dem Einsatz aus wichtigem sachlichen Grund widersprechen; der Widerspruch ist innerhalb der Ankündigungsfrist zu erklären und führt zu einer einvernehmlichen Ersatzlösung.",
    note: "Vorabinformation mit annähernd angemessener Frist von 14 Tagen und echten Widerspruchsrechten (Art. 28 Abs. 2 DSGVO, § 307 BGB).",
  },
  {
    id: "base-clause-15",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "unreasonable_audit",
    text: "Der Kunde kann die Einhaltung der vereinbarten Sicherheitsbestimmungen einmal jährlich überprüfen. Der Anbieter wird die Prüfung, einschließlich einer Sichtung der betroffenen Systeme, vorab anmelden und einen Vorlauf von 14 Tagen vorsehen; geprüft wird zu üblichen Geschäftszeiten. Die Kosten der turnusmäßigen Prüfung trägt der Anbieter, sofern keine erhebliche Pflichtverletzung festgestellt wird.",
    note: "Auditrecht mit Vorankündigung, Vorlauffrist von 14 Tagen, regulären Geschäftszeiten und Kostenübernahme durch den Anbieter - angemessen im Sinne des § 307 BGB.",
  },
  {
    id: "base-clause-16",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "marketing_reference",
    text: "Der Anbieter darf Firmenname, Bildmarke oder Logo des Kunden zu Referenzzwecken nur nach vorheriger schriftlicher Zustimmung des Kunden verwenden. Die Zustimmung bezieht sich auf den konkret angefragten Einzelfall und kann vom Kunden jederzeit widerrufen werden.",
    note: "Referenz- und Logonutzung nur mit Einzelfreigabe und jederzeit widerrufbar - kein formularmäßiger Blankoscheck.",
  },
  {
    id: "base-clause-17",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "price_escalation",
    text: "Die Vergütung wird jährlich entsprechend der prozentualen Veränderung des Verbraucherpreisindexes für Deutschland des Statistischen Bundesamtes angepasst. Eine Anpassung erfolgt nur, soweit sie 3 % pro Jahr nicht übersteigt; sinkt der Index, werden auch die Entgelte entsprechend gesenkt. Bei einer Erhöhung um mehr als 3 % kann der Kunde den Vertrag mit sofortiger Wirkung kündigen.",
    note: "Preisgleitklausel gekoppelt an einen neutralen, veröffentlichten Index mit Deckelung, automatischer Senkung und Sonderkündigungsrecht.",
  },
  {
    id: "base-clause-18",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "default_interest",
    text: "Gerät der Kunde mit einer fälligen Zahlung in Verzug, schuldet er Verzugszinsen in Höhe von fünf Prozentpunkten über dem Basiszinssatz. Die Geltendmachung eines darüber hinausgehenden, konkret nachzuweisenden Verzugsschadens bleibt beiden Parteien vorbehalten.",
    note: "Gesetzlicher Verzugszinssatz nach § 288 Abs. 2 BGB ohne Aufschlag und ohne monatliche Zinsberechnung.",
  },
  {
    id: "base-clause-19",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "dynamic_url_referral",
    text: "Bestandteil dieses Vertrages sind die Allgemeinen Geschäftsbedingungen des Anbieters in der Fassung vom 01.01.2026 (Version 4.2). Diese Fassung liegt dem Vertrag als Anlage 1 bei und wird nicht einseitig geändert; künftige Fassungen gelten ausschließlich nach ausdrücklicher Einbeziehung durch beide Parteien.",
    note: "Statische, datierte und als Anlage beigefügte Fassung ohne dynamischen Verweis auf eine jeweils aktuelle Website-Fassung (BGH III ZR 59/24).",
  },
  {
    id: "base-clause-20",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "entire_agreement",
    text: "Dieser Vertrag fasst die Vereinbarungen der Parteien zusammen. Individuell ausgehandelte und von beiden Parteien schriftlich bestätigte Zusagen, Anlagen und Leistungsbeschreibungen bleiben als Vertragsbestandteil verbindlich und haben Vorrang vor diesen Bedingungen.",
    note: "Vollständigkeitsklausel ohne Aushebelung des Vorrangs individueller Abreden (§ 305b BGB).",
  },
  {
    id: "base-clause-21",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "excessive_survival",
    text: "Die Geheimhaltungspflichten beider Parteien gelten für drei Jahre nach Beendigung des Vertrages. Nach diesem Zeitraum bestehen keine über die Beendigung hinausgehenden Pflichten; die gesetzlichen Pflichten zum Schutz von Geschäftsgeheimnissen bleiben unberührt.",
    note: "Nachvertragliche Pflichten klar auf drei Jahre befristet statt unbegrenzt fortwirkend (§ 307 BGB).",
  },
  {
    id: "base-clause-22",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "burden_of_proof",
    text: "Die Beweislast richtet sich nach den gesetzlichen Vorschriften. Eine Verlagerung der Beweislast zulasten des Kunden ist nicht vorgesehen; die Parteien bestätigen keine Fehlerfreiheit vorab, und der Zugang von Erklärungen richtet sich nach tatsächlichem Zugang (§ 130 BGB).",
    note: "Ausdrücklicher Erhalt der gesetzlichen Beweislast, keine Vorab-Bestätigung und keine Zugangsfiktion (§ 309 Nr. 12 BGB).",
  },
  {
    id: "base-clause-23",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "fictitious_receipt",
    text: "Erklärungen, Rechnungen und Kündigungen werden erst mit ihrem tatsächlichen Zugang beim Empfänger wirksam. Für den Nachweis des Zugangs kann sich jede Partei auf einen Sendebericht berufen; ein fiktiver Zugang nach Ablauf einer Frist ohne tatsächlichen Zugang wird nicht vereinbart.",
    note: "Zugangsfiktion ausdrücklich abbedungen - maßgeblich ist der tatsächliche Zugang nach § 130 BGB.",
  },
  {
    id: "base-clause-24",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "asymmetric_confidentiality",
    text: "Beide Parteien verpflichten sich gegenseitig, die von der jeweils anderen Partei als vertraulich gekennzeichneten Informationen sowie alle Informationen aus der Zusammenarbeit streng vertraulich zu behandeln und ausschließlich zur Vertragsdurchführung zu verwenden.",
    note: "Wechselseitige Geheimhaltungspflicht ohne einseitige Belastung des Kunden (GeschGehG, § 307 BGB).",
  },
  {
    id: "base-clause-25",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "feedback_ip_grab",
    text: "Verbesserungsvorschläge und Anregungen des Kunden sind unverbindlich und begründen keine Pflicht zur Umsetzung. Für den Fall, dass Vorschläge in die Produktentwicklung einfließen, prüfen die Parteien eine angemessene Vergütung oder Beteiligung gesondert; eine automatische Übertragung von Schutzrechten des Kunden findet nicht statt.",
    note: "Kein unentgeltlicher IP-Übergang für Feedback; ausdrücklich keine automatische Rechteübertragung.",
  },
  {
    id: "base-clause-26",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "assignment_restriction",
    text: "Jede Partei darf Geldforderungen aus diesem Vertrag abtreten. Die Abtretung von Ansprüchen auf höchstpersönliche Leistungen sowie die Übertragung von Pflichten bedarf der vorherigen Zustimmung der anderen Partei; die Zustimmung darf nicht unbillig verweigert werden.",
    note: "Abtretung von Geldforderungen ausdrücklich gestattet (§ 354a HGB), Beschränkung nur auf höchstpersönliche Ansprüche und Pflichtenübernahme.",
  },
  {
    id: "base-clause-27",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "force_majeure_asymmetry",
    text: "Wird eine Partei durch höhere Gewalt an der Erbringung ihrer Hauptleistung gehindert, ruhen die Leistungspflichten beider Parteien für die Dauer der Behinderung. Die Vergütungspflicht des Kunden entfällt für die Zeit der Störung; dauert die Behinderung länger als 60 Tage, kann jede Partei den Vertrag ohne Frist kündigen.",
    note: "Beidseitige Suspendierung von Leistung und Gegenleistung bei höherer Gewalt sowie beidseitiges Kündigungsrecht - keine einseitige Risikoverlagerung.",
  },
  {
    id: "base-clause-28",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "insolvency_termination",
    text: "Die Beendigung des Vertrages richtet sich nach den gesetzlichen Vorschriften, insbesondere nach den Regelungen der Insolvenzordnung. Ein Recht zur Kündigung auf Grund eines bloßen Antrags auf Eröffnung eines Insolvenzverfahrens wird nicht vereinbart.",
    note: "Verzicht auf eine insolvenzabhängige Lösungsklausel nach § 119 InsO; Verweis auf die gesetzliche Rechtslage.",
  },
];
