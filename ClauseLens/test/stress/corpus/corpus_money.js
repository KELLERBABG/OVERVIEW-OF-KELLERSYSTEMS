/**
 * Stress-Korpus: MONEY & NUMBERS
 * ==============================
 *
 * Detektoren dieses Slices: default_interest, price_escalation,
 * setoff_restriction, tax_indemnity_shift.
 *
 * Angriffsschwerpunkt: Zinssaetze, Margen und Preisanpassungen werden so
 * formuliert, dass die ziffernorientierten Regexe der Engine keine Zahl mehr
 * finden (Basiszpunkte unter 900, ausgeschriebene Zahlen, Monatsraten,
 * Fullwidth-Ziffern, englische Formulierungen, Nominalstil, Verweise,
 * Satzteilung, Verstecken in Aufzaehlungen).
 */

export const CASES = [
  // ---------------------------------------------------------------------
  // default_interest
  // ---------------------------------------------------------------------
  {
    id: "money-direct-01",
    detector: "default_interest",
    variant: "deceptive",
    technique: "direct",
    severity: "hoch",
    text: "Bei Zahlungsverzug werden Verzugszinsen in Höhe von 12 Prozentpunkten über dem Basiszinssatz berechnet.",
    note: "Kontrollfall: unverschleierte Verzugszinsklausel, muss sicher erkannt werden.",
  },
  {
    id: "money-word-number-01",
    detector: "default_interest",
    variant: "deceptive",
    technique: "word-number",
    severity: "hoch",
    text: "Gerät der Kunde in Zahlungsverzug, schuldet er Verzugszinsen in Höhe von zwoelf Prozentpunkten über dem jeweiligen Basiszinssatz.",
    note: "Evasionsluecke: Der Zinssatz ist ausgeschrieben. Der Regex fuer Prozentwerte erwartet Ziffern, daher bleibt die Marge von zwoelf Punkten ueber dem Basiszinssatz unerkannt.",
  },
  {
    id: "money-word-number-02",
    detector: "default_interest",
    variant: "deceptive",
    technique: "word-number",
    severity: "hoch",
    text: "Die Verzinsung sämtlicher offener Rechnungspositionen erfolgt mit achtzehn Prozent pro Jahr ab Fälligkeit.",
    note: "Evasionsluecke: Weder das Wort Verzugszinsen noch eine Ziffer kommen vor. Eine Nominalumschreibung der Verzugszinsregel mit ausgeschriebener Jahresrate umgeht alle Muster des Detektors.",
  },
  {
    id: "money-word-number-03",
    detector: "default_interest",
    variant: "deceptive",
    technique: "word-number",
    severity: "mittel",
    text: "Der Anbieter berechnet bei verspäteter Zahlung eine Zinsmarge von 850 Basispunkten pro Jahr auf den offenen Saldo.",
    note: "Evasionsluecke: 850 Basispunkte entsprechen 8,5 % p. a. und liegen damit knapp unter der 900-Basispunkte-Schwelle; die Prozentwertpruefung greift anschliessend ins Leere, weil keine Prozentangabe folgt.",
  },
  {
    id: "money-typo-evasion-01",
    detector: "default_interest",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Bei Zahlungsverzug werden Verzugszinsen in Höhe von \uff11\uff12 Prozent berechnet.",
    note: "Evasionsluecke: Vollbreite Ziffern (１２) sind keine ASCII-Ziffern. Der Zinssatz von 12 % passiert den Zahlenregex ungehindert, obwohl die Klausel klar ueberhoeht ist.",
  },
  {
    id: "money-euphemism-01",
    detector: "default_interest",
    variant: "deceptive",
    technique: "euphemism",
    severity: "hoch",
    text: "Bei verspäteter Zahlung stellt der Anbieter dem Kunden eine monatliche Bearbeitungspauschale von 75 EUR sowie eine Verwaltungsgebühr von 2,5 % des offenen Saldos in Rechnung, ohne dass hierdurch Verzugszinsen im Sinne des § 288 BGB begründet werden.",
    note: "Evasionsluecke: Die Gebuehr wird nicht als pro Monat erkennbar umschrieben, weil monatlich als Adjektiv vor der Pauschale steht. Der Prozentwert 2,5 % wird deshalb nur als niedrig eingestuft, obwohl er wirtschaftlich 30 % p. a. plus Pauschale bedeutet.",
  },
  {
    id: "money-nominal-style-01",
    detector: "default_interest",
    variant: "deceptive",
    technique: "nominal-style",
    severity: "hoch",
    text: "Die Verzinsung überfälliger Entgelte richtet sich nach einem Satz von 1,75 % monatlich auf den jeweiligen Saldo.",
    note: "Evasionsluecke: Verzinsung statt Verzugszinsen und monatlich nachgestellt statt vorangestellt. Die Monatsrate von 21 % p. a. wird weder als Verzugszins noch als kurzfristige Periode erkannt.",
  },
  {
    id: "money-typo-evasion-02",
    detector: "default_interest",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Bei Zahlungsverzug werden Verzugszinsen in Höhe von 14\u00a0%\u00a0je angefangenem Monat auf den offenen Saldo berechnet.",
    note: "Kontrollfall typographische Evasion: Geschuetzte Leerzeichen zwischen Zahl und Einheit werden von der Normalisierung entfernt, die 14 % je angefangenem Monat muessen erkannt werden.",
  },

  // ---------------------------------------------------------------------
  // price_escalation
  // ---------------------------------------------------------------------
  {
    id: "money-direct-02",
    detector: "price_escalation",
    variant: "deceptive",
    technique: "direct",
    severity: "hoch",
    text: "Der Anbieter behält sich vor, die Preise für die bereitgestellten Dienste nach billigem Ermessen anzupassen; ein Sonderkündigungsrecht besteht nicht.",
    note: "Kontrollfall: klassische einseitige Preisanpassung nach billigem Ermessen ohne Sonderkündigungsrecht, muss scharf erkannt werden.",
  },
  {
    id: "money-word-number-05",
    detector: "price_escalation",
    variant: "deceptive",
    technique: "word-number",
    severity: "hoch",
    text: "Der Grundpreis erhöht sich mit Ablauf des zwölften Monats um acht Prozent jährlich.",
    note: "Evasionsluecke: Die automatische Eskalation ist ausgeschrieben (acht Prozent) und wird nicht mit um bis zu eingeleitet. Eine jaehrliche Einseitigkeitserhoehung ohne Kuendigungsrecht bleibt damit unentdeckt.",
  },
  {
    id: "money-nominal-style-02",
    detector: "price_escalation",
    variant: "deceptive",
    technique: "nominal-style",
    severity: "hoch",
    text: "Eine Anpassung der Entgeltbestandteile erfolgt jeweils zum 1. Januar auf Basis der Verbraucherpreisindex-Reihe des Statistischen Bundesamtes; Kostensenkungen werden nicht berücksichtigt.",
    note: "Evasionsluecke: Entgeltbestandteile statt Entgelte, Anpassung statt Preisanpassung. Die Indexfalle ohne Weitergabe von Kostensenkungen ist begrifflich vollstaendig umschrieben und trifft kein Muster.",
  },
  {
    id: "money-split-sentence-01",
    detector: "price_escalation",
    variant: "deceptive",
    technique: "split-sentence",
    severity: "hoch",
    text: "Der Anbieter passt die Entgelte jährlich an. Maßstab hierfür ist der vom Statistischen Bundesamt veröffentlichte Verbraucherpreisindex; ein Sonderkündigungsrecht wird nicht eingeräumt.",
    note: "Evasionsluecke: Die Muster erwarten den Infinitiv anzupassen, die Klausel verwendet die konjugierte Form passt an. Zusaetzlich ist der Sonderkündigungsausschluss vom Anpassungssatz getrennt.",
  },
  {
    id: "money-euphemism-02",
    detector: "price_escalation",
    variant: "deceptive",
    technique: "euphemism",
    severity: "mittel",
    text: "Der Anbieter behält sich eine marktgerechte Fortschreibung der Grundgebühr vor; die Vergütung wird hierbei an die Entwicklung der Personal- und Energiekosten angelehnt.",
    note: "Evasionsluecke: Fortschreibung und angelehnt ersetzen Anpassung und Erhoehung. Nach dem Wort vor endet der Satzteil, sodass die Preiskomponente ausserhalb des Trefferfensters liegt.",
  },
  {
    id: "money-english-01",
    detector: "price_escalation",
    variant: "deceptive",
    technique: "english",
    severity: "mittel",
    text: "The provider may adjust the subscription fees annually to reflect changes in its cost base, without any right of termination for the customer.",
    note: "Evasionsluecke: Fuer Preisanpassungen existiert kein englisches Muster. Eine einseitige Gebuehrenerhoehung ohne Kuendigungsrecht in englischer Vertragssprache bleibt folgenlos.",
  },
  {
    id: "money-compound-multi-01",
    detector: "price_escalation",
    variant: "deceptive",
    technique: "compound-multi",
    severity: "hoch",
    text: "Die Grundgebühr erhöht sich jeweils zum Jahreswechsel um bis zu acht Prozent, mindestens jedoch um die Inflationsrate; bei einer Erhöhung über drei Prozent steht dem Kunden kein Sonderkündigungsrecht zu.",
    note: "Evasionsluecke: Die Alternation des Eskalationsmusters kennt nur Ziffern sowie zehn und fünfzehn. acht Prozent faellt heraus, und der ausgeschlossene Sonderkündigungsfall haengt an einer anderen Satzhälfte.",
  },

  // ---------------------------------------------------------------------
  // setoff_restriction
  // ---------------------------------------------------------------------
  {
    id: "money-direct-03",
    detector: "setoff_restriction",
    variant: "deceptive",
    technique: "direct",
    severity: "hoch",
    text: "Die Aufrechnung mit Gegenforderungen des Kunden ist vollumfänglich ausgeschlossen; ein Zurückbehaltungsrecht steht dem Kunden nicht zu.",
    note: "Kontrollfall: pauschales Aufrechnungsverbot kombiniert mit Ausschluss des Zurückbehaltungsrechts, muss als hoch erkannt werden.",
  },
  {
    id: "money-euphemism-03",
    detector: "setoff_restriction",
    variant: "deceptive",
    technique: "euphemism",
    severity: "hoch",
    text: "Fällige Entgelte sind vom Kunden vollständig zu begleichen; Einwendungen und Gegenansprüche bleiben bei der Abrechnung außer Betracht.",
    note: "Evasionsluecke: Die Verrechnungssperre wird als Außerbetrachtlassen beschrieben. Kein Muster kennt diese Umschreibung, obwohl sie das Aufrechnungsrecht vollstaendig entwertet.",
  },
  {
    id: "money-nominal-style-03",
    detector: "setoff_restriction",
    variant: "deceptive",
    technique: "nominal-style",
    severity: "hoch",
    text: "Bei der Verrechnung von Gegenansprüchen mit fälligen Entgelten ist die vorherige Zustimmung des Anbieters erforderlich.",
    note: "Evasionsluecke: Das Muster fuer Verrechnung erwartet eine Negation wie ausgeschlossen oder nicht gestattet. Die Zustimmungspflicht ist eine faktische Sperre, die sprachlich nicht bedient wird.",
  },
  {
    id: "money-synonym-01",
    detector: "setoff_restriction",
    variant: "deceptive",
    technique: "synonym",
    severity: "hoch",
    text: "Der Kunde verpflichtet sich, sämtliche Zahlungen ohne Abzug auf das benannte Konto zu leisten; die Geltendmachung von Einreden gegen die Hauptforderung ist ausgeklammert.",
    note: "Evasionsluecke: Ohne Abzug statt ohne jeden Abzug oder Einbehalt, Einreden statt Zurückbehaltungsrecht. Die beiden synonymen Wendungen entfernen die Klausel aus allen Mustern.",
  },
  {
    id: "money-cross-reference-01",
    detector: "setoff_restriction",
    variant: "deceptive",
    technique: "cross-reference",
    severity: "hoch",
    text: "Für die Geltendmachung von Gegenrechten, Einreden und Zurückbehaltungsansprüchen gilt § 7 Abs. 4 dieses Vertrages.",
    note: "Evasionsluecke: Die eigentliche Sperre liegt in der in Bezug genommenen Ziffer. Die Verweisklausel selbst enthaelt keine Negation und wird daher nicht erfasst.",
  },
  {
    id: "money-english-02",
    detector: "setoff_restriction",
    variant: "deceptive",
    technique: "english",
    severity: "hoch",
    text: "All invoices shall be settled in full, free of any counterclaim, deduction, retention or set-off right.",
    note: "Evasionsluecke: Das englische Muster verlangt without any set-off oder no right of set-off. Die Formulierung free of any counterclaim umgeht beide Varianten.",
  },
  {
    id: "money-synonym-02",
    detector: "setoff_restriction",
    variant: "deceptive",
    technique: "synonym",
    severity: "hoch",
    text: "Bei Zahlungsverzug ruhen die Nutzungsrechte des Kunden mit sofortiger Wirkung, während die Vergütungspflicht unverändert fortbesteht.",
    note: "Evasionsluecke: Der Kill-Switch ist als ruhen die Nutzungsrechte formuliert. Die Muster verlangen ruhen der Nutzungsrechte oder ruhen saemtliche Berechtigungen, sodass der Genuswechsel den Treffer verhindert.",
  },

  // ---------------------------------------------------------------------
  // tax_indemnity_shift
  // ---------------------------------------------------------------------
  {
    id: "money-direct-04",
    detector: "tax_indemnity_shift",
    variant: "deceptive",
    technique: "direct",
    severity: "mittel",
    text: "Sämtliche Umsatzsteuern sowie steuerliche Nachforderungen aus diesem Vertragsverhältnis sind vom Kunden nachzuentrichten und zu tragen.",
    note: "Kontrollfall: unverhohlene Abwaelzung von Umsatz- und Nachforderungsbetraegen auf den Kunden, muss als mittel erkannt werden.",
  },
  {
    id: "money-synonym-03",
    detector: "tax_indemnity_shift",
    variant: "deceptive",
    technique: "synonym",
    severity: "mittel",
    text: "Nachträgliche steuerliche Mehrbelastungen des Anbieters, insbesondere aus einer geänderten umsatzsteuerlichen Einordnung, werden vollumfänglich dem Kunden belastet, unabhängig davon, ob den Kunden ein Verschulden trifft.",
    note: "Evasionsluecke: Mehrbelastungen statt Nachforderungen und belastet statt zu tragen. Die rueckwirkende verschuldensunabhaengige Abwaelzung trifft kein Muster.",
  },
  {
    id: "money-euphemism-04",
    detector: "tax_indemnity_shift",
    variant: "deceptive",
    technique: "euphemism",
    severity: "mittel",
    text: "Ergeben sich aus der steuerlichen Würdigung des Vertragsverhältnisses nachträgliche Abgabenbelastungen, trägt diese der Kunde.",
    note: "Evasionsluecke: Abgabenbelastungen ist kein Musterbegriff. Die Klausel verlagert das steuerliche Bewertungsrisiko vollstaendig, bleibt aber sprachlich unauffaellig.",
  },
  {
    id: "money-cross-reference-02",
    detector: "tax_indemnity_shift",
    variant: "deceptive",
    technique: "cross-reference",
    severity: "mittel",
    text: "Die Verteilung steuerlicher Nachbelastungen, Säumniszuschläge und Haftungsbeträge richtet sich nach Ziffer 12.3 dieses Vertrages.",
    note: "Evasionsluecke: Nachbelastungen statt Nachforderungen. Der eigentliche Abwaelzungstatbestand liegt in der in Bezug genommenen Ziffer und ist fuer die Satzanalyse unsichtbar.",
  },
  {
    id: "money-english-03",
    detector: "tax_indemnity_shift",
    variant: "deceptive",
    technique: "english",
    severity: "mittel",
    text: "The customer shall bear any withholding, transfer or turnover taxes and any related interest, penalties or retrospective assessments arising out of this agreement.",
    note: "Evasionsluecke: Fuer steuerliche Abwaelzungen existiert kein englisches Muster. Die Klausel verlagert auch rueckwirkende Steuerfestsetzungen auf den Kunden.",
  },
  {
    id: "money-list-buried-01",
    detector: "tax_indemnity_shift",
    variant: "deceptive",
    technique: "list-buried",
    severity: "mittel",
    text: "Der Kunde stellt den Anbieter von sämtlichen steuerlichen Nachforderungen, Säumniszuschlägen und Haftungsbeträgen frei, die von Finanzbehörden im Zusammenhang mit diesem Vertragsverhältnis geltend gemacht werden.",
    note: "Teilblindspot: Die Steuerabwaelzung selbst wird vom Steuerdetektor nicht erfasst. Nur der allgemeine Freistellungsdetektor schlaegt an und uebersetzt die Klausel als pauschale Haftungsverlagerung.",
  },

  // ---------------------------------------------------------------------
  // Compliant baselines (False-Positive-Resistenz)
  // ---------------------------------------------------------------------
  {
    id: "money-compliant-01",
    detector: null,
    nearestDetector: "default_interest",
    variant: "compliant",
    technique: "direct",
    severity: null,
    text: "Bei Zahlungsverzug gelten die gesetzlichen Verzugszinsen nach § 288 BGB; weitergehende Pauschalen oder Mahngebühren werden nicht erhoben.",
    note: "Gesetzeskonform: Verweis auf die gesetzlichen Verzugszinsen ohne Aufschlag und ohne eigene Gebuehren.",
  },
  {
    id: "money-compliant-02",
    detector: null,
    nearestDetector: "price_escalation",
    variant: "compliant",
    technique: "direct",
    severity: null,
    text: "Die Vergütung wird jährlich auf Basis der vom Statistischen Bundesamt veröffentlichten Inflationsrate angepasst. Übersteigt die Anpassung 3 %, kann jede Partei den Vertrag mit Wirkung zum Anpassungszeitpunkt kündigen. Kostensenkungen werden in gleicher Weise berücksichtigt.",
    note: "Gesetzeskonform: transparente Indexkopplung, beidseitige Kostensenkung und Sonderkündigungsrecht. Darf keinen Befund ausloesen.",
  },
  {
    id: "money-compliant-03",
    detector: null,
    nearestDetector: "setoff_restriction",
    variant: "compliant",
    technique: "direct",
    severity: null,
    text: "Der Kunde ist berechtigt, mit unbestrittenen oder rechtskräftig festgestellten Gegenforderungen gegen fällige Entgelte aufzurechnen. Das Zurückbehaltungsrecht nach § 320 BGB sowie das Recht zur außerordentlichen Kündigung aus wichtigem Grund bleiben unberührt.",
    note: "Gesetzeskonform: Aufrechnung mit unbestrittenen oder rechtskraeftigen Forderungen bleibt zulaessig, gesetzliche Rechte bleiben ausdruecklich bestehen.",
  },
  {
    id: "money-compliant-04",
    detector: null,
    nearestDetector: "tax_indemnity_shift",
    variant: "compliant",
    technique: "direct",
    severity: null,
    text: "Jede Partei trägt die sie betreffenden Steuern nach den gesetzlichen Vorschriften. Umsatzsteuer wird mit dem jeweils geltenden Steuersatz gesondert ausgewiesen und abgeführt. Nachforderungen werden nur bei schuldhafter Falschangabe des Kunden weitergegeben.",
    note: "Gesetzeskonform: Steuerlast folgt der gesetzlichen Zuordnung, Nachforderungen nur bei Verschulden des Kunden.",
  },
  {
    id: "money-compliant-05",
    detector: null,
    nearestDetector: "default_interest",
    variant: "compliant",
    technique: "direct",
    severity: null,
    text: "Gerät der Kunde in Verzug, schuldet er Verzugszinsen in Höhe von fünf Prozentpunkten über dem Basiszinssatz gemäß § 288 Abs. 2 BGB.",
    note: "Gesetzeskonform: exakt der gesetzliche Verzugszinssatz. Zeigt zugleich, dass die Engine zwischen gesetzeskonformer und missbraeuchlicher ausgeschriebener Zahl nicht unterscheiden kann.",
  },
];

export default CASES;
