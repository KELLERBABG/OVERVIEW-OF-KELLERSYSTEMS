/**
 * Stress-Korpus: GENERALIZATION (Welle D)
 * =======================================
 *
 * Generalisierungstest der zuletzt gehaerteten Detektoren: neue, im uebrigen
 * Korpus und im Holdout NICHT verwendete Formulierungen derselben Fallen plus
 * Benign-Zwillinge, die die Schutzgrenzen derselben Detektoren ausloten.
 *
 * Ziel: beweisen, dass die Haertungen semantisch generalisieren und nicht nur
 * die konkreten Trainingsformulierungen treffen.
 */

export const CASES = [
  // ---------------------------------------------------------------- burden_of_proof
  {
    id: "gen-burden-nachweis-obliegt-01",
    detector: "burden_of_proof",
    variant: "deceptive",
    technique: "synonym",
    severity: "hoch",
    text: "Der Nachweis, dass ein eingetretener Schaden nicht auf Umstände aus dem Verantwortungsbereich des Anbieters zurückzuführen ist, obliegt dem Auftraggeber.",
    note: "Negativbeweislast beim Kunden in Voranstellung ('Der Nachweis ... obliegt dem Auftraggeber') statt der ueblichen Syntax 'traegt der Kunde die Beweislast'.",
  },
  {
    id: "gen-burden-beweislast-02",
    detector: "burden_of_proof",
    variant: "deceptive",
    technique: "nominal-style",
    severity: "hoch",
    text: "Für die Einhaltung der vereinbarten Reaktionszeiten trägt der Besteller die volle Darlegungs- und Beweislast.",
    note: "Darlegungs- und Beweislast fuer zugesagte Servicelevel pauschal beim Besteller - unzulaessige Beweislastumkehr (§ 309 Nr. 12 BGB).",
  },

  // ---------------------------------------------------------------- tax_indemnity_shift
  {
    id: "gen-tax-liste-03",
    detector: "tax_indemnity_shift",
    variant: "deceptive",
    technique: "list-buried",
    severity: "mittel",
    text: "Steuern, Abgaben, Gebühren und sonstige staatliche Belastungen, die im Zusammenhang mit diesem Vertrag anfallen, trägt der Auftraggeber.",
    note: "Steuer- und Abgabenabwaelzung in einer Aufzaehlung versteckt; Subjekt und Praedikat sind umgestellt ('traegt der Auftraggeber' am Satzende).",
  },
  {
    id: "gen-tax-freistellung-17",
    detector: "tax_indemnity_shift",
    variant: "deceptive",
    technique: "nominal-style",
    severity: "mittel",
    text: "Der Auftraggeber stellt den Auftragnehmer von sämtlichen Steuernachforderungen frei, die im Zusammenhang mit diesem Vertrag gegen ihn geltend gemacht werden.",
    note: "Freistellungsvariante der Steuerabwaelzung: der Kunde uebernimmt rueckwirkend alle Steuernachforderungen des Anbieters.",
  },

  // ---------------------------------------------------------------- service_suspension
  {
    id: "gen-suspension-ankuendigung-04",
    detector: "service_suspension",
    variant: "deceptive",
    technique: "euphemism",
    severity: "hoch",
    text: "Der Anbieter unterbricht die Leistungserbringung ohne vorherige Ankündigung; die Vergütung ist für die Dauer der Unterbrechung weiter zu entrichten.",
    note: "Sperre ohne Ankuendigung plus Weiterzahlungspflicht in zwei Saetzen - § 320 BGB wird auf den Kopf gestellt.",
  },

  // ---------------------------------------------------------------- jurisdiction
  {
    id: "gen-jurisdiction-unternehmenssitz-05",
    detector: "jurisdiction",
    variant: "deceptive",
    technique: "synonym",
    severity: "hoch",
    text: "Für Streitigkeiten aus diesem Vertrag ist der Unternehmenssitz der Auftragnehmerin ausschließlich zuständig.",
    note: "Ausschliesslicher Gerichtsstand am Anbietersitz ohne die Woerter 'Gerichtsstand' oder 'Sitz des Anbieters' (sondern 'Unternehmenssitz der Auftragnehmerin ... zustaendig').",
  },

  // ---------------------------------------------------------------- deemed_acceptance
  {
    id: "gen-deemed-werktage-06",
    detector: "deemed_acceptance",
    variant: "deceptive",
    technique: "direct",
    severity: "hoch",
    text: "Erfolgt innerhalb von acht Werktagen nach Lieferung keine schriftliche Mängelrüge, gilt das Werk als vorbehaltlos abgenommen und alle Mängelansprüche verfallen ersatzlos.",
    note: "Klassische Abnahmefiktion: achttaegige Ruegefrist, danach vorbehaltlose Abnahme und ersatzloser Verfall aller Maengelansprueche.",
  },

  // ---------------------------------------------------------------- entire_agreement
  {
    id: "gen-entire-verhandlungen-07",
    detector: "entire_agreement",
    variant: "deceptive",
    technique: "synonym",
    severity: "mittel",
    text: "Mündliche Vereinbarungen, Verhandlungen und Zusicherungen, die nicht in diesem Dokument enthalten sind, sind unverbindlich.",
    note: "Vollstaendigkeitsklausel mit 'sind unverbindlich' statt 'bestehen nicht' - vorvertragliche Zusagen werden entwertet (§ 305b BGB).",
  },

  // ---------------------------------------------------------------- as_is_software_disclaimer
  {
    id: "gen-as-is-zusicherung-08",
    detector: "as_is_software_disclaimer",
    variant: "deceptive",
    technique: "euphemism",
    severity: "hoch",
    text: "Die Software wird in ihrem derzeitigen Zustand ohne jede Zusicherung zur Beschaffenheit bereitgestellt.",
    note: "As-is-Bereitstellung mit qualifiziertem Zusicherungsausschluss ('ohne jede Zusicherung zur Beschaffenheit').",
  },

  // ---------------------------------------------------------------- excessive_survival
  {
    id: "gen-survival-vertraulichkeit-09",
    detector: "excessive_survival",
    variant: "deceptive",
    technique: "synonym",
    severity: "hoch",
    text: "Die Verpflichtungen zur Vertraulichkeit nach Ziffer 5 enden nicht mit der Beendigung dieses Vertrages.",
    note: "Unbefristete Nachwirkung ueber 'enden nicht mit der Beendigung' statt 'gelten unbegrenzt fort'.",
  },

  // ---------------------------------------------------------------- unverbindliche_sla_falle
  {
    id: "gen-sla-kennzahlen-10",
    detector: "unverbindliche_sla_falle",
    variant: "deceptive",
    technique: "nominal-style",
    severity: "hoch",
    text: "Die in Anlage 3 genannten Kennzahlen zur Erreichbarkeit dienen ausschließlich der internen Steuerung und sind unverbindlich.",
    note: "SLA-Entkernung ueber 'Kennzahlen ... dienen ausschliesslich der internen Steuerung' - keine einklagbaren Servicelevel.",
  },

  // ---------------------------------------------------------------- price_escalation
  {
    id: "gen-price-grundpreis-11",
    detector: "price_escalation",
    variant: "deceptive",
    technique: "euphemism",
    severity: "hoch",
    text: "Der Grundpreis wird in angemessenen Abständen an die allgemeine Preissteigerung angepasst.",
    note: "Preisgleitklausel gekoppelt an 'allgemeine Preissteigerung' ohne Obergrenze und ohne Sonderkündigungsrecht.",
  },
  {
    id: "gen-price-obergrenze-18",
    detector: "price_escalation",
    variant: "deceptive",
    technique: "synonym",
    severity: "mittel",
    text: "Eine betragsmäßige Obergrenze für Preisanpassungen wird nicht vereinbart.",
    note: "Ausdruecklicher Verzicht auf jede Preisobergrenze bei gleichzeitiger Anpassungsbefugnis (siehe Preisklauselgesetz).",
  },

  // ---------------------------------------------------------------- subcontractor_uncontrolled
  {
    id: "gen-subcontractor-nachunternehmer-12",
    detector: "subcontractor_uncontrolled",
    variant: "deceptive",
    technique: "synonym",
    severity: "mittel",
    text: "Der Auftragnehmer ist jederzeit berechtigt, Nachunternehmer einzusetzen, ohne den Auftraggeber hierüber zu unterrichten.",
    note: "Subunternehmereinsatz mit Synonym 'Nachunternehmer' und ohne Informationspflicht (§ 307 BGB, Art. 28 DSGVO).",
  },

  // ---------------------------------------------------------------- extended_retention_of_title
  {
    id: "gen-retention-begleichung-13",
    detector: "extended_retention_of_title",
    variant: "deceptive",
    technique: "nominal-style",
    severity: "hoch",
    text: "Das Eigentum an sämtlichen Liefergegenständen geht erst mit der vollständigen Begleichung aller Verbindlichkeiten aus der Geschäftsverbindung auf den Kunden über.",
    note: "Kontokorrentvorbehalt in Nominalstil ('mit der vollstaendigen Begleichung aller Verbindlichkeiten aus der Geschaeftsverbindung').",
  },

  // ---------------------------------------------------------------- non_solicitation
  {
    id: "gen-nonsolicit-beschaeftigten-14",
    detector: "non_solicitation",
    variant: "deceptive",
    technique: "euphemism",
    severity: "hoch",
    text: "Der Auftraggeber verpflichtet sich, für zwölf Monate nach Vertragsende keine Beschäftigten des Auftragnehmers zu beschäftigen.",
    note: "Nachvertragliches Beschaeftigungsverbot mit weichem Verb 'beschaeftigen' statt 'abwerben', einseitig zu Lasten des Kunden.",
  },

  // ---------------------------------------------------------------- notice_period
  {
    id: "gen-notice-fristen-asym-15",
    detector: "notice_period",
    variant: "deceptive",
    technique: "direct",
    severity: "hoch",
    text: "Die Kündigungsfrist beträgt für den Auftraggeber zwölf Monate, für den Auftragnehmer einen Monat.",
    note: "Extrem ungleiche Kuendigungsfristen (12 Monate gegen 1 Monat) in einem Satz - einseitige Bindung.",
  },

  // ---------------------------------------------------------------- data_grant
  {
    id: "gen-datagrant-erloese-16",
    detector: "data_grant",
    variant: "deceptive",
    technique: "euphemism",
    severity: "hoch",
    text: "Der Auftragnehmer darf die im Rahmen der Zusammenarbeit gewonnenen Erkenntnisse für eigene Zwecke auswerten; eine Beteiligung des Auftraggebers an den Erlösen ist nicht vorgesehen.",
    note: "Datenverwertung 'fuer eigene Zwecke' ohne Erloesbeteiligung des Auftraggebers - verdeckte Kommerzialisierung.",
  },

  // ================================================================ Compliant-Zwillinge
  {
    id: "gen-c-kuendigung-symm-01",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "notice_period",
    text: "Der Vertrag kann von beiden Parteien mit einer Frist von drei Monaten zum Quartalsende ordentlich gekündigt werden.",
    note: "Symmetrische Kuendigungsfrist von drei Monaten zum Quartalsende - B2B-uebliche und zulaessige Ausgestaltung.",
  },
  {
    id: "gen-c-preisanpassung-zustimmung-02",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "price_escalation",
    text: "Preisanpassungen werden nur mit vorheriger schriftlicher Zustimmung des Kunden vorgenommen.",
    note: "Preisanpassung nur mit ausdruecklicher Kunden-Zustimmung - das Gegenteil einer einseitigen Preiserhoehungsautomatik.",
  },
  {
    id: "gen-c-gerichtsstand-gesetzlich-03",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "jurisdiction",
    text: "Für alle Streitigkeiten gilt der gesetzliche Gerichtsstand.",
    note: "Verweis auf den gesetzlichen Gerichtsstand - ausdruecklich unproblematisch.",
  },
  {
    id: "gen-c-gewaehrleistung-04",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "as_is_software_disclaimer",
    text: "Der Anbieter gewährleistet, dass die Software bei Übergabe die vereinbarte Beschaffenheit aufweist; Mängel werden nach den gesetzlichen Gewährleistungsvorschriften behoben.",
    note: "Ausdrueckliche Beschaffenheitszusage plus gesetzliche Maengelrechte - das Gegenteil eines As-is-Disclaimers.",
  },
  {
    id: "gen-c-geheimhaltung-befristet-05",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "excessive_survival",
    text: "Die Geheimhaltungspflicht gilt für drei Jahre nach Beendigung des Vertrages; offenkundige Informationen sind hiervon ausgenommen.",
    note: "Befristete Geheimhaltung (drei Jahre) mit ueblicher Offenkundigkeitsausnahme - wirksam.",
  },
  {
    id: "gen-c-abwerben-vertraulich-06",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "non_solicitation",
    text: "Das Abwerben von Mitarbeitern der jeweils anderen Partei unter Verwendung vertraulicher Informationen ist untersagt.",
    note: "Eng begrenztes Verbot unlauterer Abwerbung unter Ausnutzung von Geschaeftsgeheimnissen (GeschGehG) - zulaessig und ueblich.",
  },
  {
    id: "gen-c-sla-verpflichtend-07",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "unverbindliche_sla_falle",
    text: "Der Anbieter verpflichtet sich, eine Verfügbarkeit der Produktionsumgebung von mindestens 99,5 % im Monatsmittel einzuhalten; bei Unterschreitung werden dem Kunden Service-Gutschriften gewährt.",
    note: "Verbindliche SLA-Zusage mit Gutschriftenmechanik - genau das, was die SLA-Entkernung gerade nicht bietet.",
  },
  {
    id: "gen-c-entire-individual-08",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "entire_agreement",
    text: "Individualvereinbarungen bleiben unberührt und gehen den Regelungen dieses Vertrages vor.",
    note: "Klarstellung des Vorrangs von Individualabreden - gesetzeskonform nach § 305b BGB.",
  },
  {
    id: "gen-c-subunternehmer-zustimmung-09",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "subcontractor_uncontrolled",
    text: "Der Einsatz von Unterauftragnehmern bedarf der vorherigen schriftlichen Zustimmung des Auftraggebers.",
    note: "Subunternehmereinsatz nur mit vorheriger Zustimmung - kontrollierte und ausgewogene Ausgestaltung.",
  },
  {
    id: "gen-c-steuern-jede-partei-10",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "tax_indemnity_shift",
    text: "Jede Partei trägt die auf sie selbst entfallenden Steuern und Abgaben.",
    note: "Symmetrische Steuertragung jeder Partei fuer sich selbst - ausgewogen und zulaessig.",
  },
  {
    id: "gen-c-abnahme-protokoll-11",
    detector: null,
    variant: "compliant",
    technique: "direct",
    severity: null,
    nearestDetector: "deemed_acceptance",
    text: "Die Abnahme des Werks erfolgt durch die beiderseitige Unterzeichnung eines Abnahmeprotokolls; bis zur vollständigen Abnahme bleiben alle Mängelrechte des Bestellers unberührt.",
    note: "Förmliche Abnahme per Protokoll ohne Fiktion, Maengelrechte bleiben bis zur Abnahme erhalten.",
  },
];

export default CASES;
