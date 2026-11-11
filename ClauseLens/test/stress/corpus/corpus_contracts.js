/**
 * Stress-Korpus: KOMPLETTE VERTRAEGE (End-to-End)
 * ==============================================
 *
 * Jeder Fall ist ein vollstaendiger, mehrklauseliger Vertrag mit realistischer
 * Struktur (§ 1 bis § 12), normalem Vertragsrauschen und 4-8 versteckten,
 * einseitigen Klauseln. Gemessen wird, ob die Engine die jeweils zuerst
 * genannten Detektoren (detector) und alle Zusatzdetektoren (alsoExpect)
 * tatsaechlich findet - der Text mischt bewusst harmlose Klauseln unter die
 * Fallen, damit die Treffer nicht aus einem einzigen Trigger-Satz entstehen.
 *
 * Technik durchgaengig "full-contract". Die letzten beiden Faelle sind rein
 * englischsprachige Vertragswerke und damit bewusst als Blindspot-Test
 * angelegt (die Engine besitzt nur wenige englische Muster).
 */

export const CASES = [
  // ---------------------------------------------------------------------
  // 1. B2B SaaS-Abonnement
  // ---------------------------------------------------------------------
  {
    id: "contract-saas-01",
    detector: "auto_renewal",
    variant: "deceptive",
    technique: "full-contract",
    severity: "hoch",
    alsoExpect: ["price_escalation", "data_grant", "service_suspension", "liability"],
    text: `Cloud-Desk Abonnementvereinbarung

§ 1 Vertragsgegenstand
Der Anbieter stellt dem Kunden die Software Cloud-Desk als Software-as-a-Service über das Internet zur Verfügung. Der Funktionsumfang ergibt sich aus der Leistungsbeschreibung in Anlage 1.

§ 2 Mitwirkung des Kunden
Der Kunde benennt einen technischen Ansprechpartner und hält die Zugangsdaten geheim. Der Anbieter stellt während der Laufzeit eine deutschsprachige Supportannahme bereit.

§ 3 Vergütung und Zahlung
Die monatliche Grundgebühr beträgt 2.900 EUR und ist innerhalb von 14 Tagen ab Rechnungsstellung zur Zahlung fällig. Die Vergütung erhöht sich automatisch jährlich um 5 %.

§ 4 Laufzeit und Verlängerung
Die Mindestlaufzeit beträgt zwölf Monate. Der Vertrag verlängert sich automatisch um jeweils weitere zwölf Monate, wenn er nicht mit einer Frist von drei Monaten zum Laufzeitende gekündigt wird.

§ 5 Verfügbarkeit und Sperre
Der Anbieter stellt eine Verfügbarkeit von 99,5 % im Jahresmittel in Aussicht. Bei Zahlungsverzug ist der Anbieter berechtigt, den Zugang des Kunden sofort ohne Vorankündigung zu sperren.

§ 6 Nutzungsrechte an Inhalten
Der Kunde räumt dem Anbieter ein uneingeschränktes, unwiderrufliches und übertragbares Nutzungsrecht an sämtlichen Inhalten ein.

§ 7 Haftung
Die Haftung des Anbieters für Schäden jeglicher Art wird vollumfänglich ausgeschlossen.

§ 8 Datenschutz
Der Anbieter verarbeitet personenbezogene Daten des Kunden ausschließlich auf Grundlage des gesonderten Auftragsverarbeitungsvertrages.

§ 9 Schlussbestimmungen
Sollte eine Bestimmung dieses Vertrages unwirksam sein, bleibt die Wirksamkeit der übrigen Bestimmungen unberührt. Anlagen sind Bestandteil dieses Vertrages.`,
    note: "Klassischer SaaS-Abovertrag. Versteckt: automatische Jahresverlaengerung (§ 4, nur Oberflaeche harmlos), jaehrliche Indexlos-Erhoehung ohne Kuendigungsrecht (§ 3), umfassende Rechtsabtretung an Kundendaten (§ 6), sofortige Sperre bei Zahlungsverzug mit unveraenderter Verguetungspflicht (§ 5) und Totalausschluss der Haftung (§ 7). Die salvatorische Klausel in § 9 ist bewusst die gesetzeskonforme Variante.",
  },

  // ---------------------------------------------------------------------
  // 2. Enterprise-Softwarelizenz
  // ---------------------------------------------------------------------
  {
    id: "contract-enterprise-licence-02",
    detector: "liability",
    variant: "deceptive",
    technique: "full-contract",
    severity: "hoch",
    alsoExpect: ["audit_log_manipulation", "written_form_trap", "statute_of_limitations", "unilateral_convenience_termination"],
    text: `Softwarelizenz- und Wartungsvertrag (Enterprise)

§ 1 Gegenstand der Lizenzierung
Der Anbieter überlässt dem Kunden die Standardsoftware nebst Dokumentation zur Nutzung auf den vereinbarten Servern. Der Quellcode wird nicht überlassen.

§ 2 Nutzungsumfang
Die Nutzung ist auf die vertraglich benannte Anzahl benannter Nutzer beschränkt. Eine Weitergabe an Dritte ist nicht gestattet.

§ 3 Vergütung
Das einmalige Lizenzentgelt beträgt 48.000 EUR und ist innerhalb von 30 Tagen nach Vertragsschluss zur Zahlung fällig.

§ 4 Mängelansprüche und Verjährung
Der Kunde prüft die Software nach Bereitstellung und zeigt Mängel unverzüglich an. Sämtliche Ansprüche des Kunden verjähren innerhalb von 180 Kalendertagen nach Bereitstellung der Software.

§ 5 Schutzrechte und Protokolle
Der Anbieter gewährleistet die Verschaffung der Nutzungsrechte. Die Systemprotokolle des Anbieters gelten als Vollbeweis für die Richtigkeit der Abrechnung und die Fehlerfreiheit der Leistung.

§ 6 Haftung und Mängelgewähr
Die Haftung des Anbieters für Schäden jeglicher Art wird vollumfänglich ausgeschlossen.

§ 7 Laufzeit und Beendigung
Der Vertrag wird auf unbestimmte Zeit geschlossen. Die ordentliche Kündigung ist dem Kunden verwehrt; nur der Anbieter ist berechtigt, ordentlich zu kündigen.

§ 8 Erklärungen und Form
Jede Kündigung bedarf der eigenhändig unterzeichneten Schriftform und muss per eingeschriebenem Brief mit Rückschein zugehen.

§ 9 Anwendbares Recht
Es gilt deutsches Recht. Anlagen sind Bestandteil dieses Vertrages.`,
    note: "Enterprise-Lizenz mit sechs versteckten Klauseln: 180-Tage-Verjaehrung ohne Karve-out fuer Koerperschaeden (§ 4), Beweismonopol durch Anbieterprotokolle (§ 5), Totalausschluss der Haftung (§ 6), nur der Anbieter darf ordentlich kuendigen (§ 7) und eine Formfalle mit Einschreiben-Rueckschein (§ 8). Die Klauseln wirken einzeln unauffaellig.",
  },

  // ---------------------------------------------------------------------
  // 3. Managed Hosting
  // ---------------------------------------------------------------------
  {
    id: "contract-hosting-03",
    detector: "unverbindliche_sla_falle",
    variant: "deceptive",
    technique: "full-contract",
    severity: "hoch",
    alsoExpect: ["service_suspension", "deemed_acceptance", "setoff_restriction", "data_lockin"],
    text: `Managed-Hosting-Vereinbarung

§ 1 Leistungsgegenstand
Der Anbieter betreibt die Serverinfrastruktur des Kunden in zwei geografisch getrennten Rechenzentren. Die Leistungsbeschreibung ist in Anlage 2 enthalten.

§ 2 Leistungsziele und Bemühen
Der Anbieter bemüht sich nach besten Kräften um eine Verfügbarkeit von 99,9 % im Monatsmittel. Ungeachtet der angegebenen Werte handelt es sich um technische Zielgrößen ohne Rechtsanspruch.

§ 3 Wartungsfenster
Regelmäßige Wartungsfenster werden mindestens 72 Stunden im Voraus angekündigt und innerhalb der vereinbarten Servicezeiten durchgeführt.

§ 4 Abnahme der Leistungen
Der Kunde prüft die bereitgestellten Leistungen unverzüglich. Unterbleibt eine Rüge innerhalb von zehn Werktagen nach Bereitstellung, gilt die Leistung als vorbehaltlos abgenommen.

§ 5 Zahlungsverzug
Bei Zahlungsverzug ist der Anbieter berechtigt, den Zugang des Kunden sofort ohne Vorankündigung zu sperren. Die monatliche Grundgebühr ist für die Dauer der Sperre weiterzuzahlen.

§ 6 Aufrechnung und Einbehalt
Die Aufrechnung mit Gegenforderungen des Kunden ist ausgeschlossen. Ein Zurückbehaltungsrecht steht dem Kunden nicht zu.

§ 7 Datenrückgabe
Nach Beendigung des Vertrages ist die Herausgabe der Daten kostenpflichtig und wird nach Aufwand abgerechnet.

§ 8 Vergütung
Die monatliche Grundgebühr beträgt 3.400 EUR und wird jeweils zum Monatsersten fällig.

§ 9 Änderungen der Leistungsbeschreibung
Anpassungen der Leistungsbeschreibung erfolgen ausschließlich im Einvernehmen der Parteien.`,
    note: "Hosting-Vertrag: Die beworbene 99,9-Prozent-Verfuegbarkeit wird in § 2 selbst entwertet (Scheingarantie ohne Minderungs- oder Kuendigungsrecht), dazu sofortige Sperre mit fortlaufender Verguetungspflicht (§ 5), Abnahmefiktion nach zehn Werktagen (§ 4), pauschales Aufrechnungs- und Zurueckbehaltungsverbot (§ 6) und kostenpflichtige Datenherausgabe (§ 7).",
  },

  // ---------------------------------------------------------------------
  // 4. Beschaffungsrahmenvertrag
  // ---------------------------------------------------------------------
  {
    id: "contract-procurement-04",
    detector: "extended_retention_of_title",
    variant: "deceptive",
    technique: "full-contract",
    severity: "hoch",
    alsoExpect: ["penalty", "default_interest", "assignment_restriction", "entire_agreement"],
    text: `Rahmenvertrag über die Lieferung von Bauteilen

§ 1 Vertragsgegenstand
Der Rahmenvertrag regelt die Lieferung von Bauteilen durch den Lieferanten an den Besteller. Einzelheiten ergeben sich aus den jeweiligen Abrufen.

§ 2 Abrufe und Lieferzeiten
Lieferungen erfolgen auf Abruf innerhalb von zehn Werktagen ab Bestelleingang. Die Ware ist auf Paletten zu liefern.

§ 3 Eigentumsvorbehalt
Die gelieferten Waren bleiben bis zum Ausgleich sämtlicher Forderungen aus der gesamten Geschäftsbeziehung Eigentum des Anbieters.

§ 4 Zahlung und Verzug
Rechnungen sind innerhalb von 30 Tagen ohne Abzug zu begleichen. Bei Zahlungsverzug werden Verzugszinsen in Höhe von 12 Prozentpunkten über dem Basiszinssatz berechnet.

§ 5 Terminüberschreitung
Bei Überschreitung der vereinbarten Lieferfrist verfällt eine Vertragsstrafe in Höhe von 2.500 EUR pro angefangenem Tag.

§ 6 Abtretung
Die Abtretung von Forderungen an Dritte ist ohne vorherige schriftliche Zustimmung des Anbieters ausgeschlossen.

§ 7 Vollständigkeit
Mündliche Nebenabreden bestehen nicht und sämtliche vorvertraglichen Zusagen verlieren ihre Gültigkeit.

§ 8 Qualitätssicherung
Der Lieferant legt für jede Charge die vereinbarten Prüfprotokolle vor. Beanstandungen erfolgen in Textform.

§ 9 Anwendbares Recht
Es gilt das Recht der Bundesrepublik Deutschland. Anlagen sind Bestandteil dieses Rahmenvertrages.`,
    note: "Beschaffungsvertrag: Kontokorrent-Eigentumsvorbehalt ueber die gesamte Geschaeftsbeziehung (§ 3), 12 Prozentpunkte Verzugszinsen (§ 4), hohe Tagesstrafe ohne Nachweisvorbehalt (§ 5), formularmaessiges Abtretungsverbot (§ 6) und Vollstaendigkeitsklausel, die vorvertragliche Zusagen entwertet (§ 7).",
  },

  // ---------------------------------------------------------------------
  // 5. Freelance-/Beratungsvertrag
  // ---------------------------------------------------------------------
  {
    id: "contract-freelance-05",
    detector: "non_compete",
    variant: "deceptive",
    technique: "full-contract",
    severity: "hoch",
    alsoExpect: ["data_grant", "feedback_ip_grab", "third_party_indemnity", "jurisdiction"],
    text: `Beratungs- und Entwicklungsvertrag

§ 1 Leistungsgegenstand
Der Anbieter erbringt Beratungs- und Entwicklungsleistungen im Umfang der jeweiligen Einzelaufträge. Der Leistungsort ist die Niederlassung des Anbieters.

§ 2 Vergütung
Die Vergütung erfolgt zu Tagessätzen von 1.100 EUR und ist innerhalb von 21 Tagen nach Rechnungszugang zu zahlen. Reisekosten werden gesondert abgerechnet.

§ 3 Nachvertragliches Tätigkeitsverbot
Dem Kunden ist es untersagt, für die Dauer von 24 Monaten nach Beendigung des Vertrages im Geschäftsbereich des Anbieters beratend oder vermittelnd tätig zu werden; eine Karenzentschädigung wird nicht gewährt.

§ 4 Rechte an Inhalten und Arbeitsergebnissen
Der Kunde räumt dem Anbieter ein uneingeschränktes, unwiderrufliches und übertragbares Nutzungsrecht an sämtlichen Inhalten und Arbeitsergebnissen ein.

§ 5 Anregungen und Vorschläge
Sämtliche Anregungen und Verbesserungsvorschläge des Kunden gehen unentgeltlich in das ausschließliche Eigentum des Anbieters über.

§ 6 Freistellung
Der Kunde stellt den Anbieter von sämtlichen Ansprüchen Dritter frei, unabhängig von einem Verschulden des Kunden.

§ 7 Streitbeilegung
Ausschließlicher Gerichtsstand ist der Sitz des Anbieters in Zürich. Die Vertragssprache ist Deutsch.

§ 8 Mitwirkung
Der Anbieter benennt einen Projektleiter und berichtet monatlich über den Leistungsfortschritt.

§ 9 Änderungen
Änderungen dieses Vertrages bedürfen der Textform (E-Mail). Einzelaufträge werden schriftlich vereinbart.`,
    note: "Beratungsvertrag mit Existenzrisiko fuer den Kunden: 24-monatiges nachvertragliches Taetigkeitsverbot ohne Karenz (§ 3), vollstaendige Rechtsabtretung (§ 4), entgeltloser Uebergang aller Ideen auf den Anbieter (§ 5), verschuldensunabhaengige Drittfreistellung (§ 6) und Zuerich als ausschliesslicher Gerichtsstand (§ 7).",
  },

  // ---------------------------------------------------------------------
  // 6. Wartung und Support
  // ---------------------------------------------------------------------
  {
    id: "contract-maintenance-06",
    detector: "deemed_acceptance",
    variant: "deceptive",
    technique: "full-contract",
    severity: "hoch",
    alsoExpect: ["unreasonable_audit", "vicarious_agents", "unilateral_change", "price_escalation"],
    text: `Wartungs- und Supportvertrag

§ 1 Vertragsgegenstand
Der Anbieter erbringt Wartungs- und Supportleistungen für die vom Kunden betriebenen Systeme. Der Leistungsumfang ist in Anlage 1 beschrieben.

§ 2 Servicezeiten und Reaktionszeiten
Der Anbieter reagiert auf Störungen der Priorität 1 innerhalb von vier Stunden während der Servicezeiten. Die Servicezeiten sind Montag bis Freitag von 8 bis 18 Uhr.

§ 3 Abnahme der Wartungsleistungen
Unterbleibt eine Rüge innerhalb von zehn Werktagen, gilt die Leistung als vorbehaltlos abgenommen.

§ 4 Erfüllungsgehilfen und Subunternehmer
Die Haftung des Anbieters für das Verschulden von Erfüllungsgehilfen und Subunternehmern ist ausgeschlossen.

§ 5 Prüfungen und Kontrollen
Der Anbieter ist berechtigt, die Geschäftsräume des Kunden jederzeit ohne Vorankündigung zu betreten und zu auditieren.

§ 6 Anpassung des Leistungsumfangs
Der Anbieter behält sich vor, Leistungsumfang und Vergütung jederzeit einseitig anzupassen.

§ 7 Preisanpassung
Die Vergütung erhöht sich automatisch jährlich um 5 %.

§ 8 Testumgebungen
Der Kunde stellt dem Anbieter die erforderlichen Testumgebungen in geeigneter Form zur Verfügung.

§ 9 Laufzeit
Dieser Vertrag läuft auf unbestimmte Zeit und kann mit einer Frist von drei Monaten gekündigt werden.`,
    note: "Wartungsvertrag: Abnahmefiktion nach zehn Werktagen (§ 3), Ausschluss der Erfuellungsgehilfenhaftung (§ 4), jederzeitiges unangekuendigtes Auditrecht in den Geschaeftsraeumen des Kunden (§ 5), einseitiges Aenderungsrecht an Umfang und Verguetung (§ 6) und automatische Preiseskalation (§ 7).",
  },

  // ---------------------------------------------------------------------
  // 7. Marketing-Agentur-Retainer
  // ---------------------------------------------------------------------
  {
    id: "contract-agency-07",
    detector: "marketing_reference",
    variant: "deceptive",
    technique: "full-contract",
    severity: "mittel",
    alsoExpect: ["feedback_ip_grab", "excessive_survival", "asymmetric_termination", "penalty"],
    text: `Retainer-Vereinbarung Marketing und Kommunikation

§ 1 Leistungsgegenstand
Der Anbieter erbringt Marketing- und Kommunikationsleistungen im Rahmen des vereinbarten Retainers. Einzelmaßnahmen werden monatlich abgestimmt.

§ 2 Vergütung
Der Retainer beträgt 6.500 EUR monatlich und wird jeweils zum Monatsersten fällig. Fremdkosten werden zu Netto-Einkaufspreisen weitergegeben.

§ 3 Referenz und Markennutzung
Der Anbieter ist berechtigt, den Kunden als Referenzkunden zu benennen und das Logo des Kunden zu Werbezwecken zu verwenden.

§ 4 Markenrichtlinien
Für jeden Verstoß gegen die Markenrichtlinien verfällt eine Vertragsstrafe in Höhe von 10.000 EUR.

§ 5 Ideen und Anregungen
Sämtliche Anregungen und Verbesserungsvorschläge des Kunden gehen unentgeltlich in das ausschließliche Eigentum des Anbieters über.

§ 6 Fortwirkende Pflichten
Die Geheimhaltungspflichten gelten zeitlich unbegrenzt fort.

§ 7 Beendigung
Das gesetzliche Recht zur außerordentlichen Kündigung aus wichtigem Grund wird vollumfänglich ausgeschlossen.

§ 8 Reporting
Der Anbieter berichtet monatlich über Kampagnenkennzahlen und abgerechnete Leistungen.

§ 9 Zusammenarbeit
Die Parteien arbeiten vertrauensvoll zusammen und stimmen Maßnahmen im Rahmen der monatlichen Jour fixe ab.`,
    note: "Agenturvertrag: unbegrenzte Referenz- und Logonutzung ohne Zustimmung (§ 3), Strafe fuer Verstoss gegen interne Richtlinien (§ 4), entgeltloser Uebergang aller Kundenvorschlaege (§ 5), zeitlich unbegrenzte Nachwirkung der Geheimhaltung (§ 6) und Ausschluss der Kuendigung aus wichtigem Grund (§ 7).",
  },

  // ---------------------------------------------------------------------
  // 8. Logistikrahmenvertrag
  // ---------------------------------------------------------------------
  {
    id: "contract-logistics-08",
    detector: "penalty",
    variant: "deceptive",
    technique: "full-contract",
    severity: "hoch",
    alsoExpect: ["extended_retention_of_title", "default_interest", "force_majeure_asymmetry", "insolvency_termination"],
    text: `Logistikrahmenvertrag

§ 1 Vertragsgegenstand
Der Anbieter erbringt Transport- und Lagerleistungen für den Kunden. Die einzelnen Ladungen werden per Abruf beauftragt.

§ 2 Abrechnung
Die Abrechnung erfolgt monatlich nach den vereinbarten Frachtsätzen. Die Zahlungsfrist beträgt 30 Tage ab Rechnungszugang.

§ 3 Kapazitätszusage
Der Anbieter stellt die vereinbarte Fahrzeugkapazität an Werktagen bereit. Bei Nichtbereitstellung der vereinbarten Fahrzeugkapazität verfällt eine Vertragsstrafe in Höhe von 5.000 EUR pro Verladetag.

§ 4 Eigentumsvorbehalt an Lagerware
Die gelagerten Waren bleiben bis zum Ausgleich sämtlicher Forderungen aus der gesamten Geschäftsbeziehung Eigentum des Anbieters.

§ 5 Zahlungsverzug
Bei Zahlungsverzug werden Verzugszinsen in Höhe von 12 Prozentpunkten über dem Basiszinssatz berechnet.

§ 6 Höhere Gewalt
Höhere Gewalt befreit den Anbieter von der Leistungspflicht; die Zahlungspflicht des Kunden bleibt unberührt.

§ 7 Insolvenz des Kunden
Der Vertrag endet automatisch mit dem Antrag auf Eröffnung eines Insolvenzverfahrens über das Vermögen des Kunden.

§ 8 Disposition
Beide Parteien benennen einen Ansprechpartner für die Disposition und stimmen Ladefenster wöchentlich ab.

§ 9 Versicherung
Der Anbieter unterhält eine Verkehrshaftpflichtversicherung mit einer Deckungssumme von 1.000.000 EUR.`,
    note: "Logistikvertrag: hohe Tagesstrafe ohne Nachweisvorbehalt (§ 3), Kontokorrent-Eigentumsvorbehalt an der Lagerware des Kunden (§ 4), ueberhoehte Verzugszinsen (§ 5), einseitige Force-Majeure-Klausel mit fortbestehender Zahlungspflicht (§ 6) und unwirksame insolvenzabhaengige Loesungsklausel (§ 7).",
  },

  // ---------------------------------------------------------------------
  // 9. Franchise-/Lizenzvertrag
  // ---------------------------------------------------------------------
  {
    id: "contract-franchise-09",
    detector: "unilateral_change",
    variant: "deceptive",
    technique: "full-contract",
    severity: "hoch",
    alsoExpect: ["price_escalation", "non_compete", "non_solicitation", "excessive_survival"],
    text: `Franchise- und Systemlizenzvertrag

§ 1 Vertragsgegenstand
Der Anbieter räumt dem Kunden das Recht ein, das System unter der Marke des Anbieters zu betreiben. Der Vertrag gilt für das vereinbarte Vertragsgebiet.

§ 2 Vergütung
Der Kunde zahlt eine Eintrittsgebühr von 35.000 EUR sowie eine laufende Lizenzgebühr in Höhe von sechs Prozent des Nettoumsatzes. Die Abrechnung erfolgt monatlich.

§ 3 Anpassung von System und Gebühren
Der Anbieter behält sich vor, Systemhandbuch und Gebühren jederzeit einseitig anzupassen.

§ 4 Laufende Vergütung
Die Lizenzgebühr erhöht sich automatisch jährlich um 5 %.

§ 5 Nachvertragliches Tätigkeitsverbot
Dem Kunden ist es untersagt, für die Dauer von 24 Monaten nach Beendigung des Vertrages im Geschäftsbereich des Anbieters tätig zu werden; eine Karenzentschädigung wird nicht gewährt.

§ 6 Personalgewinnung
Dem Kunden ist es untersagt, Mitarbeiter des Anbieters abzuwerben, wobei jeder Verstoß mit einer Vertragsstrafe von 10.000 EUR geahndet wird.

§ 7 Fortwirkende Pflichten
Die Geheimhaltungspflichten gelten zeitlich unbegrenzt fort.

§ 8 Schulungen
Der Kunde nimmt an den angebotenen Schulungen teil und stellt geeignetes Personal bereit.

§ 9 Systemhandbuch
Der Anbieter stellt ein Systemhandbuch in der jeweils gültigen Fassung bereit.`,
    note: "Franchisevertrag: einseitiges Aenderungsrecht an System und Gebuehren (§ 3), automatische Gebuehreneskalation (§ 4), 24-monatiges Taetigkeitsverbot ohne Karenz (§ 5), Abwerbeverbot mit Strafe (§ 6) und zeitlich unbegrenzte Nachwirkung (§ 7).",
  },

  // ---------------------------------------------------------------------
  // 10. Versicherungsmaklervertrag
  // ---------------------------------------------------------------------
  {
    id: "contract-insurance-brokerage-10",
    detector: "third_party_indemnity",
    variant: "deceptive",
    technique: "full-contract",
    severity: "hoch",
    alsoExpect: ["burden_of_proof", "tax_indemnity_shift", "asymmetric_confidentiality", "liability"],
    text: `Makler- und Beratungsvertrag gewerbliche Versicherungen

§ 1 Vertragsgegenstand
Der Anbieter vermittelt und betreut Versicherungsverträge für den Kunden im gewerblichen Bereich. Die Betreuung umfasst die laufende Bestandsverwaltung.

§ 2 Vollmachten
Der Kunde erteilt dem Anbieter die für die Vermittlung erforderlichen Vollmachten. Eine Vollmacht zur Annahme von Zahlungen wird nicht erteilt.

§ 3 Freistellung
Der Kunde stellt den Anbieter von sämtlichen Ansprüchen Dritter frei, unabhängig von einem Verschulden des Kunden.

§ 4 Beweislast
Die Beweislast für das Vorliegen eines Beratungsfehlers obliegt dem Kunden.

§ 5 Steuern und Abgaben
Sämtliche Umsatzsteuern sowie steuerliche Nachforderungen sind vom Kunden nachzuentrichten.

§ 6 Vertraulichkeit
Der Kunde verpflichtet sich, alle ihm bekannt gewordenen Informationen streng vertraulich zu behandeln.

§ 7 Haftung
Die Haftung des Anbieters für Schäden jeglicher Art wird vollumfänglich ausgeschlossen.

§ 8 Vergütung
Die Vergütung erfolgt als Courtage gemäß der gesonderten Provisionsvereinbarung.

§ 9 Laufzeit
Der Vertrag kann von beiden Parteien mit einer Frist von drei Monaten zum Quartalsende gekündigt werden.`,
    note: "Maklervertrag: verschuldensunabhaengige Drittfreistellung (§ 3), Beweislastumkehr zu Lasten des Kunden (§ 4), Abwaelzung steuerlicher Nachforderungen (§ 5), einseitige Vertraulichkeitspflicht (§ 6) und Totalausschluss der Anbieterhaftung (§ 7). Die Kuendigungsklausel in § 9 ist demgegenueber ausgewogen und dient als Kontrast.",
  },

  // ---------------------------------------------------------------------
  // 11. Cloud-Reseller
  // ---------------------------------------------------------------------
  {
    id: "contract-cloud-reseller-11",
    detector: "data_lockin",
    variant: "deceptive",
    technique: "full-contract",
    severity: "hoch",
    alsoExpect: ["data_grant", "subcontractor_uncontrolled", "service_suspension", "liability"],
    text: `Cloud-Reseller- und Wiederverkaufsvertrag

§ 1 Vertragsgegenstand
Der Anbieter stellt dem Kunden Cloud-Ressourcen eines Plattformbetreibers zur Weiterveräußerung bereit. Die Ressourcen werden über das Partnerportal bezogen.

§ 2 Abrechnung
Die Abrechnung erfolgt monatlich auf Basis der tatsächlich verbrauchten Ressourcen. Preislisten werden quartalsweise aktualisiert.

§ 3 Datenrückgabe
Nach Beendigung des Vertrages ist die Herausgabe der Daten kostenpflichtig und wird nach Aufwand abgerechnet.

§ 4 Nutzungsrechte an Inhalten
Der Kunde räumt dem Anbieter ein uneingeschränktes, unwiderrufliches und übertragbares Nutzungsrecht an sämtlichen Inhalten ein.

§ 5 Subunternehmer
Der Anbieter ist berechtigt, Subunternehmer einzusetzen und personenbezogene Daten ohne vorherige Ankündigung an diese weiterzugeben.

§ 6 Sperre des Zugangs
Bei Zahlungsverzug ist der Anbieter berechtigt, den Zugang des Kunden sofort ohne Vorankündigung zu sperren.

§ 7 Haftung
Die Haftung des Anbieters für Schäden jeglicher Art wird vollumfänglich ausgeschlossen.

§ 8 Support
Der Kunde benennt einen technischen Ansprechpartner für Abrechnungsfragen. Supportanfragen werden über das Portal gestellt.

§ 9 Anwendbares Recht
Es gilt deutsches Recht. Gerichtsstand ist der Sitz des Anbieters.`,
    note: "Reseller-Vertrag: kostenpflichtige Datenherausgabe als Wechselbarriere (§ 3), umfassende Rechtsabtretung (§ 4), unangekuendigte Weitergabe personenbezogener Daten an Subunternehmer (§ 5), sofortige Sperre bei Zahlungsverzug (§ 6) und pauschaler Haftungsausschluss (§ 7).",
  },

  // ---------------------------------------------------------------------
  // 12. IT-Outsourcing mit Subunternehmern
  // ---------------------------------------------------------------------
  {
    id: "contract-outsourcing-12",
    detector: "subcontractor_uncontrolled",
    variant: "deceptive",
    technique: "full-contract",
    severity: "hoch",
    alsoExpect: ["unreasonable_audit", "asymmetric_confidentiality", "written_form_trap", "burden_of_proof"],
    text: `IT-Outsourcing-Vereinbarung

§ 1 Vertragsgegenstand
Der Anbieter übernimmt den Betrieb der IT-Infrastruktur des Kunden im Rahmen des Servicevertrages. Die Servicebeschreibung ist in Anlage 3 enthalten.

§ 2 Berichterstattung
Der Anbieter stellt eine wöchentliche Statusberichterstattung sowie eine monatliche Servicebilanz bereit.

§ 3 Weitergabe von Leistungen und Daten
Der Anbieter ist berechtigt, Subunternehmer einzusetzen und personenbezogene Daten ohne vorherige Ankündigung an diese weiterzugeben.

§ 4 Prüfungen und Kontrollen
Der Anbieter ist berechtigt, die Geschäftsräume des Kunden jederzeit ohne Vorankündigung zu betreten und zu auditieren.

§ 5 Vertraulichkeit
Der Kunde verpflichtet sich, alle ihm bekannt gewordenen Informationen streng vertraulich zu behandeln.

§ 6 Erklärungen und Form
Jede Kündigung bedarf der eigenhändig unterzeichneten Schriftform und muss per eingeschriebenem Brief mit Rückschein zugehen.

§ 7 Beweislast bei Störungen
Die Beweislast für Störungen der Infrastruktur obliegt dem Kunden.

§ 8 Vergütung
Die Vergütung beträgt 18.500 EUR monatlich und ist innerhalb von 30 Tagen nach Rechnungszugang zu zahlen.

§ 9 Anlagen
Anlagen sind Bestandteil dieses Vertrages. Nebenabreden bedürfen der Textform.`,
    note: "Outsourcing-Vertrag: unangekuendigte Weitergabe personenbezogener Daten an Subunternehmer (§ 3), jederzeitiges unangekuendigtes Auditrecht (§ 4), einseitige Vertraulichkeitspflicht (§ 5), Formfalle mit Rueckschein (§ 6) und Beweislastumkehr fuer Betriebsstoerungen (§ 7).",
  },

  // ---------------------------------------------------------------------
  // 13. Arbeitnehmerueberlassung / Staffing
  // ---------------------------------------------------------------------
  {
    id: "contract-staffing-13",
    detector: "non_solicitation",
    variant: "deceptive",
    technique: "full-contract",
    severity: "hoch",
    alsoExpect: ["non_compete", "notice_period", "fictitious_receipt", "jurisdiction"],
    text: `Personalüberlassungsvertrag (Staffing)

§ 1 Vertragsgegenstand
Der Anbieter überlässt dem Kunden Fachkräfte zur vorübergehenden Beschäftigung. Einzelheiten ergeben sich aus den jeweiligen Überlassungsaufträgen.

§ 2 Abrechnung
Die Abrechnung erfolgt nach geleisteten Stunden zu den vereinbarten Stundensätzen. Reisezeiten werden nicht vergütet.

§ 3 Abwerbeverbot
Dem Kunden ist es untersagt, Mitarbeiter des Anbieters abzuwerben, wobei jeder Verstoß mit einer Vertragsstrafe von 15.000 EUR geahndet wird.

§ 4 Nachvertragliches Tätigkeitsverbot
Dem Kunden ist es untersagt, für die Dauer von 18 Monaten nach Beendigung des Vertrages im Geschäftsbereich des Anbieters tätig zu werden; eine Karenzentschädigung wird nicht gewährt.

§ 5 Laufzeit und Kündigung
Die Mindestlaufzeit beträgt 24 Monate. Der Vertrag kann nur mit einer Frist von zwölf Monaten gekündigt werden.

§ 6 Erklärungen
Mitteilungen und Rechnungen gelten nach Absendung als zugegangen.

§ 7 Streitigkeiten
Ausschließlicher Gerichtsstand ist der Sitz des Anbieters in München.

§ 8 Arbeitssicherheit
Der Kunde stellt einen geeigneten Arbeitsplatz bereit und hält die gesetzlichen Vorschriften des Arbeitsschutzes ein.

§ 9 Einsatzsteuerung
Der Kunde erteilt fachliche Weisungen im Rahmen der Überlassung und meldet Ausfälle unverzüglich.`,
    note: "Staffing-Vertrag: Abwerbeverbot mit hoher Strafe (§ 3), 18-monatiges Taetigkeitsverbot ohne Karenz (§ 4), zwangsweise Bindung durch zwölfmonatige Kuendigungsfrist (§ 5), Zugangsfiktion fuer Rechnungen und Erklaerungen (§ 6) sowie ausschliesslicher Gerichtsstand am Anbietersitz (§ 7).",
  },

  // ---------------------------------------------------------------------
  // 14. ENGLISCH: Master Services Agreement (US-Stil)
  // ---------------------------------------------------------------------
  {
    id: "contract-en-msa-14",
    detector: "liability",
    variant: "deceptive",
    technique: "full-contract",
    severity: "hoch",
    alsoExpect: ["third_party_indemnity", "jurisdiction", "setoff_restriction", "as_is_software_disclaimer", "entire_agreement"],
    text: `MASTER SERVICES AGREEMENT

1. Scope of Services
Provider shall make the Services available to Customer in accordance with each Statement of Work executed by the parties.

2. Fees and Invoicing
Customer shall pay all undisputed invoices within thirty (30) days of receipt. Fees are exclusive of taxes.

3. Term
This Agreement commences on the Effective Date and continues for an initial term of twelve (12) months.

4. Limitation of Liability
In no event shall Provider be liable for any indirect, incidental or consequential damages, including lost profits and business interruption.

5. Indemnification
Customer shall defend and hold harmless Provider against any claims arising out of Customer's use of the Services.

6. Disclaimer of Warranties
The Services are provided as is without warranty of any kind, express or implied.

7. Payment Restrictions
Customer shall pay all amounts without any set-off or deduction.

8. Governing Law and Venue
This Agreement shall be governed by the laws of the State of Delaware, and the parties submit to the exclusive jurisdiction of the courts of Wilmington.

9. Confidentiality
Each party shall protect the other party's Confidential Information using reasonable care.

10. Miscellaneous
This Agreement constitutes the entire agreement between the parties and supersedes all prior proposals and understandings. Either party may terminate for material breach upon thirty (30) days' written notice and an opportunity to cure.`,
    note: "Englische MSA (US-Stil), bewusst als Blindspot-Test: Haftungsausschluss fuer Folgeschaeden (4), einseitige Verteidigungs- und Freistellungspflicht des Kunden (5), As-is-Ausschluss der Gewaehrleistung (6), Aufrechnungsverbot (7), Delaware/Wilmington als Gerichtsstand (8) und Entwertung aller vorvertraglichen Zusagen (10).",
  },

  // ---------------------------------------------------------------------
  // 15. ENGLISCH: SaaS Terms of Service
  // ---------------------------------------------------------------------
  {
    id: "contract-en-saas-15",
    detector: "auto_renewal",
    variant: "deceptive",
    technique: "full-contract",
    severity: "hoch",
    alsoExpect: ["unverbindliche_sla_falle", "liability", "jurisdiction", "entire_agreement"],
    text: `SAAS SUBSCRIPTION TERMS

1. Access Rights
Provider grants Customer a non-exclusive, non-transferable right to access the hosted platform during the term.

2. Subscription Fees
Customer shall pay the subscription fee annually in advance. Late payments may result in suspension of access.

3. Renewal
This Agreement shall automatically renew for successive twelve (12) month renewal terms unless either party gives written notice at least sixty (60) days prior to the end of the then-current term.

4. Availability
Provider will use commercially reasonable efforts to maintain uptime of 99.9% but does not warrant uninterrupted availability.

5. Data and Content
Customer grants Provider a perpetual, irrevocable, worldwide licence to use, reproduce and commercialise any data uploaded to the platform.

6. Liability
Provider shall not be liable for any loss of data, lost profits or business interruption.

7. Governing Law
The parties submit to the exclusive jurisdiction of the courts of New York.

8. Entire Agreement
This Agreement constitutes the entire agreement between the parties and supersedes all prior proposals and understandings.

9. Termination
Either party may terminate for material breach upon thirty (30) days' written notice and an opportunity to cure.`,
    note: "Englische SaaS-Bedingungen: automatische Verlaengerung um zwoelf Monate mit 60-Tage-Frist (3), entkernter Verfuegbarkeitsanspruch (4), unbefristete Lizenz an hochgeladenen Daten (5), Haftungsfreizeichnung (6), New York als Gerichtsstand (7) und vollstaendige Entwertung vorvertraglicher Zusagen (8).",
  },
];

export default CASES;
