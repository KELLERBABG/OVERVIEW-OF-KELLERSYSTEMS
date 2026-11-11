# ClauseLens Red-Team Scorecard

Corpus: 410 contracts, 3058 planted traps, 2811 benign controls, 272 uncovered clauses.
A trap is a hit only when a finding of exactly the expected detector type overlaps the marked lines. Higher recall and lower false positive count are better.

| Kennzahl | Wert |
|---|---|
| Verträge | 410 |
| Fallen erkannt | 3058/3058 (100 %) |
| Fehlalarme auf harmlosen Klauseln | 0/2811 (0 %) |
| Ungedeckte Fallen | 270 still, 2 von einem anderen Detektor erfasst |
| Label-Fehler | 0 |

## Trefferquote je Detektor (schlechteste zuerst)

| Detektor | erkannt | Quote | Fehlalarme |
|---|---|---|---|
| liability | 121/121 | 100 % | 0 |
| jurisdiction | 107/107 | 100 % | 0 |
| auto_renewal | 91/91 | 100 % | 0 |
| third_party_indemnity | 90/90 | 100 % | 0 |
| data_grant | 87/87 | 100 % | 0 |
| penalty | 86/86 | 100 % | 0 |
| price_escalation | 84/84 | 100 % | 0 |
| entire_agreement | 83/83 | 100 % | 0 |
| setoff_restriction | 80/80 | 100 % | 0 |
| notice_period | 79/79 | 100 % | 0 |
| written_form_trap | 79/79 | 100 % | 0 |
| deemed_acceptance | 71/71 | 100 % | 0 |
| burden_of_proof | 70/70 | 100 % | 0 |
| unreasonable_audit | 70/70 | 100 % | 0 |
| data_lockin | 70/70 | 100 % | 0 |
| statute_of_limitations | 67/67 | 100 % | 0 |
| excessive_survival | 67/67 | 100 % | 0 |
| marketing_reference | 66/66 | 100 % | 0 |
| unilateral_change | 66/66 | 100 % | 0 |
| non_compete | 65/65 | 100 % | 0 |
| service_suspension | 65/65 | 100 % | 0 |
| subcontractor_uncontrolled | 63/63 | 100 % | 0 |
| tax_indemnity_shift | 63/63 | 100 % | 0 |
| vicarious_agents | 61/61 | 100 % | 0 |
| default_interest | 60/60 | 100 % | 0 |
| non_solicitation | 60/60 | 100 % | 0 |
| dynamic_url_referral | 58/58 | 100 % | 0 |
| assignment_restriction | 58/58 | 100 % | 0 |
| fictitious_receipt | 56/56 | 100 % | 0 |
| insolvency_termination | 56/56 | 100 % | 0 |
| international_data_processing | 56/56 | 100 % | 0 |
| salvatorische_klausel_falle | 55/55 | 100 % | 0 |
| unverbindliche_sla_falle | 55/55 | 100 % | 0 |
| audit_log_manipulation | 54/54 | 100 % | 0 |
| as_is_software_disclaimer | 53/53 | 100 % | 0 |
| service_credit_forfeiture | 53/53 | 100 % | 0 |
| force_majeure_asymmetry | 53/53 | 100 % | 0 |
| unilateral_convenience_termination | 52/52 | 100 % | 0 |
| extended_retention_of_title | 52/52 | 100 % | 0 |
| asymmetric_confidentiality | 52/52 | 100 % | 0 |
| feedback_ip_grab | 51/51 | 100 % | 0 |
| maintenance_conditional_shutdown | 51/51 | 100 % | 0 |
| asymmetric_termination | 48/48 | 100 % | 0 |
| early_termination_charge | 46/46 | 100 % | 0 |
| agency_goodwill_waiver | 45/45 | 100 % | 0 |
| agency_commission_exclusion | 42/42 | 100 % | 0 |
| mandatory_expert_determination | 41/41 | 100 % | 0 |

## Nicht abgedeckte Detektortypen in diesem Korpus

keine, alle 47 Typen sind belegt

## Fehlalarme (Detektor feuerte auf harmloser Klausel)

keine

## Verfehlte Fallen (nicht erkannt)

keine

## Ungedeckte Klauseln ohne Detektor

### still geblieben (Gap)
~~~text
daten-001:L81 :: Zweitschrift der Dokumentation gegen eine Gebühr von fünfzehn Euro
daten-002:L81 :: Mindestabnahme von fünfhundert Datensätzen je Kalendermonat
daten-003:L81 :: eine Bearbeitungsgebühr von fünfzig Euro
daten-004:L81 :: Zweitschrift der Dokumentation gegen eine Gebühr von fünfzehn Euro
daten-005:L81 :: Mindestabnahme von fünfhundert Datensätzen je Kalendermonat
daten-006:L81 :: eine Bearbeitungsgebühr von fünfzig Euro
daten-007:L81 :: Zweitschrift der Dokumentation gegen eine Gebühr von fünfzehn Euro
daten-008:L81 :: Mindestabnahme von fünfhundert Datensätzen je Kalendermonat
daten-009:L81 :: eine Bearbeitungsgebühr von fünfzig Euro
daten-010:L81 :: Zweitschrift der Dokumentation gegen eine Gebühr von fünfzehn Euro
daten-011:L81 :: Mindestabnahme von fünfhundert Datensätzen je Kalendermonat
daten-012:L81 :: eine Bearbeitungsgebühr von fünfzig Euro
daten-013:L81 :: Zweitschrift der Dokumentation gegen eine Gebühr von fünfzehn Euro
daten-014:L81 :: Mindestabnahme von fünfhundert Datensätzen je Kalendermonat
daten-015:L81 :: eine Bearbeitungsgebühr von fünfzig Euro
daten-016:L81 :: Zweitschrift der Dokumentation gegen eine Gebühr von fünfzehn Euro
daten-017:L81 :: Mindestabnahme von fünfhundert Datensätzen je Kalendermonat
daten-018:L81 :: eine Bearbeitungsgebühr von fünfzig Euro
daten-019:L81 :: Zweitschrift der Dokumentation gegen eine Gebühr von fünfzehn Euro
daten-020:L81 :: Mindestabnahme von fünfhundert Datensätzen je Kalendermonat
daten-021:L81 :: eine Bearbeitungsgebühr von fünfzig Euro
daten-022:L81 :: Zweitschrift der Dokumentation gegen eine Gebühr von fünfzehn Euro
daten-023:L81 :: Mindestabnahme von fünfhundert Datensätzen je Kalendermonat
daten-024:L81 :: eine Bearbeitungsgebühr von fünfzig Euro
daten-025:L81 :: Zweitschrift der Dokumentation gegen eine Gebühr von fünfzehn Euro
daten-026:L81 :: Mindestabnahme von fünfhundert Datensätzen je Kalendermonat
daten-027:L81 :: eine Bearbeitungsgebühr von fünfzig Euro
daten-028:L81 :: Zweitschrift der Dokumentation gegen eine Gebühr von fünfzehn Euro
daten-029:L81 :: Mindestabnahme von fünfhundert Datensätzen je Kalendermonat
daten-030:L81 :: eine Bearbeitungsgebühr von fünfzig Euro
daten-031:L81 :: Zweitschrift der Dokumentation gegen eine Gebühr von fünfzehn Euro
daten-032:L81 :: Mindestabnahme von fünfhundert Datensätzen je Kalendermonat
einkauf-001:L19 :: wird ein gesondertes Nutzungsprotokoll geführt
einkauf-002:L19 :: richten einen gemeinsamen Eskalationsweg über die jeweiligen Werkleitu
einkauf-003:L17 :: wird ein gesondertes Nutzungsprotokoll geführt
einkauf-004:L17 :: wird ein gesondertes Nutzungsprotokoll geführt
einkauf-005:L19 :: Die Parteien benennen jeweils einen technischen Ansprechpartner
einkauf-006:L15 :: Die Parteien benennen jeweils eine verantwortliche Person für den Date
einkauf-007:L15 :: Die Parteien benennen jeweils einen kaufmännischen Ansprechpartner
einkauf-008:L15 :: Die Parteien führen bei Bedarf einen technischen Jour fixe durch
einkauf-009:L15 :: Die Parteien benennen jeweils einen technischen Ansprechpartner
einkauf-010:L15 :: Die Parteien benennen jeweils einen kaufmännischen Ansprechpartner
einkauf-011:L15 :: Die Parteien benennen jeweils eine verantwortliche Person
einkauf-012:L15 :: Die Parteien führen bei Bedarf einen technischen Jour fixe durch
einkauf-013:L17 :: Die Parteien benennen jeweils einen technischen Ansprechpartner
einkauf-014:L17 :: Die Parteien benennen jeweils einen technischen Ansprechpartner
einkauf-015:L17 :: Die Parteien benennen jeweils einen technischen Ansprechpartner
einkauf-016:L15 :: Die Parteien benennen jeweils einen technischen Ansprechpartner
finanz-001:L77 :: zuerst mit Nebenforderungen und Kosten
finanz-002:L81 :: keine Vorfälligkeitsentschädigung.
finanz-003:L77 :: eine Bearbeitungsgebühr von fünfzehn Euro zu erheben
finanz-004:L81 :: eine Bearbeitungsgebühr von fünfzehn Euro zu erheben
finanz-005:L81 :: zuerst mit Nebenforderungen und Kosten
finanz-006:L81 :: keine Vorfälligkeitsentschädigung.
finanz-007:L81 :: eine Bearbeitungsgebühr von fünfzehn Euro zu erheben
finanz-008:L81 :: zuerst mit Nebenforderungen und Kosten
finanz-009:L81 :: keine Vorfälligkeitsentschädigung.
finanz-010:L81 :: eine Bearbeitungsgebühr von fünfzehn Euro zu erheben
finanz-011:L81 :: zuerst mit Nebenforderungen und Kosten
finanz-012:L81 :: keine Vorfälligkeitsentschädigung.
finanz-013:L81 :: eine Bearbeitungsgebühr von fünfzehn Euro zu erheben
finanz-014:L81 :: zuerst mit Nebenforderungen und Kosten
finanz-015:L81 :: keine Vorfälligkeitsentschädigung.
finanz-016:L81 :: eine Bearbeitungsgebühr von fünfzehn Euro zu erheben
finanz-017:L81 :: zuerst mit Nebenforderungen und Kosten
finanz-018:L81 :: keine Vorfälligkeitsentschädigung.
finanz-019:L81 :: eine Bearbeitungsgebühr von fünfzehn Euro zu erheben
finanz-020:L81 :: zuerst mit Nebenforderungen und Kosten
finanz-021:L81 :: keine Vorfälligkeitsentschädigung.
finanz-022:L81 :: eine Bearbeitungsgebühr von fünfzehn Euro zu erheben
finanz-023:L81 :: zuerst mit Nebenforderungen und Kosten
finanz-024:L81 :: keine Vorfälligkeitsentschädigung.
finanz-025:L81 :: eine Bearbeitungsgebühr von fünfzehn Euro zu erheben
finanz-026:L81 :: zuerst mit Nebenforderungen und Kosten
finanz-027:L81 :: keine Vorfälligkeitsentschädigung.
finanz-028:L81 :: eine Bearbeitungsgebühr von fünfzehn Euro zu erheben
finanz-029:L81 :: zuerst mit Nebenforderungen und Kosten
finanz-030:L81 :: keine Vorfälligkeitsentschädigung.
finanz-031:L81 :: eine Bearbeitungsgebühr von fünfzehn Euro zu erheben
finanz-032:L81 :: zuerst mit Nebenforderungen und Kosten
immobilien-001:L12 :: Büroflächen im Objekt Hafenstraße 12 zum gewerblichen Gebrauch
immobilien-002:L12 :: Der Verpächter überlässt dem Pächter die Gaststätte mit Küche und Bier
immobilien-003:L12 :: Der Vermieter überlässt dem Mieter Lager- und Umschlagsflächen in Linz
immobilien-004:L12 :: Der Vermieter überlässt der Praxisgemeinschaft Etage 4 im Objekt Centr
immobilien-005:L12 :: Der Verkäufer verkauft dem Käufer das Bürogebäude Ludwigstraße 44
immobilien-006:L12 :: Der Verkäufer verkauft dem Käufer die Grundstücke Flurstück 118/4 und 
immobilien-007:L12 :: Der Eigentümer bestellt dem Erbbauberechtigten ein Erbbaurecht
immobilien-008:L12 :: Der Verkäufer verkauft dem Käufer die Wohnanlage Donaufeld 3 mit 42 Ei
immobilien-009:L12 :: Der Makler wird mit dem Nachweis und der Vermittlung von Gewerbeimmobi
immobilien-010:L12 :: Die Maklerin übernimmt die Vermittlung von Renditeliegenschaften
immobilien-011:L12 :: Projektgrundstück Nordpark mit aufstehendem Rohbau
immobilien-012:L12 :: Der Bauträger errichtet auf dem Grundstück Rheinufer 22 ein Mehrfamili
immobilien-013:L12 :: Verwaltung der Wohnungseigentumsanlage Lindenpark mit 64 Einheiten
immobilien-014:L12 :: Grundlagenermittlung, Vorplanung und Bauüberwachung
immobilien-015:L12 :: Ingenieurbauwerke der Losabschnitte 3 und 4
immobilien-016:L12 :: Fläche 2.12 im Einkaufszentrum Donau-Park
immobilien-017:L7 :: Erwerbergemeinschaft Aydin und Kroll GbR, Bremen
immobilien-018:L12 :: technische Anlagentechnik der Gebäude A bis D
immobilien-019:L12 :: Areal Güterbahnhof Rheinfeld bis zur Baureife
immobilien-020:L12 :: Flächen Flurstück 114 und 117 zur Ackernutzung
immobilien-021:L12 :: vier Arbeitsplätze im Co-Working-Center Hafenkante
immobilien-022:L12 :: Wohn- und Geschäftshaus Salzstraße 14 mit 22 Mieteinheiten
immobilien-023:L12 :: Nutzung der Dachflächen der Halle 3
immobilien-024:L12 :: Innenausbau der Stationen 2 und 3 des Klinikparks
immobilien-025:L12 :: Fläche C 7 im Fachmarktzentrum Nordharz
immobilien-026:L12 :: Quartiers Rosengarten mit 88 Einheiten
immobilien-027:L12 :: Wohnanlage Seeblick mit 18 Eigentumswohnungen
immobilien-028:L12 :: Grundstück An der Kohlebahn 9 mit einer Fläche von 42.000 Quadratmeter
immobilien-029:L12 :: Konferenztrakt mit 14 Räumen
immobilien-030:L12 :: 320 Stellplätze im Parkhaus City Nord
immobilien-031:L12 :: Seniorenresidenz Lindenhof mit 96 Pflegeplätzen
immobilien-032:L12 :: Nutzung der Fassadenflächen zur Außenwerbung
marketing-001:L19 :: Ein Abruf ist die schriftliche Beauftragung einzelner Leistungen durch
marketing-002:L21 :: Art und Umfang der Werbeleistungen werden zu Saisonbeginn in einem Anh
marketing-003:L21 :: Kennzeichnungspflichten nach den Vorgaben der Plattformen werden einge
marketing-004:L19 :: Platzierungen erfolgen nach Verfügbarkeit und werden im Kampagnenplan 
marketing-005:L21 :: Die Freigabe erfolgt durch die Projektleitung des Auftraggebers in Tex
marketing-006:L21 :: Nicht abgerufene Stunden verfallen am Ende des Kalendermonats
marketing-007:L21 :: Eine Unterlizenzierung bedarf der Zustimmung des Lizenzgebers
marketing-008:L16 :: Der Anbieter betreibt für den Kunden Auswertungen zu Absatz- und Sorti
marketing-009:L16 :: Der Anbieter betreibt für den Kunden den programmatischen Einkauf digi
marketing-010:L19 :: Die Agentur erhält eine Provision von fünfzehn Prozent der vermittelte
marketing-011:L16 :: Der Auftragnehmer trägt die Marke des Auftraggebers während der verein
marketing-012:L16 :: Der Lieferant produziert und liefert Werbemittel und Druckerzeugnisse
marketing-013:L16 :: Der Auftragnehmer überlässt dem Besteller Räume und Veranstaltungstech
marketing-014:L16 :: Der Auftragnehmer produziert Film- und Bildinhalte für die Markenkommu
marketing-015:L16 :: Der Auftragnehmer betreut die Kanäle des Auftraggebers und erstellt re
marketing-016:L16 :: Der Auftragnehmer vermittelt dem Kunden Werbeplätze und erhält dafür e
marketing-017:L16 :: Der Auftragnehmer betreut die Krisenkommunikation des Kunden und berät
marketing-018:L16 :: Der Auftragnehmer stellt Werbeplätze und Creatornutzung für die Kampag
marketing-019:L16 :: Der Auftragnehmer räumt dem Auftraggeber Namensrechte an der Mehrzweck
marketing-020:L16 :: Der Lizenzgeber überlässt dem Lizenznehmer Bild- und Videomaterial zur
marketing-021:L16 :: Der Auftragnehmer erstellt mit eigenen Creatorn werbliche Inhalte für 
marketing-022:L16 :: Der Auftragnehmer konzipiert und betreut die Sponsoringaktivierung des
marketing-023:L16 :: Der Auftragnehmer plant und kauft Werbeplätze für die Kampagnen des Au
marketing-024:L16 :: Der Auftragnehmer betreut die Presse- und Öffentlichkeitsarbeit des Au
marketing-025:L16 :: Der Verein trägt Werbung des Sponsors auf Trikots und Banden bei den R
marketing-026:L16 :: Der Auftragnehmer belegt Werbeflächen im Stadtgebiet und dokumentiert 
marketing-027:L16 :: Der Auftragnehmer plant und errichtet den Messestand des Kunden an der
marketing-028:L16 :: Der Auftragnehmer erstellt eine Fallstudie über die Zusammenarbeit mit
marketing-029:L16 :: Der Anbieter trägt das Logo des Kunden auf den Trikots der ersten Mann
marketing-030:L16 :: Der Auftragnehmer wertet öffentliche Beiträge über den Kunden und sein
marketing-031:L16 :: Der Auftragnehmer bucht und betreibt digitale Werbeflächen für den Kun
marketing-032:L16 :: Der Lizenzgeber überlässt dem Lizenznehmer Musikstücke und Bildmateria
personal-001:L41 :: Die Geltendmachung eines weitergehenden Schadens bleibt vorbehalten
personal-002:L75 :: Die Arbeitgeberin weist die Arbeitnehmerin auf die Möglichkeit der Ein
personal-003:L31 :: Der Kunde prüft die vorgelegten Nachweise vor dem ersten Einsatz
personal-004:L15 :: Die Parteien können den Vertrag einvernehmlich jederzeit beenden
personal-005:L55 :: Nebenabreden werden nur mit Zustimmung beider Parteien wirksam
personal-006:L43 :: Der Anbieter darf die gewonnenen Erkenntnisse nach Projektende für eig
personal-007:L53 :: Der Vertrag endet mit Ablauf der vereinbarten Projektlaufzeit
personal-009:L21 :: Eine Verlängerung des Vertrages ist nur mit Zustimmung beider Parteien
personal-010:L57 :: Der Vertrag endet mit Ablauf der vereinbarten Einsatzdauer
personal-011:L61 :: Der Vertrag läuft bis zur Abnahme des letzten Arbeitspakets
personal-012:L51 :: Der Vertrag endet mit Abschluss der Vermittlung
personal-013:L57 :: Der Vertrag wird auf unbestimmte Zeit geschlossen
personal-014:L55 :: Die Überlasserin kann den Einsatzort nach betrieblichem Bedarf verlege
personal-015:L7 :: Der Interim Manager berichtet an den Vorstand des Auftraggebers
personal-016:L43 :: Rechnungen sind innerhalb von vierzehn Tagen ohne Abzug zu zahlen
personal-017:L51 :: Die Zusammenarbeit beginnt am 1. März 2026
personal-018:L5 :: Der Geschäftsführer leitet die Gesellschaft als alleinvertretungsberec
personal-019:L7 :: Der Arbeitsort ist Berlin
personal-020:L5 :: Der Anbieter überlässt dem Kunden gewerbliche Arbeitnehmer zur Arbeits
personal-021:L7 :: Der Interim Manager berichtet an den Vorstand des Auftraggebers
personal-022:L5 :: Der Anbieter erbringt Beratungsleistungen für die Datenplattform des K
personal-023:L5 :: Der Anbieter unterstützt den Kunden bei der Besetzung von Führungsposi
personal-024:L7 :: (2) Der Arbeitsort ist Köln.
personal-025:L5 :: Der Anbieter überlässt dem Kunden gewerbliche Arbeitnehmer zur Arbeits
personal-026:L7 :: Der Interim Manager berichtet an den Vorstand des Auftraggebers
personal-027:L5 :: Der Anbieter erbringt Beratungsleistungen für die Datenplattform des K
personal-028:L5 :: Der Anbieter unterstützt den Kunden bei der Besetzung von Führungsposi
personal-029:L5 :: Der Geschäftsführer vertritt die Gesellschaft allein
personal-030:L7 :: (2) Der Arbeitsort ist Bremen.
personal-031:L5 :: Der Anbieter erbringt Beratungsleistungen für die Vertragsplattform de
personal-032:L5 :: Der Anbieter überlässt dem Kunden gewerbliche Arbeitnehmer zur Arbeits
service-001:L80 :: Die Einhaltung terminlicher Zusagen des Anbieters steht unter dem Vorb
service-001:L81 :: Der Kunde trägt Aufwendungen für Reisen, Unterkunft und Schulungen auc
service-002:L80 :: Über die Verlängerung notwendiger Zertifizierungen entscheidet der Anb
service-002:L81 :: Der Anbieter darf eingesetzte Fachkräfte nach Anlage 5 austauschen, oh
service-003:L80 :: Die Einhaltung terminlicher Zusagen des Anbieters steht unter dem Vorb
service-003:L81 :: Der Kunde trägt Aufwendungen für Reisen, Unterkunft und Schulungen auc
service-004:L80 :: Über die Verlängerung notwendiger Zertifizierungen entscheidet der Anb
service-004:L81 :: Der Anbieter darf eingesetzte Fachkräfte nach Anlage 5 austauschen, oh
service-005:L80 :: Die Einhaltung terminlicher Zusagen des Anbieters steht unter dem Vorb
service-005:L81 :: Der Kunde trägt Aufwendungen für Reisen, Unterkunft und Schulungen auc
service-006:L80 :: Über die Verlängerung notwendiger Zertifizierungen entscheidet der Anb
service-006:L81 :: Der Anbieter darf eingesetzte Fachkräfte nach Anlage 5 austauschen, oh
service-007:L80 :: Die Einhaltung terminlicher Zusagen des Anbieters steht unter dem Vorb
service-007:L81 :: Der Kunde trägt Aufwendungen für Reisen, Unterkunft und Schulungen auc
service-008:L80 :: Über die Verlängerung notwendiger Zertifizierungen entscheidet der Anb
service-008:L81 :: Der Anbieter darf eingesetzte Fachkräfte nach Anlage 5 austauschen, oh
service-009:L80 :: Die Einhaltung terminlicher Zusagen des Anbieters steht unter dem Vorb
service-009:L81 :: Der Kunde trägt Aufwendungen für Reisen, Unterkunft und Schulungen auc
service-010:L80 :: Über die Verlängerung notwendiger Zertifizierungen entscheidet der Anb
service-010:L81 :: Der Anbieter darf eingesetzte Fachkräfte nach Anlage 5 austauschen, oh
service-011:L80 :: Die Einhaltung terminlicher Zusagen des Anbieters steht unter dem Vorb
service-011:L81 :: Der Kunde trägt Aufwendungen für Reisen, Unterkunft und Schulungen auc
service-012:L80 :: Über die Verlängerung notwendiger Zertifizierungen entscheidet der Anb
service-012:L81 :: Der Anbieter darf eingesetzte Fachkräfte nach Anlage 5 austauschen, oh
service-013:L80 :: Die Einhaltung terminlicher Zusagen des Anbieters steht unter dem Vorb
service-013:L81 :: Der Kunde trägt Aufwendungen für Reisen, Unterkunft und Schulungen auc
service-014:L80 :: Über die Verlängerung notwendiger Zertifizierungen entscheidet der Anb
service-014:L81 :: Der Anbieter darf eingesetzte Fachkräfte nach Anlage 5 austauschen, oh
service-015:L80 :: Die Einhaltung terminlicher Zusagen des Anbieters steht unter dem Vorb
service-015:L81 :: Der Kunde trägt Aufwendungen für Reisen, Unterkunft und Schulungen auc
service-016:L80 :: Über die Verlängerung notwendiger Zertifizierungen entscheidet der Anb
service-016:L81 :: Der Anbieter darf eingesetzte Fachkräfte nach Anlage 5 austauschen, oh
service-017:L80 :: Die Einhaltung terminlicher Zusagen des Anbieters steht unter dem Vorb
service-017:L81 :: Der Kunde trägt Aufwendungen für Reisen, Unterkunft und Schulungen auc
service-018:L80 :: Über die Verlängerung notwendiger Zertifizierungen entscheidet der Anb
service-018:L81 :: Der Anbieter darf eingesetzte Fachkräfte nach Anlage 5 austauschen, oh
service-019:L80 :: Die Einhaltung terminlicher Zusagen des Anbieters steht unter dem Vorb
service-019:L81 :: Der Kunde trägt Aufwendungen für Reisen, Unterkunft und Schulungen auc
service-020:L80 :: Über die Verlängerung notwendiger Zertifizierungen entscheidet der Anb
service-020:L81 :: Der Anbieter darf eingesetzte Fachkräfte nach Anlage 5 austauschen, oh
service-021:L80 :: Die Einhaltung terminlicher Zusagen des Anbieters steht unter dem Vorb
service-021:L81 :: Der Kunde trägt Aufwendungen für Reisen, Unterkunft und Schulungen auc
service-022:L80 :: Über die Verlängerung notwendiger Zertifizierungen entscheidet der Anb
service-022:L81 :: Der Anbieter darf eingesetzte Fachkräfte nach Anlage 5 austauschen, oh
service-023:L80 :: Die Einhaltung terminlicher Zusagen des Anbieters steht unter dem Vorb
service-023:L81 :: Der Kunde trägt Aufwendungen für Reisen, Unterkunft und Schulungen auc
service-024:L80 :: Über die Verlängerung notwendiger Zertifizierungen entscheidet der Anb
service-024:L81 :: Der Anbieter darf eingesetzte Fachkräfte nach Anlage 5 austauschen, oh
service-025:L80 :: Die Einhaltung terminlicher Zusagen des Anbieters steht unter dem Vorb
service-025:L81 :: Der Kunde trägt Aufwendungen für Reisen, Unterkunft und Schulungen auc
service-026:L80 :: Über die Verlängerung notwendiger Zertifizierungen entscheidet der Anb
service-026:L81 :: Der Anbieter darf eingesetzte Fachkräfte nach Anlage 5 austauschen, oh
service-027:L80 :: Die Einhaltung terminlicher Zusagen des Anbieters steht unter dem Vorb
service-027:L81 :: Der Kunde trägt Aufwendungen für Reisen, Unterkunft und Schulungen auc
service-028:L80 :: Über die Verlängerung notwendiger Zertifizierungen entscheidet der Anb
service-028:L81 :: Der Anbieter darf eingesetzte Fachkräfte nach Anlage 5 austauschen, oh
service-029:L80 :: Die Einhaltung terminlicher Zusagen des Anbieters steht unter dem Vorb
service-029:L81 :: Der Kunde trägt Aufwendungen für Reisen, Unterkunft und Schulungen auc
service-030:L80 :: Über die Verlängerung notwendiger Zertifizierungen entscheidet der Anb
service-030:L81 :: Der Anbieter darf eingesetzte Fachkräfte nach Anlage 5 austauschen, oh
service-031:L80 :: Die Einhaltung terminlicher Zusagen des Anbieters steht unter dem Vorb
service-031:L81 :: Der Kunde trägt Aufwendungen für Reisen, Unterkunft und Schulungen auc
service-032:L80 :: Über die Verlängerung notwendiger Zertifizierungen entscheidet der Anb
service-032:L81 :: Der Anbieter darf eingesetzte Fachkräfte nach Anlage 5 austauschen, oh
werk-001:L45 :: Der Kunde übermittelt dem Anbieter die erforderlichen Testdaten in ein
werk-002:L36 :: Die Sperrung lässt die Zahlungspflicht des Kunden unberührt
werk-003:L16 :: Die Parteien arbeiten bei der Fehleranalyse eng zusammen
werk-004:L19 :: Der Kunde stellt die erforderlichen Räume kostenfrei zur Verfügung
werk-005:L49 :: Der Kunde kann der Nennung jederzeit widersprechen
werk-007:L49 :: Der Kunde kann der Nennung jederzeit widersprechen
werk-008:L51 :: Der Kunde kann eine gemeinsame Auswertung verlangen
werk-009:L63 :: Die Parteien benennen je einen Ansprechpartner für die Projektabwicklu
werk-010:L63 :: Der Kunde erhält Zugang zu einer Testumgebung
werk-011:L63 :: Die Parteien benennen je einen Ansprechpartner für die Projektabwicklu
werk-012:L63 :: Der Kunde erhält Zugang zu einer Testumgebung
werk-013:L63 :: Die Datenübertragung erfolgt verschlüsselt über eine gesicherte Verbin
werk-014:L63 :: Der Anbieter stellt die Anwendung ab dem vereinbarten Bereitstellungst
werk-015:L63 :: Jede Partei trägt ihre eigenen Kosten aus der Vertragsverhandlung
werk-016:L63 :: Die Parteien behandeln die Vertragsverhandlung vertraulich
werk-017:L63 :: Die Parteien benennen je einen Ansprechpartner für die Projektabwicklu
werk-018:L63 :: Der Kunde erhält Zugang zu einer Testumgebung
werk-019:L63 :: Der Anbieter übergibt den Quellcode nach vollständiger Zahlung der Ver
werk-020:L63 :: Der Kunde darf die Anwendung für eigene Geschäftszwecke nutzen
werk-021:L63 :: Die Datenübertragung erfolgt verschlüsselt über eine gesicherte Verbin
werk-022:L63 :: Der Anbieter stellt die Anwendung ab dem vereinbarten Bereitstellungst
werk-023:L63 :: Jede Partei trägt ihre eigenen Kosten aus der Vertragsverhandlung
werk-024:L63 :: Die Parteien behandeln die Vertragsverhandlung vertraulich
werk-025:L63 :: Die Parteien benennen je einen Ansprechpartner für die Projektabwicklu
werk-026:L63 :: Der Kunde erhält Zugang zu einer Testumgebung
werk-027:L63 :: Der Anbieter übergibt den Quellcode nach vollständiger Zahlung der Ver
werk-028:L63 :: Jede Partei trägt ihre eigenen Kosten aus der Vertragsverhandlung
werk-029:L63 :: Die Datenübertragung erfolgt verschlüsselt über eine gesicherte Verbin
werk-030:L63 :: Die Parteien behandeln die Vertragsverhandlung vertraulich
werk-031:L63 :: Jede Partei trägt ihre eigenen Kosten aus der Vertragsverhandlung
werk-032:L63 :: Die Parteien behandeln die Vertragsverhandlung vertraulich
~~~
### von einem Detektor erfasst (Overreach-Kandidat)
~~~text
notice_period @personal-008:L19 :: Der Auftraggeber kann den Vertrag nur mit einer Frist von 6 
non_compete @werk-006:L41 :: Eine Karenzentschädigung wird nicht gezahlt
~~~

## Korpus je Vertrag

| Vertrag | Wurzel | Domäne | Befunde | Score | erkannt | Fehlalarme | ungedeckt |
|---|---|---|---|---|---|---|---|
| daten-001 | corpus | NDA | 9 | 100 | 8/8 | 0 | 1 |
| daten-002 | corpus | Auftragsverarbeitung | 9 | 100 | 8/8 | 0 | 1 |
| daten-003 | corpus | Lizenzvertrag | 9 | 100 | 8/8 | 0 | 1 |
| daten-004 | corpus | IP-Uebertragung | 8 | 80 | 8/8 | 0 | 1 |
| daten-005 | corpus | Open-Source-Compliance | 9 | 100 | 8/8 | 0 | 1 |
| daten-006 | corpus | Datenraum | 9 | 100 | 8/8 | 0 | 1 |
| daten-007 | corpus | Coproduktion | 9 | 100 | 8/8 | 0 | 1 |
| daten-008 | corpus | Forschungskooperation | 10 | 100 | 8/8 | 0 | 1 |
| daten-009 | corpus | NDA | 8 | 80 | 8/8 | 0 | 1 |
| daten-010 | corpus | Auftragsverarbeitung | 9 | 100 | 8/8 | 0 | 1 |
| daten-011 | corpus | Lizenzvertrag | 9 | 100 | 8/8 | 0 | 1 |
| daten-012 | corpus | IP-Uebertragung | 8 | 96 | 8/8 | 0 | 1 |
| daten-013 | corpus | Open-Source-Compliance | 8 | 80 | 8/8 | 0 | 1 |
| daten-014 | corpus | Datenraum | 10 | 100 | 8/8 | 0 | 1 |
| daten-015 | corpus | Coproduktion | 9 | 100 | 8/8 | 0 | 1 |
| daten-016 | corpus | Forschungskooperation | 9 | 100 | 8/8 | 0 | 1 |
| daten-017 | corpus | NDA | 10 | 100 | 8/8 | 0 | 1 |
| daten-018 | corpus | Auftragsverarbeitung | 9 | 100 | 8/8 | 0 | 1 |
| daten-019 | corpus | Lizenzvertrag | 8 | 100 | 8/8 | 0 | 1 |
| daten-020 | corpus | IP-Uebertragung | 8 | 96 | 8/8 | 0 | 1 |
| daten-021 | corpus | Open-Source-Compliance | 8 | 72 | 8/8 | 0 | 1 |
| daten-022 | corpus | Datenraum | 9 | 100 | 8/8 | 0 | 1 |
| daten-023 | corpus | Coproduktion | 9 | 100 | 8/8 | 0 | 1 |
| daten-024 | corpus | Forschungskooperation | 9 | 100 | 8/8 | 0 | 1 |
| daten-025 | corpus | NDA | 9 | 95 | 8/8 | 0 | 1 |
| daten-026 | corpus | Auftragsverarbeitung | 8 | 80 | 8/8 | 0 | 1 |
| daten-027 | corpus | Lizenzvertrag | 9 | 100 | 8/8 | 0 | 1 |
| daten-028 | corpus | IP-Uebertragung | 9 | 100 | 8/8 | 0 | 1 |
| daten-029 | corpus | Open-Source-Compliance | 8 | 96 | 8/8 | 0 | 1 |
| daten-030 | corpus | Datenraum | 8 | 80 | 8/8 | 0 | 1 |
| daten-031 | corpus | Coproduktion | 10 | 100 | 8/8 | 0 | 1 |
| daten-032 | corpus | Forschungskooperation | 10 | 100 | 8/8 | 0 | 1 |
| einkauf-001 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches und oesterreichisches Recht | 7 | 89 | 7/7 | 0 | 1 |
| einkauf-002 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches und oesterreichisches Recht | 10 | 100 | 7/7 | 0 | 1 |
| einkauf-003 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches und oesterreichisches Recht | 8 | 96 | 8/8 | 0 | 1 |
| einkauf-004 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches und oesterreichisches Recht | 8 | 72 | 7/7 | 0 | 1 |
| einkauf-005 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches und oesterreichisches Recht | 7 | 100 | 7/7 | 0 | 1 |
| einkauf-006 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches und oesterreichisches Recht | 7 | 73 | 7/7 | 0 | 1 |
| einkauf-007 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches und oesterreichisches Recht | 7 | 89 | 7/7 | 0 | 1 |
| einkauf-008 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches und oesterreichisches Recht | 7 | 89 | 7/7 | 0 | 1 |
| einkauf-009 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches und oesterreichisches Recht | 7 | 81 | 7/7 | 0 | 1 |
| einkauf-010 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches und oesterreichisches Recht | 7 | 89 | 7/7 | 0 | 1 |
| einkauf-011 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches und oesterreichisches Recht | 8 | 80 | 8/8 | 0 | 1 |
| einkauf-012 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches und oesterreichisches Recht | 10 | 100 | 8/8 | 0 | 1 |
| einkauf-013 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches Recht | 8 | 100 | 8/8 | 0 | 1 |
| einkauf-014 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches Recht | 8 | 96 | 8/8 | 0 | 1 |
| einkauf-015 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches Recht | 10 | 100 | 8/8 | 0 | 1 |
| einkauf-016 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches Recht | 8 | 88 | 8/8 | 0 | 1 |
| einkauf-017 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches Recht | 8 | 88 | 8/8 | 0 | 0 |
| einkauf-018 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches Recht | 8 | 100 | 8/8 | 0 | 0 |
| einkauf-019 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches Recht | 10 | 100 | 8/8 | 0 | 0 |
| einkauf-020 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches Recht | 8 | 88 | 8/8 | 0 | 0 |
| einkauf-021 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches Recht | 8 | 100 | 8/8 | 0 | 0 |
| einkauf-022 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches Recht | 8 | 80 | 8/8 | 0 | 0 |
| einkauf-023 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches Recht | 9 | 100 | 8/8 | 0 | 0 |
| einkauf-024 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches Recht | 8 | 100 | 8/8 | 0 | 0 |
| einkauf-025 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches Recht | 8 | 96 | 8/8 | 0 | 0 |
| einkauf-026 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches Recht | 8 | 88 | 8/8 | 0 | 0 |
| einkauf-027 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches Recht | 9 | 100 | 8/8 | 0 | 0 |
| einkauf-028 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches Recht | 8 | 100 | 8/8 | 0 | 0 |
| einkauf-029 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches Recht | 8 | 88 | 8/8 | 0 | 0 |
| einkauf-030 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches Recht | 8 | 100 | 8/8 | 0 | 0 |
| einkauf-031 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches Recht | 8 | 100 | 8/8 | 0 | 0 |
| einkauf-032 | corpus | Einkauf und Lieferantenrahmenvertrag, deutsches Recht | 10 | 100 | 8/8 | 0 | 0 |
| evasion-001 | corpus | evasion | 10 | 100 | 9/9 | 0 | 0 |
| evasion-002 | corpus | evasion | 9 | 100 | 9/9 | 0 | 0 |
| evasion-003 | corpus | evasion | 11 | 100 | 9/9 | 0 | 0 |
| evasion-004 | corpus | evasion | 9 | 100 | 9/9 | 0 | 0 |
| evasion-005 | corpus | evasion | 9 | 87 | 9/9 | 0 | 0 |
| evasion-006 | corpus | evasion | 10 | 100 | 9/9 | 0 | 0 |
| evasion-007 | corpus | evasion | 12 | 100 | 9/9 | 0 | 0 |
| evasion-008 | corpus | evasion | 10 | 100 | 9/9 | 0 | 0 |
| evasion-009 | corpus | evasion | 11 | 100 | 9/9 | 0 | 0 |
| evasion-010 | corpus | evasion | 23 | 100 | 9/9 | 0 | 0 |
| evasion-011 | corpus | evasion | 14 | 100 | 9/9 | 0 | 0 |
| evasion-012 | corpus | evasion | 10 | 100 | 9/9 | 0 | 0 |
| evasion-013 | corpus | evasion | 10 | 100 | 9/9 | 0 | 0 |
| evasion-014 | corpus | evasion | 10 | 100 | 9/9 | 0 | 0 |
| evasion-015 | corpus | evasion | 9 | 87 | 9/9 | 0 | 0 |
| evasion-016 | corpus | evasion | 10 | 100 | 9/9 | 0 | 0 |
| evasion-017 | corpus | evasion | 11 | 100 | 9/9 | 0 | 0 |
| evasion-018 | corpus | evasion | 11 | 100 | 9/9 | 0 | 0 |
| evasion-019 | corpus | evasion | 9 | 100 | 9/9 | 0 | 0 |
| evasion-020 | corpus | evasion | 13 | 100 | 9/9 | 0 | 0 |
| evasion-021 | corpus | evasion | 10 | 100 | 9/9 | 0 | 0 |
| evasion-022 | corpus | evasion | 10 | 100 | 9/9 | 0 | 0 |
| evasion-023 | corpus | evasion | 21 | 100 | 9/9 | 0 | 0 |
| evasion-024 | corpus | evasion | 16 | 100 | 9/9 | 0 | 0 |
| evasion-025 | corpus | evasion | 9 | 95 | 9/9 | 0 | 0 |
| evasion-026 | corpus | evasion | 10 | 100 | 9/9 | 0 | 0 |
| evasion-027 | corpus | evasion | 9 | 100 | 9/9 | 0 | 0 |
| evasion-028 | corpus | evasion | 12 | 100 | 9/9 | 0 | 0 |
| evasion-029 | corpus | evasion | 10 | 100 | 9/9 | 0 | 0 |
| evasion-030 | corpus | evasion | 11 | 100 | 9/9 | 0 | 0 |
| evasion-031 | corpus | evasion | 11 | 100 | 9/9 | 0 | 0 |
| evasion-032 | corpus | evasion | 10 | 100 | 9/9 | 0 | 0 |
| finanz-001 | corpus | Darlehen | 8 | 100 | 8/8 | 0 | 1 |
| finanz-002 | corpus | Leasing | 10 | 100 | 8/8 | 0 | 1 |
| finanz-003 | corpus | Factoring | 9 | 100 | 8/8 | 0 | 1 |
| finanz-004 | corpus | Darlehen | 9 | 100 | 8/8 | 0 | 1 |
| finanz-005 | corpus | Leasing | 8 | 100 | 8/8 | 0 | 1 |
| finanz-006 | corpus | Factoring | 9 | 100 | 8/8 | 0 | 1 |
| finanz-007 | corpus | Zahlungsdienst | 9 | 100 | 8/8 | 0 | 1 |
| finanz-008 | corpus | Inkasso | 8 | 72 | 8/8 | 0 | 1 |
| finanz-009 | corpus | Buergschaft | 8 | 80 | 8/8 | 0 | 1 |
| finanz-010 | corpus | Aval | 10 | 100 | 8/8 | 0 | 1 |
| finanz-011 | corpus | Kreditlinie | 9 | 100 | 8/8 | 0 | 1 |
| finanz-012 | corpus | Finanzierungsvermittlung | 9 | 100 | 8/8 | 0 | 1 |
| finanz-013 | corpus | Darlehen | 8 | 96 | 8/8 | 0 | 1 |
| finanz-014 | corpus | Leasing | 10 | 100 | 8/8 | 0 | 1 |
| finanz-015 | corpus | Factoring | 8 | 80 | 8/8 | 0 | 1 |
| finanz-016 | corpus | Zahlungsdienst | 9 | 100 | 8/8 | 0 | 1 |
| finanz-017 | corpus | Inkasso | 8 | 100 | 8/8 | 0 | 1 |
| finanz-018 | corpus | Buergschaft | 9 | 100 | 8/8 | 0 | 1 |
| finanz-019 | corpus | Aval | 8 | 96 | 8/8 | 0 | 1 |
| finanz-020 | corpus | Kreditlinie | 8 | 88 | 8/8 | 0 | 1 |
| finanz-021 | corpus | Finanzierungsvermittlung | 9 | 95 | 8/8 | 0 | 1 |
| finanz-022 | corpus | Darlehen | 9 | 100 | 8/8 | 0 | 1 |
| finanz-023 | corpus | Leasing | 9 | 100 | 8/8 | 0 | 1 |
| finanz-024 | corpus | Factoring | 10 | 100 | 8/8 | 0 | 1 |
| finanz-025 | corpus | Zahlungsdienst | 8 | 96 | 8/8 | 0 | 1 |
| finanz-026 | corpus | Inkasso | 10 | 100 | 8/8 | 0 | 1 |
| finanz-027 | corpus | Buergschaft | 8 | 88 | 8/8 | 0 | 1 |
| finanz-028 | corpus | Aval | 9 | 100 | 8/8 | 0 | 1 |
| finanz-029 | corpus | Kreditlinie | 9 | 100 | 8/8 | 0 | 1 |
| finanz-030 | corpus | Finanzierungsvermittlung | 8 | 100 | 8/8 | 0 | 1 |
| finanz-031 | corpus | Darlehen | 8 | 88 | 8/8 | 0 | 1 |
| finanz-032 | corpus | Leasing | 8 | 80 | 8/8 | 0 | 1 |
| immobilien-001 | corpus | Gewerbemiete | 8 | 100 | 7/7 | 0 | 1 |
| immobilien-002 | corpus | Miete und Pacht | 9 | 100 | 7/7 | 0 | 1 |
| immobilien-003 | corpus | Gewerbemiete | 7 | 81 | 7/7 | 0 | 1 |
| immobilien-004 | corpus | Gewerbemiete | 8 | 88 | 7/7 | 0 | 1 |
| immobilien-005 | corpus | Immobilienkauf | 6 | 74 | 6/6 | 0 | 1 |
| immobilien-006 | corpus | Immobilienkauf | 8 | 88 | 7/7 | 0 | 1 |
| immobilien-007 | corpus | Erbbaurecht | 7 | 81 | 7/7 | 0 | 1 |
| immobilien-008 | corpus | Immobilienkauf | 7 | 100 | 7/7 | 0 | 1 |
| immobilien-009 | corpus | Immobilienmakler | 7 | 97 | 7/7 | 0 | 1 |
| immobilien-010 | corpus | Immobilienmakler | 8 | 100 | 7/7 | 0 | 1 |
| immobilien-011 | corpus | Immobilienkauf | 7 | 89 | 7/7 | 0 | 1 |
| immobilien-012 | corpus | Bauträger | 8 | 96 | 7/7 | 0 | 1 |
| immobilien-013 | corpus | Verwaltervertrag | 7 | 81 | 7/7 | 0 | 1 |
| immobilien-014 | corpus | Architektenvertrag | 7 | 89 | 7/7 | 0 | 1 |
| immobilien-015 | corpus | Bauvertrag | 7 | 81 | 7/7 | 0 | 1 |
| immobilien-016 | corpus | Gewerbemiete | 9 | 100 | 7/7 | 0 | 1 |
| immobilien-017 | corpus | Bautraeger | 6 | 66 | 6/6 | 0 | 1 |
| immobilien-018 | corpus | Facility Management | 7 | 73 | 7/7 | 0 | 1 |
| immobilien-019 | corpus | Projektentwicklung | 7 | 73 | 7/7 | 0 | 1 |
| immobilien-020 | corpus | Miete und Pacht | 7 | 81 | 7/7 | 0 | 1 |
| immobilien-021 | corpus | Gewerbemiete | 6 | 74 | 6/6 | 0 | 1 |
| immobilien-022 | corpus | Immobilienkauf | 7 | 81 | 7/7 | 0 | 1 |
| immobilien-023 | corpus | Miete und Pacht | 7 | 89 | 7/7 | 0 | 1 |
| immobilien-024 | corpus | Bauvertrag | 7 | 97 | 7/7 | 0 | 1 |
| immobilien-025 | corpus | Gewerbemiete | 6 | 66 | 6/6 | 0 | 1 |
| immobilien-026 | corpus | Verwaltervertrag | 8 | 80 | 7/7 | 0 | 1 |
| immobilien-027 | corpus | Bautraeger | 7 | 89 | 7/7 | 0 | 1 |
| immobilien-028 | corpus | Immobilienkauf | 9 | 100 | 7/7 | 0 | 1 |
| immobilien-029 | corpus | Bauvertrag | 8 | 100 | 7/7 | 0 | 1 |
| immobilien-030 | corpus | Gewerbemiete | 7 | 81 | 7/7 | 0 | 1 |
| immobilien-031 | corpus | Seniorenimmobilie | 7 | 73 | 7/7 | 0 | 1 |
| immobilien-032 | corpus | Aussenwerbung | 9 | 100 | 7/7 | 0 | 1 |
| logistik-001 | corpus | Spedition nationaler Strassengueterverkehr (DE) | 8 | 100 | 8/8 | 0 | 0 |
| logistik-002 | corpus | Spedition international / CMR (EU, DE/NL) | 10 | 100 | 8/8 | 0 | 0 |
| logistik-003 | corpus | Spedition und Kontraktlogistik (AT) | 9 | 95 | 7/7 | 0 | 0 |
| logistik-004 | corpus | Gefahrgutlogistik / ADR (CH) | 7 | 89 | 6/6 | 0 | 0 |
| logistik-005 | corpus | Kurier-, Express- und Paketdienste | 8 | 100 | 6/6 | 0 | 0 |
| logistik-006 | corpus | Lagerlogistik | 8 | 88 | 7/7 | 0 | 0 |
| logistik-007 | corpus | Just-in-Sequence-Zulieferung | 8 | 96 | 7/7 | 0 | 0 |
| logistik-008 | corpus | Kontraktlogistik Chemie | 7 | 65 | 7/7 | 0 | 0 |
| logistik-009 | corpus | Fulfillment und E-Commerce-Logistik | 8 | 100 | 7/7 | 0 | 0 |
| logistik-010 | corpus | Kühllogistik | 8 | 100 | 7/7 | 0 | 0 |
| logistik-011 | corpus | Ersatzteillogistik Schweiz | 11 | 100 | 7/7 | 0 | 0 |
| logistik-012 | corpus | Internationaler Straßentransport CMR | 8 | 100 | 7/7 | 0 | 0 |
| logistik-013 | corpus | Multimodale Logistik | 9 | 100 | 6/6 | 0 | 0 |
| logistik-014 | corpus | Schienenlogistik und Umschlag | 7 | 81 | 6/6 | 0 | 0 |
| logistik-015 | corpus | Luftfracht | 6 | 74 | 6/6 | 0 | 0 |
| logistik-016 | corpus | Retail-Logistik | 8 | 88 | 7/7 | 0 | 0 |
| logistik-017 | corpus | Pharmalogistik | 9 | 95 | 7/7 | 0 | 0 |
| logistik-018 | corpus | Gefahrgutlagerung | 7 | 65 | 6/6 | 0 | 0 |
| logistik-019 | corpus | Baustellenlogistik | 7 | 89 | 6/6 | 0 | 0 |
| logistik-020 | corpus | Zollagentur | 7 | 89 | 6/6 | 0 | 0 |
| logistik-021 | corpus | Zolllager | 6 | 74 | 6/6 | 0 | 0 |
| logistik-022 | corpus | Exportkontrolle | 7 | 81 | 6/6 | 0 | 0 |
| logistik-023 | corpus | Präferenznachweise | 7 | 73 | 7/7 | 0 | 0 |
| logistik-024 | corpus | Verpackung | 7 | 97 | 6/6 | 0 | 0 |
| logistik-025 | corpus | Lohnverpackung | 7 | 81 | 6/6 | 0 | 0 |
| logistik-026 | corpus | Verpackungsmanagement | 6 | 58 | 6/6 | 0 | 0 |
| logistik-027 | corpus | Facility Management | 9 | 95 | 7/7 | 0 | 0 |
| logistik-028 | corpus | Facility Management Technik | 7 | 89 | 7/7 | 0 | 0 |
| logistik-029 | corpus | Facility Management Sicherheit | 8 | 100 | 6/6 | 0 | 0 |
| logistik-030 | corpus | Gebäudereinigung | 7 | 97 | 7/7 | 0 | 0 |
| logistik-031 | corpus | Glasfassadenreinigung | 6 | 66 | 6/6 | 0 | 0 |
| logistik-032 | corpus | Unterhaltsreinigung Österreich | 8 | 100 | 7/7 | 0 | 0 |
| marketing-001 | corpus | Marketingagentur, deutsches Recht | 8 | 100 | 8/8 | 0 | 1 |
| marketing-002 | corpus | Sponsoring, österreichisches Recht | 7 | 89 | 7/7 | 0 | 1 |
| marketing-003 | corpus | Influencer-Kooperation, deutsches Recht | 7 | 81 | 7/7 | 0 | 1 |
| marketing-004 | corpus | Mediaeinkauf, schweizerisches Recht | 8 | 88 | 7/7 | 0 | 1 |
| marketing-005 | corpus | Eventmanagement, deutsches Recht | 7 | 81 | 7/7 | 0 | 1 |
| marketing-006 | corpus | PR-Retainer, schweizerisches Recht | 8 | 96 | 7/7 | 0 | 1 |
| marketing-007 | corpus | Contentlizenz, österreichisches Recht | 8 | 96 | 7/7 | 0 | 1 |
| marketing-008 | corpus | Referenznutzung, deutsches Recht | 8 | 88 | 7/7 | 0 | 1 |
| marketing-009 | corpus | Programmatischer Mediaeinkauf, EU-Bezug | 8 | 100 | 7/7 | 0 | 1 |
| marketing-010 | corpus | Influencer-Talentmanagement, deutsches Recht | 8 | 100 | 7/7 | 0 | 1 |
| marketing-011 | corpus | Sportsponsoring, schweizerisches Recht | 7 | 73 | 7/7 | 0 | 1 |
| marketing-012 | corpus | Werbemittel und Druck, deutsches Recht | 7 | 100 | 7/7 | 0 | 1 |
| marketing-013 | corpus | Veranstaltungsstätte und Messe, deutsches Recht | 8 | 100 | 7/7 | 0 | 1 |
| marketing-014 | corpus | Content-Produktion Werkvertrag, deutsches Recht | 7 | 89 | 7/7 | 0 | 1 |
| marketing-015 | corpus | Social Media Management, deutsches Recht | 7 | 65 | 7/7 | 0 | 1 |
| marketing-016 | corpus | Mediaagentur und Provision, österreichisches Recht | 7 | 89 | 7/7 | 0 | 1 |
| marketing-017 | corpus | PR-Krisenkommunikation, deutsches Recht | 7 | 81 | 8/8 | 0 | 1 |
| marketing-018 | corpus | Influencer Whitelisting, deutsches Recht | 8 | 72 | 7/7 | 0 | 1 |
| marketing-019 | corpus | Naming Rights Sponsoring, österreichisches Recht | 7 | 97 | 7/7 | 0 | 1 |
| marketing-020 | corpus | Stock-Content-Lizenz, fremdes Recht | 8 | 100 | 7/7 | 0 | 1 |
| marketing-021 | corpus | Influencer-Kooperation, deutsches Recht | 9 | 100 | 7/7 | 0 | 1 |
| marketing-022 | corpus | Eventaktivierung Sponsoring, schweizerisches Recht | 8 | 100 | 7/7 | 0 | 1 |
| marketing-023 | corpus | Mediaeinkauf mit Agenturprovision, deutsches Recht | 8 | 100 | 7/7 | 0 | 1 |
| marketing-024 | corpus | PR-Agentur, österreichisches Recht | 7 | 89 | 7/7 | 0 | 1 |
| marketing-025 | corpus | Sponsoring Leistungssport, deutsches Recht | 8 | 100 | 7/7 | 0 | 1 |
| marketing-026 | corpus | Werbeplakatierung, deutsches Recht | 7 | 89 | 7/7 | 0 | 1 |
| marketing-027 | corpus | Messeservice, schweizerisches Recht | 7 | 73 | 7/7 | 0 | 1 |
| marketing-028 | corpus | Referenznutzung und Case Studies, deutsches Recht | 7 | 65 | 7/7 | 0 | 1 |
| marketing-029 | corpus | Trikotsponsoring, deutsches Recht | 8 | 100 | 7/7 | 0 | 1 |
| marketing-030 | corpus | Social Listening und Medienanalyse, deutsches Recht | 7 | 81 | 7/7 | 0 | 1 |
| marketing-031 | corpus | Digital Out of Home, österreichisches Recht | 6 | 82 | 6/6 | 0 | 1 |
| marketing-032 | corpus | Contentlizenz Musik und Bild, schweizerisches Recht | 7 | 89 | 7/7 | 0 | 1 |
| personal-001 | corpus | Freelance und Beratung | 8 | 100 | 7/7 | 0 | 1 |
| personal-002 | corpus | Arbeitsvertrag | 10 | 86 | 7/7 | 0 | 1 |
| personal-003 | corpus | Zeitarbeit und ANUE | 9 | 100 | 8/8 | 0 | 1 |
| personal-004 | corpus | Interim-Management | 9 | 100 | 7/7 | 0 | 1 |
| personal-005 | corpus | Zeitarbeit und ANUE | 8 | 100 | 7/7 | 0 | 1 |
| personal-006 | corpus | Freelance und Beratung | 7 | 73 | 7/7 | 0 | 1 |
| personal-007 | corpus | Personalberatung | 8 | 80 | 7/7 | 0 | 1 |
| personal-008 | corpus | Arbeitsvertrag | 8 | 88 | 7/7 | 0 | 1 |
| personal-009 | corpus | Zeitarbeit und ANUE | 7 | 81 | 7/7 | 0 | 1 |
| personal-010 | corpus | Interim-Management | 8 | 88 | 7/7 | 0 | 1 |
| personal-011 | corpus | Freelance und Beratung | 9 | 95 | 8/8 | 0 | 1 |
| personal-012 | corpus | Personalberatung | 7 | 81 | 7/7 | 0 | 1 |
| personal-013 | corpus | Arbeitsvertrag | 11 | 100 | 7/7 | 0 | 1 |
| personal-014 | corpus | Zeitarbeit und ANUE | 8 | 96 | 7/7 | 0 | 1 |
| personal-015 | corpus | Interim-Management | 8 | 96 | 7/7 | 0 | 1 |
| personal-016 | corpus | Freelance und Beratung | 8 | 80 | 8/8 | 0 | 1 |
| personal-017 | corpus | Personalberatung | 11 | 100 | 8/8 | 0 | 1 |
| personal-018 | corpus | Geschaeftsfuehrer | 11 | 100 | 7/7 | 0 | 1 |
| personal-019 | corpus | Arbeitsvertrag | 11 | 100 | 8/8 | 0 | 1 |
| personal-020 | corpus | Zeitarbeit und ANUE | 7 | 89 | 7/7 | 0 | 1 |
| personal-021 | corpus | Interim-Management | 7 | 81 | 7/7 | 0 | 1 |
| personal-022 | corpus | Freelance und Beratung | 7 | 81 | 7/7 | 0 | 1 |
| personal-023 | corpus | Personalberatung | 9 | 100 | 7/7 | 0 | 1 |
| personal-024 | corpus | Arbeitsvertrag | 11 | 100 | 7/7 | 0 | 1 |
| personal-025 | corpus | Zeitarbeit und ANUE | 8 | 100 | 7/7 | 0 | 1 |
| personal-026 | corpus | Interim-Management | 7 | 81 | 7/7 | 0 | 1 |
| personal-027 | corpus | Freelance und Beratung | 8 | 80 | 8/8 | 0 | 1 |
| personal-028 | corpus | Personalberatung | 10 | 100 | 8/8 | 0 | 1 |
| personal-029 | corpus | Geschaeftsfuehrer | 9 | 95 | 8/8 | 0 | 1 |
| personal-030 | corpus | Arbeitsvertrag | 12 | 100 | 8/8 | 0 | 1 |
| personal-031 | corpus | Freelance und Beratung | 11 | 100 | 8/8 | 0 | 1 |
| personal-032 | corpus | Zeitarbeit und ANUE | 8 | 100 | 8/8 | 0 | 1 |
| saas-001 | corpus | Cloud-Infrastruktur (IaaS), deutsches Recht | 8 | 100 | 7/7 | 0 | 0 |
| saas-002 | corpus | Managed Hosting, oesterreichisches Recht | 10 | 100 | 7/7 | 0 | 0 |
| saas-003 | corpus | Software-Abonnement CRM, Schweizer Recht | 10 | 100 | 7/7 | 0 | 0 |
| saas-004 | corpus | API-Plattform Partnervertrag, deutsches Recht | 8 | 100 | 8/8 | 0 | 0 |
| saas-005 | corpus | Cloud-Monitoring, deutsches Recht | 7 | 100 | 7/7 | 0 | 0 |
| saas-006 | corpus | HR-Suite Software-Abonnement, oesterreichisches Recht | 9 | 100 | 7/7 | 0 | 0 |
| saas-007 | corpus | Managed Security Service, Schweizer Recht | 9 | 100 | 8/8 | 0 | 0 |
| saas-008 | corpus | Payment-API Plattform, deutsches Recht | 8 | 96 | 7/7 | 0 | 0 |
| saas-009 | corpus | Colocation und Hosting, oesterreichisches Recht | 6 | 90 | 6/6 | 0 | 0 |
| saas-010 | corpus | SaaS Marketing-Automation, deutsches Recht | 7 | 65 | 7/7 | 0 | 0 |
| saas-011 | corpus | Managed DevOps, Schweizer Recht | 7 | 97 | 6/6 | 0 | 0 |
| saas-012 | corpus | ERP-Abonnement, deutsches Recht | 8 | 96 | 7/7 | 0 | 0 |
| saas-013 | corpus | Cloud Backup, oesterreichisches Recht | 7 | 100 | 7/7 | 0 | 0 |
| saas-014 | corpus | SaaS CRM-Addon, Schweizer Recht | 8 | 100 | 7/7 | 0 | 0 |
| saas-015 | corpus | API-Plattform, deutsches Recht | 8 | 96 | 7/7 | 0 | 0 |
| saas-016 | corpus | Managed WAF, oesterreichisches Recht | 7 | 97 | 7/7 | 0 | 0 |
| saas-017 | corpus | Cloud Identity, Schweizer Recht | 8 | 88 | 7/7 | 0 | 0 |
| saas-018 | corpus | SaaS Recruiting, deutsches Recht | 7 | 73 | 7/7 | 0 | 0 |
| saas-019 | corpus | Dedicated Hosting, oesterreichisches Recht | 7 | 100 | 7/7 | 0 | 0 |
| saas-020 | corpus | SaaS Analytics, Schweizer Recht | 7 | 89 | 7/7 | 0 | 0 |
| saas-021 | corpus | Managed Kubernetes, deutsches Recht | 7 | 97 | 7/7 | 0 | 0 |
| saas-022 | corpus | API-Marktplatz, oesterreichisches Recht | 8 | 96 | 7/7 | 0 | 0 |
| saas-023 | corpus | Cloud Object Storage, Schweizer Recht | 7 | 100 | 7/7 | 0 | 0 |
| saas-024 | corpus | SaaS Field Service, deutsches Recht | 7 | 100 | 7/7 | 0 | 0 |
| saas-025 | corpus | Managed NOC, oesterreichisches Recht | 8 | 100 | 7/7 | 0 | 0 |
| saas-026 | corpus | SaaS E-Learning, Schweizer Recht | 8 | 80 | 7/7 | 0 | 0 |
| saas-027 | corpus | Cloud CDN, deutsches Recht | 7 | 97 | 7/7 | 0 | 0 |
| saas-028 | corpus | API-Partnerprogramm, oesterreichisches Recht | 8 | 100 | 7/7 | 0 | 0 |
| saas-029 | corpus | SaaS Ticketing, Schweizer Recht | 7 | 73 | 7/7 | 0 | 0 |
| saas-030 | corpus | Managed DMS, deutsches Recht | 7 | 100 | 7/7 | 0 | 0 |
| saas-031 | corpus | Cloud Edge, oesterreichisches Recht | 7 | 97 | 7/7 | 0 | 0 |
| saas-032 | corpus | SaaS-Plattform, Schweizer Recht | 8 | 100 | 7/7 | 0 | 0 |
| service-001 | corpus | Versicherungsvermittlung | 9 | 100 | 9/9 | 0 | 2 |
| service-002 | corpus | Versicherungsvermittlung | 10 | 100 | 9/9 | 0 | 2 |
| service-003 | corpus | Versicherungsvermittlung | 9 | 100 | 9/9 | 0 | 2 |
| service-004 | corpus | Versicherungsvermittlung | 11 | 100 | 9/9 | 0 | 2 |
| service-005 | corpus | Maklervertrag | 9 | 100 | 9/9 | 0 | 2 |
| service-006 | corpus | Maklervertrag | 13 | 100 | 9/9 | 0 | 2 |
| service-007 | corpus | Maklervertrag | 9 | 95 | 9/9 | 0 | 2 |
| service-008 | corpus | Maklervertrag | 9 | 100 | 9/9 | 0 | 2 |
| service-009 | corpus | Reise- und Veranstaltungsvertrag | 10 | 100 | 9/9 | 0 | 2 |
| service-010 | corpus | Reise- und Veranstaltungsvertrag | 10 | 100 | 9/9 | 0 | 2 |
| service-011 | corpus | Reise- und Veranstaltungsvertrag | 10 | 100 | 9/9 | 0 | 2 |
| service-012 | corpus | Reise- und Veranstaltungsvertrag | 10 | 100 | 9/9 | 0 | 2 |
| service-013 | corpus | Garantie- und Serviceverlängerung | 12 | 100 | 9/9 | 0 | 2 |
| service-014 | corpus | Garantie- und Serviceverlängerung | 9 | 100 | 9/9 | 0 | 2 |
| service-015 | corpus | Garantie- und Serviceverlängerung | 11 | 100 | 9/9 | 0 | 2 |
| service-016 | corpus | Garantie- und Serviceverlängerung | 10 | 100 | 9/9 | 0 | 2 |
| service-017 | corpus | SLA und Support | 11 | 100 | 9/9 | 0 | 2 |
| service-018 | corpus | SLA und Support | 11 | 100 | 9/9 | 0 | 2 |
| service-019 | corpus | SLA und Support | 10 | 100 | 9/9 | 0 | 2 |
| service-020 | corpus | SLA und Support | 11 | 100 | 9/9 | 0 | 2 |
| service-021 | corpus | SLA und Support | 10 | 100 | 9/9 | 0 | 2 |
| service-022 | corpus | SLA und Support | 9 | 79 | 9/9 | 0 | 2 |
| service-023 | corpus | Callcenter | 9 | 100 | 9/9 | 0 | 2 |
| service-024 | corpus | Callcenter | 10 | 100 | 9/9 | 0 | 2 |
| service-025 | corpus | Callcenter | 9 | 100 | 9/9 | 0 | 2 |
| service-026 | corpus | Callcenter | 11 | 100 | 9/9 | 0 | 2 |
| service-027 | corpus | Callcenter | 10 | 100 | 9/9 | 0 | 2 |
| service-028 | corpus | Wartungsabo | 11 | 100 | 9/9 | 0 | 2 |
| service-029 | corpus | Wartungsabo | 10 | 100 | 9/9 | 0 | 2 |
| service-030 | corpus | Wartungsabo | 9 | 100 | 9/9 | 0 | 2 |
| service-031 | corpus | Wartungsabo | 10 | 100 | 9/9 | 0 | 2 |
| service-032 | corpus | Wartungsabo | 9 | 100 | 9/9 | 0 | 2 |
| vertrieb-001 | corpus | handelsvertreter | 8 | 100 | 8/8 | 0 | 0 |
| vertrieb-002 | corpus | franchise | 9 | 100 | 8/8 | 0 | 0 |
| vertrieb-003 | corpus | haendler | 9 | 100 | 8/8 | 0 | 0 |
| vertrieb-004 | corpus | reseller | 9 | 100 | 8/8 | 0 | 0 |
| vertrieb-005 | corpus | distributor | 8 | 72 | 8/8 | 0 | 0 |
| vertrieb-006 | corpus | affiliate | 9 | 95 | 8/8 | 0 | 0 |
| vertrieb-007 | corpus | kommission | 8 | 100 | 8/8 | 0 | 0 |
| vertrieb-008 | corpus | vertriebsvereinbarung | 11 | 100 | 8/8 | 0 | 0 |
| vertrieb-009 | corpus | handelsvertreter | 9 | 100 | 8/8 | 0 | 0 |
| vertrieb-010 | corpus | franchise | 10 | 100 | 8/8 | 0 | 0 |
| vertrieb-011 | corpus | haendler | 10 | 100 | 8/8 | 0 | 0 |
| vertrieb-012 | corpus | reseller | 9 | 95 | 8/8 | 0 | 0 |
| vertrieb-013 | corpus | distributor | 9 | 100 | 8/8 | 0 | 0 |
| vertrieb-014 | corpus | affiliate | 8 | 100 | 8/8 | 0 | 0 |
| vertrieb-015 | corpus | kommission | 10 | 100 | 8/8 | 0 | 0 |
| vertrieb-016 | corpus | vertriebsvereinbarung | 8 | 96 | 8/8 | 0 | 0 |
| vertrieb-017 | corpus | handelsvertreter | 8 | 88 | 8/8 | 0 | 0 |
| vertrieb-018 | corpus | franchise | 9 | 95 | 8/8 | 0 | 0 |
| vertrieb-019 | corpus | haendler | 9 | 100 | 8/8 | 0 | 0 |
| vertrieb-020 | corpus | reseller | 10 | 100 | 8/8 | 0 | 0 |
| vertrieb-021 | corpus | distributor | 9 | 100 | 8/8 | 0 | 0 |
| vertrieb-022 | corpus | affiliate | 10 | 100 | 8/8 | 0 | 0 |
| vertrieb-023 | corpus | kommission | 10 | 100 | 8/8 | 0 | 0 |
| vertrieb-024 | corpus | vertriebsvereinbarung | 9 | 100 | 8/8 | 0 | 0 |
| vertrieb-025 | corpus | handelsvertreter | 9 | 100 | 8/8 | 0 | 0 |
| vertrieb-026 | corpus | franchise | 9 | 100 | 8/8 | 0 | 0 |
| vertrieb-027 | corpus | haendler | 9 | 100 | 8/8 | 0 | 0 |
| vertrieb-028 | corpus | reseller | 8 | 88 | 8/8 | 0 | 0 |
| vertrieb-029 | corpus | distributor | 8 | 80 | 8/8 | 0 | 0 |
| vertrieb-030 | corpus | affiliate | 9 | 100 | 8/8 | 0 | 0 |
| vertrieb-031 | corpus | kommission | 10 | 100 | 8/8 | 0 | 0 |
| vertrieb-032 | corpus | vertriebsvereinbarung | 10 | 100 | 8/8 | 0 | 0 |
| werk-001 | corpus | Werkvertrag Softwareentwicklung, deutsches Recht | 8 | 96 | 7/7 | 0 | 1 |
| werk-002 | corpus | Wartungs- und Servicevertrag Systemhaus, deutsches Recht | 12 | 100 | 8/8 | 0 | 1 |
| werk-003 | corpus | Migrationsprojekt IT, schweizerisches Recht | 12 | 100 | 9/9 | 0 | 1 |
| werk-004 | corpus | Bau-IT, Errichtung und Betrieb, deutsches Recht | 9 | 100 | 8/8 | 0 | 1 |
| werk-005 | corpus | Softwareerstellung Versicherungswirtschaft, deutsches Recht | 9 | 100 | 8/8 | 0 | 1 |
| werk-006 | corpus | Migrationsprojekt und Betriebsuebernahme Logistik, deutsches Recht | 12 | 100 | 10/10 | 0 | 1 |
| werk-007 | corpus | Sondermaschinenbau mit Steuerungssoftware, deutsches Recht | 8 | 100 | 8/8 | 0 | 1 |
| werk-008 | corpus | IT-Rahmenvertrag kommunaler Versorger, deutsches Recht | 10 | 100 | 8/8 | 0 | 1 |
| werk-009 | corpus | Wartung und Service, deutsches Recht | 10 | 100 | 9/9 | 0 | 1 |
| werk-010 | corpus | Energiewirtschaft, deutsches Recht | 10 | 100 | 9/9 | 0 | 1 |
| werk-011 | corpus | Prozessindustrie, deutsches Recht | 10 | 100 | 9/9 | 0 | 1 |
| werk-012 | corpus | Legal Tech, deutsches Recht | 9 | 100 | 9/9 | 0 | 1 |
| werk-013 | corpus | Versicherungswirtschaft, deutsches Recht | 11 | 100 | 9/9 | 0 | 1 |
| werk-014 | corpus | Öffentliche Verwaltung, deutsches Recht | 10 | 100 | 9/9 | 0 | 1 |
| werk-015 | corpus | Industrie und Anlagenbau, deutsches Recht | 11 | 100 | 9/9 | 0 | 1 |
| werk-016 | corpus | Einzelhandel, deutsches Recht | 9 | 100 | 9/9 | 0 | 1 |
| werk-017 | corpus | Medienproduktion, deutsches Recht | 10 | 100 | 9/9 | 0 | 1 |
| werk-018 | corpus | Telekommunikation, deutsches Recht | 9 | 100 | 9/9 | 0 | 1 |
| werk-019 | corpus | Wehrtechnik, deutsches Recht | 9 | 100 | 9/9 | 0 | 1 |
| werk-020 | corpus | Automobilzulieferer, deutsches Recht | 11 | 100 | 9/9 | 0 | 1 |
| werk-021 | corpus | Sportstätten, deutsches Recht | 9 | 95 | 9/9 | 0 | 1 |
| werk-022 | corpus | Tourismus, deutsches Recht | 9 | 100 | 9/9 | 0 | 1 |
| werk-023 | corpus | Bahninfrastruktur, deutsches Recht | 9 | 100 | 9/9 | 0 | 1 |
| werk-024 | corpus | Prozessindustrie, deutsches Recht | 9 | 95 | 9/9 | 0 | 1 |
| werk-025 | corpus | Lebensmittelhandel, deutsches Recht | 10 | 100 | 9/9 | 0 | 1 |
| werk-026 | corpus | Bildungsträger, deutsches Recht | 9 | 100 | 9/9 | 0 | 1 |
| werk-027 | corpus | Legal Tech, deutsches Recht | 11 | 100 | 9/9 | 0 | 1 |
| werk-028 | corpus | Agrarwirtschaft, deutsches Recht | 9 | 100 | 9/9 | 0 | 1 |
| werk-029 | corpus | Immobilienwirtschaft, deutsches Recht | 11 | 100 | 9/9 | 0 | 1 |
| werk-030 | corpus | Lebensmittelhandel, deutsches Recht | 9 | 100 | 9/9 | 0 | 1 |
| werk-031 | corpus | Wehrtechnik, deutsches Recht | 9 | 100 | 9/9 | 0 | 1 |
| werk-032 | corpus | Sportstätten, deutsches Recht | 10 | 100 | 9/9 | 0 | 1 |
| public-bonterms-business-associate-agreement | public | Auftragsverarbeitung Healthcare (Bonterms) | 0 | 0 | 0/0 | 0 | 0 |
| public-bonterms-cloud-terms | public | Cloud-Services-Vertrag (Bonterms Standard) | 5 | 59 | 5/5 | 0 | 0 |
| public-bonterms-data-protection-addendum | public | Datenverarbeitungsvertrag (Bonterms) | 0 | 0 | 0/0 | 0 | 0 |
| public-bonterms-end-user-agreement | public | Endnutzervereinbarung (Bonterms) | 6 | 66 | 5/5 | 0 | 0 |
| public-bonterms-marketplace-transactions | public | Marktplatzvereinbarung (Bonterms) | 6 | 66 | 5/5 | 0 | 0 |
| public-bonterms-mutual-nda | public | Geheimhaltungsvereinbarung (Bonterms) | 2 | 14 | 2/2 | 0 | 0 |
| public-bonterms-online-cloud-terms | public | Online-Cloud-Bedingungen (Bonterms) | 6 | 66 | 5/5 | 0 | 0 |
| public-bonterms-professional-services-agreement | public | Dienstleistungsvertrag (Bonterms) | 7 | 89 | 7/7 | 0 | 0 |
| public-bonterms-reseller-agreement | public | Reseller-Vereinbarung (Bonterms) | 6 | 66 | 5/5 | 0 | 0 |
| public-bonterms-service-level-agreement | public | Service Level Agreement (Bonterms) | 1 | 15 | 1/1 | 0 | 0 |
| public-commonpaper-ai-addendum | public | KI-Addendum (Common Paper) | 0 | 0 | 0/0 | 0 | 0 |
| public-commonpaper-business-associate-agreement | public | Auftragsverarbeitung Healthcare (Common Paper) | 1 | 15 | 1/1 | 0 | 0 |
| public-commonpaper-cloud-service-agreement | public | SaaS Vertrag (Standardvertrag, Common Paper) | 5 | 59 | 5/5 | 0 | 0 |
| public-commonpaper-data-processing-agreement | public | Datenverarbeitungsvertrag (Common Paper) | 1 | 7 | 1/1 | 0 | 0 |
| public-commonpaper-design-partner-agreement | public | Design-Partner-Vereinbarung (Common Paper) | 2 | 14 | 2/2 | 0 | 0 |
| public-commonpaper-mutual-nda | public | Geheimhaltungsvereinbarung (Common Paper) | 0 | 0 | 0/0 | 0 | 0 |
| public-commonpaper-partnership-agreement | public | Partnervertrag (Common Paper) | 4 | 44 | 4/4 | 0 | 0 |
| public-commonpaper-pilot-agreement | public | Pilotvereinbarung (Common Paper) | 2 | 14 | 2/2 | 0 | 0 |
| public-commonpaper-professional-services-agreement | public | Dienstleistungsvertrag (Common Paper) | 4 | 44 | 4/4 | 0 | 0 |
| public-commonpaper-service-level-agreement | public | Service Level Agreement (Common Paper) | 1 | 15 | 1/1 | 0 | 0 |
| public-commonpaper-software-license-agreement | public | Softwarelizenzvertrag (Common Paper) | 4 | 44 | 4/4 | 0 | 0 |
| public-eupl-1-2-de | public | Open-Source-Lizenzvertrag (EUPL 1.2, deutsch) | 1 | 15 | 1/1 | 0 | 0 |
| public-evb-it-systemvertrag | public | IT-Systemvertrag (öffentliche Verwaltung) | 8 | 100 | 7/7 | 0 | 0 |
| public-evb-it-ueberlassung-typ-a-langfassung | public | Standardsoftware-Überlassungsvertrag (öffentliche Verwaltung) | 3 | 45 | 2/2 | 0 | 0 |
| public-far-52-212-4 | public | US-Bundesbeschaffung: Commercial Products and Commercial Services | 1 | 15 | 1/1 | 0 | 0 |
| public-gdz-basis-dlm-nutzungsbedingungen | public | Geodaten-Nutzungsbedingungen (Bund, Basis-DLM) | 0 | 0 | 0/0 | 0 | 0 |
