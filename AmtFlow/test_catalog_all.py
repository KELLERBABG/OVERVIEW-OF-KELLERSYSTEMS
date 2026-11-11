# -*- coding: utf-8 -*-
"""
Umfassender Test- und Benchmark-Generator für alle deutschen Behörden-, Gerichts-, 
Bildungs-, Versicherungs- und Vertragsdokumente.
"""

import json
import re

# Definition aller Kern-Dokumenttypen mit realistischen Textmustern
DOC_SAMPLES = [
    # --- Steuern & Finanzamt ---
    {
        "expected_key": "tax_id",
        "sample": "Bundeszentralamt für Steuern 53221 Bonn. Mitteilung über die Zuteilung der steuerlichen Identifikationsnummer nach § 139b der Abgabenordnung (AO). Ihre Identifikationsnummer lautet: 42 819 304 912. Bitte bewahren Sie dieses Schreiben dauerhaft auf."
    },
    {
        "expected_key": "steuerbescheid",
        "sample": "Finanzamt Berlin Mitte. Bescheid für 2025 über Einkommensteuer und Solidaritätszuschlag. Festgesetzte Steuer: 1.250,00 Euro. Es ergibt sich ein Guthaben von 340,50 Euro. Steuernummer: 21/815/08150."
    },
    {
        "expected_key": "finanzamt_erinnerung",
        "sample": "Finanzamt Hamburg-Nord. Erinnerung an die Abgabe der Steuererklärung für das Kalenderjahr 2024. Bitte reichen Sie Ihre Einkommensteuererklärung bis spätestens 15.10.2026 über ELSTER ein."
    },
    {
        "expected_key": "steuer_vorauszahlung",
        "sample": "Finanzamt München. Vorauszahlungsbescheid zur Einkommensteuer. Festsetzung der Vorauszahlungen: Zum 10.09.2026 ist ein Betrag von 450,00 Euro fällig."
    },
    {
        "expected_key": "lohnsteuerbescheinigung",
        "sample": "Ausdruck der elektronischen Lohnsteuerbescheinigung für 2025. Arbeitgeber: Muster GmbH. Bruttoarbeitslohn: 36.000,00 Euro. Einbehaltene Lohnsteuer: 4.200,00 Euro."
    },

    # --- Jobcenter & Bundesagentur für Arbeit ---
    {
        "expected_key": "jobcenter_bewilligung",
        "sample": "Jobcenter Köln. Bewilligungsbescheid von Leistungen zur Sicherung des Lebensunterhalts nach dem Zweiten Buch Sozialgesetzbuch (SGB II) Bürgergeld. Bewilligungszeitraum: 01.10.2026 bis 30.09.2027. Monatlicher Regelbedarf und Kosten der Unterkunft."
    },
    {
        "expected_key": "job",
        "sample": "Jobcenter Region Hannover. Aufforderung zur Mitwirkung nach §§ 60, 66 SGB I. Zur Prüfung Ihres Anspruchs auf Bürgergeld fehlen noch folgende Unterlagen: Mietbescheinigung und Kontoauszüge der letzten 3 Monate. Frist bis zum 28.09.2026."
    },
    {
        "expected_key": "jobcenter_weiterbewilligung",
        "sample": "Jobcenter Frankfurt am Main. Weiterbewilligungsantrag (WBA) auf Leistungen zur Sicherung des Lebensunterhalts nach dem SGB II. Bitte stellen Sie rechtzeitig Ihren Folgeantrag."
    },
    {
        "expected_key": "alg_bewilligung",
        "sample": "Agentur für Arbeit Nürnberg. Bewilligungsbescheid über Arbeitslosengeld I (ALG 1) nach dem SGB III. Ihr täglicher Leistungsbetrag beträgt 48,20 Euro."
    },

    # --- Renten- & Sozialversicherung ---
    {
        "expected_key": "rvnr",
        "sample": "Deutsche Rentenversicherung Bund. Versicherungsnummernachweis. Ihre persönliche Versicherungsnummer lautet: 12 150896 M 014. Gültig als Nachweis der Sozialversicherungsnummer."
    },
    {
        "expected_key": "renteninfo",
        "sample": "Deutsche Rentenversicherung. Ihre Renteninformation zum Stichtag 01.01.2026. Bislang erworbene monatliche Altersrente: 890,40 Euro. Voraussichtliche Regelaltersrente."
    },
    {
        "expected_key": "rentenbescheid",
        "sample": "Deutsche Rentenversicherung Hessen. Rentenbescheid über Regelaltersrente. Die monatliche Rente beträgt 1.420,00 Euro und wird ab dem 01.11.2026 gezahlt."
    },
    {
        "expected_key": "renten_kontoklarung",
        "sample": "Deutsche Rentenversicherung. Kontenklärungsverfahren zur Feststellung Ihrer Beitragszeiten und Versicherungsbiografie. Bitte füllen Sie den beiliegenden Kontenklärungsbogen aus."
    },

    # --- Krankenkassen & Gesundheit ---
    {
        "expected_key": "krankenkasse",
        "sample": "Techniker Krankenkasse (TK). Mitgliedsbescheinigung zur Vorlage beim Arbeitgeber oder der Hochschule. Herr Lukas Keller ist seit dem 01.08.2024 bei uns gesetzlich krankenversichert. Versichertennummer: A123456789."
    },
    {
        "expected_key": "krankenkasse_einkommen",
        "sample": "AOK Bayern. Fragebogen zur Überprüfung der Familienversicherung nach § 10 SGB V. Bitte reichen Sie den Einkommensnachweis bis zum 20.10.2026 ein."
    },
    {
        "expected_key": "arbeitsunfaehigkeit",
        "sample": "Ärztliche Bescheinigung. Arbeitsunfähigkeitsbescheinigung (eAU). Erstbescheinigung. Arbeitsunfähig seit: 12.09.2026. Voraussichtlich arbeitsunfähig bis einschließlich: 25.09.2026."
    },
    {
        "expected_key": "krankengeld",
        "sample": "BARMER. Mitteilung über die Bewilligung von Krankengeld nach Ablauf der sechswöchigen Entgeltfortzahlung durch den Arbeitgeber."
    },
    {
        "expected_key": "zahnersatz",
        "sample": "DAK Gesundheit. Genehmigung des Heil- und Kostenplans für Zahnersatz. Festzuschuss der Krankenkasse: 650,00 Euro."
    },
    {
        "expected_key": "pflegekasse_bescheid",
        "sample": "Pflegekasse der AOK. Bescheid über die Zuerkennung von Pflegegrad 2 nach Begutachtung durch den Medizinischen Dienst (MD)."
    },

    # --- Bürgeramt, Melde- & Passwesen ---
    {
        "expected_key": "melde",
        "sample": "Bürgeramt Stadt Leipzig. Amtliche Meldebestätigung nach § 24 Bundesmeldegesetz (BMG). Hauptwohnsitz angemeldet in 04109 Leipzig, Gottschedstraße 12."
    },
    {
        "expected_key": "online_ausweis",
        "sample": "Bundesdruckerei / Personalausweisbehörde. PIN-Brief zur Aktivierung der Online-Ausweisfunktion (eID). Ihre 5-stellige Transport-PIN: 12345. PUK und Sperrkennwort."
    },
    {
        "expected_key": "ausweis_abholung",
        "sample": "Bürgeramt München. Ihr neu ausgestellter Personalausweis liegt zur Abholung bereit. Bitte bringen Sie Ihren bisherigen Ausweis mit."
    },
    {
        "expected_key": "fuehrungszeugnis",
        "sample": "Bundesamt für Justiz. Führungszeugnis (Belegart N) nach § 30 BZRG. Eintragung: Keine Eintragung im Bundeszentralregister."
    },

    # --- Ausländerbehörde & Migration ---
    {
        "expected_key": "auslaender",
        "sample": "Landesamt für Einwanderung (LEA Berlin). Bescheid zur Erteilung eines elektronischen Aufenthaltstitels (eAT) nach § 18b AufenthG. Gültig bis 30.09.2028."
    },
    {
        "expected_key": "integrationskurs",
        "sample": "Bundesamt für Migration und Flüchtlinge (BAMF). Berechtigungsschein zur Teilnahme am Integrationskurs gemäß § 44 Abs. 4 AufenthG."
    },
    {
        "expected_key": "einbuergerung",
        "sample": "Einbürgerungsbehörde Stadt Stuttgart. Einbürgerungszusicherung zur Verleihung der deutschen Staatsangehörigkeit nach § 10 StAG."
    },

    # --- Gerichte, Justiz & Mahnwesen ---
    {
        "expected_key": "mahnbescheid",
        "sample": "Amtsgericht Wedding - Zentrales Mahngericht. Mahnbescheid. Aktenzeichen: 26-1049281-0-1. Antragsteller fordert 450,00 Euro. Widerspruchsfrist: Zwei Wochen."
    },
    {
        "expected_key": "vollstreckungsbescheid",
        "sample": "Amtsgericht Stuttgart - Mahngericht. Vollstreckungsbescheid im automatisierten gerichtlichen Mahnverfahren. Vollstreckbarer Betrag: 890,00 Euro."
    },
    {
        "expected_key": "bussgeldbescheid",
        "sample": "Zentrale Bußgeldstelle Bayern. Bußgeldbescheid wegen einer Ordnungswidrigkeit im Straßenverkehr. Geldbuße: 70,00 Euro. Aktenzeichen: BG-2026-9941."
    },
    {
        "expected_key": "pfaendungsschutz",
        "sample": "Bescheinigung nach § 903 ZPO über das Pfändungsschutzkonto (P-Konto). Der monatliche Grundfreibetrag ist vor Pfändung geschützt."
    },

    # --- Verträge & Ausbildung ---
    {
        "expected_key": "ausbildungsvertrag",
        "sample": "Berufsausbildungsvertrag nach dem Berufsbildungsgesetz (BBiG). Ausbildungsbetrieb: Keller Tech GmbH. Auszubildender: Lukas Keller. Ausbildungsberuf: Fachinformatiker. Eingetragen im Verzeichnis der IHK."
    },
    {
        "expected_key": "leihvertrag",
        "sample": "Leihvertrag / Sachüberlassung von betrieblichen Arbeitsmitteln. Dem Mitarbeiter wird ein Laptop ThinkPad T14 und ein Diensthandy unentgeltlich für dienstliche Zwecke überlassen."
    },
    {
        "expected_key": "arbeitsvertrag",
        "sample": "Arbeitsvertrag. Zwischen Arbeitgeber Alpha GmbH und Arbeitnehmer. Beginn des Beschäftigungsverhältnisses: 01.10.2026. Monatliches Bruttogehalt: 3.800,00 Euro."
    },
    {
        "expected_key": "schulzeugnis",
        "sample": "Staatliche Berufsschule II Bamberg. Jahreszeugnis der Berufsschule für das Schuljahr 2025/2026. Ausbildungsberuf: Fachinformatiker für Anwendungsentwicklung. Notendurchschnitt: 1,2."
    },
    {
        "expected_key": "hochschulzeugnis",
        "sample": "Technische Universität München. Bachelor-Urkunde. Herr Lukas Keller hat den akademischen Grad Bachelor of Science (B.Sc.) in Informatik erfolgreich erworben."
    },
    {
        "expected_key": "arbeitszeugnis",
        "sample": "Qualifiziertes Arbeitszeugnis. Herr Lukas Keller war vom 01.01.2023 bis 30.09.2026 als Softwareentwickler beschäftigt. Er erledigte alle Aufgaben stets zu unserer vollsten Zufriedenheit."
    },
    {
        "expected_key": "bafoeg_bescheid",
        "sample": "Amt für Ausbildungsförderung. BAföG-Bescheid über die Bewilligung von Leistungen nach dem Bundesausbildungsförderungsgesetz. Monatlicher Förderbetrag: 650,00 Euro."
    },

    # --- Rundfunk & Wohnen & Stadtwerke ---
    {
        "expected_key": "gez",
        "sample": "ARD ZDF Deutschlandradio Beitragsservice. Ihr Rundfunkbeitrag für die Wohnung. Beitragsnummer: 123 456 789. Zahlungsaufforderung über den rückständigen Betrag von 55,08 Euro."
    },
    {
        "expected_key": "nebenkosten",
        "sample": "Hausverwaltung Schmidt. Betriebskostenabrechnung / Nebenkostenabrechnung für das Kalenderjahr 2025. Heizkosten und Warmwasserkosten. Nachzahlungsbetrag: 185,40 Euro."
    },
    {
        "expected_key": "stadtwerke",
        "sample": "Stadtwerke München. Jahresverbrauchsabrechnung Strom und Erdgas. Verbrauchszeitraum 2025. Neuer monatlicher Abschlagsbetrag: 89,00 Euro."
    },
    {
        "expected_key": "kindergeld",
        "sample": "Bundesagentur für Arbeit - Familienkasse. Kindergeldbescheid nach dem Einkommensteuergesetz (EStG). Kindergeld wird in Höhe von 250,00 Euro monatlich bewilligt."
    }
]

print(f"Total test cases configured: {len(DOC_SAMPLES)}")
