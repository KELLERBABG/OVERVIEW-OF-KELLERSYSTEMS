/**
 * Stress-Korpus — Slice 1/8: "Accountability"
 * ==========================================
 * Detektoren: liability, penalty, statute_of_limitations, burden_of_proof,
 *             vicarious_agents, third_party_indemnity, audit_log_manipulation
 *
 * Ziel: realistische, versteckte Klauseln gegen eine satzweise arbeitende
 * Regex-Engine stellen. Techniken: Paraphrase/Euphemismus, Nominalstil,
 * Archaismen, ausgeschriebene Zahlen, Satzteilung, Aufzaehlungsbegrabung,
 * Querverweis, Englisch, Typografie-Evasion (Soft-Hyphen/Zero-Width).
 *
 * Jeder Fall wird durch test/stress/harness.js bewertet:
 *   deceptive -> Zieldetektor muss treffen, Schwere >= erwartete Schwere
 *   compliant -> kein Befund mit Schwere mittel/hoch (False-Positive-Test)
 */

export const CASES = [
  // -------------------------------------------------------------------------
  // liability (§ 307 Abs. 2 Nr. 2, § 309 Nr. 7 BGB)
  // -------------------------------------------------------------------------
  {
    id: "acc-euphemism-01",
    detector: "liability",
    variant: "deceptive",
    technique: "euphemism",
    severity: "hoch",
    text: "Sämtliche Ansprüche wegen Schlechtleistung finden ihre abschließende Erledigung in der vom Auftragnehmer geschuldeten Nachbesserung; ein darüber hinausgehender Ersatz des aus der Schlechtleistung entstehenden Schadens ist wirtschaftlich nicht vorgesehen.",
    note: "Sole-Remedy-Falle in beschwichtigendem Register: 'finden ihre abschließende Erledigung' ersetzt das keyword 'ausschließliche Kompensation', 'wirtschaftlich nicht vorgesehen' ersetzt 'ausgeschlossen'. Die Entkernung von Minderung, Rücktritt und Schadensersatz ist damit sprachlich weichgespült.",
  },
  {
    id: "acc-synonym-02",
    detector: "liability",
    variant: "deceptive",
    technique: "synonym",
    severity: "hoch",
    text: "Jede Einstandspflicht des Auftragnehmers für mittelbare Nachteile, Folgeschäden, Betriebsunterbrechungen und entgangenen Gewinn wird hiermit ausbedungen und aufgehoben, soweit nicht zwingendes Recht entgegensteht.",
    note: "Archaischer Juristenjargon: 'ausbedungen' statt 'abbedungen' und 'Einstandspflicht' statt 'Haftung'. Der scheinbare Vorbehalt 'zwingendes Recht' rettet die Klausel nicht, weil grobe Fahrlässigkeit und Kardinalpflichten gerade nicht ausgenommen sind.",
  },
  {
    id: "acc-word-number-03",
    detector: "liability",
    variant: "deceptive",
    technique: "word-number",
    severity: "hoch",
    text: "Die Ersatzpflicht des Auftragnehmers ist dem Grunde nach auf die Vergütung begrenzt, die auf die drei Monate vor der anspruchsbegründenden Pflichtverletzung entfällt.",
    note: "Aushöhlung von Kardinalpflichten durch Deckelung auf wenige Monatsentgelte — aber als ausgeschriebene Zahl in umgekehrter Zeitrichtung formuliert ('die drei Monate vor ...' statt 'in den letzten drei Monaten').",
  },
  {
    id: "acc-split-sentence-04",
    detector: "liability",
    variant: "deceptive",
    technique: "split-sentence",
    severity: "hoch",
    text: "Eine Haftung des Auftragnehmers besteht ausschließlich bei vorsätzlichem Handeln. Für grobe Fahrlässigkeit, für die Verletzung von Kardinalpflichten und für Schäden aus der Verletzung von Leben, Körper oder Gesundheit wird keine Verantwortung übernommen.",
    note: "Die Vorsatz-Beschränkung und die Freizeichnung von grober Fahrlässigkeit, Kardinalpflichten und Körperschäden sind auf zwei Sätze verteilt; der zweite Satz verwendet 'keine Verantwortung übernommen' statt 'keine Haftung'.",
  },
  {
    id: "acc-english-05",
    detector: "liability",
    variant: "deceptive",
    technique: "english",
    severity: "hoch",
    text: "In no event shall the Provider be liable to the Customer for any indirect, incidental, special, consequential or punitive damages, or for any loss of profits, revenue or data, arising out of or in connection with this Agreement, even if advised of the possibility of such damages.",
    note: "Klassische englische Haftungsfreizeichnung in einem sonst deutschsprachigen Vertrag. Kontrollfall: die englischen Liability-Patterns sollten hier greifen.",
  },
  {
    id: "acc-typo-evasion-06",
    detector: "liability",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Die Haf\u00adtung des Anbieters für Sch\u00e4den jeder Art – gleich aus welchem Rechtsgrund – ist voll\u00adumf\u00e4nglich aus\u00adge\u00adschlossen.",
    note: "Soft-Hyphen-Evasion (\u00ad) mitten in 'Haftung', 'vollumfänglich' und 'ausgeschlossen'. Kontrollfall: normalizeText muss die Trennzeichen entfernen, damit die Haftungsfreizeichnung sichtbar wird.",
  },

  // -------------------------------------------------------------------------
  // penalty — Vertragsstrafe / verdeckte Pauschale (§ 309 Nr. 5 lit. b, Nr. 6 BGB)
  // -------------------------------------------------------------------------
  {
    id: "acc-synonym-07",
    detector: "penalty",
    variant: "deceptive",
    technique: "synonym",
    severity: "hoch",
    text: "Für jeden Fall der Zuwiderhandlung gegen die Nutzungsbedingungen verwirkt eine Vertragsbuße in Höhe von 7.500 EUR, deren konkrete Höhe der Anbieter nach billigem Ermessen festsetzt.",
    note: "Verdeckte Vertragsstrafe: 'Vertragsbuße' und 'verwirkt' umgehen die Keywords 'Vertragsstrafe'/'Konventionalstrafe'/'Pönale'. Der Nachweis fehlenden Verschuldens oder geringeren Schadens wird nicht eröffnet.",
  },
  {
    id: "acc-euphemism-08",
    detector: "penalty",
    variant: "deceptive",
    technique: "euphemism",
    severity: "hoch",
    text: "Bei Überschreitung des vereinbarten Liefertermins wird eine Aufwandspauschale von zwei Prozent des jeweiligen Auftragswertes fällig; ein Nachweis, dass ein geringerer Ausfall entstanden ist, ist nicht zugelassen.",
    note: "Pauschalierter Schadensersatz mit Ausschluss des Gegenbeweises (§ 309 Nr. 5 lit. b BGB), getarnt als neutrale 'Aufwandspauschale' und mit ausgeschriebener Prozentangabe.",
  },
  {
    id: "acc-word-number-09",
    detector: "penalty",
    variant: "deceptive",
    technique: "word-number",
    severity: "hoch",
    text: "Vereinbart wird eine Konventionalstrafe in Höhe von fünfzehn Prozent der Netto-Auftragssumme je Kalenderwoche des Verzugs.",
    note: "DACH/österreichische Terminologie mit ausgeschriebener Zahl. Kontrollfall: 'Konventionalstrafe' ist als Keyword erfasst.",
  },
  {
    id: "acc-list-buried-10",
    detector: "penalty",
    variant: "deceptive",
    technique: "list-buried",
    severity: "hoch",
    text: "Der Auftragnehmer sichert zu: (i) die Einhaltung der Servicezeiten an Werktagen zwischen acht und achtzehn Uhr, (ii) eine Reaktionszeit von höchstens vier Stunden, (iii) die Zahlung einer Vertragsstrafe von 250 EUR je angefangenem Tag der Überschreitung der Reaktionszeit, (iv) die monatliche Vorlage eines Statusberichts.",
    note: "Vertragsstrafe mitten in einer harmlosen Leistungszusageliste; kein Verschuldenserfordernis, keine Obergrenze, kein Nachweisvorbehalt. Kontrollfall für die Aufzaehlungsbegrabung.",
  },
  {
    id: "acc-typo-evasion-11",
    detector: "penalty",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Bei Verzug verf\u00e4llt eine Ver\u200btrags\u200bstrafe in H\u00f6he von 1.200 EUR je angefangenem Kalendertag sowie eine Pauschale f\u00fcr die entstandenen Mahnkosten.",
    note: "Zero-Width-Space (\u200b) mitten in 'Vertragsstrafe'. Kontrollfall: nach Normalisierung muss die Strafe erkannt werden.",
  },

  // -------------------------------------------------------------------------
  // statute_of_limitations — Verkürzung/Verfall (§ 309 Nr. 7, § 307 BGB)
  // -------------------------------------------------------------------------
  {
    id: "acc-word-number-12",
    detector: "statute_of_limitations",
    variant: "deceptive",
    technique: "word-number",
    severity: "hoch",
    text: "Sämtliche Ansprüche des Bestellers wegen Sach- und Rechtsmängeln verfallen mit Ablauf des sechsten Monats nach Gefahrübergang.",
    note: "Verfallsklausel mit ausgeschriebener Zahl und dem Verb 'verfallen' statt 'verjähren'/'erlöschen endgültig'; keine Ausnahme für Körperschäden, Vorsatz oder grobe Fahrlässigkeit.",
  },
  {
    id: "acc-euphemism-13",
    detector: "statute_of_limitations",
    variant: "deceptive",
    technique: "euphemism",
    severity: "hoch",
    text: "Mängelansprüche können ausschließlich innerhalb von neun Monaten nach Übergabe der Werkleistung geltend gemacht werden; nach Fristablauf gilt die Leistung als vertragsgemäß erbracht.",
    note: "Verkürzung der Verjährung sprachlich als reine 'Geltendmachung' umschrieben ('ausschließlich innerhalb von neun Monaten'), ergänzt um eine Erfüllungsfiktion. Die Verjährungs-Patterns greifen nicht.",
  },
  {
    id: "acc-dach-14",
    detector: "statute_of_limitations",
    variant: "deceptive",
    technique: "dach",
    severity: "hoch",
    text: "Sämtliche Gewährleistungs- und Mängelansprüche verwirken mit Ablauf von zwölf Monaten nach Ablieferung; die Geltendmachung nach Fristablauf ist ausgeschlossen.",
    note: "Schweizer/österreichische Terminologie: 'verwirken' statt 'verjähren', 'Gewährleistungsansprüche' statt Mängelansprüche. Kein Ausnahmevorbehalt für Körperschäden oder grobes Verschulden.",
  },
  {
    id: "acc-english-15",
    detector: "statute_of_limitations",
    variant: "deceptive",
    technique: "english",
    severity: "hoch",
    text: "Any claim arising out of or relating to this Agreement must be asserted within six (6) months after the date on which the claim accrues, failing which it shall be forever barred, irrespective of the claimant's knowledge or of the nature of the claim.",
    note: "Englische Verjährungsverkürzung inklusive Körperschäden und grober Fahrlässigkeit — die Limitation-Patterns der Engine sind rein deutschsprachig.",
  },
  {
    id: "acc-direct-16",
    detector: "statute_of_limitations",
    variant: "deceptive",
    technique: "direct",
    severity: "hoch",
    text: "Ansprüche des Kunden wegen Sachmängeln verjähren innerhalb von sechs Monaten ab Ablieferung der Ware.",
    note: "Kontrollfall im Klartext: pauschale Verjährungsverkürzung ohne Ausnahme für Körperschäden, Vorsatz und grobe Fahrlässigkeit.",
  },

  // -------------------------------------------------------------------------
  // burden_of_proof — Beweislastumkehr (§ 309 Nr. 12 BGB)
  // -------------------------------------------------------------------------
  {
    id: "acc-nominal-style-17",
    detector: "burden_of_proof",
    variant: "deceptive",
    technique: "nominal-style",
    severity: "hoch",
    text: "Die Darlegungs- und Beweislast für das Nichtvorliegen eines Fehlers der überlassenen Software liegt beim Auftraggeber.",
    note: "Beweislastumkehr im Nominalstil mit negativer Tatsache ('Nichtvorliegen eines Fehlers'). Kontrollfall: 'Beweislast ... liegt beim Auftraggeber' sollte erfasst sein.",
  },
  {
    id: "acc-euphemism-18",
    detector: "burden_of_proof",
    variant: "deceptive",
    technique: "euphemism",
    severity: "hoch",
    text: "Der Auftraggeber anerkennt die überlassene Fassung als ordnungsgemäß und in jeder Hinsicht vollständig; der Nachweis abweichender Umstände ist von ihm zu erbringen.",
    note: "Vorab-Bestätigung der Mangelfreiheit plus Gegenbeweislast, aber ohne die Keywords 'bestätigt ... mangelfrei/fehlerfrei' — stattdessen 'anerkennt ... als ordnungsgemäß und vollständig'.",
  },
  {
    id: "acc-synonym-19",
    detector: "burden_of_proof",
    variant: "deceptive",
    technique: "synonym",
    severity: "hoch",
    text: "Es obliegt dem Auftraggeber, den Nachweis dafür zu führen, dass eine Störung nicht auf Gegebenheiten seines eigenen Verantwortungsbereichs zurückzuführen ist.",
    note: "Negativbeweis-Pflicht in untypischer Syntax ('Es obliegt dem Auftraggeber, den Nachweis zu führen' statt 'obliegt es dem Kunden nachzuweisen').",
  },
  {
    id: "acc-split-sentence-20",
    detector: "burden_of_proof",
    variant: "deceptive",
    technique: "split-sentence",
    severity: "hoch",
    text: "Über Fehlerfreiheit und Umfang der Leistung entscheiden die Aufzeichnungen des Anbieters abschließend. Gelingt dem Auftraggeber der Gegenbeweis nicht, gilt die Leistung als ordnungsgemäß erbracht und alle Mängelrechte entfallen.",
    note: "Beweismonopol plus Beweislastumkehr, verteilt auf zwei Sätze und ohne die Formulierungen 'gelten als Vollbeweis' oder 'Beweislast obliegt'.",
  },
  {
    id: "acc-dach-21",
    detector: "burden_of_proof",
    variant: "deceptive",
    technique: "dach",
    severity: "hoch",
    text: "Die Beweislast für das Vorliegen eines Mangels trägt nach Abnahme ausschließlich der Besteller.",
    note: "Beweislastumkehr in DACH-Register ('Besteller', 'Abnahme'). Kontrollfall: Muster 'Beweislast ... trägt ...' ist erfasst.",
  },

  // -------------------------------------------------------------------------
  // vicarious_agents — Erfüllungsgehilfen (§ 278, § 307 BGB)
  // -------------------------------------------------------------------------
  {
    id: "acc-synonym-22",
    detector: "vicarious_agents",
    variant: "deceptive",
    technique: "synonym",
    severity: "hoch",
    text: "Der Auftragnehmer haftet für das Verhalten der von ihm herangezogenen Dritten nur im Rahmen der sorgfältigen Auswahl und Überwachung dieser Dritten.",
    note: "Reduktion auf Auswahlverschulden ohne die Keywords 'Erfüllungsgehilfe'/'Subunternehmer' und ohne 'Auswahlverschulden' — die Haftung für eingesetzte Dritte wird faktisch auf ein Organisationsverschulden verkürzt.",
  },
  {
    id: "acc-euphemism-23",
    detector: "vicarious_agents",
    variant: "deceptive",
    technique: "euphemism",
    severity: "hoch",
    text: "Für das Verhalten der von ihm beauftragten Nachunternehmer übernimmt der Auftragnehmer keine Verantwortung, soweit ihn nicht der Vorwurf einer sorgfaltswidrigen Auswahl trifft.",
    note: "Haftungsflucht für Nachunternehmer in weicher Sprache ('keine Verantwortung') mit dem Scheinvorbehalt der Auswahlpflicht; § 278 BGB wird damit abbedungen.",
  },
  {
    id: "acc-direct-24",
    detector: "vicarious_agents",
    variant: "deceptive",
    technique: "direct",
    severity: "hoch",
    text: "Eine Haftung des Anbieters für das Verschulden seiner Erfüllungsgehilfen und Unterauftragnehmer ist ausgeschlossen.",
    note: "Kontrollfall im Klartext: vollständiger Ausschluss der Erfüllungsgehilfenhaftung.",
  },
  {
    id: "acc-english-25",
    detector: "vicarious_agents",
    variant: "deceptive",
    technique: "english",
    severity: "hoch",
    text: "The Provider shall not be responsible for the acts or omissions of its subcontractors and any liability in respect thereof is hereby excluded, whether or not the Provider supervised their work.",
    note: "Englischer Ausschluss der Gehilfenhaftung; 'shall not be responsible' umgeht auch die englische Liability-Pattern der Engine ('shall not be liable').",
  },
  {
    id: "acc-cross-reference-26",
    detector: "vicarious_agents",
    variant: "deceptive",
    technique: "cross-reference",
    severity: "hoch",
    text: "Für die Einstandspflicht des Auftragnehmers für die von ihm eingesetzten Nachunternehmer gilt § 9 Absatz 3 (Haftungsbeschränkungen und Auswahlpflichten) dieses Vertrages entsprechend.",
    note: "Versteckte Gehilfenhaftungs-Beschränkung, die nur über den Querverweis auf § 9 Abs. 3 sichtbar wird; der Klauseltext selbst enthält keinerlei belastende Formulierung.",
  },

  // -------------------------------------------------------------------------
  // third_party_indemnity — verschuldensunabhängige Drittfreistellung (§ 307 BGB)
  // -------------------------------------------------------------------------
  {
    id: "acc-euphemism-27",
    detector: "third_party_indemnity",
    variant: "deceptive",
    technique: "euphemism",
    severity: "hoch",
    text: "Der Auftraggeber hält den Auftragnehmer von jeder Inanspruchnahme durch Dritte schadlos, die mit der Nutzung der Leistung zusammenhängt, ohne dass es auf ein Verschulden des Auftraggebers ankommt.",
    note: "Verschuldensunabhängige Freistellung als 'schadlos halten' formuliert — beide Keywords 'freistellen' und 'freizuhalten' werden umgangen.",
  },
  {
    id: "acc-synonym-28",
    detector: "third_party_indemnity",
    variant: "deceptive",
    technique: "synonym",
    severity: "hoch",
    text: "Der Kunde stellt den Anbieter von Ansprüchen Dritter frei, die im Zusammenhang mit der Nutzung der vereinbarten Leistung geltend gemacht werden.",
    note: "Freistellung ohne Verschuldenserfordernis in umgekehrter Wortstellung ('stellt ... frei' statt '... freizustellen'); die Indemnity-Patterns erwarten das Verb am Satzende.",
  },
  {
    id: "acc-list-buried-29",
    detector: "third_party_indemnity",
    variant: "deceptive",
    technique: "list-buried",
    severity: "hoch",
    text: "Der Kunde ist verpflichtet, den Anbieter von sämtlichen Ansprüchen Dritter freizustellen; dies gilt unabhängig von einem Verschulden des Kunden sowie für Bußgelder von Aufsichtsbehörden und behördliche Anordnungen.",
    note: "Starre, verschuldensunabhängige Freistellung: der entscheidende Halbsatz 'unabhängig von einem Verschulden' steht nach dem Semikolon, die Freistellung davor.",
  },
  {
    id: "acc-english-30",
    detector: "third_party_indemnity",
    variant: "deceptive",
    technique: "english",
    severity: "hoch",
    text: "The Customer shall defend, indemnify and hold harmless the Provider against any and all third-party claims, demands, fines and penalties, regardless of fault or of whether the claim is attributable to the Provider.",
    note: "Englische No-Fault-Indemnity. Kontrollfall: 'hold harmless ... third-party claims' sollte die englischen Patterns treffen.",
  },

  // -------------------------------------------------------------------------
  // audit_log_manipulation — Beweismonopol (§ 309 Nr. 12 BGB)
  // -------------------------------------------------------------------------
  {
    id: "acc-direct-31",
    detector: "audit_log_manipulation",
    variant: "deceptive",
    technique: "direct",
    severity: "hoch",
    text: "Die Systemprotokolle des Anbieters gelten als vollbeweislicher Nachweis der vertragsgemäßen Leistungserbringung; abweichende Nachweise des Kunden sind unbeachtlich.",
    note: "Kontrollfall im Klartext: unwiderlegbares Beweismonopol durch Serverprotokolle.",
  },
  {
    id: "acc-synonym-32",
    detector: "audit_log_manipulation",
    variant: "deceptive",
    technique: "synonym",
    severity: "hoch",
    text: "Den Aufzeichnungen des Dienstleisters kommt die Bedeutung eines unwiderlegbaren Nachweises für Umfang und Güte der erbrachten Leistungen zu; der Kunde unterwirft sich dieser Feststellung.",
    note: "Beweismonopol ohne 'gelten als Vollbeweis': 'kommt die Bedeutung eines unwiderlegbaren Nachweises zu' plus Unterwerfungserklärung.",
  },
  {
    id: "acc-nominal-style-33",
    detector: "audit_log_manipulation",
    variant: "deceptive",
    technique: "nominal-style",
    severity: "hoch",
    text: "Für die Feststellung von Fehlern sowie für die Abrechnung ist ausschließlich die Protokollierung des Auftragnehmers maßgeblich; abweichende Nachweise des Auftraggebers bleiben ohne Bedeutung.",
    note: "Beweismonopol im Nominalstil ('Protokollierung des Auftragnehmers ist maßgeblich') — die Patterns verlangen 'Protokolle des Anbieters' bzw. 'gelten als maßgeblich'.",
  },
  {
    id: "acc-english-34",
    detector: "audit_log_manipulation",
    variant: "deceptive",
    technique: "english",
    severity: "hoch",
    text: "The Provider's server logs shall constitute conclusive and binding evidence of the Customer's consumption and of the absence of any defect, and the Customer irrevocably waives any right to challenge them.",
    note: "Englisches Beweismonopol; alle Audit-Log-Patterns sind deutschsprachig.",
  },

  // -------------------------------------------------------------------------
  // Compliant baselines — False-Positive-Resistenz
  // -------------------------------------------------------------------------
  {
    id: "acc-compliant-01",
    detector: null,
    nearestDetector: "liability",
    variant: "compliant",
    technique: "direct",
    severity: null,
    text: "Der Auftragnehmer haftet unbeschränkt für Vorsatz, grobe Fahrlässigkeit sowie für Schäden aus der Verletzung von Leben, Körper oder Gesundheit. Bei leichter Fahrlässigkeit haftet er ausschließlich für die Verletzung wesentlicher Vertragspflichten und lediglich in Höhe des vertragstypischen, vorhersehbaren Schadens.",
    note: "Gesetzeskonforme Haftungsklausel (unbeschränkte Haftung für Vorsatz, grobe Fahrlässigkeit und Körperschäden, Beschränkung nur für leichte Fahrlässigkeit und Kardinalpflichten).",
  },
  {
    id: "acc-compliant-02",
    detector: null,
    nearestDetector: "penalty",
    variant: "compliant",
    technique: "direct",
    severity: null,
    text: "Für jeden Tag des Verzugs kann der Auftragnehmer einen pauschalierten Schadensersatz in Höhe von 0,5 % der jeweiligen Auftragssumme verlangen. Dem Auftraggeber bleibt der Nachweis eines wesentlich geringeren Schadens ausdrücklich vorbehalten.",
    note: "Zulaessige Pauschalierung nach § 309 Nr. 5 lit. b BGB: der Gegenbeweis eines wesentlich geringeren Schadens ist ausdrücklich eröffnet.",
  },
  {
    id: "acc-compliant-03",
    detector: null,
    nearestDetector: "statute_of_limitations",
    variant: "compliant",
    technique: "direct",
    severity: null,
    text: "Mängelansprüche verjähren nach den gesetzlichen Vorschriften. Für Ansprüche aus Vorsatz, grober Fahrlässigkeit sowie aus der Verletzung von Leben, Körper oder Gesundheit gelten die regelmäßigen gesetzlichen Fristen.",
    note: "Gesetzliche Verjährung mit ausdrücklichem Schutz für Körperschäden und grobes Verschulden.",
  },
  {
    id: "acc-compliant-04",
    detector: null,
    nearestDetector: "burden_of_proof",
    variant: "compliant",
    technique: "direct",
    severity: null,
    text: "Die gesetzliche Beweislastverteilung bleibt unberührt. Den Anbieter trifft die Beweislast dafür, dass die überlassene Leistung im Zeitpunkt der Übergabe frei von Fehlern war.",
    note: "Gesetzliche Beweislastverteilung, Beweislast liegt beim Anbieter — ausdrücklich keine Umkehr zu Lasten des Kunden.",
  },
];

export default CASES;
