"use strict";

/**
 * Stress-Korpus: Ingestion- und Encoding-Schäden.
 *
 * Jede Klausel ist im unbeschädigten Zustand eine klare Falle; der Schaden
 * entsteht erst durch Textgewinnung (PDF/DOCX-Parser) oder Zeichenkodierung.
 * normalizeText() entfernt nur Soft-Hyphens, Zero-Width-Zeichen und
 * geschützte Leerzeichen — alles andere erreicht Satzsegmentierung und Regexes.
 *
 * Alle unsichtbaren Zeichen sind als JS-Escape notiert, damit die Datei lesbar
 * bleibt: \u00ad Soft-Hyphen · \u200b Zero-Width-Space · \u00c3\u00bc Mojibake-ü
 * \ufb01 fi-Ligatur · \ufb02 fl-Ligatur · \uff11 Fullwidth-Digit · Homoglyphen.
 */

export const CASES = [
  // ---------------------------------------------------------------- 1) PDF-Zeilenumbruch-Hyphenation
  {
    id: "ingest-hyphenation-01",
    detector: "liability",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Die Haf-\ntung des Anbieters für sämtliche Schäden aus der Verletzung von Kardinalpflichten wird hiermit voll-\numfänglich ausgeschlossen.",
    note: "PDF-Zeilenumbruch trennt 'Haftung' (Haf-/tung) und 'vollumfänglich' (voll-/umfänglich). Unbeschädigt: 'Die Haftung ... wird vollumfänglich ausgeschlossen.' Das Triggerwort 'Haftung' existiert im Text nicht mehr, nur 'Haf-' + Zeilenumbruch + 'tung'.",
  },
  {
    id: "ingest-hyphenation-02",
    detector: "penalty",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Für jeden Kalendertag der Verspätung verfällt eine Ver-\ntragsstrafe, deren Höhe 2 % der Nettoauftragssumme beträgt.",
    note: "Silbentrennung am Zeilenende zerlegt 'Vertragsstrafe' in 'Ver-' + Umbruch + 'tragsstrafe'. Unbeschädigt: '... eine Vertragsstrafe, deren Höhe 2 % ... beträgt.' Auch die Alternative 'Strafe in Höhe von' greift hier bewusst nicht.",
  },
  {
    id: "ingest-hyphenation-03",
    detector: "jurisdiction",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Für sämtliche Streitigkeiten aus diesem Vertrag ist aus-\nschließlich der Gerichts-\nstand am Sitz des Anbieters vereinbart.",
    note: "Sowohl 'ausschließlich' (aus-/schließlich) als auch 'Gerichtsstand' (Gerichts-/stand) sind umbrochen. Unbeschädigt: '... ist ausschließlich der Gerichtsstand am Sitz des Anbieters vereinbart.' Damit fehlen beide Trigger der Forum-Klausel.",
  },

  // ---------------------------------------------------------------- 2) Extraktion ganz ohne Wortzwischenräume
  {
    id: "ingest-glued-01",
    detector: "statute_of_limitations",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "SämtlicheAnsprüchedesKundenwegenSachmängelnverjähreninnerhalbvonsechsMonatenabAblieferungderWare;diesgiltauchfürAnsprüchewegenVerletzungvonKardinalpflichten.",
    note: "PDF-Extraktion ohne Wortzwischenräume: der Fließtext ist vollständig verklebt. Unbeschädigt: 'Sämtliche Ansprüche des Kunden wegen Sachmängeln verjähren innerhalb von sechs Monaten ab Ablieferung der Ware.' Die auf \\s+ aufbauenden Fristmuster finden keine Wortgrenzen mehr.",
  },
  {
    id: "ingest-glued-02",
    detector: "deemed_acceptance",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "ErkennbareMängelsindbinneneinerAusschlussfristvon48StundennachAblieferungschriftlichzurügen;widrigenfallsgiltdieWarealsvorbehaltlosgenehmigt.",
    note: "Verklebte Extraktion der Rügeklausel. Unbeschädigt: 'Erkennbare Mängel sind binnen einer Ausschlussfrist von 48 Stunden nach Ablieferung schriftlich zu rügen; widrigenfalls gilt die Ware als vorbehaltlos genehmigt.' Das Semikolon trennt die Sätze wie gewohnt, aber 'Ausschlussfrist von 48 Stunden' und 'widrigenfalls gilt ... genehmigt' brauchen Leerzeichen.",
  },

  // ---------------------------------------------------------------- 3) OCR-Verwechslungen (1/l, 0/O, m/rn)
  {
    id: "ingest-ocr-01",
    detector: "default_interest",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Bei Zahlungsverzug berechnet der Anbieter Verzugszinsen in Höhe von l2 Prozentpunkten über dem Basiszinssatz.",
    note: "OCR liest '1' als 'l' ('l2' statt '12'). Unbeschädigt: '... Verzugszinsen in Höhe von 12 Prozentpunkten über dem Basiszinssatz.' Der Trigger 'Verzugszinsen' wird noch erkannt, die Prozentwert-Extraktion liefert aber keinen Ziffernwert mehr — die Schwere fällt auf null und es entsteht gar kein Befund.",
  },
  {
    id: "ingest-ocr-02",
    detector: "jurisdiction",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Ausschließlicher Gerichtsstand für alle Streitigkeiten aus diesem Vertrag ist Wi1mington, De1aware.",
    note: "OCR ersetzt 'l' durch '1' in 'Wilmington' und 'Delaware'. Unbeschädigt: '... ist Wilmington, Delaware.' Der ausschließliche Gerichtsstand wird erkannt, die Auslands-Erkennung (delaware/wilmington) schlägt jedoch fehl — die Klausel wird nur noch als 'mittel' eingestuft.",
  },
  {
    id: "ingest-ocr-03",
    detector: "written_form_trap",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Die Kündigung bedarf der Schriftforrn und ist ausschließlich an die im Handelsregister eingetragene Geschäftsanschrift zu richten.",
    note: "OCR-Verwechslung 'm' ↔ 'rn' in 'Schriftform' (Schriftforrn). Unbeschädigt: 'Die Kündigung bedarf der Schriftform ...'. Das Formerfordernis ist damit nicht mehr nachweisbar, obwohl es unverändert gilt; 'Einschreiben'/'Rückschein' fehlen als Ersatztrigger.",
  },

  // ---------------------------------------------------------------- 4) Umlaut-Transliteration aus Legacy-Kodierung
  {
    id: "ingest-transliteration-01",
    detector: "notice_period",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Der Vertrag kann von beiden Parteien mit einer Frist von zwoelf Monaten zum Quartalsende gekuendigt werden.",
    note: "Legacy-Kodierung transliteriert Umlaute: 'gekündigt' wird zu 'gekuendigt'. Unbeschädigt: '... mit einer Frist von zwölf Monaten zum Quartalsende gekündigt werden.' Der Fristen-Sammler der Kündigungsfrist-Regel verlangt das Wort 'kündig' und übersieht die 12-Monats-Bindung völlig.",
  },
  {
    id: "ingest-transliteration-02",
    detector: "statute_of_limitations",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Saemtliche Maengelansprueche des Kaeufers verjaehren innerhalb von sechs Monaten ab Ablieferung der Ware.",
    note: "Transliterierte Umlaute in allen Schlüsselwörtern ('Sämtliche' → 'Saemtliche', 'Mängelansprüche' → 'Maengelansprueche', 'verjähren' → 'verjaehren'). Unbeschädigt: 'Sämtliche Mängelansprüche ... verjähren innerhalb von sechs Monaten ...'. Die Verkürzung auf sechs Monate wird unsichtbar.",
  },
  {
    id: "ingest-transliteration-03",
    detector: "price_escalation",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "mittel",
    text: "Der Anbieter behaelt sich vor, die Preise fuer die Nutzung der Plattform jeweils zum Jahreswechsel einseitig zu erhoehen.",
    note: "Transliteration von 'behält'/'für'/'erhöhen' zu 'behaelt'/'fuer'/'erhoehen'. Unbeschädigt: 'Der Anbieter behält sich vor, die Preise für die Nutzung der Plattform jeweils zum Jahreswechsel einseitig zu erhöhen.' Beide Preiserhöhungs-Muster hängen an den Umlautformen ('behält sich vor', 'zu erhöhen').",
  },

  // ---------------------------------------------------------------- 5) Mojibake / falsche Zeichenkodierung
  {
    id: "ingest-mojibake-01",
    detector: "setoff_restriction",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Ein Zur\u00c3\u00bcckbehaltungsrecht des Kunden wegen M\u00c3\u00a4ngeln ist ausgeschlossen.",
    note: "UTF-8-Bytes als latin1 dekodiert: 'ü' wird zu '\u00c3\u00bc', 'ä' zu '\u00c3\u00a4'. Unbeschädigt: 'Ein Zurückbehaltungsrecht des Kunden wegen Mängeln ist ausgeschlossen.' Der Trigger 'Zurückbehaltungsrecht' ist zerstört, der Ausschluss des gesetzlichen Druckmittels bleibt unentdeckt.",
  },
  {
    id: "ingest-mojibake-02",
    detector: "deemed_acceptance",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "M\u00c3\u00a4ngelr\u00c3\u00bcgen sind sp\u00c3\u00a4testens innerhalb von sieben Werktagen nach Ablieferung schriftlich zu erheben; andernfalls verfallen s\u00c3\u00a4mtliche Anspr\u00c3\u00bcche ersatzlos.",
    note: "Mojibake in 'Mängelrügen', 'spätestens', 'sämtliche', 'Ansprüche'. Unbeschädigt: 'Mängelrügen sind spätestens innerhalb von sieben Werktagen ... zu erheben; andernfalls verfallen sämtliche Ansprüche ersatzlos.' Die 7-Tage-Falle mit Verfall aller Ansprüche wird komplett übersehen.",
  },

  // ---------------------------------------------------------------- 6) Ligaturen
  {
    id: "ingest-ligature-01",
    detector: "written_form_trap",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Kündigungen und sonstige Erklärungen bedürfen der Schr\ufb01ftform und sind an die im Handelsregister eingetragene Anschrift zu richten.",
    note: "Die fi-Ligatur ('\ufb01') ersetzt 'fi' in 'Schriftform'. Unbeschädigt: 'Kündigungen und sonstige Erklärungen bedürfen der Schriftform ...'. Damit bleibt die strenge Formklausel ohne Einschreiben/Rückschein und ohne Erkennung.",
  },
  {
    id: "ingest-ligature-02",
    detector: "audit_log_manipulation",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Die Logf\ufb01les des Anbieters gelten als unwiderlegbarer Vollbeweis für die Fehlerfreiheit der Systeme; ein Gegenbeweis des Kunden ist ausgeschlossen.",
    note: "fi-Ligatur in 'Logfiles'. Unbeschädigt: 'Die Logfiles des Anbieters gelten als unwiderlegbarer Vollbeweis ... ein Gegenbeweis des Kunden ist ausgeschlossen.' Das Beweismonopol bleibt unentdeckt, weil auch 'Protokolle des Anbieters' nicht wörtlich vorkommt.",
  },

  // ---------------------------------------------------------------- 7) NFD-dekomponierte Umlaute
  {
    id: "ingest-nfd-01",
    detector: "written_form_trap",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Ku\u0308ndigungen bedu\u0308rfen der Schriftform; die Textform ist ausgeschlossen.",
    note: "Unicode-NFD: Umlaut als Basisbuchstabe + Combining Diaeresis ('u' + '\\u0308'). Unbeschädigt: 'Kündigungen bedürfen der Schriftform; die Textform ist ausgeschlossen.' Weder 'kündigung' noch 'schriftform' (via Kündigung-Muster) sind als Zeichenfolge noch auffindbar.",
  },
  {
    id: "ingest-nfd-02",
    detector: "data_grant",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Der Kunde ra\u0308umt dem Anbieter ein zeitlich und inhaltlich uneingeschra\u0308nktes Nutzungsrecht an allen Übermittlungsdaten ein.",
    note: "NFD-Dekonstruktion von 'räumt' und 'uneingeschränktes'. Unbeschädigt: 'Der Kunde räumt dem Anbieter ein zeitlich und inhaltlich uneingeschränktes Nutzungsrecht an allen Übermittlungsdaten ein.' Die unbegrenzte Rechtseinräumung wirkt unverändert, ist aber nicht mehr matchbar.",
  },

  // ---------------------------------------------------------------- 8) Homoglyphen
  {
    id: "ingest-homoglyph-01",
    detector: "liability",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Die H\u0430ftung des Anbieters für sämtliche Schäden und Folgeschäden ist vollst\u0430ndig \u0430usgeschlossen.",
    note: "Kyrillisches 'а' (U+0430) ersetzt das lateinische 'a' in 'Haftung', 'vollständig' und 'ausgeschlossen'. Unbeschädigt: 'Die Haftung ... ist vollständig ausgeschlossen.' Visuell identisch, für beide Haftungsrichtung-Muster unauffindbar.",
  },
  {
    id: "ingest-homoglyph-02",
    detector: "penalty",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Bei Verzug verfällt eine Konventi\u03bfnalstrafe, deren Höhe sich auf 5.000 EUR je Woche beläuft.",
    note: "Griechisches Omikron (U+03BF) ersetzt das 'o' in 'Konventionalstrafe'. Unbeschädigt: 'Bei Verzug verfällt eine Konventionalstrafe, deren Höhe sich auf 5.000 EUR je Woche beläuft.' Die Vertragsstrafe wird weder über 'Konventionalstrafe' noch über 'Strafe in Höhe von' erkannt.",
  },

  // ---------------------------------------------------------------- 9) Fullwidth-Zeichen
  {
    id: "ingest-fullwidth-01",
    detector: "default_interest",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Verzugszinsen in Höhe von \uff11\uff12 Prozentpunkten über dem Basiszinssatz werden ab dem ersten Mahntag berechnet.",
    note: "Fullwidth-Ziffern (U+FF11/U+FF12) statt '12', wie sie aus asiatischen Satzsystemen oder PDF-Fonts mit falschem CMap entstehen. Unbeschädigt: '... in Höhe von 12 Prozentpunkten ...'. Der Trigger wird erkannt, die Prozentwert-Extraktion scheitert an \\d und liefert keinen Befund.",
  },
  {
    id: "ingest-fullwidth-02",
    detector: "jurisdiction",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Ausschließlicher \uff27erichtsstand für alle Streitigkeiten aus diesem Vertrag ist Wilmington, Delaware.",
    note: "Fullwidth-Latein 'Ｇ' statt 'G' im Wort 'Gerichtsstand'. Unbeschädigt: 'Ausschließlicher Gerichtsstand ... ist Wilmington, Delaware.' Beide Forum-Trigger ('ausschließlicher Gerichtsstand', 'Gerichtsstand ist ...') brechen.",
  },

  // ---------------------------------------------------------------- 10) Whitespace-Schäden
  {
    id: "ingest-whitespace-01",
    detector: "auto_renewal",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Der Vertrag ver\tlängert sich um jeweils weitere zwölf Monate, sofern er nicht mit einer Frist von drei Monaten gekündigt wird.",
    note: "Tabulator mitten im Wort 'verlängert' (aus Spalten-/Blocksatz-Extraktion). Unbeschädigt: 'Der Vertrag verlängert sich um jeweils weitere zwölf Monate ...'. Die Verlängerungsautomatik mit Zwölfmonatsbindung wird nicht erkannt; 'Verlängerung' ist als Substantiv nicht im Text.",
  },
  {
    id: "ingest-whitespace-02",
    detector: "burden_of_proof",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Die Beweis\tlast für sämtliche Mängel und Störungen obliegt dem Kunden.",
    note: "Tabulator zerlegt 'Beweislast'. Unbeschädigt: 'Die Beweislast für sämtliche Mängel und Störungen obliegt dem Kunden.' Die Beweislastumkehr zu Lasten des Kunden bleibt unentdeckt.",
  },

  // ---------------------------------------------------------------- 11) Spalten-/Lesereihenfolge
  {
    id: "ingest-column-01",
    detector: "liability",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Die Haftung des Anbieters\nDie Parteien benennen je einen Projektleiter.\nfür sämtliche Schäden, gleich aus welchem Rechtsgrund,\nDie Projektleiter stimmen sich wöchentlich ab.\nwird hiermit vollumfänglich ausgeschlossen.",
    note: "Zweispaltiges PDF, die Extraktion liest beide Spalten zeilenweise durchmischt. Unbeschädigt: 'Die Haftung des Anbieters für sämtliche Schäden, gleich aus welchem Rechtsgrund, wird hiermit vollumfänglich ausgeschlossen.' Subjekt und Rechtsfolge landen in getrennten Sätzen, die Fremdsätze enthalten keinen Haftungsbezug.",
  },

  // ---------------------------------------------------------------- 12) Tabellen-Abflachung
  {
    id: "ingest-table-01",
    detector: "default_interest",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Mahnstufe 1 | ab dem 1. Tag nach Fälligkeit | Zinssatz zwölf Prozent pro Jahr | Mahngebühr 15,00 EUR | Mahnstufe 2 | ab dem 30. Tag | Zinssatz fünfzehn Prozent pro Jahr | Mahngebühr 25,00 EUR | Mahnstufe 3 | ab dem 60. Tag | Zinssatz achtzehn Prozent pro Jahr | Mahngebühr 40,00 EUR",
    note: "Eine Mahnstufen-Tabelle wird zu einer einzigen Zeile mit Pipe-Trennern abgeflacht. Die Staffelung bis 18 % p.a. plus Mahngebühren bleibt wirtschaftlich identisch, aber die Wörter 'Verzugszinsen'/'Zahlungsverzug' sind nicht mehr im Text — nur noch 'Zinssatz'.",
  },

  // ---------------------------------------------------------------- 13) Fußnoten-Interleaving
  {
    id: "ingest-footnote-01",
    detector: "unilateral_change",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Soweit der Kunde auf eine Änderungsmitteilung nicht innerhalb von 30 Tagen reagiert, gilt[1] das Schweigen des Kunden als Zustimmung. [1] Der Zugriff auf das Kundenportal steht dem Schweigen gleich.",
    note: "Fußnotenmarker und Fußnotentext werden mitten in die Klausel interpoliert. Unbeschädigt: 'Soweit der Kunde ... nicht innerhalb von 30 Tagen reagiert, gilt das Schweigen des Kunden als Zustimmung.' Das starre Muster für die Zustimmungsfiktion durch Schweigen (BGH XI ZR 26/20) verlangt das Wort 'gilt' direkt vor 'das Schweigen'.",
  },

  // ---------------------------------------------------------------- 14) Seitenkopf/-fuß mitten im Satz
  {
    id: "ingest-pagebreak-01",
    detector: "liability",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Die Haftung des Anbieters für sämtliche Schäden\nSeite 7 von 24\nsowie für entgangenen Gewinn wird vollumfänglich ausgeschlossen.",
    note: "Seitenfuß 'Seite 7 von 24' wird mitten in die Haftungsklausel eingelesen. Unbeschädigt: 'Die Haftung des Anbieters für sämtliche Schäden sowie für entgangenen Gewinn wird vollumfänglich ausgeschlossen.' Die Rechtsfolge liegt in einer Zeile hinter dem Fuß, alle Fenstermuster ([^.!?;:\\n]{0,N}) enden am Zeilenumbruch.",
  },
  {
    id: "ingest-pagebreak-02",
    detector: "jurisdiction",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Ausschließlicher Gerichtsstand für alle Streitigkeiten aus diesem Vertrag\nVertraulich – Nur für den internen Gebrauch\nist Wilmington, Delaware (USA).",
    note: "Kopfzeile wird in die Forum-Klausel eingelesen. Unbeschädigt: 'Ausschließlicher Gerichtsstand ... ist Wilmington, Delaware (USA).' Kontrollfall: Hier bleibt die Erkennung intakt, weil Trigger und Auslandsbezug (Delaware/USA) in derselben Zeile wie die Rechtsfolge stehen.",
  },

  // ---------------------------------------------------------------- 15) Zahlen-Mangling
  {
    id: "ingest-number-01",
    detector: "penalty",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Bei Verzug wird eine Vertragsstrafe in Höhe von 10 000 EUR je Woche fällig; die Höhe wird pauschal ohne Rücksicht auf den tatsächlichen Schaden festgesetzt.",
    note: "Tausendertrennzeichen als Leerzeichen ('10 000 EUR'). Die Pauschale ohne Gegenbeweiszulassung bleibt erkennbar; Kontrollfall für die Penalty-Erkennung mit zerstörtem Zahlenformat, damit die Verwechslung von Zahlenformat und Trigger nicht als Blindspot fehlgedeutet wird.",
  },
  {
    id: "ingest-number-02",
    detector: "default_interest",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Bei Zahlungsverzug werden Verzugszinsen in Höhe von 12 Punkten über dem Basiszinssatz berechnet; zusätzlich wird eine Mahnpauschale von 15,00 EUR je Mahnung fällig.",
    note: "Das Prozentzeichen bzw. 'Prozentpunkte' ist verlorengegangen (Extraktion schluckt das Symbol), es bleibt nur '12 Punkten über dem Basiszinssatz'. Unbeschädigt: '... in Höhe von 12 Prozentpunkten über dem Basiszinssatz ...'. Die Prozentwert-Extraktion findet keine Einheit mehr und erzeugt keinen Befund.",
  },

  // ---------------------------------------------------------------- Kontrollfälle (unbeschädigt)
  {
    id: "ingest-control-01",
    detector: "liability",
    variant: "deceptive",
    technique: "direct",
    severity: "hoch",
    text: "Die Haftung des Anbieters für sämtliche Schäden, gleich aus welchem Rechtsgrund, wird vollumfänglich ausgeschlossen, soweit gesetzlich zulässig.",
    note: "Kontrollfall ohne Ingestion-Schaden: identische Falle wie ingest-hyphenation-01 und ingest-homoglyph-01 in sauberem Text. Dient als Delta-Referenz für die Schadensmodi.",
  },
  {
    id: "ingest-control-02",
    detector: "jurisdiction",
    variant: "deceptive",
    technique: "direct",
    severity: "hoch",
    text: "Ausschließlicher Gerichtsstand für alle Streitigkeiten aus diesem Vertrag ist Wilmington, Delaware (USA).",
    note: "Kontrollfall ohne Ingestion-Schaden: identische Falle wie ingest-ocr-02, ingest-fullwidth-02 und ingest-hyphenation-03 in sauberem Text.",
  },
  {
    id: "ingest-control-03",
    detector: "statute_of_limitations",
    variant: "deceptive",
    technique: "direct",
    severity: "hoch",
    text: "Sämtliche Ansprüche des Kunden wegen Sachmängeln verjähren innerhalb von sechs Monaten ab Ablieferung der Ware.",
    note: "Kontrollfall ohne Ingestion-Schaden: identische Falle wie ingest-transliteration-02 und ingest-glued-01 in sauberem Text.",
  },
  {
    id: "ingest-control-04",
    detector: "deemed_acceptance",
    variant: "deceptive",
    technique: "direct",
    severity: "hoch",
    text: "Erkennbare Mängel sind binnen einer Ausschlussfrist von 48 Stunden nach Ablieferung schriftlich zu rügen; widrigenfalls gilt die Ware als vorbehaltlos abgenommen.",
    note: "Kontrollfall ohne Ingestion-Schaden: identische Falle wie ingest-glued-02 und ingest-mojibake-02 in sauberem Text.",
  },

  // ---------------------------------------------------------------- Compliant-Baselines (inkl. Schaden auf harmloser Klausel)
  {
    id: "ingest-compliant-01",
    variant: "compliant",
    detector: null,
    nearestDetector: "liability",
    technique: "direct",
    severity: null,
    text: "Der Anbieter haftet unbeschränkt für Schäden aus der Verletzung des Lebens, des Körpers oder der Gesundheit sowie für vorsätzlich oder grob fahrlässig verursachte Schäden. Bei einfacher Fahrlässigkeit haftet der Anbieter nur bei Verletzung wesentlicher Vertragspflichten (Kardinalpflichten), begrenzt auf den vertragstypischen, vorhersehbaren Schaden.",
    note: "Harmlos: vollständiger gesetzlicher Haftungskatalog nach § 309 Nr. 7 BGB mit Kardinalpflichten-Klarstellung. Eine Einstufung als mittel/hoch wäre ein False Positive.",
  },
  {
    id: "ingest-compliant-02",
    variant: "compliant",
    detector: null,
    nearestDetector: "jurisdiction",
    technique: "direct",
    severity: null,
    text: "Für alle Streitigkeiten aus oder im Zusammenhang mit diesem Vertrag gilt der gesetzliche Gerichtsstand nach den Vorschriften der Zivilprozessordnung.",
    note: "Harmlos: Verweis auf den gesetzlichen Gerichtsstand (§§ 12 ff. ZPO), kein Forum beim Anbieter und kein Auslandsbezug.",
  },
  {
    id: "ingest-compliant-03",
    variant: "compliant",
    detector: null,
    nearestDetector: "notice_period",
    technique: "direct",
    severity: null,
    text: "Jede Partei kann diesen Vertrag mit einer Frist von einem Monat zum Monatsende in Textform (z. B. per E-Mail) kündigen. Das Recht beider Parteien zur außerordentlichen Kündigung aus wichtigem Grund bleibt unberührt.",
    note: "Harmlos: symmetrische Kündigungsfrist von einem Monat, Textform genügt, außerordentliche Kündigung ausdrücklich erhalten.",
  },
  {
    id: "ingest-compliant-04",
    variant: "compliant",
    detector: null,
    nearestDetector: "written_form_trap",
    technique: "typo-evasion",
    severity: null,
    text: "K\u00c3\u00bcndigungen bedürfen der Textform (z. B. per E-Mail); das Recht beider Parteien zur außerordentlichen Kündigung aus wichtigem Grund bleibt unberührt.",
    note: "Harmlose Klausel (Textform genügt, außerordentliche Kündigung erhalten), auf die ein Schaden angewendet wurde: Mojibake in 'Kündigungen'. Prüft, ob Ingestion-Schäden allein False Positives erzeugen — hier darf kein mittel/hoch-Befund entstehen.",
  },
];

export default CASES;
