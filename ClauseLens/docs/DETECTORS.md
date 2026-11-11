# ClauseLens — Detector Catalog (47 Specialized Legal Detectors)

All 47 detectors shipped in ClauseLens (`src/analyze.js`, `RULES` array).
Severity values: `niedrig` (weight 2), `mittel` (7), `hoch` (15).

Every finding carries a plain-German `why` and a negotiation-ready `suggestion`.

---

## 1. `auto_renewal` — Automatische Vertragsverlängerung (§ 307 BGB)
Trigger: "verlängert sich …", "stillschweigende Verlängerung", "automatische Verlängerung", Fortsetzungssynonyme ("setzt sich … fort", "erneuert sich jeweils um …").

## 2. `notice_period` — Lange/ungleiche Kündigungsfristen (§ 307 BGB)
Custom Collector: Ermittelt Kündigungsfristen numerisch. Fristen > 3 Monate oder asymmetrische Fristen (Verhältnis ≥ 2.5) werden markiert. Als Kündigungsbezug genügt das Wort "Kündigung"; synonyme Umschreibungen ("Vorlaufzeit", "Periodenende", "Vertragsende", "Loslösung") werden ebenfalls erfasst, reine Laufzeitangaben ("Laufzeit von zwölf Monaten") dagegen bewusst nicht.

## 3. `liability` — Weitgehender Haftungsausschluss & Sole Remedy (§ 309 Nr. 7, § 307 BGB)
Trigger: Haftungsausschluss, Begrenzung auf letzte 2 Monate, unzumutbare manuelle Vollsicherung, Service Credits als einzige Abhilfe (*Sole Remedy*). Seit dem MDR-Audit zusätzlich: ausdrücklicher Verzicht auf Ersatzansprüche sowie lang gestreckte Kategorie-Ausschlüsse (Lösegeld, DSGVO-Bußgelder, Reputationsschäden). Seit dem B2B-Audit weiterhin: abschwächende Umschreibungen ("sieht von Ansprüchen ab", "wird nicht übernommen", "keine Zusage").

## 4. `penalty` — Vertragsstrafe & verdeckte Pauschalen (§ 309 Nr. 5/6 BGB)
Trigger: "Vertragsstrafe", "Pönale", pauschalierte Bereitstellungsvergütungen mit Ausschluss des Gegenbeweises ("überhaupt kein Aufwand"), Pauschalen unabhängig vom Schadensnachweis ("unabhängig davon, ob ein Schaden entstanden ist", "der Nachweis eines geringeren Schadens ist nicht zulässig").

## 5. `jurisdiction` — Gerichtsstand beim Anbieter oder im Ausland (§ 38 ZPO, § 305c BGB)
Trigger: Ausschließlicher Gerichtsstand am Sitz des Anbieters oder im Ausland (Delaware, Schweiz, UK etc.); erfasst werden auch Auftragnehmer, Lieferanten und Systemzentralen als Begünstigte.

## 6. `non_compete` — Wettbewerbsverbot ohne Karenz (§ 74 HGB, § 138 BGB)
Trigger: Nachvertragliche Tätigkeitsverbote ohne gesetzliche Karenzentschädigung, synonyme Umschreibungen ("betreibt … kein System", "enthält sich jeder Wettbewerbstätigkeit") sowie Bindungsfristen, die das Wort Kündigung vermeiden.

## 7. `data_grant` — Exzessive Rechte an Kundendaten & IP-Transfer (§ 31 UrhG, GeschGehG)
Trigger: Unbeschränkte Übertragung von Rechten, dauerhafte Verwertung von KI-Gewichtungen/Parametern, Kontokorrentvorbehalte.

## 8. `default_interest` — Überhöhte Verzugszinsen (§ 288 BGB)
Trigger: Zinssätze > 9 % über Basiszinssatz oder monatliche Zinsberechnungen.

## 9. `unilateral_change` — Einseitiges Änderungsrecht & Zustimmungsfiktion (§ 307, § 308 Nr. 4/5 BGB)
Trigger: Einseitige Funktionsentkernung ohne Minderung, Zustimmungsfiktion durch Weiternutzung/Schweigen (BGH XI ZR 26/20), Kostenfreigabe-Fiktionen, erzwungene Zustimmung zu Notfall-/Mehrstundenabrechnung ("gilt der Kunde als einverstanden").

## 10. `setoff_restriction` — Aufrechnungs- und Zurückbehaltungsverbot (§ 309 Nr. 2/3 BGB)
Trigger: Ausschluss der Aufrechnung mit unbestrittenen Forderungen, Einschränkung von Zurückbehaltungsrechten auf dieselbe Monatsrechnung, Lizenz-Kill-Switches.

## 11. `deemed_acceptance` — Fingierte Abnahme & Rügefristfalle (§ 308 Nr. 5, § 640 Abs. 2 S. 2 BGB)
Trigger: Ultrakurze Rügefristen, Abnahmefiktion bei Einbindung in Vorsysteme ohne gesetzliche Belehrung.

## 12. `statute_of_limitations` — Pauschale Verjährungsverkürzung (§ 309 Nr. 7 BGB)
Trigger: Verjährung < 12 Monate oder Verfall ohne Ausnahme für Körperschäden und grobes Verschulden sowie sämtliche Verfalls- und Ausschlussfristen mit Monats-, Wochen- oder Jahresfrist ("Ansprüche verfallen sechs Wochen nach Fälligkeit", "erlöschen neun Monate nach der Inbetriebnahme").

## 13. `written_form_trap` — Formfalle & Kündigungserschwernis (§ 309 Nr. 13, § 305b BGB)
Trigger: Doppelte Schriftformklauseln, Ausschluss der Textform (E-Mail), Pflicht zu eigenhändiger Unterschrift / Einschreiben mit Rückschein, notarielle Beurkundung oder öffentliche Beglaubigung als Formvoraussetzung.

## 14. `burden_of_proof` — Beweislastumkehr & Negativbeweis (§ 309 Nr. 12 BGB)
Trigger: Vorab-Bestätigung der Mangelfreiheit, Zugangsfiktionen, Pflicht des Kunden zum Negativbeweis fremder Systemursachen, ausdrücklich beim Kunden liegende Beweislast ("Der Kunde trägt die Beweislast dafür, dass ...").

## 15. `dynamic_url_referral` — Dynamischer Online-AGB-Verweis (BGH III ZR 59/24)
Trigger: Verweise auf "jeweils aktuelle AGB unter www..." ohne unveränderlichen Versionsstand.

## 16. `vicarious_agents` — Haftungsausschluss für Erfüllungsgehilfen (§ 278, § 307 BGB)
Trigger: Beschränkung auf bloßes Auswahlverschulden bei Subunternehmern und Gehilfen.

## 17. `unreasonable_audit` — Übergriffiges Audit- und Kontrollrecht (§ 307 BGB)
Trigger: Unangekündigte Audits, jederzeitiges Zutrittsrecht, pauschale Kostenabwälzung auf den Kunden.

## 18. `data_lockin` — Daten-Lock-in & Herausgabeentgelte (Art. 20 DSGVO, § 307 BGB)
Trigger: Kostenpflichtiger Datenexport, Verweigerung offener maschinenlesbarer Standardformate, Pauschalen für Sichtung/Herausgabe/Wiederherstellung des Datenbestands. Eine Pauschale ohne Datenbezug im Satz (Mahngebühr, Rücksendekosten) bleibt bewusst unbeanstandet.

## 19. `third_party_indemnity` — Verschuldensunabhängige Drittfreistellung (§ 307 BGB)
Trigger: Freistellung des Anbieters von Drittansprüchen ohne eigenes Verschulden des Kunden.

## 20. `asymmetric_termination` — Ausschluss der außerordentlichen Kündigung (§ 314 BGB)
Trigger: Ausschluss oder Verzicht auf das Recht zur fristlosen Kündigung aus wichtigem Grund, verneintes Sonder- bzw. außerordentliches Kündigungsrecht ("kein außerordentliches Sonderkündigungsrecht zu").

## 21. `assignment_restriction` — Abtretungsverbot von Geldforderungen (§ 354a HGB, § 307 BGB)
Trigger: Formularmäßiges Abtretungsverbot für Geldforderungen oder Entgeltansprüche; behindert Factoring und Finanzierung.

## 22. `force_majeure_asymmetry` — Einseitige Force-Majeure-Klausel (§ 307 BGB)
Trigger: Suspendierung nur der Pflichten des Anbieters, während die Zahlungspflicht des Kunden bei höherer Gewalt uneingeschränkt fortbesteht.

## 23. `non_solicitation` — Unangemessenes Abwerbe- & Einstellungsverbot (§ 307 BGB, § 75f HGB)
Trigger: Pauschales Verbot, Mitarbeiter des Anbieters einzustellen oder abzuwerben, oft kombiniert mit hohen Vertragsstrafen; synonyme Fassungen ("keine Fachkräfte abwerben, die für die andere Seite tätig waren") werden ebenfalls erkannt.

## 24. `unilateral_convenience_termination` — Einseitiges ordentliches Kündigungsrecht (§ 307 BGB)
Trigger: Ordentliche Kündigung steht formularmäßig nur dem Anbieter zu, während der Kunde fest gebunden bleibt.

## 25. `excessive_survival` — Exzessive fortwirkende Pflichten / Survival (§ 307 BGB)
Trigger: Pflichten und Bindungen, die nach Vertragsende zeitlich unbegrenzt oder auf unbestimmte Zeit fortbestehen.

## 26. `price_escalation` — Einseitige Preiserhöhungsautomatik (§ 307 BGB, PrKG)
Trigger: Automatische Preiserhöhungen ohne Kopplung an beidseitige Kostenänderungen und ohne Sonderkündigungsrecht, Notfall-/Mehrstundensätze ohne Budgetobergrenze.

## 27. `salvatorische_klausel_falle` — Geltungserhaltende Reduktion (§ 306 BGB)
Trigger: Automatische Ersetzung unwirksamer Bestimmungen nach billigem Ermessen des Verwenders statt Geltung des gesetzlichen BGB-Rechts.

## 28. `extended_retention_of_title` — Übersichernder Kontokorrent-Eigentumsvorbehalt (§ 449 BGB)
Trigger: Eigentumsvorbehalt erstreckt sich auf alle offenen Forderungen aus der gesamten Geschäftsbeziehung statt auf den konkreten Kaufpreis.

## 29. `subcontractor_uncontrolled` — Unkontrollierter Subunternehmereinsatz (Art. 28 DSGVO, § 307 BGB)
Trigger: Berechtigung zur Weiterdelegierung an Subunternehmer ohne vorherige Benachrichtigung und ohne Widerspruchsrecht.

## 30. `audit_log_manipulation` — Beweismonopol durch Anbieterprotokolle (§ 309 Nr. 12 BGB)
Trigger: System- und Serverlogs des Verwenders werden als unwiderlegbarer Vollbeweis für Fehlerfreiheit oder Abrechnung festgeschrieben.

## 31. `service_suspension` — Unverhältnismäßige Dienstsperre (§ 307, § 320 BGB)
Trigger: Sofortige Leistungsabschaltung ohne Mahnung und Fristsetzung bei fortlaufender Vergütungspflicht; auch die Unterbrechung der Leistungserbringung ohne vorherige Ankündigung.

## 32. `insolvency_termination` — Insolvenzabhängige Lösungsklausel (§ 119 InsO)
Trigger: Automatisches Vertragsende oder Kündigungsrecht bei bloßem Insolvenzantrag (nach BGH IX ZR 169/11 unwirksam).

## 33. `tax_indemnity_shift` — Einseitige Steuer- und Nachforderungsabwälzung (§ 307 BGB)
Trigger: Rückwirkende Abwälzung von Umsatz- oder Quellensteuernachforderungen ohne Verschulden des Kunden.

## 34. `asymmetric_confidentiality` — Einseitige Geheimhaltungspflicht (§ 307 BGB, GeschGehG)
Trigger: Nur der Kunde wird zur Geheimhaltung verpflichtet, während der Anbieter Kundendaten ungeschützt verwerten darf. Seit dem MDR-Audit zusätzlich: einseitige Durchbrechung der Vertraulichkeit (Threat-Intel-Sharing, Behördenherausgabe ohne vorherige Information des Kunden).

## 35. `as_is_software_disclaimer` — Vollständiger Gewährleistungsausschluss / As-Is (§ 309 Nr. 8 BGB)
Trigger: Überlassung von Standardleistungen und Software "wie gesehen" unter vollständigem Ausschluss der Sachmängelhaftung, Ausschluss von Funktions- und Erkennungsgarantien in Dienstleistungsverträgen ("garantiert daher weder ... noch", "begründet keinen Mangel der Dienstleistung").

## 36. `entire_agreement` — Vollständigkeitsklausel & Ausschluss mündlicher Zusagen (§ 305b BGB)
Trigger: "Mündliche Nebenabreden bestehen nicht", frühere Absprachen, Verhandlungen oder Leistungsbeschreibungen verlieren ihre Gültigkeit (Verstoß gegen Vorrang der Individualabrede).

## 37. `marketing_reference` — Ungefragte Referenznennung & Markennutzung (§ 307 BGB, MarkenG)
Trigger: Berechtigung des Anbieters, den Kunden ohne gesonderte Zustimmung als Referenzkunden zu benennen und das Logo zu Werbezwecken zu nutzen; erfasst werden auch Nennung und Verwendung von Zeichen ohne Zustimmungserfordernis.

## 38. `feedback_ip_grab` — Kostenlose Abtretung von Kundenideen & Feedback (§ 307 BGB, UrhG)
Trigger: Vorschläge, Anregungen und Feature-Requests des Kunden gehen automatisch und unentgeltlich in das ausschließliche Eigentum des Anbieters über.

## 39. `fictitious_receipt` — Fingierter Zugang von Erklärungen & Rechnungen (§ 308 Nr. 5 BGB)
Trigger: Zugangsfiktion, wonach Rechnungen oder Erklärungen wenige Tage nach Absendung als zugegangen gelten.

## 40. `unverbindliche_sla_falle` — Scheingarantie & unverbindliche Service-Levels (§ 307 BGB)
Trigger: Verfügbarkeitszusagen (z. B. 99,9 %) werden im Kleingedruckten als unverbindliche Zielgrößen ohne Minderungs- oder Kündigungsrechte entkernt.

## 41. `agency_commission_exclusion` — Ausschluss von Provisionen auf Direktgeschäfte (§ 87 Abs. 2 HGB)
Trigger: Ausschluss von Provisionen für Direktaufträge oder Direktverkäufe im zugewiesenen Gebiet/Kundenkreis. Ein expliziter Ausschluss nur für Geschäfte außerhalb des zugewiesenen Bereichs bleibt unbeanstandet. Die tatsächliche Anspruchsberechtigung ist einzelfallabhängig.

## 42. `agency_goodwill_waiver` — Ausschluss des Handelsvertreterausgleichs (§ 89b HGB)
Trigger: Ausschluss eines Ausgleichsanspruchs für den aufgebauten Kundenstamm oder die Kundschaft nach Vertragsende. Die Anspruchsvoraussetzungen hängen vom Einzelfall ab; ein vorab vereinbarter Ausschluss ist ein Prüfhinweis.

## 43. `international_data_processing` — Datenverarbeitung außerhalb EU/EWR (Art. 44–49 DSGVO)
Trigger: Personenbezogene, Kunden-, Mess- oder Nutzungsdaten werden außerhalb der EU/des EWR verarbeitet oder einem Drittland zugänglich gemacht. Ein Drittlandtransfer ist nicht automatisch rechtswidrig; Transfergrundlage, Empfänger und Garantien sind zu prüfen.

## 44. `mandatory_expert_determination` — Sachverständigenverfahren als Rechtswegvoraussetzung
Trigger: Ein Sachverständigenverfahren muss vor Klage/Rechtsweg abgeschlossen werden, besonders wenn der Kunde es vorfinanzieren muss. Die Klausel löst einen Review-Hinweis aus, kein Urteil über die Zulässigkeit eines gestuften Verfahrens.

## 45. `service_credit_forfeiture` — Verfall von Service-Gutschriften wegen Update-Frist
Trigger: Versäumte kritische Updates/Sicherheitspatches führen zum Verfall von Service-Gutschriften. Prüfen, welche Gutschriften betroffen sind und ob Frist, Verantwortlichkeit und Nachfrist angemessen geregelt sind.

## 46. `early_termination_charge` — Pauschale Zahlung bei vorzeitiger Beendigung
Trigger: Bei vorzeitiger Beendigung wird eine pauschale Abstandszahlung oder ein Beendigungsentgelt verlangt. Das ist ein Prüfhinweis auf Schadenshöhe, ersparte Aufwendungen und mögliche Doppelkompensation, keine automatische Wirksamkeitsbewertung.

## 47. `maintenance_conditional_shutdown` — Abschaltung bei fehlendem Wartungsvertrag
Trigger: Anlage, System oder Dienst kann abgeschaltet werden, weil kein Wartungsvertrag besteht. Zu prüfen sind Vorankündigung, Sicherheitsgrund, Verhältnismäßigkeit und Daten-/Übergangszugriff.

---

## Scoring-Logik

```
riskScore = min(100, Σ weight(severity))      hoch=15 · mittel=7 · niedrig=2
riskLevel = [70..100]→hoch · [35..69]→mittel · [0..34]→niedrig
```
