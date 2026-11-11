"use strict";

/**
 * Stress-Korpus — Slice 6/8
 * =========================
 *
 * PART A — eigene Detektoren:
 *   as_is_software_disclaimer, entire_agreement, salvatorische_klausel_falle,
 *   unverbindliche_sla_falle, dynamic_url_referral
 *
 * PART B — "evade": typografische Evasion gegen die Regex-Engine fuer Detektoren
 *   anderer Slices (liability, penalty, jurisdiction, data_grant,
 *   setoff_restriction, deemed_acceptance).
 *
 * Schemavetrag: siehe test/stress/harness.js (Export `CASES` / Fall-Schema).
 * Unsichtbare Zeichen sind bewusst als \u-Escapes notiert, damit die Datei lesbar bleibt.
 * \u00ad (Soft-Hyphen), \u200b (Zero-Width Space) und \u00a0 (NBSP) werden von
 * normalizeText entfernt bzw. zu Space normalisiert; Homoglyphe (Kyrillisch/Griechisch),
 * Fullwidth-Zeichen, Letter-Spacing und Zeilenumbrueche innerhalb eines Kompositums
 * ueberleben die Normalisierung und koennen Regex-Trigger zerreissen.
 */

export const CASES = [
  // ---------------------------------------------------------------------------
  // PART A.1 — as_is_software_disclaimer
  // ---------------------------------------------------------------------------
  {
    id: "misc-direct-01",
    detector: "as_is_software_disclaimer",
    variant: "deceptive",
    technique: "direct",
    severity: "hoch",
    text: "Die Software wird wie gesehen bereitgestellt; die Sachmängelhaftung ist vollständig ausgeschlossen.",
    note: "Kontrollfall: klassischer As-Is-Disclaimer. Erwartung: Regex greift (Trigger 'wie gesehen' + 'wird ... bereitgestellt').",
  },
  {
    id: "misc-euphemism-01",
    detector: "as_is_software_disclaimer",
    variant: "deceptive",
    technique: "euphemism",
    severity: "hoch",
    text: "Die Überlassung der Software erfolgt ohne Zusicherung einer bestimmten Beschaffenheit oder Eignung für einen konkreten Einsatzzweck des Kunden.",
    note: "As-Is-Falle in Beschwichtigungsdeutsch: 'ohne Zusicherung einer bestimmten Beschaffenheit' statt 'wie gesehen'/'ohne Mängelgewähr'. Evasion: Triggerwort fehlt vollständig.",
  },
  {
    id: "misc-split-sentence-01",
    detector: "as_is_software_disclaimer",
    variant: "deceptive",
    technique: "split-sentence",
    severity: "hoch",
    text: "Die Bereitstellung der Anwendung erfolgt im Ist-Zustand. Ein Anspruch auf Beseitigung von Funktionsstörungen oder auf Anpassung an einen bestimmten Einsatzzweck besteht nicht.",
    note: "Gewährleistungsausschluss über zwei Sätze verteilt und mit dem Synonym 'Ist-Zustand' getarnt. Evasion: weder 'wie gesehen' noch 'Sachmängelhaftung' erscheinen.",
  },
  {
    id: "misc-english-01",
    detector: "as_is_software_disclaimer",
    variant: "deceptive",
    technique: "english",
    severity: "hoch",
    text: "The Customer acknowledges that the Software is licensed in its current state, with all faults and defects, and accepts it as such.",
    note: "Englischsprachiger As-Is-Ausschluss ohne die Standardfloskel 'AS IS'. Evasion: alle Detektor-Patterns sind deutsch bzw. verlangen 'as is'/'without warranty of any kind'.",
  },
  {
    id: "misc-synonym-01",
    detector: "as_is_software_disclaimer",
    variant: "deceptive",
    technique: "synonym",
    severity: "hoch",
    text: "Die Software wird wie besehen überlassen; Zusicherungen zu Verfügbarkeit, Fehlerfreiheit und Eignung für einen bestimmten Zweck sind nicht Bestandteil dieses Vertrages.",
    note: "Archaischer Parallelbegriff 'wie besehen' statt 'wie gesehen' und Verb 'überlassen' statt 'bereitgestellt'. Evasion: zwei Wörter des Triggers minimal verschoben.",
  },

  // ---------------------------------------------------------------------------
  // PART A.2 — entire_agreement
  // ---------------------------------------------------------------------------
  {
    id: "misc-direct-02",
    detector: "entire_agreement",
    variant: "deceptive",
    technique: "direct",
    severity: "mittel",
    text: "Dieser Vertrag enthält alle Vereinbarungen der Parteien; mündliche Nebenabreden bestehen nicht.",
    note: "Kontrollfall: Standard-Entire-Agreement-Klausel. Erwartung: Regex greift über 'mündliche Nebenabreden bestehen nicht'.",
  },
  {
    id: "misc-euphemism-02",
    detector: "entire_agreement",
    variant: "deceptive",
    technique: "euphemism",
    severity: "mittel",
    text: "Mit Unterzeichnung dieses Vertrages treten sämtliche zuvor kommunizierten Leistungsbeschreibungen, Kostenschätzungen und Angebote außer Kraft.",
    note: "Vernichtung vorvertraglicher Zusagen ohne § 305b-Vokabular. Evasion: 'zuvor kommunizierten' statt 'frühere/bisherige/vorherige Leistungsbeschreibungen'.",
  },
  {
    id: "misc-synonym-02",
    detector: "entire_agreement",
    variant: "deceptive",
    technique: "synonym",
    severity: "mittel",
    text: "Mündliche Erklärungen von Vertriebsmitarbeitern begründen keine vertraglichen Pflichten und sind für die Auslegung dieses Vertrages nicht heranzuziehen.",
    note: "Entwertung mündlicher Vertriebszusagen: Wort 'Erklärungen' statt 'Absprachen/Zusagen/Nebenabreden'. Evasion: Nomenklatur weicht vom Patternkatalog ab.",
  },
  {
    id: "misc-english-02",
    detector: "entire_agreement",
    variant: "deceptive",
    technique: "english",
    severity: "mittel",
    text: "This Agreement sets forth the complete and final understanding of the parties and replaces any earlier quotation, representation or collateral warranty.",
    note: "Entire-Agreement-Klausel in Englisch ohne den Trigger 'entire agreement'/'supersedes all prior'. Evasion: Patternliste ist deutsch und an Standardphrasen gebunden.",
  },
  {
    id: "misc-list-buried-01",
    detector: "entire_agreement",
    variant: "deceptive",
    technique: "list-buried",
    severity: "mittel",
    text: "Bestandteil dieses Vertrages sind ausschließlich die vorstehenden Regelungen, die Leistungsbeschreibung in Anlage 2 sowie die Preisliste in Anlage 4; weitergehende Zusagen, Nebenabreden oder Vertriebszusicherungen entfalten keine rechtliche Wirkung.",
    note: "In einer Anlagen-Aufzählung begrabene Klausel: 'entfalten keine rechtliche Wirkung' statt 'haben keine Geltung'. Evasion: Rechtsfolge nominal umschrieben.",
  },

  // ---------------------------------------------------------------------------
  // PART A.3 — salvatorische_klausel_falle
  // ---------------------------------------------------------------------------
  {
    id: "misc-direct-03",
    detector: "salvatorische_klausel_falle",
    variant: "deceptive",
    technique: "direct",
    severity: "hoch",
    text: "Sollte eine Bestimmung dieses Vertrages unwirksam sein, so gilt diejenige Regelung als vereinbart, die dem wirtschaftlichen Zweck am nächsten kommt.",
    note: "Kontrollfall: unwirksame geltungserhaltende Reduktion (§ 306 BGB). Erwartung: Regex greift über 'gilt ... als vereinbart' + 'Zweck am nächsten kommt'.",
  },
  {
    id: "misc-synonym-03",
    detector: "salvatorische_klausel_falle",
    variant: "deceptive",
    technique: "synonym",
    severity: "hoch",
    text: "An die Stelle nichtiger Klauseln tritt diejenige Gestaltung, die den Parteien wirtschaftlich am nächsten steht.",
    note: "Geltungserhaltende Reduktion mit 'nichtiger Klauseln' und 'am nächsten steht'. Evasion: 'unwirksamen' wird zu 'nichtiger', 'kommt' zu 'steht'.",
  },
  {
    id: "misc-euphemism-03",
    detector: "salvatorische_klausel_falle",
    variant: "deceptive",
    technique: "euphemism",
    severity: "hoch",
    text: "Unwirksame Regelungen werden durch eine interessengerechte Ersatzregelung ersetzt, die der Anbieter nach billigem Ermessen bestimmt.",
    note: "Ersetzungsrecht des Verwenders statt gesetzlicher Rechtsfolge. Evasion: 'Regelungen' statt 'Bestimmung', 'interessengerecht' statt 'dem Zweck am nächsten'.",
  },
  {
    id: "misc-english-03",
    detector: "salvatorische_klausel_falle",
    variant: "deceptive",
    technique: "english",
    severity: "hoch",
    text: "If any provision of this Agreement is held to be invalid or unenforceable, the parties shall replace it with a valid provision which comes closest to the commercial intent pursued by them.",
    note: "Klassische 'geltungserhaltende Reduktion' in Englisch (§ 306 Abs. 2 BGB verbietet die Ersatzbestimmung in AGB). Evasion: reine Deutschland-Patterns.",
  },
  {
    id: "misc-compound-multi-01",
    detector: "salvatorische_klausel_falle",
    variant: "deceptive",
    technique: "compound-multi",
    severity: "hoch",
    text: "Sollten einzelne Bestimmungen dieses Vertrages unwirksam sein oder werden, so bleibt der Vertrag im Übrigen wirksam; die Parteien werden die unwirksame Bestimmung durch eine wirksame ersetzen, die dem wirtschaftlichen Zweck am nächsten kommt.",
    note: "Kontrollfall: Standard-Salvatorik mit Ersatzklausel im Folge-Satz. Erwartung: Regex greift im zweiten Satzteil über 'dem wirtschaftlichen Zweck am nächsten kommt'.",
  },

  // ---------------------------------------------------------------------------
  // PART A.4 — unverbindliche_sla_falle
  // ---------------------------------------------------------------------------
  {
    id: "misc-direct-04",
    detector: "unverbindliche_sla_falle",
    variant: "deceptive",
    technique: "direct",
    severity: "hoch",
    text: "Der Anbieter bemüht sich nach besten Kräften um eine Verfügbarkeit der Plattform von 99,9 % im Monatsmittel.",
    note: "Kontrollfall: entkernte Verfügbarkeitszusage ('bemüht sich nach besten Kräften'). Erwartung: Regex greift über 'bemüht sich ... Verfügbarkeit'.",
  },
  {
    id: "misc-euphemism-04",
    detector: "unverbindliche_sla_falle",
    variant: "deceptive",
    technique: "euphemism",
    severity: "hoch",
    text: "Die in der Leistungsbeschreibung genannten Verfügbarkeitswerte sind Serviceziele und begründen keine einklagbaren Ansprüche des Kunden.",
    note: "SLAs werden zu 'Servicezielen' erklärt. Evasion: 'Verfügbarkeitswerte' (nicht 'Verfügbarkeitsangaben') sowie 'keine einklagbaren Ansprüche' statt 'keine Ansprüche' zerreissen das Pattern.",
  },
  {
    id: "misc-synonym-04",
    detector: "unverbindliche_sla_falle",
    variant: "deceptive",
    technique: "synonym",
    severity: "hoch",
    text: "Sämtliche Angaben zur Erreichbarkeit der Dienste stellen rechtlich unverbindliche Richtwerte ohne Rechtsanspruch dar.",
    note: "Verfügbarkeitszusage über 'Erreichbarkeit' und 'Richtwerte' entkernt. Evasion: Wortfeld 'Erreichbarkeit' steht nur im 'bemüht sich'-Pattern, nicht im Unverbindlichkeits-Pattern.",
  },
  {
    id: "misc-english-04",
    detector: "unverbindliche_sla_falle",
    variant: "deceptive",
    technique: "english",
    severity: "hoch",
    text: "Availability figures published in the service description are aspirational targets only and do not constitute a guarantee or give rise to any claim for credits.",
    note: "Illusorische SLA in Englisch ('aspirational targets only'). Evasion: alle SLA-Patterns sind deutschsprachig.",
  },
  {
    id: "misc-nominal-style-01",
    detector: "unverbindliche_sla_falle",
    variant: "deceptive",
    technique: "nominal-style",
    severity: "hoch",
    text: "Eine bestimmte Betriebsbereitschaft wird nicht zugesichert; die angegebenen Uptime-Werte dienen ausschließlich der internen Kapazitätsplanung des Anbieters.",
    note: "Reine Nominalkonstruktion ohne Zusicherung. Evasion: 'Uptime' ist nur im 'bemüht sich'-Pattern verankert, das hier nicht greift.",
  },

  // ---------------------------------------------------------------------------
  // PART A.5 — dynamic_url_referral
  // ---------------------------------------------------------------------------
  {
    id: "misc-direct-05",
    detector: "dynamic_url_referral",
    variant: "deceptive",
    technique: "direct",
    severity: "hoch",
    text: "Es gelten die jeweils aktuellen AGB des Anbieters, abrufbar unter www.anbieter-cloud.example/agb in ihrer jeweils gültigen Fassung.",
    note: "Kontrollfall: dynamischer Online-Verweis (BGH III ZR 59/24). Erwartung: Regex greift über 'jeweils aktuellen AGB ... abrufbar'.",
  },
  {
    id: "misc-euphemism-05",
    detector: "dynamic_url_referral",
    variant: "deceptive",
    technique: "euphemism",
    severity: "hoch",
    text: "Ergänzend gelten die auf der Website des Anbieters veröffentlichten Nutzungsbedingungen in der dort jeweils abrufbaren Fassung.",
    note: "Dynamischer Verweis ohne URL, Version oder Datumsstand. Evasion: 'jeweils abrufbaren' statt 'jederzeit abrufbaren' – ein Wort Unterschied zum Pattern.",
  },
  {
    id: "misc-synonym-05",
    detector: "dynamic_url_referral",
    variant: "deceptive",
    technique: "synonym",
    severity: "hoch",
    text: "Für den Leistungsumfang maßgeblich ist die zum Zeitpunkt der jeweiligen Nutzung gültige Online-Fassung der Servicebedingungen, die der Anbieter fortschreibt.",
    note: "Dynamisch fortgeschriebene Bedingungen als Vertragsgegenstand. Evasion: 'Online-Fassung' und 'fortschreibt' fehlen im Pattern-Vokabular (AGB/Richtlinien/Bedingungen + unter/abrufbar).",
  },
  {
    id: "misc-cross-reference-01",
    detector: "dynamic_url_referral",
    variant: "deceptive",
    technique: "cross-reference",
    severity: "hoch",
    text: "Im Übrigen gelten die im Kundenportal hinterlegten Policies, deren Wortlaut der Anbieter jederzeit fortschreiben kann.",
    note: "Verweis auf einseitig änderbares Portal-Regelwerk. Evasion: Falle ist nur über den Querverweis erkennbar, es fehlt jede URL- und Versionsangabe.",
  },
  {
    id: "misc-list-buried-02",
    detector: "dynamic_url_referral",
    variant: "deceptive",
    technique: "list-buried",
    severity: "hoch",
    text: "Vertragsbestandteil sind ferner die Servicebeschreibung (Anlage 1), die Preisliste (Anlage 3) und die im Übrigen unter https://anbieter.example/terms abrufbaren Regelungen, die der Anbieter jeweils aktualisiert.",
    note: "Dynamisch aktualisierte Online-Regelungen in einer Anlagenliste. Evasion: 'abrufbaren ... die der Anbieter jeweils aktualisiert' – Reihenfolge invertiert, Pattern verlangt 'abrufbar unter https://… in ihrer jeweils gültigen Fassung'.",
  },
  {
    id: "misc-english-05",
    detector: "dynamic_url_referral",
    variant: "deceptive",
    technique: "english",
    severity: "hoch",
    text: "The Service Terms applicable to this Agreement are those published at vendor.example/terms, as amended by the vendor from time to time.",
    note: "Dynamischer AGB-Verweis in Englisch. Evasion: deutsche Patterns; zusätzlich fehlt 'abrufbar unter … in der jeweils gültigen Fassung'.",
  },

  // ---------------------------------------------------------------------------
  // PART B — typografische Evasion (Soft-Hyphen, Zero-Width, NBSP, Homoglyphe,
  //          Fullwidth, Letter-Spacing, Zeilenumbruch im Kompositum)
  // ---------------------------------------------------------------------------
  {
    id: "evade-typo-evasion-01",
    detector: "liability",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Die Haf\u00adtung für mittelbare Schäden und entgangenen Gewinn ist vollumfänglich ausgeschlossen.",
    note: "Trick: Soft-Hyphen (U+00AD) mitten in 'Haftung'. Kontrollfall – normalizeText entfernt U+00AD, der Trigger wird also wieder zusammengesetzt und erkannt.",
  },
  {
    id: "evade-typo-evasion-02",
    detector: "penalty",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Für jeden Fall der schuldhaften Zuwiderhandlung verwirkt eine Vertrags\u200bstrafe in Höhe von 25.000 EUR.",
    note: "Trick: Zero-Width Space (U+200B) in 'Vertragsstrafe'. Kontrollfall – U+200B steht auf der Strip-Liste von normalizeText, der Trigger bleibt intakt.",
  },
  {
    id: "evade-typo-evasion-03",
    detector: "penalty",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Verstößt der Kunde gegen eine Vertraulichkeitspflicht, verwirkt er eine Vertrags\u00a0strafe von 15.000 EUR je Verstoß.",
    note: "Trick: NBSP (U+00A0) im Kompositum 'Vertragsstrafe'. Blindspot – die Normalisierung wandelt U+00A0 in ein normales Leerzeichen um, wodurch 'vertragsstrafe' in zwei Wörter zerbricht; auch 'strafe in Höhe von' fehlt hier.",
  },
  {
    id: "evade-typo-evasion-04",
    detector: "liability",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Die H\u0430ftung des Auftragnehmers für Sach- und Rechtsmängel ist auf die Höhe der Einzelauftragssumme begrenzt.",
    note: "Trick: kyrillisches 'а' (U+0430) statt lateinischem 'a' in 'Haftung'. Blindspot – Homoglyphe überleben normalizeText, das Wort 'Haftung' ist damit für die Regex nicht mehr vorhanden.",
  },
  {
    id: "evade-typo-evasion-05",
    detector: "liability",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Ｈａｆｔｕｎｇ und Gewährleistung des Auftragnehmers sind für Schäden ausgeschlossen, die nicht auf Vorsatz oder grober Fahrlässigkeit beruhen.",
    note: "Trick: Fullwidth-Latin ('Haftung' als U+FF28/U+FF41/U+FF46/U+FF54/U+FF55/U+FF4E/U+FF47). Blindspot – Kopfregexe matchen keine Fullwidth-Zeichen, das Schlüsselwort fehlt komplett.",
  },
  {
    id: "evade-typo-evasion-06",
    detector: "jurisdiction",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Ausschließlicher G e r i c h t s s t a n d für sämtliche Streitigkeiten ist der Sitz des Anbieters in Hamburg.",
    note: "Trick: Letter-Spacing (Leerzeichen zwischen allen Buchstaben). Blindspot – 'Gerichtsstand' ist nicht mehr als zusammenhängender String auffindbar; der Rest der Klausel ist bewusst triggerfrei formuliert.",
  },
  {
    id: "evade-typo-evasion-07",
    detector: "liability",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Eine Haftungs\nbeschränkung auf die Auftragssumme gilt auch für Schäden aus der Verletzung von Kardinalpflichten.",
    note: "Trick: Zeilenumbruch mitten im Kompositum (typisches PDF-Extraktionsartefakt). Blindspot – die Satzsegmentierung trennt hier nicht, aber alle Patterns schließen \\n im Zwischenraum aus und das Kompositum ist zerrissen.",
  },
  {
    id: "evade-typo-evasion-08",
    detector: "penalty",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Bei Zahlungsverzug verwirkt der Kunde eine K\u03bfnventionalstrafe von 10 % der Nettovergütung; der Nachweis einer abweichenden Schadenshöhe ist ausgeschlossen.",
    note: "Trick: griechisches Omikron (U+03BF) statt 'o' in 'Konventionalstrafe'. Blindspot – Wort bricht auf; 'strafe in Höhe von' ist bewusst vermieden, sodass kein Ersatzpattern greift.",
  },
  {
    id: "evade-typo-evasion-09",
    detector: "data_grant",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Der Anbieter ist berechtigt, die aus Telemetri\u0435daten und Diagnoseprotokollen abgeleiteten Kennzahlen weltweit und unbegrenzt kommerziell zu verwerten.",
    note: "Trick: kyrillisches 'е' (U+0435) statt 'e' in 'Telemetriedaten'. Blindspot – Trigger 'telemetriedaten' zerbricht; 'Workflows'/'Feedback' sind bewusst nicht verwendet, weil die Regex sonst über die Workflow-Alternative greifen würde.",
  },
  {
    id: "evade-typo-evasion-10",
    detector: "setoff_restriction",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Ein Aufre\u0441hnungsrecht des Kunden ist ausgeschlossen; Gegenforderungen können nur mit schriftlicher Zustimmung des Anbieters berücksichtigt werden.",
    note: "Trick: kyrillisches 'с' (U+0441) statt 'c' in 'Aufrechnung'. Blindspot – sowohl das Wort-Pattern als auch 'ausgeschlossen … aufrechnung' verfehlen das Wort; die Klausel ist inhaltlich ein pauschales Aufrechnungsverbot.",
  },
  {
    id: "evade-typo-evasion-11",
    detector: "deemed_acceptance",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Erkennbare Mängel sind spätestens innerhalb von fünf Werktagen nach Ablieferung schriftlich zu rügen; unterbleibt die Mängel\u00a0rüge, werden sämtliche Gewährleistungsansprüche verwirkt.",
    note: "Trick: NBSP (U+00A0) in 'Mängelrüge'. Blindspot – nach der Normalisierung lautet der Text 'Mängel rüge' und das Pattern 'mängelrüge' sowie die Wortfolge 'unterbleibt die Mängelrüge' greifen nicht mehr.",
  },
  {
    id: "evade-typo-evasion-12",
    detector: "jurisdiction",
    variant: "deceptive",
    technique: "typo-evasion",
    severity: "hoch",
    text: "Sämtliche Streitigkeiten aus diesem Vertrag werden ausschließlich vor den staatlichen Gerichten in Del\u0430ware verhandelt; für Klagen des Anbieters gilt der gesetzliche Gerichtsstand.",
    note: "Trick: kyrillisches 'а' (U+0430) in 'Delaware'. Blindspot – das Auslandsforum ist nur noch über 'Delaware' erkennbar, das Wort zerbricht; der zweite Halbsatz nennt bewusst nur den gesetzlichen Gerichtsstand des Anbieters.",
  },

  // ---------------------------------------------------------------------------
  // Compliant baselines (False-Positive-Resistenz)
  // ---------------------------------------------------------------------------
  {
    id: "misc-compliant-01",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "unverbindliche_sla_falle",
    text: "Der Anbieter gewährleistet eine monatliche Verfügbarkeit von 99,5 % und gewährt bei Unterschreitung eine Gutschrift von 5 % der Monatsgebühr. Unterschreitet die Verfügbarkeit 98 %, ist der Kunde berechtigt, den Vertrag mit sofortiger Wirkung zu kündigen.",
    note: "Verbindliches SLA mit Gutschrift und Sonderkündigungsrecht – bewusst gegen die SLA-Patterns gebaut ('gewährleistet' statt 'bemüht sich', Gutschrift nicht als 'ausschließliche Kompensation').",
  },
  {
    id: "misc-compliant-02",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "dynamic_url_referral",
    text: "Für die Leistungen gilt die als Anlage 1 beigefügte Fassung der Allgemeinen Geschäftsbedingungen des Anbieters in der Fassung vom 01.01.2026. Spätere Änderungen der AGB werden nur mit ausdrücklicher schriftlicher Zustimmung des Kunden Vertragsbestandteil.",
    note: "Statisch versionierte AGB als Vertragsanlage ohne dynamischen Online-Verweis und ohne Zustimmungsfiktion.",
  },
  {
    id: "misc-compliant-03",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "salvatorische_klausel_falle",
    text: "Sollte eine Bestimmung dieses Vertrages unwirksam sein oder werden, tritt an ihre Stelle die gesetzliche Regelung des Bürgerlichen Gesetzbuchs; im Übrigen bleibt der Vertrag mit den übrigen Bestimmungen wirksam.",
    note: "Zulässige Salvatorik: es gilt das Gesetz (§ 306 Abs. 2 BGB), keine geltungserhaltende Ersatzbestimmung nach Ermessen des Verwenders.",
  },
  {
    id: "misc-compliant-04",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "entire_agreement",
    text: "Dieser Vertrag enthält die vollständigen Vereinbarungen der Parteien. Individuelle Absprachen sowie die in Anlage 2 dokumentierten Leistungszusagen des Vertriebs bleiben unabhängig davon verbindlich; mündliche Individualabreden haben Vorrang vor diesen AGB.",
    note: "Entire-Agreement-Klausel mit ausdrücklichem Vorrang der Individualabrede (§ 305b BGB) – bewusst triggerfrei formuliert.",
  },
  {
    id: "misc-uncovered-agency-commission-clean",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "agency_commission_exclusion",
    text: "Für Direktgeschäfte außerhalb des dem Handelsvertreter zugewiesenen Gebiets wird keine Provision gezahlt.",
    note: "Provision exclusion is expressly limited to transactions outside the assigned district.",
  },
];

export default CASES;
