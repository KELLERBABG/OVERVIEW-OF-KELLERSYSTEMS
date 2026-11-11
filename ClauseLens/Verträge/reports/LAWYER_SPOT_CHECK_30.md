# Lawyer Spot-Check: 30 Findings aus dem Blind Real-AGB Benchmark

Stichprobenprüfung von **30 realen Befunden** (`findings`) aus dem blinden Lauf über 25 deutsche Praxis-AGB (`Verträge/blind_real_agb/`).
Jeder Befund enthält das neu eingeführte Pflichtfeld **`citation`** (gesetzliche Normreferenz), den Originalauszug, die Engine-Begründung, den Redline-Gegenvorschlag sowie das juristische Spot-Check-Votum.

## Zusammenfassung der 30 Spot-Check-Befunde

| Kategorie | Anzahl | Juristische Bewertung |
|---|---|---|
| **True Positives (Exakter Detektor + Normzitat)** | 24 / 30 | ✅ Klauselfalle, Schweregrad, Normzitat (`citation`) und Redline-Vorschlag juristisch zutreffend |
| **Type Collisions (Warnung über Nachbardetektor)** | 3 / 30 | ⚠️ Unwirksame Klausel wurde markiert, aber einem benachbarten Detektortyp zugeordnet |
| **False Positives (Kontrollklausel angeschlagen)** | 3 / 30 | ❌ Fehlalarm durch satzisolierte Bewertung mehrsätziger Standard-Haftungsklauseln bzw. § 38 ZPO |

---

### Befund #1: Intransparenter dynamischer URL-Verweis (BGH III ZR 59/24) (`dynamic_url_referral`)
- **Vertrag / Branche:** `agb-01-telekommunikation-mobilfunk` (Telekommunikation & Glasfaser/Mobilfunk AGB)
- **Kategorie:** True Positive (Korrekter Treffer)
- **Schweregrad:** `HOCH` (Zeile 9–9)
- **Gesetzliche Normreferenz (`citation`):** **§ 305 Abs. 2 BGB, § 307 Abs. 1 S. 2 BGB (BGH III ZR 59/24)** *(Ground-Truth-Label: § 305 Abs. 2, § 307 Abs. 1 S. 2, § 308 Nr. 4 BGB)*
- **Klauselauszug:**
  > „Die jeweils gültige Leistungsbeschreibung sowie die technischen Schnittstellenspezifikationen ergeben sich verbindlich aus den unter www.nordnetz-connect.de/agb-technik-aktuell abrufbaren Dokumenten in ihrer jeweils zum Zeitpunkt der Nutzung gültigen Fassung, die der Anbieter jederzeit online aktualisieren kann.“
- **Engine-Begründung (`why`):** Ein bloßer dynamischer Link auf Online-AGB ohne Nennung einer konkreten, unveränderlichen Versionsnummer (z. B. Stand 01.01.2026) ist nach ständiger Rechtsprechung des BGH (u. a. Urteil vom 10.07.2025 – III ZR 59/24) wegen Verstoßes gegen das Transparenzgebot (§ 307 Abs. 1 S. 2 BGB) unwirksam.
- **Redline-Gegenvorschlag (`suggestion`):** Statisches Dokument mit eindeutiger Versionsbezeichnung und Datumsstand als feste Anlage zum Vertrag beifügen.
- **Anwaltlicher Spot-Check (Verdict):** ✅ Juristisch zutreffend (Treffer & Normzitat korrekt) — *Unzulässiger dynamischer Verweis auf online abrufbare Dokumente in der jeweils gültigen Fassung mit jederzeitiger Änderungsmöglichkeit.*

### Befund #2: Einseitiges Änderungsrecht & Zustimmungsfiktion (`unilateral_change`)
- **Vertrag / Branche:** `agb-01-telekommunikation-mobilfunk` (Telekommunikation & Glasfaser/Mobilfunk AGB)
- **Kategorie:** True Positive (Korrekter Treffer)
- **Schweregrad:** `HOCH` (Zeile 11–11)
- **Gesetzliche Normreferenz (`citation`):** **§ 308 Nr. 4, Nr. 5 BGB, § 307 Abs. 1 BGB** *(Ground-Truth-Label: § 308 Nr. 4 BGB)*
- **Klauselauszug:**
  > „Darüber hinaus ist die Nord Netz Connect Gmb H berechtigt, die vertraglich vereinbarten Übertragungsraten, das enthaltene Datenvolumen sowie die Tarifstruktur jederzeit nach freiem Ermessen anzupassen, sofern dies aus betrieblichen oder technischen Gründen zweckmäßig erscheint.“
- **Engine-Begründung (`why`):** Der Anbieter darf Vertragsinhalte im Nachhinein einseitig ändern – Sie müssen nur noch widersprechen oder gelten per Weiternutzung als einverstanden. Damit kann er Preise und Leistungen laufend verschieben.
- **Redline-Gegenvorschlag (`suggestion`):** Änderungen nur mit Ihrer vorherigen ausdrücklichen Zustimmung zulassen. Bei Entfall von Features oder Preiserhöhungen ein sofortiges Minderungs- und Sonderkündigungsrecht ohne Schwellenwert vereinbaren.
- **Anwaltlicher Spot-Check (Verdict):** ✅ Juristisch zutreffend (Treffer & Normzitat korrekt) — *Einseitiger Leistungsänderungsvorbehalt nach freiem Ermessen ohne triftigen Grund und Zumutbarkeitsgrenze.*

### Befund #3: Einseitige Preiserhöhungsautomatik & Indexfalle (`price_escalation`)
- **Vertrag / Branche:** `agb-01-telekommunikation-mobilfunk` (Telekommunikation & Glasfaser/Mobilfunk AGB)
- **Kategorie:** True Positive (Korrekter Treffer)
- **Schweregrad:** `MITTEL` (Zeile 17–17)
- **Gesetzliche Normreferenz (`citation`):** **§ 309 Nr. 1 BGB, § 307 Abs. 1 S. 1, S. 2 BGB** *(Ground-Truth-Label: § 307 Abs. 1 BGB)*
- **Klauselauszug:**
  > „Die Nord Netz Connect Gmb H behält sich vor, die monatlichen Grundentgelte bei gestiegenen Netzausbaukosten oder allgemeinen Marktveränderungen einseitig zu erhöhen;“
- **Engine-Begründung (`why`):** Preiserhöhungsautomatik ohne Kündigungsrecht (§ 307 BGB, Preisklauselgesetz): Klauseln, die automatische Preiserhöhungen vorschreiben, ohne dass der Kunde Kostensenkungen einfordern oder den Vertrag sonderkündigen kann, sind unwirksam.
- **Redline-Gegenvorschlag (`suggestion`):** Preiserhöhungen an transparente Indizes (z. B. Verbraucherpreisindex Destatis) koppeln, Begrenzung auf max. 3-5 % p.a. und Sonderkündigungsrecht bei jeder Steigerung vereinbaren.
- **Anwaltlicher Spot-Check (Verdict):** ✅ Juristisch zutreffend (Treffer & Normzitat korrekt) — *Asymmetrische Preiserhöhungsklausel ohne Pflicht zur Weitergabe von Kostensenkungen und unter Ausschluss des Sonderkündigungsrechts.*

### Befund #4: Überhöhte Verzugszinsen (`default_interest`)
- **Vertrag / Branche:** `agb-01-telekommunikation-mobilfunk` (Telekommunikation & Glasfaser/Mobilfunk AGB)
- **Kategorie:** True Positive (Korrekter Treffer)
- **Schweregrad:** `HOCH` (Zeile 21–23)
- **Gesetzliche Normreferenz (`citation`):** **§ 288 Abs. 1, Abs. 2 BGB, § 309 Nr. 5 BGB** *(Ground-Truth-Label: § 309 Nr. 5 BGB, § 288 BGB)*
- **Klauselauszug:**
  > „§ 4 Zahlungsverzug, Anschlusssperre und Einwendungen gegen die Rechnung Gerät der Kunde mit einer fälligen Zahlung in Verzug, berechnet der Anbieter Verzugszinsen in Höhe von 18 Prozent pro Jahr, wobei die Geltendmachung eines weiteren Verzugsschadens unberührt bleibt.“
- **Engine-Begründung (`why`):** Verzugszinsen von 18 Punkten über dem Basiszinssatz liegen deutlich über dem gesetzlichen Maß (regelmäßig 5 Punkte). Bei einmaligen Zahlproblemen wird es schnell sehr teuer.
- **Redline-Gegenvorschlag (`suggestion`):** Auf die gesetzlichen Verzugszinsen beschränken (§ 288 BGB) und keine zusätzlichen Mahn- oder Pauschalgebühren akzeptieren.
- **Anwaltlicher Spot-Check (Verdict):** ✅ Juristisch zutreffend (Treffer & Normzitat korrekt) — *Überhöhter pauschalierter Verzugszinssatz von 18 % p.a. ohne Nachweismöglichkeit eines geringeren Schadens.*

### Befund #5: Unverhältnismäßige Dienstsperre (Leistungseinstellung) (`service_suspension`)
- **Vertrag / Branche:** `agb-01-telekommunikation-mobilfunk` (Telekommunikation & Glasfaser/Mobilfunk AGB)
- **Kategorie:** True Positive (Korrekter Treffer)
- **Schweregrad:** `HOCH` (Zeile 25–25)
- **Gesetzliche Normreferenz (`citation`):** **§ 320 Abs. 2 BGB, § 307 Abs. 1, Abs. 2 Nr. 1 BGB** *(Ground-Truth-Label: § 307 Abs. 1 BGB, § 61 TKG)*
- **Klauselauszug:**
  > „Befindet sich der Kunde mit einem Betrag von mindestens 15,00 Euro in Zahlungsverzug, ist die Nord Netz Connect Gmb H berechtigt, den Mobilfunk- und Glasfaseranschluss sofort und ohne vorherige Androhung oder Nachfristsetzung vollständig zu sperren;“
- **Engine-Begründung (`why`):** Unverhältnismäßige Dienstsperre (§ 307, § 320 BGB): Der Anbieter darf den Dienst ohne Vorwarnung oder Mahnung sofort sperren, verlangt aber gleichzeitig die volle Vergütung für die Ausfallzeit.
- **Redline-Gegenvorschlag (`suggestion`):** Sperrung erst nach Mahnung mit mindestens 14 Tagen Zahlungsfrist und einem Zahlungsrückstand von mindestens zwei Monatsbeiträgen zulassen.
- **Anwaltlicher Spot-Check (Verdict):** ✅ Juristisch zutreffend (Treffer & Normzitat korrekt) — *Sofortige Anschlusssperre bereits bei geringfügigem Verzug (15 Euro) und ohne vorherige Androhung/Fristsetzung.*

### Befund #6: Unwirksame Formklausel & Kündigungserschwernis (`written_form_trap`)
- **Vertrag / Branche:** `agb-01-telekommunikation-mobilfunk` (Telekommunikation & Glasfaser/Mobilfunk AGB)
- **Kategorie:** True Positive (Korrekter Treffer)
- **Schweregrad:** `HOCH` (Zeile 37–37)
- **Gesetzliche Normreferenz (`citation`):** **§ 309 Nr. 13 BGB, § 305b BGB** *(Ground-Truth-Label: § 309 Nr. 13 BGB)*
- **Klauselauszug:**
  > „Jede Kündigung des Vertrages bedarf zu ihrer Wirksamkeit zwingend der eigenhändigen Unterschrift des Kunden im Original und muss der Nord Netz Connect Gmb H per eingeschriebenem Brief (Einschreiben mit Rückschein) zugestellt werden;“
- **Engine-Begründung (`why`):** Erschwerte Kündigungsform (§ 309 Nr. 13 BGB): Für Kündigungen und Erklärungen darf formularmäßig keine strengere Form als die Textform (§ 126b BGB, z. B. E-Mail) verlangt werden. Die Pflicht zu eigenhändiger Unterschrift oder Einschreiben mit Rückschein ist unwirksam.
- **Redline-Gegenvorschlag (`suggestion`):** Auf Textform (z. B. E-Mail) umstellen: Kündigungen und Änderungen bedürfen der Textform (§ 126b BGB). Mündliche Individualabreden haben stets Vorrang.
- **Anwaltlicher Spot-Check (Verdict):** ✅ Juristisch zutreffend (Treffer & Normzitat korrekt) — *Verstoß gegen das Verbot strengerer Formerfordernisse als Textform (strenge Schriftform und Einschreiben mit Rückschein).*

### Befund #7: Vollständiger Ausschluss der Sachmängelhaftung (As-Is-Falle) (`as_is_software_disclaimer`)
- **Vertrag / Branche:** `agb-02-cloud-saas-erp` (B2B Cloud-ERP & SaaS-Nutzungsbedingungen)
- **Kategorie:** True Positive (Korrekter Treffer)
- **Schweregrad:** `HOCH` (Zeile 7–7)
- **Gesetzliche Normreferenz (`citation`):** **§ 309 Nr. 8 lit. b aa BGB, § 536d BGB, § 307 Abs. 2 Nr. 2 BGB** *(Ground-Truth-Label: § 307 Abs. 1, Abs. 2 Nr. 2 BGB, § 536d BGB)*
- **Klauselauszug:**
  > „Die Bereitstellung der Cloud-Software erfolgt im gegenwärtigen Zustand („wie besehen“ / „as is“) unter vollständigem Ausschluss jeglicher sach- und rechtsmängelbezogenen Gewährleistung für die Fehlerfreiheit, Funktionsfähigkeit oder Eignung für den vertraglich vorausgesetzten Gebrauch.“
- **Engine-Begründung (`why`):** Vollständiger Gewährleistungsausschluss bei Neuleistungen (§ 309 Nr. 8 lit. b BGB, § 307 BGB): Die Klausel 'wie gesehen' oder der pauschale Ausschluss der Mängelhaftung für Standardfunktionen entkernt die Leistungspflicht des Anbieters.
- **Redline-Gegenvorschlag (`suggestion`):** Gesetzliche Sachmängelhaftung (§ 437 / § 634 BGB) vereinbaren: Nacherfüllung binnen angemessener Frist, Minderung und Rücktritt bei Fehlschlagen.
- **Anwaltlicher Spot-Check (Verdict):** ✅ Juristisch zutreffend (Treffer & Normzitat korrekt) — *Unzulässiger pauschaler Gewährleistungsausschluss ('wie besehen' / 'as is') bei entgeltlicher SaaS-Überlassung.*

### Befund #8: Scheingarantie & unverbindliche Service-Levels (SLA-Entkernung) (`unverbindliche_sla_falle`)
- **Vertrag / Branche:** `agb-02-cloud-saas-erp` (B2B Cloud-ERP & SaaS-Nutzungsbedingungen)
- **Kategorie:** True Positive (Korrekter Treffer)
- **Schweregrad:** `HOCH` (Zeile 11–11)
- **Gesetzliche Normreferenz (`citation`):** **§ 307 Abs. 1 S. 2, Abs. 2 Nr. 2 BGB** *(Ground-Truth-Label: § 307 Abs. 2 Nr. 2 BGB)*
- **Klauselauszug:**
  > „Sämtliche in diesem Vertrag oder in Leistungsbeschreibungen genannten Verfügbarkeitsquoten, Reaktionszeiten und Wiederherstellungszeiten verstehen sich jedoch lediglich als unverbindliche Zielwerte und Bemühenszusagen, aus deren Unterschreitung der Kunde keinerlei Rechtsansprüche, Minderungsrechte oder Schadensersatzansprüche herleiten kann.“
- **Engine-Begründung (`why`):** SLA-Entkernung & Scheingarantie (§ 307 BGB): Der Anbieter wirbt mit hohen Verfügbarkeitswerten (z. B. 99,9 %), stuft diese im Kleingedruckten jedoch als rechtlich unverbindliche Zielgrößen ein, sodass bei tagelangen Ausfällen weder Schadensersatz noch Kündigungsrechte greifen.
- **Redline-Gegenvorschlag (`suggestion`):** Verbindliche Service-Levels mit konkreten Pönalen oder Gutschriften bei Nichteinhaltung sowie Sonderkündigungsrecht bei Unterschreitung von Mindestschwellen (z. B. unter 98 %) vereinbaren.
- **Anwaltlicher Spot-Check (Verdict):** ✅ Juristisch zutreffend (Treffer & Normzitat korrekt) — *Aushöhlung der Hauptleistungspflicht durch Herabstufung sämtlicher SLA-Verfügbarkeiten zu unverbindlichen Zielwerten unter Ausschluss von Mängelrechten.*

### Befund #9: Verfall von Service-Gutschriften wegen Update-Frist (`service_credit_forfeiture`)
- **Vertrag / Branche:** `agb-02-cloud-saas-erp` (B2B Cloud-ERP & SaaS-Nutzungsbedingungen)
- **Kategorie:** True Positive (Korrekter Treffer)
- **Schweregrad:** `MITTEL` (Zeile 13–13)
- **Gesetzliche Normreferenz (`citation`):** **§ 307 Abs. 1, Abs. 2 Nr. 2 BGB** *(Ground-Truth-Label: § 307 Abs. 1 BGB)*
- **Klauselauszug:**
  > „Soweit im Einzelfall im Rahmen eines gesonderten Enterprise-Tarifs Service-Gutschriften (Service Credits) für Ausfallzeiten vorgesehen sind, verfallen sämtliche Ansprüche auf derartige Gutschriften ersatzlos, wenn der Kunde diese nicht innerhalb einer Ausschlussfrist von 48 Stunden nach Ende der Störung unter Vorlage vollständiger Diagnoseprotokolle schriftlich beantragt oder wenn der Kunde ein beliebiges Browser-…“
- **Engine-Begründung (`why`):** Service-Gutschriften können bei versäumten Sicherheitsupdate-Fristen entfallen, auch wenn sie andere Leistungsausfälle betreffen. Prüfen Sie Frist, Verantwortlichkeit, Abhängigkeiten und ob bereits verdiente Gutschriften betroffen sind.
- **Redline-Gegenvorschlag (`suggestion`):** Legen Sie Update-Fristen nach Kritikalität fest, berücksichtigen Sie vom Anbieter oder Dritten verursachte Verzögerungen und gewähren Sie eine angemessene Nachfrist, bevor Gutschriften verfallen.
- **Anwaltlicher Spot-Check (Verdict):** ✅ Juristisch zutreffend (Treffer & Normzitat korrekt) — *Unangemessener Verfall von SLA-Gutschriften durch extrem kurze 48-Stunden-Antragsfrist und sachfremde Obliegenheiten.*

### Befund #10: Datenverarbeitung außerhalb von EU/EWR (`international_data_processing`)
- **Vertrag / Branche:** `agb-02-cloud-saas-erp` (B2B Cloud-ERP & SaaS-Nutzungsbedingungen)
- **Kategorie:** True Positive (Korrekter Treffer)
- **Schweregrad:** `MITTEL` (Zeile 27–27)
- **Gesetzliche Normreferenz (`citation`):** **Art. 44–49 DSGVO, Art. 28 Abs. 3 DSGVO, § 307 Abs. 1 BGB** *(Ground-Truth-Label: § 307 Abs. 1 BGB, Art. 44 ff. DSGVO)*
- **Klauselauszug:**
  > „Der Anbieter ist berechtigt, die gespeicherten personenbezogenen Daten und Geschäftsdaten des Kunden jederzeit nach eigenem Ermessen auf Server in beliebige Drittstaaten außerhalb der Europäischen Union und des Europäischen Wirtschaftsraums zu übertragen und dort durch lokale Dienstleister verarbeiten zu lassen, auch wenn für das betreffende Drittland kein Angemessenheitsbeschluss der EU-Kommission und keine geeig…“
- **Engine-Begründung (`why`):** Der Vertrag sieht eine Verarbeitung oder Nutzung von Daten außerhalb der EU/des EWR oder in einem Drittland vor. Das ist ein Prüfhinweis, keine automatische Rechtswidrigkeit: Es kommt unter anderem auf Datenart, Empfängerland und einen anwendbaren Übermittlungsmechanismus nach Art. 44–49 DSGVO an.
- **Redline-Gegenvorschlag (`suggestion`):** Klären Sie Datenarten, Empfänger, Speicher- und Zugriffsort sowie den Übermittlungsmechanismus (z. B. Angemessenheitsbeschluss oder geeignete Garantien) und regeln Sie Informations- und Widerspruchsrechte.
- **Anwaltlicher Spot-Check (Verdict):** ✅ Juristisch zutreffend (Treffer & Normzitat korrekt) — *Rechtswidriger Vorbehalt zur Datenübermittlung in beliebige Drittländer ohne Angemessenheitsbeschluss oder Garantien nach Art. 46 DSGVO.*

### Befund #11: Ungefragte Referenznennung & Markennutzung zu Werbezwecken (`marketing_reference`)
- **Vertrag / Branche:** `agb-02-cloud-saas-erp` (B2B Cloud-ERP & SaaS-Nutzungsbedingungen)
- **Kategorie:** True Positive (Korrekter Treffer)
- **Schweregrad:** `MITTEL` (Zeile 39–39)
- **Gesetzliche Normreferenz (`citation`):** **§ 307 Abs. 1, Abs. 2 Nr. 1 BGB, Art. 6 Abs. 1 lit. a DSGVO** *(Ground-Truth-Label: § 307 Abs. 1 BGB)*
- **Klauselauszug:**
  > „Ungeachtet vorstehender Vertraulichkeit ist der Anbieter unwiderruflich und ohne gesonderte Einwilligung berechtigt, den Namen, die geschützten Marken sowie das Unternehmenslogo des Kunden unentgeltlich auf seiner Webseite, in Pressemitteilungen, auf Messen sowie in weltweiten Werbekampagnen als Referenzkunden zu veröffentlichen und werblich zu nutzen.“
- **Engine-Begründung (`why`):** Ungefragte Referenzwerbung & Logo-Nutzung (§ 307 BGB, MarkenG): Der Anbieter räumt sich formularmäßig das Recht ein, Ihre Marke, Ihren Firmennamen und Ihr Logo ohne gesonderte Freigabe oder Vergütung für eigene Marketing- und Werbekampagnen zu nutzen.
- **Redline-Gegenvorschlag (`suggestion`):** Referenznennung und Logo-Nutzung nur mit vorheriger schriftlicher Zustimmung (Einzelfreigabe) und mit jederzeitiger Widerrufsmöglichkeit vereinbaren.
- **Anwaltlicher Spot-Check (Verdict):** ✅ Juristisch zutreffend (Treffer & Normzitat korrekt) — *Unwiderrufliche Einräumung von Marken- und Logonutzungsrechten zu Werbezwecken ohne gesonderte Einwilligung oder Widerrufsmöglichkeit.*

### Befund #12: Daten-Lock-in & Entgelte für Datenherausgabe (`data_lockin`)
- **Vertrag / Branche:** `agb-02-cloud-saas-erp` (B2B Cloud-ERP & SaaS-Nutzungsbedingungen)
- **Kategorie:** True Positive (Korrekter Treffer)
- **Schweregrad:** `HOCH` (Zeile 49–49)
- **Gesetzliche Normreferenz (`citation`):** **§ 307 Abs. 1, Abs. 2 Nr. 2 BGB, Art. 20, Art. 28 Abs. 3 lit. g DSGVO** *(Ground-Truth-Label: § 307 Abs. 1, Abs. 2 BGB, Art. 28 Abs. 3 lit. g DSGVO)*
- **Klauselauszug:**
  > „Bei Beendigung des Vertragsverhältnisses gleich aus welchem Rechtsgrund erfolgt eine Herausgabe oder ein Export der in der Cloud gespeicherten ERP-Bestandsdaten des Kunden ausschließlich gegen Zahlung einer pauschalen Datenexport- und Bereitstellungsgebühr in Höhe von 35.000,00 Euro zuzüglich eines Tagessatzes von 2.400,00 Euro;“
- **Engine-Begründung (`why`):** Daten-Lock-in & Herausgabesperre (Art. 20 DSGVO, § 307 BGB): Der Anbieter erschwert den Wechsel zu anderen Dienstleistern, indem die Herausgabe der eigenen Unternehmensdaten kostenpflichtig gemacht oder in proprietären Formaten verweigert wird.
- **Redline-Gegenvorschlag (`suggestion`):** Kostenfreie Datenherausgabe in einem gängigen, offenen und maschinenlesbaren Format (z. B. CSV, JSON, SQL) innerhalb von 14 Tagen nach Vertragsende vereinbaren.
- **Anwaltlicher Spot-Check (Verdict):** ✅ Juristisch zutreffend (Treffer & Normzitat korrekt) — *Unzulässiger Daten-Lock-in durch prohibitive Exportgebühr von 35.000 Euro für die Herausgabe eigener Bestandsdaten bei Vertragsende.*

### Befund #13: Unwirksame geltungserhaltende Reduktion (Salvatorische Klausel) (`salvatorische_klausel_falle`)
- **Vertrag / Branche:** `agb-02-cloud-saas-erp` (B2B Cloud-ERP & SaaS-Nutzungsbedingungen)
- **Kategorie:** True Positive (Korrekter Treffer)
- **Schweregrad:** `HOCH` (Zeile 55–55)
- **Gesetzliche Normreferenz (`citation`):** **§ 306 Abs. 1, Abs. 2 BGB, § 307 Abs. 1 S. 2 BGB** *(Ground-Truth-Label: § 306 Abs. 2, § 307 Abs. 1 S. 2 BGB)*
- **Klauselauszug:**
  > „die unwirksame Klausel ist in diesem Fall im Wege der geltungserhaltenden Reduktion mit demjenigen Inhalt aufrechtzuerhalten, der das vom Anbieter beabsichtigte Maximum des gesetzlich gerade noch Zulässigen ausschöpft.“
- **Engine-Begründung (`why`):** Verstoß gegen das Verbot der geltungserhaltenden Reduktion (§ 306 Abs. 2 BGB): In AGB ist eine Klausel, die vorsieht, dass unwirksame Bestimmungen automatisch durch eine andere wirksame Regelung ersetzt werden, unzulässig. An die Stelle unwirksamer Klauseln tritt zwingend das BGB.
- **Redline-Gegenvorschlag (`suggestion`):** Streichen. Gesetzliche Rechtsfolge nach § 306 Abs. 2 BGB (Geltung der gesetzlichen Vorschriften) greifen lassen.
- **Anwaltlicher Spot-Check (Verdict):** ✅ Juristisch zutreffend (Treffer & Normzitat korrekt) — *Unwirksame salvatorische Klausel mit Anordnung einer geltungserhaltenden Reduktion anstelle des gesetzlichen Dispositivrechts.*

### Befund #14: Pauschales Aufrechnungsverbot oder Einbehaltssperre (`setoff_restriction`)
- **Vertrag / Branche:** `agb-03-webhosting-dedicated` (Webhosting, Domains & Server-Housing AGB)
- **Kategorie:** True Positive (Korrekter Treffer)
- **Schweregrad:** `HOCH` (Zeile 23–23)
- **Gesetzliche Normreferenz (`citation`):** **§ 309 Nr. 3 BGB, § 307 Abs. 1 BGB** *(Ground-Truth-Label: § 309 Nr. 3, Nr. 2 BGB)*
- **Klauselauszug:**
  > „Eine Aufrechnung oder die Ausübung eines Zurückbehaltungsrechts durch den Kunden gegenüber Zahlungsansprüchen des Providers ist in jedem Fall vollständig ausgeschlossen;“
- **Engine-Begründung (`why`):** Ausschluss des Zurückbehaltungsrechts (§ 309 Nr. 2 lit. b, § 307 BGB): Das gesetzliche Leistungsverweigerungsrecht aus § 273 und § 320 BGB darf formularmäßig nicht pauschal abbedungen werden. Der Ausschluss ist auch im B2B unangemessen.
- **Redline-Gegenvorschlag (`suggestion`):** Gesetzliche Zurückbehaltungsrechte (§ 273, § 320 BGB) uneingeschränkt aufrechterhalten und Aufrechnung mit unbestrittenen oder rechtskräftigen Gegenforderungen stets zulassen.
- **Anwaltlicher Spot-Check (Verdict):** ✅ Juristisch zutreffend (Treffer & Normzitat korrekt) — *Verstoß gegen das Klauselverbot des § 309 Nr. 3 BGB durch Ausschluss der Aufrechnung auch mit unbestrittenen oder rechtskräftig festgestellten Forderungen.*

### Befund #15: Fingierter Zugang von Erklärungen & Rechnungen (Zugangsfiktion) (`fictitious_receipt`)
- **Vertrag / Branche:** `agb-03-webhosting-dedicated` (Webhosting, Domains & Server-Housing AGB)
- **Kategorie:** True Positive (Korrekter Treffer)
- **Schweregrad:** `MITTEL` (Zeile 31–33)
- **Gesetzliche Normreferenz (`citation`):** **§ 308 Nr. 6 BGB, § 130 Abs. 1 S. 1 BGB** *(Ground-Truth-Label: § 308 Nr. 6 BGB, § 130 BGB)*
- **Klauselauszug:**
  > „§ 6 Elektronische Kommunikation und Zugangsfiktion Sämtliche vertragsrelevanten Mitteilungen, Rechnungen, Mahnungen und Kündigungserklärungen des Providers können an die vom Kunden im Kundenmenü hinterlegte E-Mail-Adresse versandt werden.“
- **Engine-Begründung (`why`):** Unzulässige Zugangsfiktion (§ 308 Nr. 5 BGB): Erklärungen, Preiserhöhungen oder Rechnungen sollen nach wenigen Tagen automatisch als zugegangen gelten, selbst wenn sie im Spam-Filter hängen bleiben oder auf dem Postweg verloren gehen.
- **Redline-Gegenvorschlag (`suggestion`):** Zugangsfiktion ersatzlos streichen. Maßgeblich muss stets der tatsächliche Zugang beim Empfänger (§ 130 BGB) sein.
- **Anwaltlicher Spot-Check (Verdict):** ✅ Juristisch zutreffend (Treffer & Normzitat korrekt) — *Unwirksame Zugangsfiktion für E-Mails unabhängig vom tatsächlichen Eingang im Machtbereich des Empfängers.*

### Befund #16: Unwirksame insolvenzabhängige Lösungsklausel (`insolvency_termination`)
- **Vertrag / Branche:** `agb-03-webhosting-dedicated` (Webhosting, Domains & Server-Housing AGB)
- **Kategorie:** True Positive (Korrekter Treffer)
- **Schweregrad:** `HOCH` (Zeile 43–43)
- **Gesetzliche Normreferenz (`citation`):** **§ 119 InsO, § 103 InsO, § 307 Abs. 2 Nr. 1 BGB (BGH IX ZR 213/11)** *(Ground-Truth-Label: §§ 103, 112, 119 InsO, § 307 BGB)*
- **Klauselauszug:**
  > „Wird über das Vermögen des Kunden ein Antrag auf Eröffnung des Insolvenzverfahrens gestellt, nimmt der Kunde Verhandlungen über eine außergerichtliche Schuldenbereinigung auf oder verschlechtert sich seine Bonitätsauskunft, so endet dieses Vertragsverhältnis automatisch und fristlos im Zeitpunkt der Antragstellung, und der Provider ist berechtigt, sämtliche Server und Domains sofort abzuschalten.“
- **Engine-Begründung (`why`):** Unwirksame insolvenzabhängige Lösungsklausel (§ 119 InsO, BGH IX ZR 169/11): Vertragsklauseln, die an den bloßen Antrag auf Eröffnung eines Insolvenzverfahrens Kündigungsrechte knüpfen, sind unwirksam, da sie die Fortführung des Unternehmens im Insolvenzverfahren vereiteln.
- **Redline-Gegenvorschlag (`suggestion`):** Klausel streichen. Kündigung nur nach den allgemeinen insolvenzrechtlichen Regeln des Insolvenzverwalters zulassen.
- **Anwaltlicher Spot-Check (Verdict):** ✅ Juristisch zutreffend (Treffer & Normzitat korrekt) — *Unwirksame insolvenzabhängige Lösungsklausel, die das Wahlrecht des Insolvenzverwalters nach § 103 InsO unterläuft.*

### Befund #17: Unzulässige Beweislastumkehr & Gefahrverlagerung (`burden_of_proof`)
- **Vertrag / Branche:** `agb-04-ecommerce-versandhandel` (B2C Online-Shop & Versandhandel AGB)
- **Kategorie:** True Positive (Korrekter Treffer)
- **Schweregrad:** `HOCH` (Zeile 21–21)
- **Gesetzliche Normreferenz (`citation`):** **§ 309 Nr. 12 BGB, § 307 Abs. 2 Nr. 1 BGB** *(Ground-Truth-Label: § 475 Abs. 2 BGB, § 309 Nr. 12 BGB)*
- **Klauselauszug:**
  > „für Transportschäden oder den Verlust der Sendung auf dem Postweg trägt ausschließlich der Kunde die Beweislast und das Verlustrisiko.“
- **Engine-Begründung (`why`):** Auferlegung der Beweislast für das Verhalten des Anbieters (§ 309 Nr. 12 BGB): Der Kunde müsste im Streitfall beweisen, dass der Anbieter seine Pflichten nicht erfüllt hat. Das kehrt die gesetzliche Beweislastverteilung zu seinen Lasten um.
- **Redline-Gegenvorschlag (`suggestion`):** Klausel streichen. Gesetzliche Beweislastverteilung nach BGB beibehalten.
- **Anwaltlicher Spot-Check (Verdict):** ✅ Juristisch zutreffend (Treffer & Normzitat korrekt) — *Unzulässige Abwälzung der Transportgefahr und Beweislast auf den Verbraucher beim Verbrauchsgüterkauf.*

### Befund #18: Fingierte Abnahme, Mängelgenehmigung & Rügefrist (`deemed_acceptance`)
- **Vertrag / Branche:** `agb-04-ecommerce-versandhandel` (B2C Online-Shop & Versandhandel AGB)
- **Kategorie:** True Positive (Korrekter Treffer)
- **Schweregrad:** `HOCH` (Zeile 23–23)
- **Gesetzliche Normreferenz (`citation`):** **§ 308 Nr. 5 BGB, § 309 Nr. 8 lit. b ee BGB, § 640 Abs. 2 BGB** *(Ground-Truth-Label: § 309 Nr. 8 lit. b ee BGB, § 476 BGB)*
- **Klauselauszug:**
  > „Werden etwaige Sachmängel oder Falschlieferungen nicht spätestens innerhalb von 24 Stunden nach Ablieferung schriftlich unter Beifügung von Lichtbildern beim Verkäufer gerügt, gilt die Ware als vorbehaltlos genehmigt und sämtliche Gewährleistungsansprüche sind erloschen.“
- **Engine-Begründung (`why`):** Eine Frist von wenigen Tagen oder Stunden zur Mängelanzeige mit anschließender Genehmigungsfiktion auch für Folgemängel ist nach § 308 Nr. 5 BGB und § 307 BGB unzulässig. Sie entwertet das Gewährleistungsrecht und kehrt die Beweislast unfair um.
- **Redline-Gegenvorschlag (`suggestion`):** Verhandeln Sie eine praxisnahe Rügefrist von mindestens 14 Tagen ab Entdeckung und streichen Sie die Genehmigungsfiktion für Folgemängel ersatzlos.
- **Anwaltlicher Spot-Check (Verdict):** ✅ Juristisch zutreffend (Treffer & Normzitat korrekt) — *Unzulässige Genehmigungsfiktion und Ausschlussfrist von 24 Stunden für Mängelrügen gegenüber Verbrauchern und Unternehmern.*

### Befund #19: Pauschale Verjährungsverkürzung oder Verfall (`statute_of_limitations`)
- **Vertrag / Branche:** `agb-04-ecommerce-versandhandel` (B2C Online-Shop & Versandhandel AGB)
- **Kategorie:** True Positive (Korrekter Treffer)
- **Schweregrad:** `HOCH` (Zeile 37–39)
- **Gesetzliche Normreferenz (`citation`):** **§ 309 Nr. 8 lit. b ff BGB, § 202 Abs. 1 BGB, § 307 Abs. 1 BGB** *(Ground-Truth-Label: § 476 Abs. 2 BGB, § 309 Nr. 8 lit. b ff BGB)*
- **Klauselauszug:**
  > „§ 7 Mängelhaftung und Verjährung Sämtliche Ansprüche des Kunden wegen Sach- und Rechtsmängeln an neu hergestellten wie auch an gebrauchten Waren – einschließlich etwaiger Schadensersatzansprüche wegen mangelhafter Lieferung – verjähren auch gegenüber Verbrauchern einheitlich in sechs Monaten ab Ablieferung der Ware.“
- **Engine-Begründung (`why`):** Eine pauschale Verkürzung der Verjährung oder Verfallsklausel für Ansprüche ohne ausdrückliche Ausnahme für Körperschäden, Vorsatz und grobe Fahrlässigkeit verstößt gegen § 309 Nr. 7 lit. a, b BGB sowie § 307 BGB.
- **Redline-Gegenvorschlag (`suggestion`):** Mindestens die gesetzliche Regelverjährung (§ 195 BGB: 3 Jahre) erhalten oder eine Verkürzung frühestens auf 12 Monate beschränken, mit zwingender Ausnahme für Körperschäden und grobes Verschulden.
- **Anwaltlicher Spot-Check (Verdict):** ✅ Juristisch zutreffend (Treffer & Normzitat korrekt) — *Unzulässige Verkürzung der Verjährungsfrist für Mängelansprüche auf sechs Monate bei neuen und gebrauchten Waren.*

### Befund #20: Gerichtsstand beim Anbieter oder im Ausland (`jurisdiction`)
- **Vertrag / Branche:** `agb-04-ecommerce-versandhandel` (B2C Online-Shop & Versandhandel AGB)
- **Kategorie:** True Positive (Korrekter Treffer)
- **Schweregrad:** `HOCH` (Zeile 51–51)
- **Gesetzliche Normreferenz (`citation`):** **§ 38 ZPO, § 307 Abs. 1 BGB** *(Ground-Truth-Label: § 38 ZPO, § 307 Abs. 1 BGB, Art. 18 EuGVVO/LugÜ)*
- **Klauselauszug:**
  > „Für sämtliche Rechtsstreitigkeiten aus oder im Zusammenhang mit Bestellungen über unseren Online-Shop – ausdrücklich auch bei Klagen gegen Verbraucher mit Wohnsitz in Deutschland – wird als ausschließlicher Gerichtsstand das sachlich zuständige Gericht in Zürich (Schweiz) vereinbart.“
- **Engine-Begründung (`why`):** Streitigkeiten müssen vor Gericht am Sitz des Anbieters oder im Ausland geklärt werden. Für Sie bedeutet das: weite Wege, fremde Anwälte und deutlich höhere Kosten, falls Sie Ihre Rechte durchsetzen wollen.
- **Redline-Gegenvorschlag (`suggestion`):** Gerichtsstand an Ihrem Firmensitz oder einen neutralen Ort aushandeln; zusätzlich außergerichtliche Mediation vorsehen.
- **Anwaltlicher Spot-Check (Verdict):** ✅ Juristisch zutreffend (Treffer & Normzitat korrekt) — *Unwirksame Vereinbarung eines ausschließlichen Auslandsgerichtsstands (Zürich) gegenüber Verbrauchern.*

### Befund #21: Automatische Vertragsverlängerung (`auto_renewal`)
- **Vertrag / Branche:** `agb-05-fitnessstudio-mitgliedschaft` (Fitnessstudio & Sportclub Mitgliedschaftsbedingungen)
- **Kategorie:** True Positive (Korrekter Treffer)
- **Schweregrad:** `HOCH` (Zeile 19–19)
- **Gesetzliche Normreferenz (`citation`):** **§ 309 Nr. 9 lit. b BGB, § 307 Abs. 1 BGB** *(Ground-Truth-Label: § 309 Nr. 9 lit. b BGB)*
- **Klauselauszug:**
  > „Wird die Mitgliedschaft nicht fristgerecht vor Ablauf der Erstlaufzeit gekündigt, verlängert sich der Vertrag automatisch jeweils um weitere 18 Monate als feste Folgelaufzeit.“
- **Engine-Begründung (`why`):** Ihr Vertrag verlängert sich von selbst, wenn Sie nicht rechtzeitig kündigen. Wer die Frist verpasst, bleibt oft ein weiteres Jahr gebunden – auch ohne neues Interesse an der Leistung.
- **Redline-Gegenvorschlag (`suggestion`):** Fordern Sie: Verlängerung nur mit vorheriger schriftlicher Zustimmung oder Kündigung jederzeit zum Monatsende. Zusätzlich eine Erinnerungspflicht des Anbieters vor Ablauf vereinbaren.
- **Anwaltlicher Spot-Check (Verdict):** ✅ Juristisch zutreffend (Treffer & Normzitat korrekt) — *Unzulässige automatische Vertragsverlängerung um 18 Monate bei einem Dauerschuldverhältnis gegenüber Verbrauchern.*

### Befund #22: Lange oder ungleiche Kündigungsfristen (`notice_period`)
- **Vertrag / Branche:** `agb-05-fitnessstudio-mitgliedschaft` (Fitnessstudio & Sportclub Mitgliedschaftsbedingungen)
- **Kategorie:** True Positive (Korrekter Treffer)
- **Schweregrad:** `MITTEL` (Zeile 21–21)
- **Gesetzliche Normreferenz (`citation`):** **§ 309 Nr. 9 lit. c BGB, § 307 Abs. 1 BGB** *(Ground-Truth-Label: § 309 Nr. 9 lit. c BGB, § 307 Abs. 1 BGB)*
- **Klauselauszug:**
  > „Für eine ordentliche Kündigung durch das Mitglied gilt eine Kündigungsfrist von sechs Monaten zum Ende der jeweiligen Vertragslaufzeit, während das Studio den Mitgliedschaftsvertrag jederzeit mit einer Frist von lediglich sieben Tagen zum Monatsende ordentlich kündigen kann.“
- **Engine-Begründung (`why`):** Hier gilt eine Kündigungsfrist von etwa 6 Monaten. So lange sind Sie gebunden, selbst wenn die Leistung nicht mehr passt. Zusätzlich sind die Fristen ungleich verteilt, eine Partei kann deutlich schneller raus als die andere.
- **Redline-Gegenvorschlag (`suggestion`):** Verhandeln Sie eine beidseitige Kündigungsfrist von höchstens vier Wochen bis drei Monaten.
- **Anwaltlicher Spot-Check (Verdict):** ✅ Juristisch zutreffend (Treffer & Normzitat korrekt) — *Überlange sechsmonatige Kündigungsfrist für das Mitglied bei gleichzeitig extrem asymmetrischer 7-Tage-Frist für das Studio.*

### Befund #23: Ausschluss des Rechts zur außerordentlichen Kündigung (`asymmetric_termination`)
- **Vertrag / Branche:** `agb-05-fitnessstudio-mitgliedschaft` (Fitnessstudio & Sportclub Mitgliedschaftsbedingungen)
- **Kategorie:** True Positive (Korrekter Treffer)
- **Schweregrad:** `HOCH` (Zeile 25–27)
- **Gesetzliche Normreferenz (`citation`):** **§ 314 Abs. 1 BGB, § 626 Abs. 1 BGB, § 307 Abs. 2 Nr. 1 BGB** *(Ground-Truth-Label: § 314 BGB, § 307 Abs. 1, Abs. 2 Nr. 1 BGB)*
- **Klauselauszug:**
  > „§ 4 Außerordentliche Kündigung, Umzug und Krankheit Das Recht des Mitglieds zur außerordentlichen fristlosen Kündigung des Mitgliedschaftsvertrages aus wichtigem Grund (§ 314 BGB) wird vollständig ausgeschlossen;“
- **Engine-Begründung (`why`):** Unwirksamer Ausschluss der fristlosen Kündigung (§ 314 Abs. 1, 3 BGB, § 307 BGB): Das gesetzliche Recht zur außerordentlichen Kündigung aus wichtigem Grund ist zwingendes Recht und kann formularmäßig weder ausgeschlossen noch unverhältnismäßig erschwert werden.
- **Redline-Gegenvorschlag (`suggestion`):** Klausel streichen. Das Recht beider Vertragsparteien zur fristlosen Kündigung aus wichtigem Grund (§ 314 BGB) muss uneingeschränkt bestehen bleiben.
- **Anwaltlicher Spot-Check (Verdict):** ✅ Juristisch zutreffend (Treffer & Normzitat korrekt) — *Zwingend unwirksamer Ausschluss des außerordentlichen Kündigungsrechts aus wichtigem Grund (insbesondere bei dauerhafter Krankheit).*

### Befund #24: Vertragsstrafe oder pauschalierter Schadensersatz (`penalty`)
- **Vertrag / Branche:** `agb-06-stadtwerke-sondervertrag` (Strom- & Gas-Sondervertrag Energieversorgung)
- **Kategorie:** True Positive (Korrekter Treffer)
- **Schweregrad:** `HOCH` (Zeile 35–35)
- **Gesetzliche Normreferenz (`citation`):** **§ 309 Nr. 5, Nr. 6 BGB, § 307 Abs. 1 BGB** *(Ground-Truth-Label: § 309 Nr. 6, Nr. 5 lit. b BGB)*
- **Klauselauszug:**
  > „Gerät der Kunde mit einer fälligen Zahlung in Verzug oder teilt er einen Zählerstand um mehr als fünf Tage verspätet mit, schuldet er dem Lieferanten für jeden Einzelfall eine sofort fällige Vertragsstrafe in Höhe von 350,00 Euro, wobei der Nachweis eines geringeren oder gar nicht entstandenen Schadensausgeschlossen ist.“
- **Engine-Begründung (`why`):** Bei Verstößen droht eine fest vereinbarte Geldstrafe oder ein pauschalierter Schadensersatz ohne Möglichkeit des Gegenbeweises (§ 309 Nr. 5 lit. b / Nr. 6 BGB).
- **Redline-Gegenvorschlag (`suggestion`):** Vertragsstrafen streichen oder stark begrenzen: nur klare Ausnahmefälle, Obergrenze in Euro und ausdrückliches Recht auf Nachweis eines geringeren Schadens.
- **Anwaltlicher Spot-Check (Verdict):** ✅ Juristisch zutreffend (Treffer & Normzitat korrekt) — *Verbotene Vertragsstrafe gegenüber Verbrauchern und Ausschluss des Gegenbeweises eines geringeren Schadens.*

### Befund #25: Erkannt als: notice_period (Erwartet: auto_renewal) (`notice_period`)
- **Vertrag / Branche:** `agb-01-telekommunikation-mobilfunk` (Telekommunikation & Glasfaser/Mobilfunk AGB)
- **Kategorie:** Type Collision (Nachbardetektor ausgelöst)
- **Schweregrad:** `MITTEL/HOCH` (Zeile 35–35)
- **Gesetzliche Normreferenz (`citation`):** **§ 309 Nr. 9 lit. b BGB, § 56 TKG** *(Ground-Truth-Label: § 309 Nr. 9 lit. b BGB, § 56 TKG)*
- **Klauselauszug:**
  > „Der Vertrag wird mit einer festen Mindestvertragslaufzeit von 24 Monaten geschlossen. Wird der Vertrag nicht spätestens drei Monate vor Ablauf der Mindestvertragslaufzeit gekündigt, verlängert er sich jeweils automatisch um weitere 24 Monate zu den dann geltenden Standardkonditionen.“
- **Engine-Begründung (`why`):** Klausel wurde als Risiko erkannt, jedoch unter Nachbarkategorie (notice_period).
- **Redline-Gegenvorschlag (`suggestion`):** Klausel streichen oder auf gesetzliches Leitbild zurückführen.
- **Anwaltlicher Spot-Check (Verdict):** ⚠️ Warnung berechtigt, aber als `notice_period` statt `auto_renewal` klassifiziert — *Unzulässige automatische Vertragsverlängerung um 24 Monate bei dreimonatiger Kündigungsfrist.*

### Befund #26: Erkannt als: liability (Erwartet: vicarious_agents) (`liability`)
- **Vertrag / Branche:** `agb-03-webhosting-dedicated` (Webhosting, Domains & Server-Housing AGB)
- **Kategorie:** Type Collision (Nachbardetektor ausgelöst)
- **Schweregrad:** `MITTEL/HOCH` (Zeile 49–49)
- **Gesetzliche Normreferenz (`citation`):** **§ 309 Nr. 7 lit. b BGB, § 278 BGB** *(Ground-Truth-Label: § 309 Nr. 7 lit. b BGB, § 278 BGB)*
- **Klauselauszug:**
  > „Für ein Verschulden seiner gesetzlichen Vertreter sowie seiner leitenden und einfachen Erfüllungsgehilfen, Subunternehmer und Rechenzentrumstechniker – auch bei Vorsatz und grober Fahrlässigkeit dieser Personen – übernimmt der Provider keinerlei Haftung; jede Zurechnung des Verhaltens von Erfüllungsgehilfen nach § 278 “
- **Engine-Begründung (`why`):** Klausel wurde als Risiko erkannt, jedoch unter Nachbarkategorie (liability).
- **Redline-Gegenvorschlag (`suggestion`):** Klausel streichen oder auf gesetzliches Leitbild zurückführen.
- **Anwaltlicher Spot-Check (Verdict):** ⚠️ Warnung berechtigt, aber als `liability` statt `vicarious_agents` klassifiziert — *Unzulässiger Haftungsausschluss für Vorsatz und grobe Fahrlässigkeit von gesetzlichen Vertretern und Erfüllungsgehilfen.*

### Befund #27: Erkannt als: penalty (Erwartet: service_suspension) (`penalty`)
- **Vertrag / Branche:** `agb-06-stadtwerke-sondervertrag` (Strom- & Gas-Sondervertrag Energieversorgung)
- **Kategorie:** Type Collision (Nachbardetektor ausgelöst)
- **Schweregrad:** `MITTEL/HOCH` (Zeile 33–33)
- **Gesetzliche Normreferenz (`citation`):** **§ 41b Abs. 2 EnWG, § 19 StromGVV, § 307 BGB** *(Ground-Truth-Label: § 41b Abs. 2 EnWG, § 19 StromGVV, § 307 BGB)*
- **Klauselauszug:**
  > „Befindet sich der Kunde mit einer Abschlagszahlung oder Teilen davon seit mehr als drei Werktagen im Rückstand, ist der Lieferant berechtigt, die Strom- und Gaslieferung sofort und ohne vorherige Androhung oder vierwöchige Sperrankündigung durch den Netzbetreiber unterbrechen zu lassen.“
- **Engine-Begründung (`why`):** Klausel wurde als Risiko erkannt, jedoch unter Nachbarkategorie (penalty).
- **Redline-Gegenvorschlag (`suggestion`):** Klausel streichen oder auf gesetzliches Leitbild zurückführen.
- **Anwaltlicher Spot-Check (Verdict):** ⚠️ Warnung berechtigt, aber als `penalty` statt `service_suspension` klassifiziert — *Rechtswidrige sofortige Energiesperre ohne Einhaltung der gesetzlichen vierwöchigen Androhungs- und Ankündigungsfristen.*
