const { test } = require('node:test');
const assert = require('node:assert');
const { classify } = require('./classifier.js');

// Realistic German document samples
const SAMPLES = {
  bank_mahnung: `Commerzbank AG
Kundennummer: 1234567890

Zahlungserinnerung / Mahnung

Sehr geehrte Kundin,
wir erinnern an den offenen Betrag von 249,50 EUR bis zum 15.10.2026. Bitte überweisen Sie den Betrag unter Angabe der Kundennummer. Bei Nichtzahlung entstehen Mahngebühren.`,
  bank_kontoauszug: `Sparkasse KölnBonn
IBAN: DE12 1234 5678 9012 3456 78

Kontoauszug Nr. 54

Kontostand: 1.234,56 EUR
Wertstellung 15.09.2026`,
  bank_gebuehren: `Deutsche Bank AG
Preisliste für Kontoführung

Kontogebühr: 8,90 EUR monatlich
Grundgebühr und Kontoentgelt ab Oktober 2026`,
  bank_kredit: `ING-DiBa AG
Darlehensvertrag

Kreditvertrag Nr. 987654
Ratenkredit über 15.000 EUR
Tilgungsplan beigefügt`,
  versicherung_kfz: `HUK-Coburg
KFZ-Versicherung

Versicherungsbestätigung / Schadenfreiheitsklasse
Fahrzeugversicherung Haftpflicht und Kaskoversicherung für VW Golf`,
  versicherung_schaden: `Allianz Versicherung
Ihre Schadenmeldung

Schadensnummer: 2026-1188
Wir bestätigen den Eingang Ihrer Schadenmeldung vom 08.09.2026`,
  versicherung_haftpflicht: `AXA Versicherung
Privathaftpflichtversicherung

Ihr Versicherungsschein
Privathaftpflicht für Familie Mustermann`,
  versicherung_hausrat: `HUK24
Hausratversicherung

Ihr Versicherungsvertrag
Hausratversicherungsschutz ab 01.10.2026`,
  krankenkasse: `Techniker Krankenkasse
Mitgliedsbescheinigung

Ihre Versichertennummer: A123456789
Wir bestätigen Ihre Mitgliedschaft in der Krankenversicherung`,
  rundfunk: `ARD ZDF Deutschlandradio Beitragsservice
Beitragsnummer: 1234567890

Mahnung Rundfunkbeitrag
Bitte überweisen Sie den offenen Rundfunkbeitrag von 18,36 EUR monatlich.`,
  steuerbescheid: `Finanzamt München
Einkommensteuerbescheid 2025

Festgesetzte Steuer: 2.300 EUR
Steuerguthaben wird erstattet`,
  steuererinnerung: `Finanzamt Berlin
Erinnerung zur Abgabe der Steuererklärung

Sehr geehrte Damen und Herren,
die Steuererklärung für das Jahr 2025 ist noch nicht eingegangen. Bitte reichen Sie diese bis zum 15.09.2026 ein.`,
  steuer_id: `Bundeszentralamt für Steuern
Ihre Steuer-Identifikationsnummer

Steuer-ID: 42 819 304 912
Zuteilung der Steuer-Identifikationsnummer`,
  rentenversicherung: `Deutsche Rentenversicherung Bund
Versicherungsnummernachweis

Ihre Versicherungsnummer: 12 150896 M 014
Sozialversicherungsnummer nachgewiesen`,
  renteninfo: `Deutsche Rentenversicherung
Renteninformation

Ihr Rentenanspruch und Ihre Rentenauskunft
Voraussichtliche Altersrente`,
  jobcenter_mitwirkung: `Jobcenter Berlin Mitte
Aufforderung zur Mitwirkung

Bitte reichen Sie Ihre Mietbescheinigung bis zum 08.09.2026 ein.`,
  jobcenter_bewilligung: `Jobcenter Hamburg
Bewilligungsbescheid Bürgergeld

Regelbedarf und Kosten der Unterkunft
Monatliche Leistungen werden ausgezahlt`,
  kindergeld: `Familienkasse
Kindergeldbescheid

Ihr Kindergeld für 2 Kinder
Monatlicher Betrag 500 EUR`,
  auslaender: `Landesamt für Einwanderung Berlin
Aufenthaltsangelegenheit

Ihr Aufenthaltstitel
Aufforderung zur Vorlage von Nachweisen`,
  melde: `Bürgeramt Berlin
Meldebestätigung

Bestätigung über die Anmeldung Ihres Wohnsitzes`,
  mahnbescheid: `Amtsgericht Köln
Gerichtlicher Mahnbescheid

Sie haben 2 Wochen Widerspruchsfrist
Mahngebühr und Forderung von 1.200 EUR`,
  bussgeld: `Polizei Düsseldorf
Bußgeldbescheid

Ordnungswidrigkeit: Geschwindigkeitsüberschreitung
Geldbuße 150 EUR, Einspruch innerhalb 2 Wochen`,
  ausweis: `Bürgeramt
Abholung bereit

Ihr neuer Personalausweis liegt zur Abholung
Abholbenachrichtigung`,
  online_ausweis: `Bundesdruckerei
PIN-Brief zur Online-Ausweisfunktion

Transport-PIN: 12345
Sperrkennwort für die eID-Funktion`,
  fuehrungszeugnis: `Bundesamt für Justiz
Führungszeugnis

Keine Eintragung
Zentralregister`,
  arbeitszeugnis: `Muster GmbH
Arbeitszeugnis

Leistung und Verhalten mit vollster Zufriedenheit
Arbeitszeugnis für Frau Mustermann`,
  schulzeugnis: `Carl-Severing-Berufskolleg
Jahreszeugnis der Berufsschule

Schuljahr 2025/2026 Noten`,
  hochschulzeugnis: `Technische Universität München
Bachelor of Science Urkunde

Hochschulabschluss Transcript of Records`,
  sprachzertifikat: `Goethe-Institut
Goethe-Zertifikat B2

Deutsch-Test für Zuwanderer
Sprachzertifikat Deutsch`,
  arbeitsunfaehig: `Praxis Dr. Schmidt
Arbeitsunfähigkeitsbescheinigung

AU bis 20.10.2026 Krankschreibung`,
  nebenkosten: `Hausverwaltung Schmidt
Betriebskostenabrechnung 2025

Aufstellung der Betriebskosten
Nachzahlung von 300 EUR`,
  stadtwerke: `Stadtwerke München
Jahresverbrauchsabrechnung Strom 2025

Abschlag und Verbrauchsabrechnung`,
  mietvertrag: `Vermieter Max Weber
Mietvertrag für Wohnung

Kaltmiete 850 EUR Warmmiete 1.100 EUR`,
  wohngeld: `Wohngeldstelle Köln
Wohngeldbescheid 2026

Bewilligung von Wohngeld
Monatlich 210 EUR`,
  bafoeg: `Amt für Ausbildungsförderung
BAföG-Bescheid

Ausbildungsförderung ab Oktober 2026
Monatlicher Förderbetrag 750 EUR`,
  versicherung_hausrat3: `HUK24
Hausratversicherung

Ihr Versicherungsvertrag
Hausratversicherungsschutz ab 01.10.2026`,
  ausweis_abholung2: `Bürgeramt Stuttgart
Abholung bereit

Ihr neuer Ausweis liegt bereit
Abholbenachrichtigung`,
  pflegeversicherung: `Pflegekasse der Allianz
Pflegeversicherung

Ihr Pflegegrad wird überprüft
Pflegebedürftigkeit ab 01.10.2026`,
  versicherungsverlauf: `Deutsche Rentenversicherung
Versicherungsverlauf

Ihre Versicherungszeiten und Beiträge
Sozialversicherungsverlauf 2026`,
  lebensversicherung: `Allianz Leben
Lebensversicherung

Ihre Kapitallebensversicherung
Ablauf und Auszahlung`,
  kuendigung: `Vermieter Max Weber
Kündigung des Mietverhältnisses

Kündigungsfrist bis 30.09.2026
Beendigung des Mietvertrags zum 31.12.2026`,
  elster: `Finanzamt Berlin
ELSTER-Zugangsdaten

Elektronische Steuererklärung
Ihre Zugangsdaten zum Finanzamt`,
  gez_befreiung: `ARD ZDF Deutschlandradio Beitragsservice
Befreiungsantrag Rundfunkbeitrag

Vorlage eines Nachweises zur Befreiung
Befreiung vom Rundfunkbeitrag`,
  iban_nachweis: `Deutsche Bank
IBAN-Benachrichtigung

Unsere IBAN für Ihre Zahlung
Zahlungsempfänger IBAN: DE12 1234 5678`,
  versicherungsschein: `Allianz Versicherung
Versicherungsschein

Ihre Police und Versicherungspolice
Versicherungsbeginn 01.11.2026`,
  bonusprogramm: `Techniker Krankenkasse
Bonusprogramm

Bonusheft und Prämiensprogramm
Gesundheitskarte Bonus`,
  vollstreckung: `Amtsgericht Berlin
Vollstreckungsankündigung

Kontopfändung angeordnet
Gerichtsvollzieher angekündigt`,
  kfz_steuer: `Zollamt München
Kraftfahrzeugsteuerbescheid

Kfz-Steuer für Ihr Fahrzeug
Festsetzung der Fahrzeugsteuer`,
  abmahnung: `Geschäftsführung Muster GmbH
Arbeitsrechtliche Abmahnung

Formalabmahnung wegen Verhaltens
Verweis und Verhaltenshinweis`,
  einbuergerung: `Stadt Köln
Einbürgerungsantrag

Einbürgerung und Staatsangehörigkeit
Einbürgerungsbescheid erteilt`,
  arbeitslosengeld: `Agentur für Arbeit
Arbeitslosengeld-Bescheid

ALG 1 Leistungsbescheid
Bewilligung von Arbeitslosengeld`,
  beitragserhoehung: `Allianz Versicherung
Beitragserhöhung

Tariferhöhung und Prämienanpassung
Ihre Beiträge ab 2027`,
  sepa_mandat: `Deutsche Bank
SEPA-Lastschriftmandat

Ihr SEPA-Mandat und Mandatsreferenz
Gläubiger-ID für Abbuchung`,
  umzugsmitteilung: `Bürgeramt Hamburg
Umzugsmitteilung

Ihr Wohnungswechsel wurde registriert
Abmeldebescheinigung zum Nachweis`,
  rentenbescheid: `Deutsche Rentenversicherung
Rentenbescheid

Ihre Regelaltersrente wird festgesetzt
Rentenberechnung und Zahlung ab November 2026`,
  krankenkassen_bescheid: `Techniker Krankenkasse
Krankenkassenbescheid

Selbsteinkommen und Beitragsnachweis
Zahlungsaufforderung für Beiträge`,
  beitragsbescheid: `AOK Bayern
Beitragsbescheid

Beitragsnachweis Ihres Mitgliedsbeitrags
Beitragsrechnung für Oktober 2026`,
  steuer_erstattung: `Finanzamt Hamburg
Steuererstattung 2025

Ihr Erstattungsbetrag wird überwiesen
Steuerauszahlung auf Ihr Konto`,
  arbeitsgericht_klage: `Arbeitsgericht Berlin
Kündigungsschutzklage

Ihre Kündigungsschutzklage ist eingegangen
Klage gegen die Kündigung des Arbeitsverhältnisses`,
  widerspruch: `Finanzamt München
Widerspruch gegen den Bescheid

Ihre Widerspruchsfrist beträgt 1 Monat
Rechtsbehelf gegen den Einkommensteuerbescheid`,
  inkasso: `Credireform Inkasso
Inkassobüro

Zahlungsaufforderung und Forderungsmanagement
Inkasso für offene Rechnung`,
  schufa_auskunft: `SCHUFA Holding
Bonitätsauskunft

Selbstauskunft Ihrer Kreditwürdigkeit
Datenübersicht zur SCHUFA`,
  kontoschliessung: `Deutsche Bank
Kontoschließung

Ihr Konto wird zum 30.11.2026 geschlossen
Kontoauflösung und Restguthaben`,
  gez_nachzahlung: `ARD ZDF Deutschlandradio
Nachzahlung Rundfunkbeitrag

Säumniszuschlag und ausstehende Beiträge
Zahlungsaufforderung nach Rückstand`,
  elterngeld: `Elterngeldstelle
Elterngeldbescheid

Elterngeld für Ihr Kind
Basiselterngeld wird ausgezahlt`,
  steuer_nachzahlung: `Finanzamt Köln
Steuernachzahlung 2025

Zu zahlende Steuer 900 EUR
Nachzahlung bis zum 20.11.2026`,
  widerspruchsbescheid: `Finanzamt Frankfurt
Widerspruchsbescheid

Ihrem Widerspruch wird teilweise stattgegeben
Bescheid zum Widerspruch`,
  zahnersatz: `AOK Rheinland
Zahnersatz

Kostenvoranschlag für Zahnersatz
Heilmittel und Krankenkasse`,
  dauerauftrag: `Commerzbank
Dauerauftragsbestätigung

Terminüberweisung und Dauerauftrag
Regelüberweisung eingerichtet`,
  stadtwerke_nachzahlung: `Stadtwerke Essen
Verbrauchsabrechnung Strom 2025

Nachzahlung fällig
Stromabrechnung mit Nachzahlungsbetrag`,
  krankenkasse_einkommen: `Techniker Krankenkasse
Einkommensnachweis

Fragebogen zur Familienversicherung
Bitte Einkommensnachweis einreichen`,
  krankenkassen_wechsel: `AOK Bayern
Krankenkassenwechsel

Ihr Kassenwechsel wird bearbeitet
Mitgliedschaftswechsel bestätigt`,
  pflegekasse_bescheid: `Pflegekasse der AOK
Pflegegradbescheid

Ihr Pflegegrad wurde festgestellt
Bescheid zum Pflegegrad 3`,
  bafoeg_widerruf: `Amt für Ausbildungsförderung
Widerruf des BAföG-Bescheids

Widerruf der Ausbildungsförderung
Aufhebung des Förderungsbescheids`,
  steuerbescheid_einspruch: `Finanzamt Berlin
Einspruch gegen den Einkommensteuerbescheid

Einspruchsfrist zum Steuerbescheid
Ihr Einspruch wird geprüft`,
  alg_widerspruch: `Agentur für Arbeit
Widerspruchsbescheid Arbeitslosengeld

Ihr Widerspruch gegen den ALG-Bescheid
Widerspruchsbescheid zum Arbeitslosengeld`,
  rentenbescheid_nachzahlung: `Deutsche Rentenversicherung
Rentenbescheid mit Nachzahlung

Nachzahlungsbetrag wird überwiesen
Rentennachzahlung ab November`,
  gez_kontoauszug: `ARD ZDF Deutschlandradio
Kontoauszug zum Rundfunkbeitrag

Zahlungsübersicht Ihres Beitragskontos
Rundfunkbeitrag Kontoauszug`,
  steuer_korrektur: `Finanzamt Düsseldorf
Änderungsbescheid Steuer

Korrektur des Einkommensteuerbescheids
Steuerbescheid wurde geändert`,
  krankenkassen_zahnersatz: `AOK Berlin
Bescheid über Zahnersatz

Ihr Zahnersatz-Antrag wurde genehmigt
Krankenkasse Zahnersatz Bescheid`,
  alg_bewilligung: `Agentur für Arbeit
ALG-Bewilligungsbescheid

Arbeitslosengeld-Bewilligungsbescheid
Ihr ALG wird bewilligt`,
  gebuehrenbescheid: `Bürgeramt Düsseldorf
Gebührenbescheid

Verwaltungsgebühr für Ausweisdokument
Amtliche Gebühr 37,00 EUR`,
  versicherungswiderspruch: `HUK-Coburg
Widerspruch zur Versicherung

Ihr Widerspruch zum Versicherungsschein
Leistungsfall Widerspruch`,
  elterngeld_widerruf: `Elterngeldstelle
Widerruf des Elterngeld-Bescheids

Rückforderung des Elterngeldes
Aufhebung des Elterngeldbescheids`,
  abschlagsaenderung: `Stadtwerke Köln
Abschlagsänderung Strom

Ihr monatlicher Abschlag wurde erhöht
Abschlag neu berechnet`,
  steuerbescheid_widerruf: `Finanzamt München
Widerruf des Steuerbescheids

Widerruf der Steuerfestsetzung
Ihr Steuerbescheid wird widerrufen`,
  bafoeg_zulage: `Amt für Ausbildungsförderung
BAföG-Zulage

Auslandszuschlag zum BAföG
Bedarfssatz Zulage bewilligt`,
  krankenkassen_befreiung: `AOK Niedersachsen
Beitragsbefreiung

Befreiung vom Krankenversicherungsbeitrag
Sie sind von Beiträgen befreit`,
  alg_ablehnung: `Agentur für Arbeit
Ablehnungsbescheid ALG

Ihre ALG-Leistung wurde abgelehnt
Ablehnung des Arbeitslosengeldes`,
  kfz_versicherungserhoehung: `HUK-Coburg
Kfz-Beitragserhöhung

Ihr Kfz-Versicherungsbeitrag wird erhöht
Beitragserhöhung zur Kfz-Versicherung`,
  vermieterwechsel: `Mietverwaltung Weber
Vermieterwechsel

Neuer Eigentümer der Wohnung
Ihr Mietverhältnis geht über`,
  gez_teilerstattung: `ARD ZDF Deutschlandradio
Teilerstattung Rundfunkbeitrag

Rückerstattung Ihres Beitrags
Rundfunkbeitrag wurde erstattet`,
  rentenueberzahlung: `Deutsche Rentenversicherung
Rentenüberzahlung

Zu viel gezahlte Rente wird zurückgefordert
Rückforderung der Rentenzahlung`,
  pfaendungsschutz: `Deutsche Bank
Pfändungsschutzkonto

Ihr Konto wurde als Schutzkonto eingerichtet
Pfändungsschutz für das Konto`,
  abschlagszahlung: `Stadtwerke Düsseldorf
Abschlagszahlung

Ihr monatlicher Abschlag ist fällig
Abschlag bitte zahlen`,
  renten_widerspruchsbescheid: `Deutsche Rentenversicherung
Renten-Widerspruchsbescheid

Ihr Widerspruch zur Rente
Widerspruchsbescheid zum Rentenbescheid`,
  mitgliedsbeitrag_rueck: `AOK Hessen
Beitragsrückerstattung

Ihr Mitgliedsbeitrag wurde erstattet
Beitragsgutschrift Krankenkasse`,
  alg_weiterbewilligung: `Agentur für Arbeit
Weiterbewilligungsbescheid ALG

Ihr Arbeitslosengeld wird fortgezahlt
ALG-Weiterbewilligung zum Leistungsbescheid`,
  kfz_wechsel: `HUK-Coburg
Kfz-Versicherungswechsel

Sie wechseln den Kfz-Versicherer
Versicherungswechsel der Police`,
  mietminderung: `Hausverwaltung Weber
Mietminderung

Ihre Miete wird wegen Mängeln gemindert
Minderung der Mietzahlung`,
  gez_zahlungsplan: `ARD ZDF Deutschlandradio
Zahlungsplan Rundfunkbeitrag

Ratenzahlung für den Rundfunkbeitrag
Stundung des Beitrags`,
  konto_ueberziehung: `Commerzbank
Kontoüberziehung

Ihr Konto wurde überzogen
Überziehungszins und Dispokredit`,
  stadtwerke_steuer: `Stadtwerke München
Stromsteuerbescheid

Energiesteuer in Ihrer Jahresabrechnung
Stromsteuer ausgewiesen`,
  bafoeg_elternunabhaengig: `Amt für Ausbildungsförderung
Elternunabhängiges BAföG

Elternunabhängige Förderung bewilligt
BAföG ohne Einkommen der Eltern`,
  mitgliedsbescheinigung_beitraege: `Techniker Krankenkasse
Mitgliedsbescheinigung mit Beiträgen

Mitgliedsnachweis und Beitragsübersicht
Ihre Beiträge zur Krankenversicherung`,
  alg_antrag: `Agentur für Arbeit
Antrag auf Arbeitslosengeld

Sie stellen einen ALG-Antrag
Antragsformular zum Arbeitslosengeld`,
  kfz_schadenregulierung: `HUK-Coburg
Kfz-Schadenregulierung

Unfallbericht zur Versicherung
Regulierung des Fahrzeugschadens`,
  mieterhoehung: `Hausverwaltung Weber
Mieterhöhung

Ihre Miete wird angehoben
Erhöhung der Mietzahlung`,
  rentenantrag: `Deutsche Rentenversicherung
Rentenantrag

Sie beantragen Altersrente
Rentenantrag eingegangen`,
  kontokuendigung: `Commerzbank
Kontokündigung

Sie kündigen Ihr Girokonto
Kündigung des Kontos zum 31.12.2026`,
  verbrauchsabschlag: `Stadtwerke München
Verbrauchsabschlag

Ihr Abschlag basiert auf dem Verbrauch
Abschlagszahlung Strom`,
  bafoeg_ablehnung: `Amt für Ausbildungsförderung
Ablehnung des BAföG-Antrags

Ihr BAföG wurde abgelehnt
Ablehnungsbescheid zur Förderung`,
  steuer_datenuebersicht: `Finanzamt München
Steuer-Datenübersicht

Übersicht Ihres Steuerkontos
Meine Daten beim Finanzamt`,
  gesundheitskur: `Techniker Krankenkasse
Gesundheitskur / Reha

Ihre Rehabilitationsmaßnahme wird genehmigt
Vorsorgekur durch die Krankenkasse`,
  alg_bemessung: `Agentur für Arbeit
ALG-Bemessungsbescheid

Bemessungsgrundlage Ihres Arbeitslosengeldes
Bemessungsbescheid zum ALG`,
  kfz_vollkasko: `HUK-Coburg
Kfz-Vollkaskoversicherung

Vollkasko für Ihr Fahrzeug
Teilkasko und Vollkasko`,
  mietkaution: `Vermieter Weber
Mietkautionsabrechnung

Rückzahlung der Kaution
Kautionsabrechnung Ihrer Wohnung`,
  gez_befreiungs_ablehnung: `ARD ZDF Deutschlandradio
Befreiung abgelehnt

Ihr Befreiungsantrag wurde abgewiesen
Befreiung vom Rundfunkbeitrag verweigert`,
  renten_volksversicherung: `Deutsche Rentenversicherung
Rentenversicherung Volksversicherung

Ihr Rentenanspruch bei der Volksversicherung
Sozialversicherung Volksversicherung`,
  konto_wechselbank: `Commerzbank
Kontowechsel

Sie wechseln zu einer neuen Bank
Kontowechselservice Ihrer Bank`,
  tarifwechsel: `Stadtwerke Köln
Tarifwechsel Strom

Ihr Stromtarif wurde umgestellt
Wechsel zu einem neuen Tarif`,
  bafoeg_vorleistung: `Amt für Ausbildungsförderung
BAföG-Vorleistungsbescheid

Vorleistung der Ausbildungsförderung
Vorleistungsbescheid zum BAföG`,
  steuer_bescheidankuendigung: `Finanzamt Düsseldorf
Ankündigung Steuerbescheid

Sie erhalten in Kürze einen Bescheid
Vorankündigung zur Steuerfestsetzung`,
  krankenkasse_aenderung: `Techniker Krankenkasse
Krankenkassenänderung

Änderungsmitteilung zur Mitgliedschaft
Personalienänderung bei der Krankenkasse`,
  kfz_steuer_neu: `Zollamt München
Kfz-Steuer Neufestsetzung

Neufestsetzung der Fahrzeugsteuer
Kfz-Steueränderung für Ihr Fahrzeug`,
  mietzuschuss: `Wohngeldstelle Köln
Mietzuschuss

Wohnkostenzuschuss für Ihre Wohnung
Mietbeihilfe bewilligt`,
  gez_wohnungswechsel: `ARD ZDF Deutschlandradio
Rundfunkbeitrag Wohnungswechsel

Ihr Umzug und der Beitragsservice
Wohnungswechsel beim Rundfunkbeitrag`,
  renten_kontoklarung: `Deutsche Rentenversicherung
Kontoklärung Rente

Kontoklärungsbogen zu Ihrem Rentenkonto
Ihre Rente klären`,
  stadtwerke_verbrauchsmeldung: `Stadtwerke Köln
Verbrauchsmeldung

Bitte Zählerstand mitteilen
Verbrauchsabfrage für Ihre Abrechnung`,
  steuer_nebenrechnung: `Finanzamt Berlin
Nebenrechnung zur Steuer

Nebenrechnung des Finanzamts
Steuerliche Nebenrechnung`,
  alg_sperrzeit: `Agentur für Arbeit
Sperrzeit Arbeitslosengeld

Ihre ALG-Sperrzeit wurde festgesetzt
Sperrzeit zum Arbeitslosengeld`,
  dauerauftrag_aenderung: `Commerzbank
Dauerauftragsänderung

Ihr Dauerauftrag wurde angepasst
Änderung des Dauerauftrags`,
  zahnersatz_erstattung: `AOK Berlin
Zahnersatz-Kostenerstattung

Kostenerstattung für Zahnersatz
Zahnersatz wurde von der Kasse erstattet`,
  mietvertrag_aenderung: `Vermieter Weber
Mietvertragsänderung

Änderungsnachträglicher Mietvertrag
Mietvertrag wurde geändert`,
  gez_nachholzahlung: `ARD ZDF Deutschlandradio
Nachholzahlung Rundfunkbeitrag

Nacherhebung des Beitrages
Beitrag nacherhoben`,
  renten_berechnungshinweis: `Deutsche Rentenversicherung
Berechnungshinweis Rente

Hinweis zur Rentenberechnung
Ihre Rente wird berechnet`,
  tarifinfo: `Stadtwerke München
Tarifinformation

Tarifübersicht für Stromkunden
Information zu Ihrem Tarif`,
  steuer_vorauszahlung: `Finanzamt Berlin
Steuer-Vorauszahlungsbescheid

Vorauszahlung zur Einkommensteuer
Ihr Vorauszahlungsbetrag`,
  bafoeg_erstantrag: `Amt für Ausbildungsförderung
BAföG-Erstantrag

Erstantrag auf Ausbildungsförderung
Erstmaliges BAföG beantragt`,
  kartenwechsel: `Commerzbank
Kartenwechsel Bankkarte

Ihre neue Bankkarte wird versendet
Kartenersatz beim Konto`,
  krankengeld: `Techniker Krankenkasse
Krankengeld

Ihr Krankengeld wird ausgezahlt
Krankengeldanspruch der Krankenkasse`,
  alg_erstantrag: `Agentur für Arbeit
Erstantrag Arbeitslosengeld

Sie stellen erstmals einen ALG-Antrag
Erstantrag auf Arbeitslosengeld`,
  kfz_haftpflicht_erhoeht: `HUK-Coburg
Haftpflicht erhöht

Ihr Kfz-Haftpflichtbeitrag steigt
Haftpflichtbeitrag erhöht`,
  gez_meldedaten: `ARD ZDF Deutschlandradio
Meldedatenabgleich GEZ

Meldedatenabgleich zum Rundfunkbeitrag
Datenabgleich des Beitragsservice`,
  verbraucherzentrale: `Verbraucherzentrale NRW
Energieberatung

Verbraucherberatung zum Stromtarif
Beratung der Verbraucherzentrale`,
  steuer_bescheiderklaerung: `Finanzamt Berlin
Bescheiderklärung

Erklärung zum Einkommensteuerbescheid
Bescheiderklärung des Finanzamts`,
  bafoeg_rueckzahlung: `Amt für Ausbildungsförderung
BAföG-Rückzahlung

Rückforderung Ihrer Ausbildungsförderung
Rückzahlung des BAföG`,
  kontokartennummer: `Commerzbank
Kartennummer

Ihre Kreditkartennummer
Bankkartennummer zum Konto`,
  beitragsaenderung: `Techniker Krankenkasse
Beitragsänderung

Ihr Krankenkassenbeitrag ändert sich
Neuer Beitrag der Krankenkasse`,
  kasko_schaden: `HUK-Coburg
Kaskoschaden

Ihr Kasko-Schadensfall wird bearbeitet
Schadensmeldung zur Vollkasko`,
  mietvertragsguthaben: `Hausverwaltung Weber
Mietvertragsguthaben

Ihr Guthaben aus der Mietabrechnung
Mietguthaben wird erstattet`,
  renten_unterlagen: `Deutsche Rentenversicherung
Unterlagenanforderung Rente

Bitte reichen Sie Unterlagen ein
Nachweise für Ihre Rente`,
  stadtwerke_jahresabschluss: `Stadtwerke Köln
Jahresabschluss Strom

Ihr Jahresabschluss zur Stromabrechnung
Jahresabrechnung der Stadtwerke`,
  steuer_erklaerungsassistent: `Finanzamt Berlin
Steuer-Erklärungsassistent

Hilfe zur Steuererklärung
Assistent für Ihre Erklärung`,
  bafoeg_widerspruch: `Amt für Ausbildungsförderung
Widerspruch BAföG

Ihr Widerspruch zum BAföG-Bescheid
Widerspruch gegen die Förderung`,
  kartensperre: `Commerzbank
Kartensperre

Ihre Bankkarte wurde gesperrt
Sperrung der Karte`,
  bonusscheck: `Techniker Krankenkasse
Bonusscheck

Ihre Bonusauszahlung
Bonusgutschrift der Krankenkasse`,
  alg_neufestsetzung: `Agentur für Arbeit
ALG-Neufestsetzung

Ihr Arbeitslosengeld wird neu festgesetzt
Neufestsetzungsbescheid zum ALG`,
  kfz_steuer_zahlung: `Zollamt München
Kfz-Steuer Zahlung

Ihre Kraftfahrzeugsteuer ist fällig
Kfz-Steuerbescheid zur Zahlung`,
  gez_abmeldung: `ARD ZDF Deutschlandradio
Abmeldung Rundfunkbeitrag

Ihre Abmeldung vom Beitragsservice
Abmeldung beim Rundfunkbeitrag`,
  verbrauchsnachweis: `Stadtwerke Köln
Verbrauchsnachweis

Zählerstandsnachweis zur Abrechnung
Nachweis Ihres Stromverbrauchs`,
  steuer_erlass: `Finanzamt München
Steuererlass

Ihr Steuererlass wurde bewilligt
Erlassbescheid zur Steuer`,
  dauerauftrag_storno: `Commerzbank
Dauerauftrag storniert

Ihr Dauerauftrag wurde storniert
Storno des Dauerauftrags`,
  fragebogen_auswertung: `Techniker Krankenkasse
Fragebogen-Auswertung

Ihr Mitgliedschaftsfragebogen wurde ausgewertet
Auswertung des Fragebogens`,
  renten_uebertragung: `Deutsche Rentenversicherung
Rentenübertragung

Übertragung Ihrer Rentenansprüche
Rente wird übertragen`,
  hausordnung: `Hausverwaltung Weber
Hausordnung

Die Hausordnung Ihrer Wohnung
Ordnung im Mietverhältnis`,
  verbrauchseinbruch: `Stadtwerke Köln
Verbrauchseinbruch

Ihr Verbrauch wird überprüft
Ungewöhnlicher Verbrauchsrückgang`,
  steuer_vertretung: `Finanzamt Berlin
Steuervertretung

Vollmacht zur Steuervertretung
Ihr Steuerberater vertritt Sie`,
  bafoeg_nachweis_einkommen: `Amt für Ausbildungsförderung
Einkommensnachweis BAföG

Bitte Einkommensnachweis einreichen
Nachweis des Einkommens`,
  zahlungsverkehr: `Commerzbank
Zahlungsverkehrsübersicht

Übersicht Ihrer Zahlungsumsätze
Zahlungsverkehr des Kontos`,
  arbeitsbescheinigung: `Arbeitgeber Muster GmbH
Arbeitsbescheinigung

Arbeitsbescheinigung für die Agentur für Arbeit
Ihre Arbeitszeitbescheinigung`,
  nachmieter: `Vermieter Weber
Nachmieter

Nachmieter für Ihre Wohnung
Nachmieter soll übernehmen`,
  auslandsrente: `Deutsche Rentenversicherung
Auslandsrente

Ihre Rente bei Auslandsbezug
Rente im Ausland`,
  sammelbescheid: `Finanzamt München
Sammelbescheid

Sammelbescheid zur Steuer
Sammelbescheid des Finanzamts`,
  zahlungsanweisung: `Amt für Ausbildungsförderung
Zahlungsanweisung BAföG

Auszahlungsanweisung der Förderung
BAföG-Zahlungsanweisung`,
  widerruf_unterlagen: `Commerzbank
Widerruf Unterlagen

Widerrufsunterlagen zum Konto
Unterlagen sein Widerruf`,
  sonderzahlung: `Techniker Krankenkasse
Sonderzahlung

Ihre Sonderzahlung zur Krankenkasse
Sonderzahlung der Krankenkasse`,
  kfz_steuer_rueckerstattung: `Zollamt München
Kfz-Steuer-Rückerstattung

Rückerstattung der Fahrzeugsteuer
Kfz-Steuer wurde erstattet`,
  beschaeftigungszeiten: `Deutsche Rentenversicherung
Beschäftigungszeiten

Nachweis Ihrer Beschäftigungszeiten
Rente und Arbeitszeiten`,
  stadtwerke_widerruf: `Stadtwerke Köln
Widerruf Stromvertrag

Widerruf des Energievertrags
Widerruf bei den Stadtwerken`,
  behoerdenauskunft: `Finanzamt Berlin
Behördenauskunft

Auskunft der Finanzbehörde
Steuer-Behördenauskunft`,
  ueberweisungsauftrag: `Commerzbank
Überweisungsauftrag

Ihr Auftrag zur Überweisung
Überweisungsauftrag ausgeführt`,
  alg_weiterbewilligungsantrag: `Agentur für Arbeit
Weiterbewilligungsantrag ALG

Antrag auf Weiterbewilligung des ALG
Weiterbewilligungsantrag zum Arbeitslosengeld`,
  versicherungsschutzbestaetigung: `HUK-Coburg
Versicherungsschutzbestätigung

Ihr Doppelversicherungsschutz
Schutzbestätigung der Versicherung`,
  gez_zahlungsverbot: `ARD ZDF Deutschlandradio
Zahlungsverbot

Verfügung zur Einstellung der Beitragszahlung
Rundfunkbeitrag Zahlungsverbot`,
  renten_steuerbescheid: `Deutsche Rentenversicherung
Renten-Steuerbescheid

Steuerbescheid zur Rente
Ihre Renten werden versteuert`,
  steuer_dauerzahlung: `Finanzamt Berlin
Dauerzahlung Steuer

Ihre Steuer-Dauerzahlung
Dauerzahlung an das Finanzamt`,
  verbrauchsaenderung: `Stadtwerke Köln
Verbrauchsänderung

Ihr Verbrauch wurde geändert
Änderung des Verbrauchs`,
  foerderungsverzicht: `Amt für Ausbildungsförderung
Förderungsverzicht BAföG

Ihr Verzicht auf Förderung
BAföG-Förderungsverzicht`,
  meldepflicht: `Agentur für Arbeit
Meldepflicht ALG

Sie sind meldepflichtig
Bitte melden Sie sich arbeitsuchend`,
  kfz_abrechnung: `HUK-Coburg
Kfz-Versicherer Abrechnung

Ihre Kfz-Versicherungsabrechnung
Abrechnung zum Kfz-Tarif`,
  wohnungsabgabe: `Hausverwaltung Weber
Wohnungsabgabe

Abgabe Ihrer Wohnung
Schlüsselübergabe der Wohnung`,
  erwerbsminderung: `Deutsche Rentenversicherung
Erwerbsminderungsrente

Ihre Erwerbsminderung wird geprüft
Berufsunfähigkeitsrente`,
  vertragsverlaengerung: `Stadtwerke Köln
Vertragsverlängerung

Ihr Energievertrag wird verlängert
Verlängerung des Stromvertrags`,
  lohnsteuerbescheinigung: `Arbeitgeber Muster GmbH
Lohnsteuerbescheinigung

Ihre elektronische Lohnsteuerbescheinigung
Lohnsteuerbescheinigung ausgefüllt`,
  krankenversicherungsverlauf: `Techniker Krankenkasse
Krankenversicherungsverlauf

Ihr Beitragsverlauf zur Krankenversicherung
Krankenversicherungsverlauf`,
  kfz_bonus_malus: `HUK-Coburg
Kfz-Bonus-Malus

Ihre Bonus-Malus-Stufe
Kfz-Bonus Malus Bewertung`,
  indexierung: `Hausverwaltung Weber
Indexmiete

Indexierung der Mietzahlung
Indexmietvertrag für Ihre Wohnung`,
  nachversicherung: `Deutsche Rentenversicherung
Nachversicherung

Renten-Nachversicherung
Nachversicherung der Rente`,
  steuer_freibetrag: `Finanzamt Berlin
Steuer-Freibetrag

Ihr Freibetragsbescheid
Steuerfreibetrag für Sie`,
  bafoeg_erstausstattung: `Amt für Ausbildungsförderung
BAföG-Erstausstattung

Erstausstattungsbedarf für die Ausbildung
Erstausstattung zur Förderung`,
  anschlussverguetung: `Stadtwerke Köln
Anschlussvergütung

Vergütung für Einspeisung
Anschlussvergütung Strom`,
  auszahlungskarte: `Commerzbank
Auszahlungskarte

Ihre Karte zum Geldabheben
Auszahlungskarte für das Konto`,
  krankenkassen_ablehnung: `Techniker Krankenkasse
Ablehnungsbescheid

Ihr Antrag wurde abgelehnt
Ablehnung durch die Krankenkasse`,
  nebeneinkommen: `Agentur für Arbeit
Nebeneinkommen-Meldung

Bitte Nebeneinkommen melden
Meldung Ihres Nebenverdienstes`,
  renten_auszahlungsmitteilung: `Deutsche Rentenversicherung
Auszahlungsmitteilung Rente

Ihre Rentenauszahlung
Mitteilung zur Rentenzahlung`,
  stadtwerke_sperrung: `Stadtwerke Köln
Sperrung Strom

Ihr Strom wird gesperrt
Energiesperrung angekündigt`,
  bafoeg_verlaengerung: `Amt für Ausbildungsförderung
BAföG-Verlängerung

Weiterförderung der Ausbildung
Ihre Förderung wird verlängert`,
  waehrungsrechnung: `Commerzbank
Währungsrechnung

Rechnung in Fremdwährung
Währungsumrechnung zum Konto`,
  praevention: `Techniker Krankenkasse
Prävention

Präventionsangebot der Krankenkasse
Ihr Präventionskurs`,
  alg_nachzahlungsmitteilung: `Agentur für Arbeit
Nachzahlungsmitteilung ALG

Ihre ALG-Nachzahlung
Nachzahlungsmitteilung zum Arbeitslosengeld`,
  kfz_schaden_ablehnung: `HUK-Coburg
Schaden abgelehnt

Ihr Kfz-Schaden wurde abgelehnt
Ablehnung der Schadenszahlung`,
  mietpreisbremse: `Hausverwaltung Weber
Mietpreisbremse

Ihre Miete nach Mietpreisbremse
Mietpreisbremse für die Wohnung`,
  versorgungsausgleich: `Deutsche Rentenversicherung
Versorgungsausgleich

Versorgungsausgleich zur Rente
Rentensplitting nach Trennung`,
  begruessungsschreiben: `Stadtwerke Köln
Begrüßungsschreiben

Willkommen bei den Stadtwerken
Begrüßung des Energieversorgers`,
  weiterbildungszuschuss: `Techniker Krankenkasse
Weiterbildungszuschuss

Ihr Zuschuss zur Weiterbildung
Weiterbildungsförderung der Krankenkasse`,
  sonderkuendigungsrecht: `Hausverwaltung Weber
Sonderkündigungsrecht

Außerordentliche Kündigung möglich
Sonderkündigung des Mietvertrags`,
  hinterbliebenenrente: `Deutsche Rentenversicherung
Hinterbliebenenrente

Ihre Witwenrente
Hinterbliebenenrente`,
  einkunftsnachweis: `Finanzamt Berlin
Einkunftsnachweis

Nachweis Ihrer Einkünfte
Einkünfte nachweisen`,
  lieferaufnahme: `Stadtwerke Köln
Lieferaufnahme

Die Belieferung beginnt
Aufnahme der Stromlieferung`,
  einzahlungsbeleg: `Commerzbank
Einzahlungsbeleg

Beleg über Ihre Einzahlung
Einzahlungsquittung`,
  alg_bescheid_korrektur: `Agentur für Arbeit
Bescheid-Korrektur ALG

Ihr ALG-Bescheid wurde korrigiert
Korrektur des Arbeitslosengeld-Bescheids`,
  kfz_kostenuebernahme: `HUK-Coburg
Kostenübernahme Kfz

Die Schadenskosten werden übernommen
Kostenübernahme der Kfz-Schadens`,
  mitbewohner: `Hausverwaltung Weber
Mitbewohner

Einzug Ihres Mitbewohners
Mitbewohner im Mietverhältnis`,
  ueberfuehrungspruefung: `Deutsche Rentenversicherung
Überführungsprüfung

Prüfung zur Überführung
Überführungsprüfung der Rente`,
  umsatzsteuer_voranmeldung: `Finanzamt Berlin
Umsatzsteuer-Voranmeldung

Ihre USt-Voranmeldung
Umsatzsteuervoranmeldung`,
  bafoeg_aenderungsbescheid: `Amt für Ausbildungsförderung
BAföG-Änderungsbescheid

Ihr BAföG wurde geändert
Änderungsbescheid zur Förderung`,
  zinsabrechnung: `Commerzbank
Zinsabrechnung

Ihre Habenzinsen werden abgerechnet
Zinsabrechnung zum Konto`,
  beitragsausgleich: `Techniker Krankenkasse
Beitragsausgleich

Beitragsausgleich der Krankenkasse
Ausgleich der Beiträge`,
  selbststaendigkeit: `Agentur für Arbeit
Selbstständigkeit ALG

Ihre Selbstständigkeit beim ALG
Selbstständiges Arbeiten`,
  kfz_steuer_ummeldung: `Zollamt München
Kfz-Steuer-Ummeldung

Ummeldung Ihres Fahrzeugs
Kfz-Steuer nach Ummeldung`,
  untervermietung: `Hausverwaltung Weber
Untervermietung

Untermiete der Wohnung
Untermieter im Mietverhältnis`,
  gasbezug: `Stadtwerke Köln
Gasbezug

Die Gaslieferung beginnt
Ihr Gasbezug`,
  spendenbescheinigung: `Gemeinnütziger Verein
Spendenbescheinigung

Zuwendungsbestätigung zur Spende
Bescheinigung für das Finanzamt`,
  umbuchung: `Commerzbank
Umbuchung

Umbuchung auf Ihrem Konto
Übertrag wurde umgebucht`,
  freistellungsbescheinigung: `Techniker Krankenkasse
Freistellungsbescheinigung

Freistellung von Beiträgen
Freistellungsbescheinigung der Krankenkasse`,
  kontostandsmeldung: `Deutsche Rentenversicherung
Kontostandsmeldung

Meldung Ihres Rentenkontos
Renten-Kontostandsmeldung`,
  netzbetreiber: `Stadtwerke Köln
Netzbetreiber

Der Stromnetzbetreiber
Netzbetrieb der Stadtwerke`,
  gewinnermittlung: `Finanzamt Berlin
Gewinnermittlung

Ihr Gewinn wird ermittelt
Einnahmenüberschussrechnung`,
  einkommensanrechnung: `Amt für Ausbildungsförderung
Einkommensanrechnung BAföG

Anrechnung des Einkommens
BAföG-Einkommensanrechnung`,
  girocard: `Commerzbank
Girocard

Ihre neue Girocard
Bankkarte mit Girocard`,
  beauftragtenvollmacht: `Techniker Krankenkasse
Beauftragtenvollmacht

Ihre Vollmacht bei der Krankenkasse
Vollmacht für den Beauftragten`,
  halterwechsel: `Zollamt München
Halterwechsel

Wechsel des Fahrzeughalters
Kfz-Halterwechsel`,
  sicherheitsabschlag: `Hausverwaltung Weber
Sicherheitsabschlag

Sicherheitsabschlag zur Miete
Abschlag für Sicherheit`,
  sonderausgaben: `Finanzamt Berlin
Sonderausgaben

Ihre Sonderausgaben beim Abzug
Sonderausgaben Steuererklärung`,
  foerderungsende: `Amt für Ausbildungsförderung
Förderungsende

Ihre BAföG-Förderung endet
Ende der Förderung`,
  pin_brief: `Commerzbank
PIN-Brief

Ihr PIN-Brief für das Konto
Bank PIN-Brief`,
  gesundheitspraemie: `Techniker Krankenkasse
Gesundheitsprämie

Ihre Prämie zur Krankenkasse
Gesundheitsprämien-Erstattung`,
  kfz_steuer_aussetzung: `Zollamt München
Kfz-Steuer-Aussetzung

Ihre Kfz-Steuer wird ausgesetzt
Aussetzung der Fahrzeugsteuer`,
  raeumungsvergleich: `Hausverwaltung Weber
Räumungsvergleich

Vergleich zur Räumung der Wohnung
Räumungsvergleich im Mietverhältnis`,
  renten_abschlag: `Deutsche Rentenversicherung
Rentenabschlag

Abschlag auf Ihre Rente
Rente mit Abschlagsfaktor`,
  grundversorgung: `Stadtwerke Köln
Grundversorgung

Ihre Grundversorgung Strom
Grundversorger für Energie`,
  verlustabzug: `Finanzamt Berlin
Verlustabzug

Ihr Verlust wird abgezogen
Verlustvortrag zur Steuer`,
  kontoverbindung: `Commerzbank
Kontoverbindung

Ihre Kontoverbindung
Bank Kontoverbindung`,
  mitgliedschaftsbeginn: `Techniker Krankenkasse
Mitgliedschaftsbeginn

Beginn Ihrer Mitgliedschaft
Ihre Krankenkassen-Mitgliedschaft beginnt`,
  altersgrenze: `Deutsche Rentenversicherung
Altersgrenze

Ihre Regelaltersgrenze
Altersgrenze für die Rente`,
  erklaerungspflicht: `Finanzamt Berlin
Erklärungspflicht

Ihre Steuererklärungspflicht
Pflicht zur Steuererklärung`,
  hoechstdauer: `Amt für Ausbildungsförderung
Höchstdauer BAföG

Ihre Förderungshöchstdauer
Höchstdauer der Förderung`,
  gewerbeanzeige: `Landeshauptstadt München
Referat für Gesundheit und Umwelt
Gewerbeanzeige

Sehr geehrte Damen und Herren,
wir bestätigen den Eingang Ihrer Gewerbeanmeldung vom 12.03.2026 und übersenden Ihnen die Gewerbeurkunde.`,
  integrationskurs: `Integrationsamt Kreis Recklinghausen
Teilhabe und Beschäftigung

Sehr geehrte Damen und Herren,
bitte reichen Sie den Nachweis über die Teilnahme an einem Integrationskurs bis zum 30.06.2026 ein.`,
};

const EXPECTED = {
  bank_mahnung: 'mahnung',
  bank_kontoauszug: 'kontoauszug',
  bank_gebuehren: 'bank_gebuehren',
  bank_kredit: 'kredit_vertrag',
  versicherung_kfz: 'kfz_versicherung',
  versicherung_schaden: 'versicherung_schaden',
  versicherung_haftpflicht: 'haftpflicht_versicherung',
  versicherung_hausrat: 'hausrat_versicherung',
  krankenkasse: 'krankenkasse',
  rundfunk: 'gez',
  steuerbescheid: 'steuerbescheid',
  steuererinnerung: 'finanzamt_erinnerung',
  steuer_id: 'tax_id',
  rentenversicherung: 'rvnr',
  renteninfo: 'renteninfo',
  jobcenter_mitwirkung: 'job',
  jobcenter_bewilligung: 'jobcenter_bewilligung',
  kindergeld: 'kindergeld',
  auslaender: 'auslaender',
  melde: 'melde',
  mahnbescheid: 'mahnbescheid',
  bussgeld: 'bussgeldbescheid',
  ausweis: 'ausweis_abholung',
  online_ausweis: 'online_ausweis',
  fuehrungszeugnis: 'fuehrungszeugnis',
  arbeitszeugnis: 'arbeitszeugnis',
  schulzeugnis: 'schulzeugnis',
  hochschulzeugnis: 'hochschulzeugnis',
  sprachzertifikat: 'sprachzertifikat',
  arbeitsunfaehig: 'arbeitsunfaehigkeit',
  nebenkosten: 'nebenkosten',
  stadtwerke: 'stadtwerke',
  mietvertrag: 'mietvertrag',
  wohngeld: 'wohngeld',
  bafoeg: 'bafoeg',
  versicherung_hausrat3: 'hausrat_versicherung',
  ausweis_abholung2: 'ausweis_abholung',
  pflegeversicherung: 'pflegeversicherung',
  versicherungsverlauf: 'versicherungsverlauf',
  lebensversicherung: 'lebensversicherung',
  kuendigung: 'kuendigung',
  elster: 'elster',
  gez_befreiung: 'gez_befreiung',
  iban_nachweis: 'iban_nachweis',
  versicherungsschein: 'versicherungsschein',
  bonusprogramm: 'bonusprogramm',
  vollstreckung: 'vollstreckung',
  kfz_steuer: 'kfz_steuer',
  abmahnung: 'abmahnung',
  einbuergerung: 'einbuergerung',
  arbeitslosengeld: 'arbeitslosengeld',
  beitragserhoehung: 'beitragserhoehung',
  sepa_mandat: 'sepa_mandat',
  umzugsmitteilung: 'umzugsmitteilung',
  rentenbescheid: 'rentenbescheid',
  krankenkassen_bescheid: 'krankenkassen_bescheid',
  beitragsbescheid: 'beitragsbescheid',
  steuer_erstattung: 'steuer_erstattung',
  arbeitsgericht_klage: 'arbeitsgericht_klage',
  widerspruch: 'widerspruch',
  inkasso: 'inkasso',
  schufa_auskunft: 'schufa_auskunft',
  kontoschliessung: 'kontoschliessung',
  gez_nachzahlung: 'gez_nachzahlung',
  elterngeld: 'elterngeld',
  steuer_nachzahlung: 'steuer_nachzahlung',
  widerspruchsbescheid: 'widerspruchsbescheid',
  zahnersatz: 'zahnersatz',
  dauerauftrag: 'dauerauftrag',
  stadtwerke_nachzahlung: 'stadtwerke_nachzahlung',
  krankenkasse_einkommen: 'krankenkasse_einkommen',
  krankenkassen_wechsel: 'krankenkassen_wechsel',
  pflegekasse_bescheid: 'pflegekasse_bescheid',
  bafoeg_widerruf: 'bafoeg_widerruf',
  steuerbescheid_einspruch: 'steuerbescheid_einspruch',
  alg_widerspruch: 'alg_widerspruch',
  rentenbescheid_nachzahlung: 'rentenbescheid_nachzahlung',
  gez_kontoauszug: 'gez_kontoauszug',
  steuer_korrektur: 'steuer_korrektur',
  krankenkassen_zahnersatz: 'krankenkassen_zahnersatz',
  alg_bewilligung: 'alg_bewilligung',
  gebuehrenbescheid: 'gebuehrenbescheid',
  versicherungswiderspruch: 'versicherungswiderspruch',
  elterngeld_widerruf: 'elterngeld_widerruf',
  abschlagsaenderung: 'abschlagsaenderung',
  steuerbescheid_widerruf: 'steuerbescheid_widerruf',
  bafoeg_zulage: 'bafoeg_zulage',
  krankenkassen_befreiung: 'beitragsbefreiung',
  alg_ablehnung: 'alg_ablehnung',
  kfz_versicherungserhoehung: 'kfz_versicherungserhoehung',
  vermieterwechsel: 'vermieterwechsel',
  gez_teilerstattung: 'gez_teilerstattung',
  rentenueberzahlung: 'rentenueberzahlung',
  pfaendungsschutz: 'pfaendungsschutz',
  abschlagszahlung: 'abschlagszahlung',
  renten_widerspruchsbescheid: 'renten_widerspruchsbescheid',
  mitgliedsbeitrag_rueck: 'mitgliedsbeitrag_rueck',
  alg_weiterbewilligung: 'alg_weiterbewilligung',
  kfz_wechsel: 'kfz_wechsel',
  mietminderung: 'mietminderung',
  gez_zahlungsplan: 'gez_zahlungsplan',
  konto_ueberziehung: 'konto_ueberziehung',
  stadtwerke_steuer: 'stadtwerke_verbrauchssteuer',
  bafoeg_elternunabhaengig: 'bafoeg_elternunabhaengig',
  mitgliedsbescheinigung_beitraege: 'mitgliedsbescheinigung_beitraege',
  alg_antrag: 'alg_antrag',
  kfz_schadenregulierung: 'kfz_schadenregulierung',
  mieterhoehung: 'mieterhoehung',
  rentenantrag: 'rentenantrag',
  kontokuendigung: 'kontokuendigung',
  verbrauchsabschlag: 'verbrauchsabschlag',
  bafoeg_ablehnung: 'bafoeg_ablehnung',
  steuer_datenuebersicht: 'steuer_datenuebersicht',
  gesundheitskur: 'gesundheitskur',
  alg_bemessung: 'alg_bemessung',
  kfz_vollkasko: 'kfz_vollkasko',
  mietkaution: 'mietkaution',
  gez_befreiungs_ablehnung: 'gez_befreiungs_ablehnung',
  renten_volksversicherung: 'renten_volksversicherung',
  konto_wechselbank: 'konto_wechselbank',
  tarifwechsel: 'tarifwechsel',
  bafoeg_vorleistung: 'bafoeg_vorleistung',
  steuer_bescheidankuendigung: 'steuer_bescheidankuendigung',
  krankenkasse_aenderung: 'krankenkasse_aenderung',
  kfz_steuer_neu: 'kfz_steuer_neu',
  mietzuschuss: 'mietzuschuss',
  gez_wohnungswechsel: 'gez_wohnungswechsel',
  renten_kontoklarung: 'renten_kontoklarung',
  stadtwerke_verbrauchsmeldung: 'stadtwerke_verbrauchsmeldung',
  steuer_nebenrechnung: 'steuer_nebenrechnung',
  alg_sperrzeit: 'alg_sperrzeit',
  dauerauftrag_aenderung: 'dauerauftrag_aenderung',
  zahnersatz_erstattung: 'zahnersatz_erstattung',
  mietvertrag_aenderung: 'mietvertrag_aenderung',
  gez_nachholzahlung: 'gez_nachholzahlung',
  renten_berechnungshinweis: 'renten_berechnungshinweis',
  tarifinfo: 'tarifinfo',
  steuer_vorauszahlung: 'steuer_vorauszahlung',
  bafoeg_erstantrag: 'bafoeg_erstantrag',
  kartenwechsel: 'kartenwechsel',
  krankengeld: 'krankengeld',
  alg_erstantrag: 'alg_erstantrag',
  kfz_haftpflicht_erhoeht: 'kfz_haftpflicht_erhoeht',
  gez_meldedaten: 'gez_meldedaten',
  verbraucherzentrale: 'verbraucherzentrale',
  steuer_bescheiderklaerung: 'steuer_bescheiderklaerung',
  bafoeg_rueckzahlung: 'bafoeg_rueckzahlung',
  kontokartennummer: 'kontokartennummer',
  beitragsaenderung: 'beitragsaenderung',
  kasko_schaden: 'kasko_schaden',
  mietvertragsguthaben: 'mietvertragsguthaben',
  renten_unterlagen: 'renten_unterlagen',
  stadtwerke_jahresabschluss: 'stadtwerke_jahresabschluss',
  steuer_erklaerungsassistent: 'steuer_erklaerungsassistent',
  bafoeg_widerspruch: 'bafoeg_widerspruch',
  kartensperre: 'kartensperre',
  bonusscheck: 'bonusscheck',
  alg_neufestsetzung: 'alg_neufestsetzung',
  kfz_steuer_zahlung: 'kfz_steuer_zahlung',
  gez_abmeldung: 'gez_abmeldung',
  verbrauchsnachweis: 'verbrauchsnachweis',
  steuer_erlass: 'steuer_erlass',
  dauerauftrag_storno: 'dauerauftrag_storno',
  fragebogen_auswertung: 'fragebogen_auswertung',
  renten_uebertragung: 'renten_uebertragung',
  hausordnung: 'hausordnung',
  verbrauchseinbruch: 'verbrauchseinbruch',
  steuer_vertretung: 'steuer_vertretung',
  bafoeg_nachweis_einkommen: 'bafoeg_nachweis_einkommen',
  zahlungsverkehr: 'zahlungsverkehr',
  arbeitsbescheinigung: 'arbeitsbescheinigung',
  nachmieter: 'nachmieter',
  auslandsrente: 'auslandsrente',
  sammelbescheid: 'sammelbescheid',
  zahlungsanweisung: 'zahlungsanweisung',
  widerruf_unterlagen: 'widerruf_unterlagen',
  sonderzahlung: 'sonderzahlung',
  kfz_steuer_rueckerstattung: 'kfz_steuer_rueckerstattung',
  beschaeftigungszeiten: 'beschaeftigungszeiten',
  stadtwerke_widerruf: 'stadtwerke_widerruf',
  behoerdenauskunft: 'behoerdenauskunft',
  ueberweisungsauftrag: 'ueberweisungsauftrag',
  alg_weiterbewilligungsantrag: 'alg_weiterbewilligungsantrag',
  versicherungsschutzbestaetigung: 'versicherungsschutzbestaetigung',
  gez_zahlungsverbot: 'gez_zahlungsverbot',
  renten_steuerbescheid: 'renten_steuerbescheid',
  steuer_dauerzahlung: 'steuer_dauerzahlung',
  verbrauchsaenderung: 'verbrauchsaenderung',
  foerderungsverzicht: 'foerderungsverzicht',
  meldepflicht: 'meldepflicht',
  kfz_abrechnung: 'kfz_abrechnung',
  wohnungsabgabe: 'wohnungsabgabe',
  erwerbsminderung: 'erwerbsminderung',
  vertragsverlaengerung: 'vertragsverlaengerung',
  lohnsteuerbescheinigung: 'lohnsteuerbescheinigung',
  krankenversicherungsverlauf: 'krankenversicherungsverlauf',
  kfz_bonus_malus: 'kfz_bonus_malus',
  indexierung: 'indexierung',
  nachversicherung: 'nachversicherung',
  steuer_freibetrag: 'steuer_freibetrag',
  bafoeg_erstausstattung: 'bafoeg_erstausstattung',
  anschlussverguetung: 'anschlussverguetung',
  auszahlungskarte: 'auszahlungskarte',
  krankenkassen_ablehnung: 'krankenkassen_ablehnung',
  nebeneinkommen: 'nebeneinkommen',
  renten_auszahlungsmitteilung: 'renten_auszahlungsmitteilung',
  stadtwerke_sperrung: 'stadtwerke_sperrung',
  bafoeg_verlaengerung: 'bafoeg_verlaengerung',
  waehrungsrechnung: 'waehrungsrechnung',
  praevention: 'praevention',
  alg_nachzahlungsmitteilung: 'alg_nachzahlungsmitteilung',
  kfz_schaden_ablehnung: 'kfz_schaden_ablehnung',
  mietpreisbremse: 'mietpreisbremse',
  versorgungsausgleich: 'versorgungsausgleich',
  begruessungsschreiben: 'begruessungsschreiben',
  weiterbildungszuschuss: 'weiterbildungszuschuss',
  sonderkuendigungsrecht: 'sonderkuendigungsrecht',
  hinterbliebenenrente: 'hinterbliebenenrente',
  einkunftsnachweis: 'einkunftsnachweis',
  lieferaufnahme: 'lieferaufnahme',
  einzahlungsbeleg: 'einzahlungsbeleg',
  alg_bescheid_korrektur: 'alg_bescheid_korrektur',
  kfz_kostenuebernahme: 'kfz_kostenuebernahme',
  mitbewohner: 'mitbewohner',
  ueberfuehrungspruefung: 'ueberfuehrungspruefung',
  umsatzsteuer_voranmeldung: 'umsatzsteuer_voranmeldung',
  bafoeg_aenderungsbescheid: 'bafoeg_aenderungsbescheid',
  zinsabrechnung: 'zinsabrechnung',
  beitragsausgleich: 'beitragsausgleich',
  selbststaendigkeit: 'selbststaendigkeit',
  kfz_steuer_ummeldung: 'kfz_steuer_ummeldung',
  untervermietung: 'untervermietung',
  gasbezug: 'gasbezug',
  spendenbescheinigung: 'spendenbescheinigung',
  umbuchung: 'umbuchung',
  freistellungsbescheinigung: 'freistellungsbescheinigung',
  kfz_steuer_ummeldung: 'kfz_steuer_ummeldung',
  kontostandsmeldung: 'kontostandsmeldung',
  netzbetreiber: 'netzbetreiber',
  gewinnermittlung: 'gewinnermittlung',
  einkommensanrechnung: 'einkommensanrechnung',
  girocard: 'girocard',
  beauftragtenvollmacht: 'beauftragtenvollmacht',
  halterwechsel: 'halterwechsel',
  sicherheitsabschlag: 'sicherheitsabschlag',
  sonderausgaben: 'sonderausgaben',
  foerderungsende: 'foerderungsende',
  pin_brief: 'pin_brief',
  gesundheitspraemie: 'gesundheitspraemie',
  kfz_steuer_aussetzung: 'kfz_steuer_aussetzung',
  raeumungsvergleich: 'raeumungsvergleich',
  renten_abschlag: 'renten_abschlag',
  grundversorgung: 'grundversorgung',
  verlustabzug: 'verlustabzug',
  kontoverbindung: 'kontoverbindung',
  mitgliedschaftsbeginn: 'mitgliedschaftsbeginn',
  altersgrenze: 'altersgrenze',
  erklaerungspflicht: 'erklaerungspflicht',
  hoechstdauer: 'hoechstdauer',
  gewerbeanzeige: 'gewerbeanzeige',
  integrationskurs: 'integrationskurs',
};

test('classifies all sampled German documents correctly', () => {
  const failures = [];
  for (const [name, text] of Object.entries(SAMPLES)) {
    const r = classify(text);
    const expected = EXPECTED[name];
    if (r.docKey !== expected) {
      failures.push(`${name}: expected ${expected}, got ${r.docKey}`);
    }
  }
  assert.strictEqual(failures.length, 0, failures.join('\n'));
});

test('extracts reference, deadline and amount facts', () => {
  const r = classify(`Finanzamt Berlin\nAktenzeichen: 45/123/4567\n\nBescheid\nBitte überweisen Sie 350,00 EUR bis zum 20.10.2026.`);
  assert.strictEqual(r.reference, '45/123/4567');
  assert.strictEqual(r.deadline, '20.10.2026');
  assert.strictEqual(r.amount, '350.00 €');
});

test('handles ambiguous / short documents without crashing', () => {
  const r1 = classify('Sehr geehrte Damen und Herren,\nwir teilen Ihnen mit, dass sich unser Vorgang geändert hat.');
  assert.ok(r1.docKey, 'docKey present');
  const r2 = classify('');
  assert.strictEqual(r2.docKey, 'general');
  const r3 = classify('ABC');
  assert.ok(r3.docKey, 'docKey present for short input');
});

test('classifies OCR-noisy documents (camera artifacts) correctly', () => {
  // Simulate common OCR errors: l/1, 0/o, rn/m, stray separators
  const noisy = 'Deutsche Rentenvers1cherung Bund\nVers1cherungsnummernachwe1s\n\nIhre Verslcherungsnummer: 12 150896 M 014';
  const noisy2 = 'Jobcenter Ber1in\nAufforderung zur M1tw1rkung\n\nMietbesche1nigung b1s zum 08.09.2026';
  assert.strictEqual(classify(noisy).docKey, 'rvnr');
  assert.strictEqual(classify(noisy2).docKey, 'job');
});
