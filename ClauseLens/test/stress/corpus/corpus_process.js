/**
 * Stress-Korpus Slice: Form, Prozess & Verfahren
 * Detektoren: written_form_trap, deemed_acceptance, fictitious_receipt,
 *             unilateral_change, jurisdiction, unreasonable_audit,
 *             non_compete, non_solicitation
 *
 * Ziel: Blindspots eines satzweise arbeitenden Regex-Detektors finden.
 * Angriffsflächen in diesem Slice: Form- und Forenklauseln ohne die
 * gesetzlichen Schlüsselwörter, Fristen- und Genehmigungsfiktionen,
 * einseitige Änderungsrechte im Gewand redaktioneller Anpassungen,
 * Kontrollrechte als Kooperationspflichten, nachvertragliche Bindungen.
 */

export const CASES = [
  // ==========================================================================
  // written_form_trap (§ 309 Nr. 13 BGB, § 305b BGB)
  // ==========================================================================
  {
    id: "proc-synonym-01",
    detector: "written_form_trap",
    variant: "deceptive",
    technique: "synonym",
    severity: "hoch",
    text: "Die Beendigung dieses Vertragsverhältnisses ist nur durch ein an den Vorstand gerichtetes, handschriftlich unterzeichnetes Schreiben in Papierform möglich. Eine Nachricht auf elektronischem Weg entfaltet insoweit keine Rechtswirkung.",
    note: "Kündigungsformfalle ohne die Wörter 'Schriftform' oder 'Einschreiben': 'handschriftlich unterzeichnetes Schreiben in Papierform' schließt Textform (§ 126b BGB) aus und verstößt gegen § 309 Nr. 13 BGB.",
  },
  {
    id: "proc-synonym-02",
    detector: "written_form_trap",
    variant: "deceptive",
    technique: "synonym",
    severity: "hoch",
    text: "Sämtliche Willenserklärungen des Kunden sind in unterschriftsbeglaubigter Ausfertigung zu übermitteln; die Übermittlung per Telefax, E-Mail oder über das Kundenportal ist für die Wirksamkeit ohne Bedeutung.",
    note: "Beglaubigung verlangt eine strengere Form als die gesetzlich zulässige Textform; zudem wird der elektronische Zugang ausdrücklich für unwirksam erklärt.",
  },
  {
    id: "proc-nominal-style-01",
    detector: "written_form_trap",
    variant: "deceptive",
    technique: "nominal-style",
    severity: "mittel",
    text: "Der Wirksamkeit jedweder Vertragsbeendigung vorgeschaltet ist die Einreichung eines unterzeichneten Originaldokuments bei der Registrierungsstelle des Anbieters innerhalb der vertraglichen Beendigungsperiode.",
    note: "Völlig nominalisierte Formfalle: 'unterzeichnetes Originaldokument' plus 'Registrierungsstelle' erschwert die Kündigung erheblich, ohne die juristischen Standardbegriffe zu verwenden.",
  },
  {
    id: "proc-cross-reference-01",
    detector: "written_form_trap",
    variant: "deceptive",
    technique: "cross-reference",
    severity: "hoch",
    text: "Für alle Erklärungen der Parteien gilt die in Ziffer 14.3 dieses Vertrages definierte Übermittlungsart. Übermittlungsart im Sinne von Ziffer 14.3 ist die eigenhändig unterzeichnete, per Postkarte zu übersendende Exklusivform; andere Übermittlungswege bleiben ausgeschlossen.",
    note: "Formfalle ausschließlich über Querverweis und definierten Begriff ('Übermittlungsart'): der Trigger sitzt in einer Definitionsziffer, die für sich genommen harmlos wirkt.",
  },
  {
    id: "proc-typo-evasion-01",
    detector: "written_form_trap",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Kündigungen bedürfen der eigenh\u00e4ndigen Unter\u00adschrift und sind per Ein\u200bschreiben mit Rück\u00adschein zu erklären; die Text\u00adform ist ausdrücklich aus\u00adgeschlossen.",
    note: "Typografische Evasion: Soft-Hyphens (\\u00ad) und Zero-Width-Spaces (\\u200b) zerreißen 'Unterschrift', 'Einschreiben', 'Rückschein' und 'ausgeschlossen', sodass Zeichenketten-Regexe nicht mehr greifen.",
  },

  // ==========================================================================
  // deemed_acceptance (§ 640 Abs. 2 S. 2 BGB, § 308 Nr. 5 BGB)
  // ==========================================================================
  {
    id: "proc-euphemism-01",
    detector: "deemed_acceptance",
    variant: "deceptive",
    technique: "euphemism",
    severity: "hoch",
    text: "Nach Übergabe der Arbeitsergebnisse beginnt ein Prüfzeitraum von sieben Werktagen. Äußert sich der Kunde in diesem Zeitraum nicht in der vorgesehenen Form, werden die Ergebnisse als vertragsgemäß behandelt und das Projekt ohne weitere Förmlichkeiten abgeschlossen.",
    note: "Abnahmefiktion ohne das Wort 'abgenommen' und ohne die gesetzliche Belehrung nach § 640 Abs. 2 S. 2 BGB: 'als vertragsgemäß behandelt' plus 'Prüfzeitraum' erzeugt die Fiktion verdeckt.",
  },
  {
    id: "proc-euphemism-02",
    detector: "deemed_acceptance",
    variant: "deceptive",
    technique: "euphemism",
    severity: "hoch",
    text: "Beanstandungen zu Funktionsumfang und Qualität erbittet der Anbieter innerhalb von zehn Kalendertagen nach Bereitstellung. Danach betrachtet der Anbieter die Positionen als verbindlich übernommen und schließt die Bearbeitung ab.",
    note: "Ultrakurze Rügefrist mit Bindungsfiktion, formuliert als höfliche Bitte ('erbittet') ohne Rechtsfolgenhinweis - typische Umgehung des § 308 Nr. 5 BGB.",
  },
  {
    id: "proc-split-sentence-01",
    detector: "deemed_acceptance",
    variant: "deceptive",
    technique: "split-sentence",
    severity: "hoch",
    text: "Der Kunde nimmt die Lieferung unverzüglich nach Bereitstellung in Augenschein. Nicht angezeigte Abweichungen sind einer späteren Beanstandung nicht mehr zugänglich; die Lieferung verbleibt endgültig beim Kunden.",
    note: "Trigger und Rechtsfolge sind auf zwei Sätze verteilt: der zweite Satz enthält kein Verbindungswort zur Abnahmefiktion und umgeht damit satzweise arbeitende Regeln.",
  },
  {
    id: "proc-list-buried-01",
    detector: "deemed_acceptance",
    variant: "deceptive",
    technique: "list-buried",
    severity: "hoch",
    text: "Der Kunde hat bei der Übernahme der Projektartefakte Folgendes zu beachten: (a) Vollständigkeitskontrolle der übergebenen Dateien; (b) Prüfung der Zugriffsberechtigungen; (c) Meldung von Abweichungen binnen fünf Werktagen über das Ticketsystem; unterbleibt eine Meldung, sind die Artefakte nach Ablauf der Frist abschließend freigegeben; (d) Archivierung der Prüfprotokolle; (e) Rückmeldung an den Projektleiter.",
    note: "Die Abnahmefiktion steckt in Punkt (c) einer fünfteiligen Aufzählung; die Segmentierung nach Semikolon verhindert, dass die Regel den Kern der Aufzählung als zusammenhängende Falle liest.",
  },
  {
    id: "proc-word-number-01",
    detector: "deemed_acceptance",
    variant: "deceptive",
    technique: "word-number",
    severity: "hoch",
    text: "Mängel der übergebenen Werkleistung sind spätestens achtundvierzig Stunden nach Übergabe schr\u00ADiftlich anzumelden; widrigenfalls wird die Werkleistung als vollständig und in jeder Hinsicht vertragsgemäß angesehen.",
    note: "Ausschlussfrist von achtundvierzig Stunden als ausgeschriebenes Zahlwort mit eingeschobenem Soft-Hyphen: numerische Fristmuster erfassen '48' oder 'zweiundsiebzig', nicht diese Schreibweise.",
  },
  {
    id: "proc-dach-01",
    detector: "deemed_acceptance",
    variant: "deceptive",
    technique: "dach",
    severity: "hoch",
    text: "Die Prüfung des Werks erfolgt nach den Regeln der Branchenübung. Erhebt der Besteller keinen Mängelrüge-Vorbehalt, gilt das Werk nach den anwendbaren Regeln des Schweizer Obligationenrechts als genehmigt.",
    note: "Schweizer Terminologie (OR, Werk, Genehmigung) kombiniert mit knapper Rügevermutung - die deutsche Abnahme-/Rügefrist-Systematik greift sprachlich nicht.",
  },

  // ==========================================================================
  // fictitious_receipt (§ 308 Nr. 5 BGB, § 130 BGB)
  // ==========================================================================
  {
    id: "proc-euphemism-03",
    detector: "fictitious_receipt",
    variant: "deceptive",
    technique: "euphemism",
    severity: "mittel",
    text: "Der Anbieter stellt Mitteilungen in das Benachrichtigungsportal ein. Mit der Einstellung in das Portal sind Mitteilungen dem Kunden bekannt; auf den tatsächlichen Abruf kommt es nicht an.",
    note: "Zugangsfiktion ohne die Wörter 'gelten als zugegangen': das Abstellen auf die 'Einstellung in das Portal' ersetzt den tatsächlichen Zugang nach § 130 BGB.",
  },
  {
    id: "proc-synonym-03",
    detector: "fictitious_receipt",
    variant: "deceptive",
    technique: "synonym",
    severity: "mittel",
    text: "Abrechnungen, Mahnungen und Anpassungsmitteilungen sind mit Übergabe an den Versanddienstleister als in den Herrschaftsbereich des Kunden gelangt anzusehen. Verzögerungen oder Verluste auf dem Transportweg gehen zu Lasten des Kunden.",
    note: "Zugangsersatz durch 'in den Herrschaftsbereich gelangt' + Absende-Fiktion mit Risikotragung des Empfängers; kein Standardwortlaut vorhanden.",
  },
  {
    id: "proc-split-sentence-02",
    detector: "fictitious_receipt",
    variant: "deceptive",
    technique: "split-sentence",
    severity: "mittel",
    text: "Preisanpassungsmitteilungen versendet der Anbieter per E-Mail an die zuletzt hinterlegte Adresse. Sie entfalten ihre Wirkung sieben Werktage nach Versand, unabhängig vom Empfang und vom Verbleib der Nachricht.",
    note: "Rechtsfolge ('Wirkung nach sieben Werktagen unabhängig vom Empfang') ist in einen eigenen Satz ausgelagert und nutzt weder 'Zugang' noch 'zugegangen'.",
  },
  {
    id: "proc-nominal-style-02",
    detector: "fictitious_receipt",
    variant: "deceptive",
    technique: "nominal-style",
    severity: "mittel",
    text: "Der Nachweis der Abwesenheit von Störfaktoren obliegt dem Kunden. Eine Fristversäumnisbewertung erfolgt ausschließlich anhand des internen Protokolls; die Zustellung ist mit Absendung bewirkt.",
    note: "Nominalstil verdeckt die Zugangsfiktion im letzten Halbsatz ('Zustellung ist mit Absendung bewirkt') und kombiniert sie mit Beweislastumkehr.",
  },
  {
    id: "proc-typo-evasion-02",
    detector: "fictitious_receipt",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "mittel",
    text: "Erkl\u00e4rungen des Anbieters gelten mit A\u00ads\u200bs\u200bendung als zuge\u00adgangen, wenn sie an die im Kunden\u00adstamm\u00addaten\u00adsatz hinterlegte Anschrift gerichtet sind.",
    note: "Zero-Width-Spaces und Soft-Hyphens in 'Absendung' und 'zugegangen' brechen die Fiktionsmuster; die Falle bleibt inhaltlich unverändert.",
  },

  // ==========================================================================
  // unilateral_change (§ 307, § 308 Nr. 4/5 BGB)
  // ==========================================================================
  {
    id: "proc-euphemism-04",
    detector: "unilateral_change",
    variant: "deceptive",
    technique: "euphemism",
    severity: "hoch",
    text: "Der Anbieter behält sich vor, den Leistungskatalog zu präzisieren, an den Stand der Technik anzupassen und einzeln benannte Funktionen durch gleichwertige Mechanismen zu ersetzen. Ein Anspruch auf Beibehaltung einzelner Funktionsmerkmale besteht nicht; die Parteien sind sich einig, dass die Fortführung des Nutzungsverhältnisses diesen Präzisierungen Rechnung trägt.",
    note: "Änderungsvorbehalt mit Entkernung, verpackt als 'Präzisierung' und 'Stand der Technik'; die Zustimmungsfiktion steckt in 'die Fortführung ... trägt Rechnung'.",
  },
  {
    id: "proc-euphemism-05",
    detector: "unilateral_change",
    variant: "deceptive",
    technique: "euphemism",
    severity: "hoch",
    text: "Erforderliche Nachjustierungen des Leistungsumfangs und der Vergütungsstruktur werden den veränderten Marktgegebenheiten angepasst. Sofern die Anpassung die Gesamtvergütung um nicht mehr als fünfzehn Prozent erhöht, sind die damit verbundenen Aufwendungen als kaufmännisch mitgetragen anzusehen; ein Sonderkündigungsrecht soll erst bei einer darüber hinausgehenden Steigerung bestehen.",
    note: "Preisgleitklausel mit Schwellenwert und faktischer Kostenfreigabefiktion ('kaufmännisch mitgetragen'), formuliert ohne 'einseitig' oder 'genehmigt'.",
  },
  {
    id: "proc-synonym-04",
    detector: "unilateral_change",
    variant: "deceptive",
    technique: "synonym",
    severity: "hoch",
    text: "Der Anbieter ist berechtigt, die Beschaffenheit der geschuldeten Dienste fortzuentwickeln und Module nach seinem betriebswirtschaftlichen Kalkül aus dem Portfolio zu nehmen. Wegen dieser Weiterentwicklung wird ein Minderungsbegehren des Kunden der Billigkeit entsprechend herabgesetzt.",
    note: "Verkapptes einseitiges Leistungsbestimmungsrecht mit Ausschluss der Minderung; 'fortentwickeln', 'aus dem Portfolio nehmen' und 'der Billigkeit entsprechend' umgehen die Standardtrigger.",
  },
  {
    id: "proc-cross-reference-02",
    detector: "unilateral_change",
    variant: "deceptive",
    technique: "cross-reference",
    severity: "mittel",
    text: "Struktur- und Funktionsanpassungen richten sich nach Anlage 3. Anlage 3 lautet: Anpassungen der Schnittstellen und der Modulzuschnitte kann der Anbieter mit Wirkung für die Zukunft vornehmen; die Fortsetzung der Nutzung wird der Neufassung zugrunde gelegt.",
    note: "Änderungsrecht und Zustimmungsfiktion sind ausschließlich in einer Anlage platziert, auf die der Hauptvertrag nur verweist.",
  },
  {
    id: "proc-list-buried-02",
    detector: "unilateral_change",
    variant: "deceptive",
    technique: "list-buried",
    severity: "hoch",
    text: "Der Anbieter informiert über Produktentscheidungen im Rahmen der folgenden Mitteilungen: (a) Release Notes; (b) Betriebsstatusberichte; (c) Sicherheitshinweise; (d) Anpassungen der Vergütungsbestandteile und Modulzuschnitte, die der Anbieter für erforderlich hält; (e) Roadmap-Informationen. Für die Punkte (a) bis (c) und (e) besteht keine Bindung, während die unter (d) mitgeteilten Anpassungen als betriebswirtschaftlich erforderlich und damit akzeptiert behandelt werden.",
    note: "Der Änderungsvorbehalt liegt in Punkt (d) einer Aufzählung; die Fiktion wohnt in einem zweiten Satz, der den Listenpunkt nur zitiert.",
  },

  // ==========================================================================
  // jurisdiction (§ 38 ZPO, § 305c BGB)
  // ==========================================================================
  {
    id: "proc-synonym-05",
    detector: "jurisdiction",
    variant: "deceptive",
    technique: "synonym",
    severity: "hoch",
    text: "Streitigkeiten aus diesem Vertrag werden bei der Niederlassung der Anbieterin in Zug zur Entscheidung gebracht; die Anbieterin bleibt frei, Ansprüche bei jedem beliebigen Gericht geltend zu machen.",
    note: "Forum-Wahl mit Markenbegriffen ('Niederlassung', 'zur Entscheidung gebracht') anstelle von 'Gerichtsstand', dazu asymmetrischer Vorbehalt und Auslandsbezug.",
  },
  {
    id: "proc-euphemism-06",
    detector: "jurisdiction",
    variant: "deceptive",
    technique: "euphemism",
    severity: "hoch",
    text: "Als Ort der Streitbeilegung zwischen den Parteien wird der Hauptsitz des Anbieters in Delaware vorgesehen. Klagen des Kunden werden dort anhängig gemacht; Klagen des Anbieters richten sich nach Wahl des Anbieters nach den allgemeinen Regeln.",
    note: "Delaware-Forum ohne das Wort 'Gerichtsstand': 'Ort der Streitbeilegung' und 'anhängig gemacht' umgehen die Forum-Regexe, bleiben aber eine kostenintensive Zwangszuweisung.",
  },
  {
    id: "proc-english-01",
    detector: "jurisdiction",
    variant: "deceptive",
    technique: "english",
    severity: "hoch",
    text: "Venue shall lie exclusively in the courts of the Provider's seat in Singapore, and the Customer hereby submits to that venue; the Provider may nevertheless bring claims before any other court of competent jurisdiction.",
    note: "Englischsprachige Zwangsforumsklausel mit asymmetrischem Wahlrecht; das Muster 'exclusive jurisdiction of' wird nicht getroffen, da 'venue' und 'Provider's seat' verwendet werden.",
  },
  {
    id: "proc-english-02",
    detector: "jurisdiction",
    variant: "deceptive",
    technique: "english",
    severity: "hoch",
    text: "This Agreement and all disputes arising out of it are subject to the laws of the State of New York, without regard to conflict-of-law principles, and the parties waive the application of the United Nations Convention on Contracts for the International Sale of Goods.",
    note: "Rechts- und Forenwahl über 'subject to the laws of' statt der gematchten Wendungen 'governed by'/'construed in accordance with'; CISG-Ausschluss mit anderer Umschreibung.",
  },
  {
    id: "proc-split-sentence-03",
    detector: "jurisdiction",
    variant: "deceptive",
    technique: "split-sentence",
    severity: "hoch",
    text: "Für Streitigkeiten wird ein einheitlicher Ort bestimmt. Der Ort ist der Sitz des Anbieters in London. Der Kunde erhebt seine Klagen ausschließlich dort. Dem Anbieter bleibt die Klage an dessen allgemeinem Forum unbenommen.",
    note: "Kurze Sätze zerteilen die Forumsklausel; kein einzelner Satz enthält mehr Forum, Ort und Ausschließlichkeit zusammen.",
  },
  {
    id: "proc-word-number-01b",
    detector: "jurisdiction",
    variant: "deceptive",
    technique: "word-number",
    severity: "mittel",
    text: "Die Parteien verständigen sich auf den dritten Abschnitt des Handelszentrums Zürich-Nord als gemeinsamen Verhandlungs- und Entscheidungsort für alle Meinungsverschiedenheiten aus dem Vertragsverhältnis.",
    note: "Auslandsforum wird über eine Ortsbeschreibung (Abschnitt, Handelszentrum) eingeführt; juristische Forenbegriffe fehlen vollständig.",
  },

  // ==========================================================================
  // unreasonable_audit (§ 307 BGB, GeschGehG)
  // ==========================================================================
  {
    id: "proc-euphemism-07",
    detector: "unreasonable_audit",
    variant: "deceptive",
    technique: "euphemism",
    severity: "hoch",
    text: "Der Kunde unterstützt Maßnahmen des Anbieters zur Qualitätssicherung und Compliance dadurch, dass er Vertretern des Anbieters den Zutritt zu seinen Betriebsstätten während der Geschäftszeiten sowie zu sämtlichen abrechnungsrelevanten Unterlagen ermöglicht. Die Einsichtnahme erfolgt anlassbezogen; ein Vorlauf entfällt, die Kosten der Maßnahme trägt der Kunde.",
    note: "Unangekündigtes Prüfungsrecht als 'Unterstützungspflicht' deklariert, mit Kostenabwälzung - die Regel sucht Auditbegriffe mit Ankündigungs-/Zutrittsmustern.",
  },
  {
    id: "proc-nominal-style-03",
    detector: "unreasonable_audit",
    variant: "deceptive",
    technique: "nominal-style",
    severity: "mittel",
    text: "Zur Sicherstellung der Vertragskonformität findet eine Einsichtnahme in die prozessrelevanten Systeme des Kunden statt. Der Zeitpunkt der Einsichtnahme liegt im Ermessen des Anbieters; eine Vorabinformation über den Zeitpunkt ist nicht vorgesehen.",
    note: "Nominalstil ersetzt 'auditieren/jederzeit' durch 'Einsichtnahme' und 'Vorabinformation nicht vorgesehen' - semantisch ein Audit ohne Vorlauf.",
  },
  {
    id: "proc-cross-reference-03",
    detector: "unreasonable_audit",
    variant: "deceptive",
    technique: "cross-reference",
    severity: "hoch",
    text: "Es gilt die Prüfungsordnung nach Anhang B. Anhang B bestimmt: Prüfungen werden unmittelbar durchgeführt, sobald dies erforderlich erscheint; eine Terminabstimmung entfällt; Aufwendungen für Prüfungshandlungen gehen zulasten des Kunden.",
    note: "Auditrecht vollständig in einen Anhang verlagert: der Haupttext enthält keinen einzigen Audit-Trigger.",
  },
  {
    id: "proc-list-buried-03",
    detector: "unreasonable_audit",
    variant: "deceptive",
    technique: "list-buried",
    severity: "mittel",
    text: "Der Kunde stellt dem Anbieter folgende Unterstützung bereit: (a) technische Ansprechpartner; (b) Testumgebungen; (c) Zugang zu den Geschäftsräumen zur Inspektion; (d) Unterlagen zur Rechnungsprüfung mit Nachweispflicht; (e) jährliche Schulungen. Punkt (c) und (d) bilden weitreichende Kontrollrechte ab, die der Kunde unabhängig von einem konkreten Anlass zu erfüllen hat.",
    note: "Weitreichendes Kontroll- und Nachweispaket in einer Unterstützungsliste; die einzelnen Elemente werden satzweise nicht als Audit gelesen.",
  },

  // ==========================================================================
  // non_compete (§ 74 HGB, § 138 BGB)
  // ==========================================================================
  {
    id: "proc-euphemism-08",
    detector: "non_compete",
    variant: "deceptive",
    technique: "euphemism",
    severity: "hoch",
    text: "Der Kunde verpflichtet sich, während einer Übergangs- und Nachvollzugsphase von achtzehn Monaten die Leistungen des Anbieters nicht durch gleichartige Angebote Dritter zu substituieren. Ein Ausgleich für diese Zurückhaltung ist im vereinbarten Entgelt enthalten.",
    note: "Nachvertragliches Tätigkeitsverbot als 'Zurückhaltung' und 'Nachvollzugsphase' verpackt; die Pauschalaussage 'im Entgelt enthalten' ersetzt die entfallende Karenzentschädigung.",
  },
  {
    id: "proc-synonym-06",
    detector: "non_compete",
    variant: "deceptive",
    technique: "synonym",
    severity: "hoch",
    text: "Der Auftragnehmer verpflichtet sich, für zweiundzwanzig Monate nach Auslaufen der Zusammenarbeit im Wirtschaftsraum DACH keine Beschäftigung oder Gelegenheit anzunehmen, die mit dem Geschäftsfeld des Auftraggebers in Überschneidung steht. Eine Erwerbsersatzleistung wird nicht zugesagt.",
    note: "Wettbewerbsverbot ohne Kernbegriffe 'Wettbewerbsverbot'/'Konkurrenz': 'Beschäftigung oder Gelegenheit', 'Überschneidung', 'Erwerbsersatzleistung'.",
  },
  {
    id: "proc-split-sentence-04",
    detector: "non_compete",
    variant: "deceptive",
    technique: "split-sentence",
    severity: "hoch",
    text: "Die Zusammenarbeit endet mit dem Ablauf des Projekts. Der Auftragnehmer übt sodann für vierundzwanzig Monate keine Tätigkeit im Geschäftsbereich des Auftraggebers aus. Eine Entschädigung hierfür ist nicht vorgesehen.",
    note: "Das Verbot ergibt sich erst aus der Kombination von Projektende und Folgesatz; kein einzelner Satz verbindet Vertragsende und Tätigkeitsuntersagung.",
  },
  {
    id: "proc-cross-reference-04",
    detector: "non_compete",
    variant: "deceptive",
    technique: "cross-reference",
    severity: "mittel",
    text: "Auf die Nachwirkung dieses Vertrags findet Ziffer 9 Anwendung. Ziffer 9 regelt, dass die schuldrechtlichen Bindungswirkungen im Sinne der üblichen Vertragsabwicklung nach Ende der Laufzeit weitergelten.",
    note: "Nachwirkung nur im Querverweis: 'Bindungswirkungen' und 'Nachwirkung' deuten das Verbot an, ohne wettbewerbsrechtliche Begriffe zu verwenden.",
  },
  {
    id: "proc-dach-02",
    detector: "non_compete",
    variant: "deceptive",
    technique: "dach",
    severity: "hoch",
    text: "Der Handelsvertreter ist nach Beendigung des Agenturverhältnisses für ein Jahr in seinem Tätigkeitsgebiet untätig zu bleiben. Eine Entschädigung im Sinne von Artikel 418 des Schweizer Obligationenrechts wird ausdrücklich nicht zugesprochen.",
    note: "Schweizer Regelungslogik (Art. 418 OR anstelle § 74 HGB) mit ausdrücklichem Ausschluss der Entschädigung.",
  },

  // ==========================================================================
  // non_solicitation (§ 307 BGB, § 75f HGB)
  // ==========================================================================
  {
    id: "proc-euphemism-09",
    detector: "non_solicitation",
    variant: "deceptive",
    technique: "euphemism",
    severity: "hoch",
    text: "Die Parteien behandeln den Personalstamm der Gegenseite als vertrauensbildende Ressource. Der Kunde verzichtet darauf, Beschäftigte des Anbieters abzuwerben, aktiv anzusprechen oder in ein Beschäftigungsverhältnis zu überführen; für jeden Fall des Zuwiderhandelns wird eine Ausgleichszahlung von zwölf Monatsgehältern fällig.",
    note: "Abwerbeverbot mit Vertragsstrafenwirkung, formuliert als 'vertrauensbildende Ressource' und 'Ausgleichszahlung' - die Strafe kaschiert die Kundenbindung.",
  },
  {
    id: "proc-synonym-07",
    detector: "non_solicitation",
    variant: "deceptive",
    technique: "synonym",
    severity: "mittel",
    text: "Der Auftraggeber unterlässt während der Vertragslaufzeit und für sechs Monate danach jegliche Personalgewinnung aus den Reihen des Auftragnehmers, insbesondere mittels Direktansprache über soziale Netzwerke.",
    note: "Abwerbeverbot als 'Personalgewinnung' und 'Direktansprache' umschrieben; 'mitarbeiter'/'abzuwerben' fehlen, sodass die Muster nicht greifen.",
  },
  {
    id: "proc-typo-evasion-03",
    detector: "non_solicitation",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "mittel",
    text: "Es besteht ein Ab\u200bwerbe\u00adverbot hinsichtlich der Mit\u00adarbeiter des Anbieters; ein Ein\u00adstellungs\u00adverbot gilt für zwölf Monate nach Vertragsende fort.",
    note: "Soft-Hyphens und Zero-Width-Space zerlegen 'Abwerbeverbot', 'Mitarbeiter' und 'Einstellungsverbot' und brechen die Wortmuster.",
  },
  {
    id: "proc-english-03",
    detector: "non_solicitation",
    variant: "deceptive",
    technique: "english",
    severity: "mittel",
    text: "During the Term and for eighteen months thereafter, the Customer shall not solicit the Provider's personnel for engagement or hire, whether directly or through a third party, and a penalty of EUR 50,000 applies per breach.",
    note: "Englische Abwerbeklausel mit Pönale; die deutschen Muster ('mitarbeiter', 'abzuwerben', 'abwerbeverbot') können den Sachverhalt nicht erfassen.",
  },

  // ==========================================================================
  // Compliant baselines
  // ==========================================================================
  {
    id: "proc-compliant-01",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "jurisdiction",
    text: "Für alle Streitigkeiten aus oder im Zusammenhang mit diesem Vertrag gilt der gesetzliche Gerichtsstand nach den Vorschriften der Zivilprozessordnung. Vor Einleitung eines Klageverfahrens verpflichten sich die Parteien zu einem außergerichtlichen Klärungsversuch.",
    note: "Verweist schlicht auf §§ 12 ff. ZPO und lässt die gesetzliche Gerichtsstandsregelung unangetastet.",
  },
  {
    id: "proc-compliant-02",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "written_form_trap",
    text: "Kündigungen und Vertragsänderungen bedürfen der Textform (§ 126b BGB, z. B. E-Mail). Mündliche Individualabreden und Nebenabreden haben stets Vorrang vor diesen Bedingungen.",
    note: "Lässt die gesetzlich zulässige Textform zu und wahrt den Vorrang der Individualabrede nach § 305b BGB.",
  },
  {
    id: "proc-compliant-03",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "deemed_acceptance",
    text: "Der Kunde prüft die Arbeitsergebnisse nach Übergabe und zeigt Mängel innerhalb von dreißig Kalendertagen unter Angabe der festgestellten Abweichungen an. Der Anbieter weist den Kunden bei Fristbeginn ausdrücklich auf die Rechtsfolgen einer unterlassenen Anzeige hin. Ungeachtet des Fristablaufs bleiben die gesetzlichen Mängelrechte für verdeckte Mängel unberührt.",
    note: "Enthält Belehrung über die Fiktionsfolgen (§ 640 Abs. 2 S. 2 BGB) und die gesetzlich geforderte angemessene Frist.",
  },
  {
    id: "proc-compliant-04",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "non_compete",
    text: "Der Freelancer verpflichtet sich, für sechs Monate nach Projektende nicht für direkte Wettbewerber des Auftraggebers im selben Produktsegment tätig zu werden. Für diese Zeit zahlt der Auftraggeber eine Karenzentschädigung von 60 Prozent des durchschnittlichen Monatshonorars der letzten sechs Monate; die Vertragsparteien haben die Entschädigung nach § 74 Abs. 2 HGB ausdrücklich vereinbart.",
    note: "Sachlich, zeitlich und räumlich eng begrenztes Verbot mit überobligatorischer Karenzentschädigung, damit rechtlich zulässig.",
  },
];

export default CASES;
