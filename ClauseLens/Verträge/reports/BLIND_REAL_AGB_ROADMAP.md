# Blind Real-AGB Benchmark Report & Engineering Roadmap

## 1. Executive Verdict: Corpus vs. Blind Real-AGB

Gemäß der Vorgabe (*"Stop tuning patterns against your own corpus; freeze the engine and run the blind real-AGB test"*) wurde die Engine eingefroren und gegen **25 vollständig blind verfasste und vorab gelabelte deutsche Praxis-AGB** (`Verträge/blind_real_agb/`) getestet.

| Benchmark-Suite | Verträge | Fallen (Traps) | Treffer (Recall) | Kontrollklauseln (Benign) | Fehlalarme (FP) | Status |
|---|---|---|---|---|---|---|
| **Synthetischer Red-Team-Korpus** (`Verträge/corpus`) | 384 | 2.988 | 2.988 (**100,0 %**) | 2.659 | 0 (**0,0 %**) | In-Sample (trainiert) |
| **Öffentliche Standardverträge** (`Verträge/public`) | 26 | 70 | 70 (**100,0 %**) | 152 | 0 (**0,0 %**) | In-Sample (bekannt) |
| **Out-of-Sample Holdout** (`test/holdout`) | 10 | 109 | 109 (**100,0 %**) | 63 | 0 (**0,0 %**) | Holdout (gleiche Domänen) |
| **Blind Real-AGB Benchmark** (`Verträge/blind_real_agb`) | **25** | **198** | **107 Klausel-Treffer (54,0 %)** / **90 Typ-exakt (45,5 %)** | **141** | **18 (12,8 %)** | **100 % Blind (Frozen Engine)** |

### Kernerkenntnis & Produkt-Positionierung
- Auf dem Trainings- und Holdout-Korpus erreicht die Engine **100 % Recall bei 0 % False Positives**.
- Im **echten Blindtest** auf 25 ungesehenen deutschen Branchen-AGB erkennt die eingefrorene Engine **54,0 % aller problematischen Klauseln** (davon **45,5 %** mit exakt demselben Detektortyp und **8,5 %** über einen benachbarten Risiko-Detektor) bei **12,8 % Fehlalarmen** auf Kontrollklauseln.
- **Konsequenz für das Produkt:** ClauseLens wird als **deterministischer Vorprüfungs-Assistent (Public Beta / Audit Aid gem. §§ 1, 2 RDG)** mit transparenter Angabe beider Messwerte ausgeliefert – **nicht als rechtliche Vollständigkeitsgarantie**. Jeder Befund trägt ab sofort eine explizite gesetzliche Normreferenz (`citation`). Die blinden Misses und False Positives bilden die nachfolgende technische Roadmap.

---

## 2. Warum die Recall-Lücke im Blindtest entsteht (5 technische Ursachen)

1. **Satzisolierte Haftungsprüfung erzeugt 11 der 18 False Positives (`liability`):**
   - In realen deutschen AGB besteht eine gesetzeskonforme Haftungsklausel fast immer aus 3–4 Sätzen im selben Absatz: *Satz 1: Unbeschränkte Haftung für Vorsatz/grobe Fahrlässigkeit und Leben/Körper/Gesundheit (§ 309 Nr. 7 BGB). Satz 2: Begrenzung auf den vertragstypischen vorhersehbaren Schaden bei leicht fahrlässiger Kardinalpflichtverletzung. Satz 3: „Im Übrigen ist die Haftung für leichte Fahrlässigkeit ausgeschlossen.“*
   - Weil `collectPerSentence` Satz 3 isoliert betrachtet, schlägt `liability` auf dem Schlusssatz an, obwohl Satz 1 und 2 im selben Absatz alle zwingenden Carve-outs enthalten.
   - **Roadmap-Fix P1:** Absatzweiter Kontext-Veto (`paragraphHasFullLiabilityCarveout`): Wenn derselbe Absatz Vorsatz, grobe Fahrlässigkeit, Körper-/Lebensschäden und Kardinalpflichten ausdrücklich ausnimmt, darf der abschließende Ausschluss einfacher Fahrlässigkeit nicht als Fehler gemeldet werden.

2. **Kaufmännischer Gerichtsstand (§ 38 ZPO) erzeugt 5 der 18 False Positives (`jurisdiction`):**
   - Sätze wie *„Ist der Kunde Kaufmann, juristische Person des öffentlichen Rechts oder öffentlich-rechtliches Sondervermögen, ist ausschließlicher Gerichtsstand…“* sind nach § 38 Abs. 1 ZPO wirksam, triggern aber aktuell den `jurisdiction`-Detektor.
   - **Roadmap-Fix P1:** `MATCH_VETO.jurisdiction` um den § 38 ZPO Kaufmanns-Vorbehalt (*„sofern/ist der Kunde Kaufmann… juristische Person des öffentlichen Rechts“*) erweitern.

3. **Branchenspezifische Synonyme bei `burden_of_proof`, `entire_agreement`, `subcontractor_uncontrolled` und `deemed_acceptance` (Top Silent Misses):**
   - Praxis-AGB nutzen branchenspezifische Formulierungen wie *„Protokollnotizen bei Objektbegehung sind gegenstandslos“* (`entire_agreement`), *„Nachunternehmerketten weitervergeben“* (`subcontractor_uncontrolled`), *„48 Stunden nach Anlieferung auf dem Werksgelände“* (`deemed_acceptance`) oder *„obliegt der Nachweis dem Besteller“* (`burden_of_proof`).
   - **Roadmap-Fix P2:** Strukturmuster (Subjekt + Modalverb + Rechtsfolge) statt eng gefasster Substantivlisten.

4. **Typ-Kollisionen zwischen überlappenden Detektoren (17 Fälle):**
   - Klauseln mit mehreren Rechtsmängeln in einem Satz (z. B. *„Kündigungsfrist 9 Monate, sonst Verlängerung um 24 Monate“* oder *„Abwerbeverbot für 5 Jahre bei 50.000 € Vertragsstrafe“*) lösen `notice_period` bzw. `penalty` aus, während das Label `auto_renewal` bzw. `non_solicitation` erwartete.
   - **Roadmap-Fix P2:** Multi-Label-Dedup pro Absatz zulassen, wenn zwei verschiedene Rechtsgüter im selben Absatz verletzt werden.

5. **26 Uncovered Klauseltypen außerhalb der 47 Detektoren:**
   - Wiederkehrende reale AGB-Fallen ohne eigenen Detektor: Vertragsübernahme durch unbenannte Dritte (§ 309 Nr. 10 BGB), Verbraucher-Widerrufshürden / Wiedereinlagerungsgebühren (§§ 312g, 357 BGB), Negative-Option-Zusatzpakete (§ 312a Abs. 3 BGB), formularmäßige Endrenovierung unabhängig vom Abnutzungsgrad (§ 307 BGB) und Aufrechnungs-/Auszahlungssperren für Energieguthaben (§ 40c EnWG).
   - **Roadmap-Fix P3:** Ausbau von 47 auf 52 Detektoren in v1.1.

---

## 3. Priorisierte Detektor-Roadmap nach Blindtest-Fehltreffern

| Detektor-Typ | Silent Misses | Typ-Kollisionen | Beispiel aus dem Blindtest |
|---|---|---|---|
| `deemed_acceptance` | 4 | 1 | `agb-08-systemhaus-it-projekt` (L17): „Die gesamte Werkleistung gilt auch ohne ausdrückliche Erklärung bereits nach Ablauf“ |
| `audit_log_manipulation` | 4 | 0 | `agb-01-telekommunikation-mobilfunk` (L27): „gelten dabei als unwiderlegbarer Beweis für die Richtigkeit der abgerechneten“ |
| `feedback_ip_grab` | 3 | 1 | `agb-02-cloud-saas-erp` (L21): „gehen sämtliche geistigen Eigentums- und Verwertungsrechte hieran automatisch,“ |
| `third_party_indemnity` | 4 | 0 | `agb-03-webhosting-dedicated` (L17): „unabhängig von einem eigenen Verschulden vollumfänglich von sämtlichen Ansprüchen“ |
| `force_majeure_asymmetry` | 3 | 1 | `agb-03-webhosting-dedicated` (L29): „der Kunde bleibt während des gesamten Zeitraums der Nichtleistung jedoch“ |
| `vicarious_agents` | 0 | 4 | `agb-03-webhosting-dedicated` (L49): „Für ein Verschulden seiner gesetzlichen Vertreter sowie seiner leitenden und“ |
| `entire_agreement` | 4 | 0 | `agb-04-ecommerce-versandhandel` (L11): „sämtliche vorvertraglichen Auskünfte unseres Kundenservice, Produktbeschreibungen im“ |
| `tax_indemnity_shift` | 4 | 0 | `agb-06-stadtwerke-sondervertrag` (L17): „sowie auch allgemeine Körperschaft- und Gewerbesteuern oder Bußgelder des“ |
| `mandatory_expert_determination` | 4 | 0 | `agb-06-stadtwerke-sondervertrag` (L23): „entscheidet hierüber ein vom Lieferanten bestimmter Sachverständiger als“ |
| `service_suspension` | 3 | 1 | `agb-06-stadtwerke-sondervertrag` (L33): „sofort und ohne vorherige Androhung oder vierwöchige Sperrankündigung durch den“ |
| `setoff_restriction` | 4 | 0 | `agb-07-spedition-logistik-lager` (L23): „Dem Auftraggeber ist jede Aufrechnung sowie die Geltendmachung eines“ |
| `data_grant` | 3 | 0 | `agb-02-cloud-saas-erp` (L19): „sämtliche vom Kunden in das ERP-System eingegebenen Geschäfts-, Finanz-, Kunden- und“ |
| `extended_retention_of_title` | 3 | 0 | `agb-04-ecommerce-versandhandel` (L35): „ein Anspruch des Kunden auf Freigabe von Sicherheiten besteht auch dann nicht, wenn“ |
| `statute_of_limitations` | 2 | 1 | `agb-07-spedition-logistik-lager` (L35): „ausdrücklich einschließlich solcher Ansprüche, die auf Vorsatz oder einem“ |
| `asymmetric_confidentiality` | 3 | 0 | `agb-08-systemhaus-it-projekt` (L33): „Der Auftragnehmer ist hinsichtlich sämtlicher vom Auftraggeber übergebenen“ |
| `dynamic_url_referral` | 3 | 0 | `agb-11-bauhandwerk-vob-falle` (L7): „der aktuelle Wortlaut sowie unsere technischen Zusatznormen sind jederzeit unter“ |
| `marketing_reference` | 2 | 1 | `agb-12-performance-marketing-agentur` (L23): „unwiderruflich berechtigt, den Firmennamen, sämtliche eingetragenen Marken,“ |
| `non_solicitation` | 0 | 3 | `agb-16-softwarepflege-support` (L57): „für einen Zeitraum von fünf Jahren nach dessen Beendigung keine Mitarbeiter“ |
| `auto_renewal` | 0 | 2 | `agb-01-telekommunikation-mobilfunk` (L35): „verlängert er sich jeweils automatisch um weitere 24 Monate zu den dann geltenden“ |
| `early_termination_charge` | 2 | 0 | `agb-01-telekommunikation-mobilfunk` (L39): „100 Prozent sämtlicher bis zum regulären Ende der Mindestvertragslaufzeit noch“ |
| `assignment_restriction` | 2 | 0 | `agb-03-webhosting-dedicated` (L25): „gegen den Provider gerichtete Ansprüche – einschließlich reiner Geldforderungen aus“ |
| `liability` | 2 | 0 | `agb-05-fitnessstudio-mitgliedschaft` (L39): „Das Studio schließt jegliche Haftung für Sach-, Vermögens- und Körperschäden des“ |
| `fictitious_receipt` | 2 | 0 | `agb-06-stadtwerke-sondervertrag` (L45): „gelten vierundzwanzig Stunden nach Bereitstellung auf dem Server als dem Kunden“ |
| `maintenance_conditional_shutdown` | 2 | 0 | `agb-08-systemhaus-it-projekt` (L23): „ist der Auftragnehmer berechtigt, auch die bereits vollständig bezahlte und auf den“ |
| `asymmetric_termination` | 2 | 0 | `agb-09-hausverwaltung-gewerbemiete` (L11): „Das Recht des Mieters zur außerordentlichen fristlosen Kündigung des“ |
| `insolvency_termination` | 2 | 0 | `agb-17-einkaufsbedingungen-industrie` (L61): „enden sämtliche bestehenden Rahmen- und Einzelverträge automatisch und entschädigungslos“ |
| `price_escalation` | 1 | 0 | `agb-05-fitnessstudio-mitgliedschaft` (L13): „jederzeit einseitig durch Aushang im Empfangsbereich zu erhöhen, um allgemeine“ |
| `unverbindliche_sla_falle` | 1 | 0 | `agb-12-performance-marketing-agentur` (L11): „handelt es sich hierbei ausschließlich um unverbindliche Orientierungswerte ohne“ |
| `notice_period` | 0 | 1 | `agb-12-performance-marketing-agentur` (L37): „Während der Auftraggeber an die vorstehende neunmonatige Kündigungsfrist zum Ende“ |
| `agency_commission_exclusion` | 1 | 0 | `agb-13-personalvermittlung-headhunter` (L13): „erhält der Vermittler für sämtliche Direktgeschäfte, Eigenrekrutierungen oder“ |
| `agency_goodwill_waiver` | 1 | 0 | `agb-13-personalvermittlung-headhunter` (L17): „ist jeglicher Ausgleichsanspruch des Vermittlers für den aufgebauten Kunden- und“ |
| `excessive_survival` | 1 | 0 | `agb-13-personalvermittlung-headhunter` (L37): „gelten nach Beendigung dieses Vertrages für einen Zeitraum von 30 Jahren“ |
| `jurisdiction` | 1 | 0 | `agb-15-veranstaltung-kongress-messe` (L63): „auch gegenüber Verbrauchern und Nichtkaufleuten – der Sitz der Muttergesellschaft des Veranstalters in Zürich“ |
| `service_credit_forfeiture` | 0 | 1 | `agb-16-softwarepflege-support` (L21): „verfällt jedoch ersatzlos, wenn der Kunde den Antrag nicht innerhalb einer Ausschlussfrist von 48 Stunden“ |
| `burden_of_proof` | 1 | 0 | `agb-17-einkaufsbedingungen-industrie` (L35): „dem Lieferanten obliegt die volle Beweislast dafür, dass ein Schaden im Werk des Auftraggebers“ |
| `non_compete` | 1 | 0 | `agb-19-unternehmensberatung-consulting` (L37): „für einen Zeitraum von drei Jahren nach Beendigung des Mandats untersagt, andere Strategie-“ |
| `unilateral_change` | 1 | 0 | `agb-22-coworking-flexible-office` (L9): „jederzeit nach freiem Ermessen ohne Ankündigung gegen einen beliebigen Steharbeitsplatz in der offenen Gemeinschaftslounge“ |
| `written_form_trap` | 1 | 0 | `agb-22-coworking-flexible-office` (L17): „zwingend in Papierform mit beglaubigter Unterschrift durch persönlichen Boten in der Hauptverwaltung“ |
| `unreasonable_audit` | 1 | 0 | `agb-22-coworking-flexible-office` (L35): „auch die fest abgeschlossenen privaten Einzelbüros des Mitglieds jederzeit ohne Vorankündigung, ohne klopfendes Anmelden und ohne Vorliegen eines Notfalls bei Tag und Nacht“ |
| `international_data_processing` | 1 | 0 | `agb-22-coworking-flexible-office` (L41): „an Drittunternehmen in beliebigen Staaten außerhalb der Europäischen Union und des Europäischen Wirtschaftsraums, in denen kein angemessenes Datenschutzniveau besteht“ |

---

## 4. Liste der 26 Uncovered Real-AGB-Klauseln (Kandidaten für Detektor #48–#52)

1. **§ 312a Abs. 3 BGB, § 307 Abs. 1 BGB** (`agb-01-telekommunikation-mobilfunk`, L13): „Mit Freischaltung der SIM-Karte wird automatisch das kostenpflichtige“ — *Unzulässige Negative-Option-Klausel (automatisch aktiviertes kostenpflichtiges Zusatzpaket ohne ausdrückliche Vereinbarung).*
2. **§ 309 Nr. 10 BGB** (`agb-01-telekommunikation-mobilfunk`, L53): „diesen Vertrag mit sämtlichen Rechten und Pflichten jederzeit ohne Zustimmung des“ — *Unzulässige Klausel zur Vertragsübernahme durch einen unbenannten Dritten ohne Kündigungsrecht des Kunden.*
3. **§ 307 Abs. 1 BGB** (`agb-02-cloud-saas-erp`, L57): „so gilt dies in sämtlichen künftigen Fällen als endgültiger Rechtsverzicht auch für“ — *Überraschende und unangemessene Verzichtsfiktion zulasten des Kunden, gekoppelt an das Nichtgeltendmachen von Rechten durch den Anbieter.*
4. **§ 307 Abs. 1, Abs. 2 Nr. 1 BGB** (`agb-03-webhosting-dedicated`, L45): „erhebt der Provider für die Freigabe des gesetzlich geschuldeten AuthInfo-Codes eine“ — *Unzulässiges Entgelt für die Erfüllung einer gesetzlichen Nebenpflicht bei Vertragsende (Herausgabe des Domain-AuthInfo-Codes für den Providerwechsel).*
5. **§§ 355, 357, 361 BGB** (`agb-04-ecommerce-versandhandel`, L29): „zieht der Verkäufer vom zu erstattenden Kaufpreis unabhängig vom Zustand der“ — *Rechtswidrige Einschränkung des gesetzlichen Verbraucherwiderrufsrechts durch pauschale Wiedereinlagerungsgebühr (Restocking Fee) und Originalverpackungszwang.*
6. **§ 307 Abs. 1 BGB** (`agb-05-fitnessstudio-mitgliedschaft`, L35): „strikt untersagt, selbst mitgebrachte Getränke wie Wasserflaschen zu konsumieren;“ — *Nach ständiger Rechtsprechung unangemessenes Mitnahmeverbot für eigene Getränke beim Sport sowie rechtswidriges verdachtsloses Taschendurchsuchungsrecht.*
7. **§ 40c Abs. 3 EnWG, § 307 Abs. 1 BGB** (`agb-06-stadtwerke-sondervertrag`, L27): „Ergibt die Jahresabrechnung ein Guthaben des Kunden, wird dieses erst nach Ablauf“ — *Verstoß gegen § 40c Abs. 3 EnWG, wonach Guthaben aus der Jahresabrechnung unverzüglich (binnen zwei Wochen) auszuzahlen oder mit dem nächsten Abschlag zu verrechnen sind.*
8. **§ 412 Abs. 3 HGB, § 307 Abs. 1 BGB, § 309 Nr. 5 BGB** (`agb-07-spedition-logistik-lager`, L47): „um mehr als zwölf Minuten überschritten, schuldet der Auftraggeber je angefangener“ — *Wucherähnlich überhöhtes Standgeld (3.800 Euro/Stunde) nach minimaler Wartezeit von 12 Minuten unter Ausschluss des Gegenbeweises.*
9. **§ 305c Abs. 1, § 307 Abs. 2 Nr. 2 BGB** (`agb-08-systemhaus-it-projekt`, L51): „Auch soweit im Einzelauftrag ausdrücklich die Erstellung und Übereignung einer“ — *Überraschende und den Vertragszweck gefährdende Klausel bei Individualsoftwareerstellung, die die geschuldete Quellcode-Herausgabe von einer 150-%-Ablösezahlung abhängig macht.*
10. **§ 307 Abs. 1, Abs. 2 Nr. 1 BGB** (`agb-09-hausverwaltung-gewerbemiete`, L27): „Unabhängig vom Grad der Abnutzung bei Mietbeginn und unabhängig vom tatsächlichen“ — *Nach ständiger BGH-Rechtsprechung unwirksame bedingungslose Endrenovierungsklausel mit Fachhandwerkerzwang unabhängig vom Zustand bei Mietbeginn und Abnutzungsgrad.*
11. **§ 307 Abs. 1, Abs. 2 Nr. 2 BGB** (`agb-10-kfz-gewerbeleasing`, L21): „strikt untersagt, das Leasingfahrzeug durch Ehegatten, Lebenspartner oder“ — *Unangemessenes Drittfahrerverbot gegenüber Ehegatten und Familienangehörigen selbst im Beisein des Leasingnehmers, verbunden mit einer überhöhten Strafgebühr.*
12. **§ 309 Nr. 8 lit. b cc BGB, § 635 Abs. 2 BGB** (`agb-11-bauhandwerk-vob-falle`, L33): „zudem hat der Auftraggeber vor jeder Mängelrüge einen Kostenvorschuss von 1.500,00“ — *Rechtswidrige Erschwerung der Mängelgewährleistung durch Forderung eines pauschalen Mängelprüfungsvorschusses von 1.500 Euro vor jeder Mängelrüge.*
13. **§ 667 BGB, § 675 BGB, § 307 Abs. 2 Nr. 1 BGB** (`agb-12-performance-marketing-agentur`, L41): „Sämtliche im Auftrag und auf Kosten des Auftraggebers eingerichteten“ — *Verstoß gegen die gesetzliche Herausgabepflicht des Geschäftsbesorgers (§§ 675, 667 BGB) hinsichtlich der auf Kosten des Kunden angelegten Werbekonten durch Erpressung einer 25.000-Euro-Ablösesumme.*
14. **§ 652 Abs. 1 BGB, § 305c Abs. 1 BGB, § 307 Abs. 2 Nr. 1 BGB** (`agb-13-personalvermittlung-headhunter`, L21): „schuldet der Auftraggeber der Personalberatung dennoch für jedes einzelne unverlangt“ — *Mit dem gesetzlichen Leitbild des Maklervertrags (§ 652 BGB: Erfolgsprinzip) unvereinbare und überraschende erfolgsunabhängige Gebühr von 4.800 Euro je zugesandtem Bewerberprofil.*
15. **§ 305c Abs. 1 BGB / § 307 Abs. 1 BGB** (`agb-14-zahlungsdienstleister-fintech`, L37): „aktiviert der Zahlungsdienstleister für jeden Händler automatisch das kostenpflichtige Zusatzpaket“ — *Negative-Option-Klausel (automatische kostenpflichtige Zusatzleistung bei ausbleibendem Widerspruch) umgeht das Vertragsabschlussprinzip und ist als überraschende Klausel unwirksam.*
16. **§ 309 Nr. 10 BGB / § 307 BGB** (`agb-15-veranstaltung-kongress-messe`, L55): „jederzeit ohne Zustimmung des Teilnehmers mit schuldbefreiender Wirkung auf ein beliebiges anderes Messe-“ — *Unzulässige Vertragsübernahmeklausel (Eintritt eines unbenannten Dritten in den gesamten Vertrag ohne Zustimmung und ohne Kündigungsrecht des Vertragspartners).*
17. **§ 91 ZPO / § 307 Abs. 2 Nr. 1 BGB** (`agb-16-softwarepflege-support`, L67): „Unabhängig vom Ausgang eines Rechtsstreits oder eines gerichtlichen Vergleichs verpflichtet sich der Kunde“ — *Einseitige Prozess- und Anwaltskostenabwälzung unabhängig vom Obsiegen oder Unterliegen verstößt gegen das gesetzliche Leitbild des § 91 ZPO und erschwert den Rechtszugang unzulässig.*
18. **§ 138 BGB / § 307 Abs. 1 BGB / § 119 InsO** (`agb-17-einkaufsbedingungen-industrie`, L61): „und alle vom Lieferanten gefertigten Werkzeuge gehen unentgeltlich in das Eigentum des Auftraggebers über“ — *Entschädigungslose Zwangsenteignung eigener Werkzeuge des Lieferanten im Insolvenzfall benachteiligt den Lieferanten und die Insolvenzmasse sittenwidrig.*
19. **§ 307 Abs. 2 Nr. 1 BGB / Art. 101 AEUV** (`agb-18-maschinenbau-lieferbedingungen`, L39): „sobald der Besteller Verschleißteile, Schmierstoffe oder Reinigungsmittel verwendet, die nicht direkt vom Lieferanten“ — *Kopplungsklausel (Erlöschen jeglicher Mängelgewährleistung bei Verwendung gleichwertiger Dritt-Betriebsmittel ohne Kausalität zum Mangel) benachteiligt den Besteller unangemessen.*
20. **§ 307 Abs. 1 und 2 Nr. 1 BGB** (`agb-19-unternehmensberatung-consulting`, L63): „ohne jeden Zusammenhang mit der Beratungsleistung –, schuldet der Mandant dem Beratungsunternehmen eine zusätzliche Erfolgsprovision“ — *Kausalitätsunabhängige Erfolgsprovision auf künftige Unternehmensgewinne der nächsten fünf Jahre ohne ursächlichen Beitrag der Beratungsleistung verstößt gegen das Äquivalenzprinzip.*
21. **§ 307 Abs. 1 BGB** (`agb-20-arbeitnehmerueberlassung-zeitarbeit`, L59): „Änderung seines Handelsregistereintrags oder die Anschrift einer neuen Betriebsstätte innerhalb von drei Werktagen“ — *Unverhältnismäßige Strafgebühr von 5.000 Euro für die bloße Versäumung einer rein administrativen Adress- oder Registermitteilung ohne Schadenseintritt.*
22. **Art. 101 AEUV / § 1 GWB / § 307 BGB** (`agb-21-online-marktplatz-haendler`, L29): „im eigenen Online-Shop, im stationären Ladengeschäft oder auf anderen Internetplattformen zu einem günstigeren Preis“ — *Weite Bestpreisklausel (Preisparitätsverpflichtung über sämtliche Vertriebskanäle inkl. eigenem Shop und eigenem Ladengeschäft sowie eigenmächtiger Preiseingriff) ist kartell- und AGB-rechtlich unwirksam.*
23. **§ 138 BGB / § 307 Abs. 1 BGB / § 903 BGB** (`agb-22-coworking-flexible-office`, L55): „so gehen diese Gegenstände sofort und entschädigungslos in das alleinige Eigentum des Betreibers über“ — *Sofortiger entschädigungsloser Eigentumsverlust an zurückgelassenen Laptops und Aktenordnern (insb. bereits am Freitagabend nach 20 Uhr) ist als Verfallklausel sittenwidrig und unwirksam.*
24. **§ 357 Abs. 1 BGB / § 361 Abs. 1 BGB / § 307 BGB** (`agb-23-weiterbildung-fernunterricht`, L29): „behält die Akademie gleichwohl pauschal 35 % der gesamten Lehrgangsgebühr, mindestens jedoch 490,00 Euro, als nicht erstattungsfähige Storno-, Einrichtungs- und Wiedereinlagerungsgebühr“ — *Verdeckte Storno- und Wiedereinlagerungsgebühr (35 % bzw. 490 Euro) bei Ausübung des gesetzlichen Verbraucherwiderrufsrechts verstößt zwingend gegen §§ 355, 357, 361 BGB.*
25. **§ 138 BGB / § 307 Abs. 1 BGB** (`agb-24-wach-und-sicherheitsdienst`, L53): „sämtliche künftigen Betriebsstätten, Lagerhallen und Tochtergesellschaften, die er oder seine Gesellschafter innerhalb der nächsten zehn Jahre“ — *Zehnjährige Alleinbezugs- und Andienungspflicht für sämtliche künftigen Standorte in ganz Europa zu einseitigen Standardlistenpreisen stellt eine sittenwidrige Knebelung dar.*
26. **§ 307 Abs. 1 und 2 BGB / § 647 BGB / § 858 BGB** (`agb-25-gebaeudereinigung-facility`, L53): „steht dem Auftragnehmer ein vertragliches Pfand- und Verwertungsrecht an sämtlichen in den gereinigten Büroräumen des Auftraggebers befindlichen Einrichtungsgegenständen“ — *Formularmäßiges Pfandrecht an nicht in den Besitz des Auftragnehmers eingebrachten Büroinventaren (inkl. Dritteigentum) mit Selbsthilferecht zum Abtransport ohne Titel ist unwirksam.*
